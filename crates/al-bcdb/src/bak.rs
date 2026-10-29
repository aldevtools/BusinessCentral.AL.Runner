//! The SQL Server backup path (port of `BakSource` and `TableReader.cs`): page map,
//! system catalog, IAM chains, storage decoding.
//!
//! Tables are read through their IAM chain, not through first_page/root_page: on the BC
//! demo backups both sysallocunits.pgfirst and the page-header object id can be stale (the
//! demo database was shrunk after load, which relocates pages without rewriting either).
//! The IAM bitmap is what SQL Server itself trusts and matched
//! sys.dm_db_database_page_allocations exactly (bcdb PROVENANCE.md "IAM pages").

use crate::catalog::{AllocUnit, Catalog, PhysColumn, read_page_ptr};
use crate::error::{Error, Result};
use crate::io::{ByteSource, le};
use crate::lob::LobReader;
use crate::mtf::PAGE_SIZE;
use crate::pagefile::{GAM_INTERVAL_EXTENTS, GAM_INTERVAL_PAGES, PAGES_PER_EXTENT, PageFile, header};
use crate::records::{self, CdScratch, FixedVar};
use crate::source::{BcSource, SourceTable};
use crate::types::{Cell, SysColumn, decode};
use crate::value::Value;
use std::collections::HashSet;

const BLOB_MAGIC: &[u8; 8] = b"ALBXBAK1";

/// A hash that notices truncation and flipped bytes; not a defence against an adversary (the
/// index lives in the user's own cache).
fn blob_hash(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ b.len() as u64;
    let mut chunks = b.chunks_exact(8);
    for c in &mut chunks {
        h = (h ^ u64::from_le_bytes(c.try_into().unwrap()))
            .wrapping_mul(0x0100_0000_01b3)
            .rotate_left(29);
    }
    for &x in chunks.remainder() {
        h = (h ^ u64::from(x)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn join_blob(map: &[u8], catalog: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(40 + map.len() + catalog.len());
    b.extend_from_slice(BLOB_MAGIC);
    b.extend_from_slice(&(map.len() as u64).to_le_bytes());
    b.extend_from_slice(&(catalog.len() as u64).to_le_bytes());
    b.extend_from_slice(&blob_hash(catalog).to_le_bytes());
    b.extend_from_slice(map);
    b.extend_from_slice(catalog);
    b
}

fn split_blob(b: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if b.get(..8)? != BLOB_MAGIC {
        return None;
    }
    let map_len = usize::try_from(u64::from_le_bytes(b.get(8..16)?.try_into().ok()?)).ok()?;
    let cat_len = usize::try_from(u64::from_le_bytes(b.get(16..24)?.try_into().ok()?)).ok()?;
    let hash = u64::from_le_bytes(b.get(24..32)?.try_into().ok()?);
    let map = b.get(32..32usize.checked_add(map_len)?)?;
    let cat = b.get(32 + map_len..32usize.checked_add(map_len)?.checked_add(cat_len)?)?;
    (32 + map_len + cat_len == b.len() && blob_hash(cat) == hash).then(|| (map.to_vec(), cat.to_vec()))
}

struct TableEntry {
    object_id: i32,
    rowset_id: i64,
    rows: i64,
    compression_level: u8,
}

pub struct BakSource {
    pf: PageFile,
    cat: Catalog,
    tables: Vec<SourceTable>,
    entries: Vec<TableEntry>,
}

/// The decode plan for one read: the rowset's physical layout, each wanted column bound
/// to its output slot.
struct Plan {
    /// Physical (null-bit) order. `out` is the output slot of a wanted column.
    entries: Vec<(PhysColumn, Option<usize>)>,
}

impl BakSource {
    pub fn open(src: Box<dyn ByteSource>) -> Result<BakSource> {
        Self::from_page_file(PageFile::open(src)?)
    }

    /// [`open`](BakSource::open), keeping what opening costs in `store` between runs: the page
    /// map (a check of every extent of the file) and the catalog (a scan of the system tables).
    /// With both kept, an open reads the container's framing and nothing else. Bytes that do not
    /// fit the file are ignored and replaced.
    pub fn open_indexed(src: Box<dyn ByteSource>, store: &dyn crate::pagefile::IndexStore) -> Result<BakSource> {
        use std::sync::Mutex;
        // The page map saves through this adapter, which keeps the bytes for the combined blob.
        struct Section {
            load: Option<Vec<u8>>,
            saved: Mutex<Option<Vec<u8>>>,
        }
        impl crate::pagefile::IndexStore for Section {
            fn load(&self) -> Option<Vec<u8>> {
                self.load.clone()
            }
            fn save(&self, bytes: &[u8]) {
                *self.saved.lock().unwrap() = Some(bytes.to_vec());
            }
        }
        let (map_bytes, cat_bytes) = match store.load().and_then(|b| split_blob(&b)) {
            Some((m, c)) => (Some(m), Some(c)),
            None => (None, None),
        };
        let section = Section {
            load: map_bytes,
            saved: Mutex::new(None),
        };
        let pf = PageFile::open_indexed(src, &section)?;
        let map_rebuilt = section.saved.lock().unwrap().is_some();
        // A rebuilt map means the file is not the one the catalog bytes were made from.
        let restored = if map_rebuilt {
            None
        } else {
            cat_bytes.and_then(|b| Catalog::from_index_bytes(&b))
        };
        let (mut cat, fresh) = match restored {
            Some(c) => (c, false),
            None => (Catalog::open(&pf)?, true),
        };
        // The sequences are kept with the catalog: every session of a run with test data asks for them. An index saved
        // before they were kept gains them, saved once more.
        let had_sequences = cat.has_kept_sequences();
        let gained_sequences = cat.keep_sequences(&pf) && !had_sequences;
        let this = Self::assemble(pf, cat)?;
        if fresh || gained_sequences {
            // The map was rebuilt and sits in `saved`, or it was restored and is still in `load`.
            let map = match section.saved.into_inner().unwrap().or(section.load) {
                Some(m) => m,
                None => return Ok(this),
            };
            store.save(&join_blob(&map, &this.cat.to_index_bytes()));
        }
        Ok(this)
    }

    fn from_page_file(pf: PageFile) -> Result<BakSource> {
        let cat = Catalog::open(&pf)?;
        Self::assemble(pf, cat)
    }

    fn assemble(pf: PageFile, cat: Catalog) -> Result<BakSource> {
        let mut tables = Vec::new();
        let mut entries = Vec::new();
        for o in &cat.objects {
            let Ok(rs) = cat.rowset_for(o.object_id, &[1, 0]) else {
                continue;
            };
            tables.push(SourceTable {
                name: o.name.clone(),
                compression: match rs.compression_level {
                    0 => "none".into(),
                    1 => "row".into(),
                    2 => "page".into(),
                    x => x.to_string(),
                },
                handle: entries.len(),
            });
            entries.push(TableEntry {
                object_id: o.object_id,
                rowset_id: rs.rowset_id,
                rows: rs.rows,
                compression_level: rs.compression_level,
            });
        }
        Ok(BakSource {
            pf,
            cat,
            tables,
            entries,
        })
    }

    /// The underlying page file, for the page-map commands only a backup has.
    pub fn page_file(&self) -> &PageFile {
        &self.pf
    }

    pub fn catalog(&self) -> &Catalog {
        &self.cat
    }

    /// Calls `f(page id, page)` for every data page (type 1) of the allocation unit, in
    /// IAM-bitmap order. `f` returns false to stop; the result is false when it did.
    ///
    /// One IAM page per GAM interval, chained via the page header's next-page pointer.
    /// Slot 0 is the IAM header: start_pg (the GAM-interval base this IAM's bitmap is
    /// relative to) is a 6-byte page pointer at +40; eight 6-byte single-page allocation
    /// slots (mixed-extent pages) follow at +46 (bcdb PROVENANCE.md "IAM pages").
    pub fn for_each_data_page(&self, au: &AllocUnit, f: &mut dyn FnMut(i32, &[u8]) -> Result<bool>) -> Result<bool> {
        let pf = &self.pf;
        let (mut iam_pid, mut iam_fid) = au.first_iam_page;
        let mut seen: HashSet<(u16, i32)> = HashSet::new();
        while iam_pid != 0 && seen.insert((iam_fid, iam_pid)) {
            let iam = pf.page(iam_fid as i32, iam_pid)?;
            if header::page_type(&iam) != 10 {
                return Err(crate::invalid!(
                    "page {iam_fid}:{iam_pid} in the IAM chain is not an IAM page"
                ));
            }
            if header::slot_count(&iam) != 2 {
                return Err(Error::InvalidData("unexpected IAM slot count".into()));
            }
            let (s0, s1) = (header::slot_offset(&iam, 0), header::slot_offset(&iam, 1));
            if s0 < 96 || s0 + 94 > PAGE_SIZE || s1 < 96 || s1 + 4 > PAGE_SIZE {
                return Err(Error::InvalidData("IAM slot offsets out of range".into()));
            }
            let (base_pid, base_fid) = read_page_ptr(&iam, s0 + 40);
            if base_fid != 0 && base_fid != 1 {
                return Err(crate::unsupported!(
                    "IAM interval base in file {base_fid} — only single-data-file databases are supported"
                ));
            }
            if base_pid < 0 || base_pid as usize % GAM_INTERVAL_PAGES != 0 {
                return Err(crate::invalid!(
                    "IAM interval base {base_pid} is not a GAM-interval boundary"
                ));
            }
            for sp in 0..8 {
                let (sp_pid, sp_fid) = read_page_ptr(&iam, s0 + 46 + 6 * sp);
                if sp_pid == 0 && sp_fid == 0 {
                    continue;
                }
                if sp_fid != 1 {
                    return Err(crate::unsupported!(
                        "IAM single-page slot points into file {sp_fid} — only single-data-file databases are supported"
                    ));
                }
                if !pf.is_page_allocated(sp_pid)? {
                    continue;
                }
                let page = pf.page(1, sp_pid)?;
                if header::page_type(&page) == 1 && !f(sp_pid, &page)? {
                    return Ok(false);
                }
            }
            let bitmap_len = le::u16(&iam, s1 + 2) as usize;
            if s1 + 4 + bitmap_len > PAGE_SIZE {
                return Err(Error::InvalidData("IAM bitmap runs past the page".into()));
            }
            for b in 0..bitmap_len {
                let v = iam[s1 + 4 + b];
                if v == 0 {
                    continue;
                }
                for bit in 0..8 {
                    if v & (1 << bit) == 0 {
                        continue;
                    }
                    let extent = b * 8 + bit;
                    // The bitmap is 7992 bytes (63,936 bits) but a GAM interval is 63,904
                    // extents; bits in the 32-bit overhang are not extents (observed: bits
                    // 63920/63925/63928/63933 are set on every IAM of the demo databases).
                    if extent >= GAM_INTERVAL_EXTENTS {
                        continue;
                    }
                    let first = base_pid + (extent * PAGES_PER_EXTENT) as i32;
                    for pg in first..first + PAGES_PER_EXTENT as i32 {
                        // The IAM bit covers the whole extent; individual pages can be
                        // deallocated (and then hold a stale image). The PFS allocation
                        // bit is the per-page truth. A PFS-allocated page of a mapped
                        // extent is always present in the structural map.
                        if !pf.is_page_allocated(pg)? {
                            continue;
                        }
                        let page = pf.page(1, pg)?;
                        let pt = header::page_type(&page);
                        if pt == 1 {
                            if !f(pg, &page)? {
                                return Ok(false);
                            }
                        } else if !matches!(pt, 2 | 10 | 8 | 9 | 11 | 16 | 17) {
                            // 2/10: index and IAM pages of the same object share its
                            // extents. 8/9/11/16/17: GAM/SGAM/PFS/DCM/BCM pages recur at
                            // fixed intervals and can sit inside an extent the IAM claims
                            // (measured on a 23 GB production database). Never table data.
                            return Err(crate::invalid!("page 1:{pg} in a data extent has unexpected type {pt}"));
                        }
                    }
                }
            }
            let (np, nf) = header::next_page(&iam);
            iam_pid = np;
            iam_fid = nf;
        }
        Ok(true)
    }

    /// The physical leaf layout of the table's clustered rowset (sysrscols), joined to
    /// syscolpars for names. Physical (null-bit) order governs both the compressed (CD)
    /// column array and the FixedVar layout; on tables with ALTER history it differs from
    /// declaration order and contains dropped columns that still hold slots.
    fn plan(&self, e: &TableEntry, columns: &[SysColumn]) -> Result<Plan> {
        let meta = self.cat.load_column_metadata(&self.pf, e.object_id)?;
        let phys = self.cat.rowset_columns(&self.pf, e.rowset_id)?;
        let mut entries: Vec<(PhysColumn, Option<usize>)> = Vec::with_capacity(phys.len());
        let mut bound = vec![false; columns.len()];
        for (i, c) in columns.iter().enumerate() {
            if columns[..i].iter().any(|x| x.name == c.name) {
                return Err(Error::Argument(format!("column {} was asked for twice", c.name)));
            }
        }
        for pc in phys.iter() {
            // dropped, the uniquifier (col id 0: internal, valueless for us), and internal
            // physical columns (rscolid flag 0x08000000, e.g. the in-row version column
            // change tracking adds, whose masked low bits collide with a real column id):
            // they occupy an offset and a null bit but are no user column.
            if pc.dropped || pc.col_id == 0 || pc.internal {
                entries.push((pc.clone(), None));
                continue;
            }
            let Some(sc) = meta.columns.iter().find(|c| c.col_id == pc.col_id) else {
                return Err(crate::invalid!(
                    "rowset column {} (type {}) has no syscolpars entry — schema mismatch, refusing to guess",
                    pc.col_id,
                    crate::types::type_name(pc.xtype)
                ));
            };
            let out = columns.iter().position(|c| c.name == sc.name);
            if let Some(o) = out {
                if bound[o] {
                    return Err(Error::Argument(format!("column {} was asked for twice", sc.name)));
                }
                bound[o] = true;
            }
            entries.push((pc.clone(), out));
        }
        if let Some(i) = bound.iter().position(|b| !b) {
            return Err(crate::invalid!("table has no stored column {}", columns[i].name));
        }
        Ok(Plan { entries })
    }
}

/// An uncompressed row, decoded strictly by the sysrscols leaf layout: fixed columns at
/// their recorded offsets (bit columns by recorded bit position), variable columns by their
/// recorded ordinals. A column whose null bit lies beyond the record's column count was
/// added after the row was written and reads as NULL; a variable column beyond the record's
/// variable count is a trimmed trailing empty value.
fn decode_fixed_var(
    page: &[u8],
    slot: usize,
    plan: &Plan,
    columns: &[SysColumn],
    compressed: bool,
    lob: &LobReader<'_>,
    row: &mut [Value],
) -> Result<()> {
    let rec = FixedVar::parse(page, slot)?;
    let fx = rec.fixed;
    for (pc, out) in &plan.entries {
        let Some(o) = *out else { continue };
        let col = &columns[o];
        // a null bit beyond the record's column count: the column was added after this row was written
        let cell: Cell<'_> = if pc.null_bit as usize > rec.ncols || rec.is_null(pc.null_bit as usize - 1) {
            Cell::Null
        } else if pc.is_var() {
            let n = pc.var_ordinal();
            if n >= 1 && n <= rec.var_count() {
                let (b, complex) = rec.var(n - 1)?;
                if complex { Cell::Complex(b) } else { Cell::Value(b) }
            } else {
                Cell::Value(&[])
            }
        } else {
            let off = pc.leaf_offset as usize;
            let off = off.checked_sub(4).ok_or_else(|| {
                crate::invalid!(
                    "column {} has leaf offset {} inside the record header",
                    col.name,
                    pc.leaf_offset
                )
            })?;
            if pc.xtype == 104 {
                if off >= fx.len() {
                    return Err(crate::invalid!(
                        "bit column {} at offset {} beyond fixed data",
                        col.name,
                        pc.leaf_offset
                    ));
                }
                if (fx[off] >> pc.bit_pos) & 1 == 1 {
                    Cell::Value(&[1])
                } else {
                    Cell::Value(&[0])
                }
            } else {
                let width = pc.max_length.max(0) as usize;
                if off + width > fx.len() {
                    return Err(crate::invalid!(
                        "fixed data ends inside column {} ({} of {} bytes) — schema/record mismatch, refusing to guess",
                        col.name,
                        fx.len().saturating_sub(off),
                        width
                    ));
                }
                Cell::Value(&fx[off..off + width])
            }
        };
        row[o] = decode(cell, col, compressed, Some(lob), false)?;
    }
    Ok(())
}

impl BcSource for BakSource {
    fn last_row_version(&self) -> Result<Option<u64>> {
        crate::catalog::db_timestamp(&self.pf).map(Some)
    }

    fn sequences(&self) -> Result<Vec<crate::catalog::SqlSequence>> {
        self.cat.sequences(&self.pf)
    }

    fn tables(&self) -> Result<&[SourceTable]> {
        Ok(&self.tables)
    }

    fn row_count(&self, t: &SourceTable) -> Result<i64> {
        Ok(self.entries[t.handle].rows)
    }

    fn columns(&self, t: &SourceTable) -> Result<Vec<SysColumn>> {
        Ok(self
            .cat
            .load_column_metadata(&self.pf, self.entries[t.handle].object_id)?
            .columns
            .clone())
    }

    /// The clustered index key.
    fn row_key_columns(&self, t: &SourceTable) -> Result<Vec<String>> {
        let e = &self.entries[t.handle];
        let meta = self.cat.load_column_metadata(&self.pf, e.object_id)?;
        let mut idx: Vec<_> = meta.index_columns.iter().filter(|i| i.index_id == 1).collect();
        idx.sort_by_key(|i| i.key_ordinal);
        idx.iter()
            .map(|i| {
                meta.columns
                    .iter()
                    .find(|c| c.col_id == i.col_id)
                    .map(|c| c.name.clone())
                    .ok_or_else(|| {
                        crate::invalid!(
                            "clustered key column id {} of {} is not in syscolpars",
                            i.col_id,
                            t.name
                        )
                    })
            })
            .collect()
    }

    fn read_rows(
        &self,
        t: &SourceTable,
        columns: &[SysColumn],
        sink: &mut dyn FnMut(&[Value]) -> Result<bool>,
    ) -> Result<()> {
        let e = &self.entries[t.handle];
        let compressed = e.compression_level > 0;
        let au = self.cat.au_for_rowset(e.rowset_id)?;
        let plan = self.plan(e, columns)?;
        let lob = LobReader::new(&self.pf);
        let mut row = vec![Value::Null; columns.len()];
        let mut scratch = CdScratch::default();
        self.for_each_data_page(au, &mut |_pid, page| {
            let ci = if header::type_flag_bits(page) & 0x80 != 0 {
                Some(records::parse_ci(page)?)
            } else {
                None
            };
            for s in 0..header::checked_slot_count(page)? {
                let Some(so) = records::live_slot(header::slot_offset(page, s))? else {
                    continue;
                };
                // A slot array entry of 0 is an empty slot (deleted / never completed): SQL
                // Server's own scan skips it. Anything else below the 96-byte header is corruption.
                for v in row.iter_mut() {
                    *v = Value::Null;
                }
                if records::is_cd(page, so) {
                    if records::is_cd_ghost(page, so)? {
                        continue; // deleted, not yet cleaned up
                    }
                    records::parse_cd(page, so, ci.as_ref(), &mut scratch)?;
                    // A CD record written before columns were added carries fewer entries;
                    // they map onto the first entries of the physical order, and the missing
                    // trailing columns read as NULL.
                    if scratch.cells.len() > plan.entries.len() {
                        return Err(crate::invalid!(
                            "compressed record has {} columns, sysrscols says only {}",
                            scratch.cells.len(),
                            plan.entries.len()
                        ));
                    }
                    for (i, (_, out)) in plan.entries.iter().enumerate() {
                        let Some(o) = *out else { continue };
                        let cell = match scratch.cells.get(i) {
                            Some(loc) => loc.cell(page, &scratch.arena),
                            None => Cell::Null,
                        };
                        row[o] = decode(cell, &columns[o], compressed, Some(&lob), false)?;
                    }
                } else {
                    let rt = records::record_type(page, so);
                    if records::is_ghost_type(rt) {
                        continue;
                    }
                    if rt != 0 {
                        return Err(crate::unsupported!("record type {rt} not supported"));
                    }
                    decode_fixed_var(page, so, &plan, columns, compressed, &lob, &mut row)?;
                }
                if !sink(&row)? {
                    return Ok(false);
                }
            }
            Ok(true)
        })?;
        Ok(())
    }

    fn banner(&self) -> Result<String> {
        Ok(format!(
            "[{} objects, {} pages, {} pages superseded by the changed-extent re-read]",
            self.cat.total_object_count,
            self.pf.page_count(),
            self.pf.superseded_page_count()
        ))
    }
}
