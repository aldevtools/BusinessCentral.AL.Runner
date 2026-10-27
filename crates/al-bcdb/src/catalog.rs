//! Reads the SQL Server system catalog base tables directly from page images (port of
//! `Catalog.cs` and `ClusteredSeek.cs`).
//!
//! Bootstrap: the boot page (1:9) holds a 6-byte pointer to the first sysallocunits page
//! at page offset 612 (observed on BC 27.5 and 28.1, verified by walking the chain and
//! matching all rows against sys.system_internals_allocation_units, bcdb PROVENANCE.md
//! "Boot page"). Physical column layouts of sysallocunits/sysrowsets/sysschobjs/
//! syscolpars/sysiscols were validated row-for-row against sys.* views.
//!
//! Metadata is never trusted at face value (bcdb rule `stale-metadata`): first_page and
//! root_page are measurably stale in real files, so tables are read through their IAM
//! chain, never through those pointers. The clustered-index descent is used only for
//! catalog lookups and falls back to a scan whenever the index has an underived shape.

use crate::error::{Error, Result};
use crate::io::le;
use crate::mtf::PAGE_SIZE;
use crate::pagefile::{GAM_INTERVAL_EXTENTS, GAM_INTERVAL_PAGES, PAGES_PER_EXTENT, PageFile, header};
use crate::records::{FixedVar, is_ghost_type, live_slot, record_type};
use crate::types::SysColumn;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub const BOOT_PAGE_FIRST_SYSINDEXES_OFFSET: usize = 612;
/// Fixed partition id of sysrowsets itself.
const SYSROWSETS_ROWSET_ID: i64 = 5 << 16;
const SYSSCHOBJS_ID: i32 = 34;
const SYSCOLPARS_ID: i32 = 41;
const SYSISCOLS_ID: i32 = 55;
const SYSRSCOLS_ID: i32 = 3;

/// A page pointer: page id and file id.
pub type PagePtr = (i32, u16);

#[derive(Debug, Clone)]
pub struct AllocUnit {
    pub auid: i64,
    pub ty: u8,
    pub owner_id: i64,
    pub first_page: PagePtr,
    pub root_page: PagePtr,
    pub first_iam_page: PagePtr,
}

#[derive(Debug, Clone)]
pub struct RowSet {
    pub rowset_id: i64,
    pub id_major: i32,
    pub id_minor: i32,
    pub rows: i64,
    pub compression_level: u8,
}

/// A SQL sequence: its name as the database holds it, and its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlSequence {
    pub name: String,
    pub start: i64,
    pub increment: i64,
    pub current: i64,
}

#[derive(Debug, Clone)]
pub struct SysObject {
    pub object_id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysIndexCol {
    pub index_id: i32,
    pub key_ordinal: i32,
    pub col_id: i32,
}

/// The columns and index columns of one object.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ObjectMeta {
    pub columns: Vec<SysColumn>,
    pub index_columns: Vec<SysIndexCol>,
}

/// One column of a rowset's physical leaf layout, from sysrscols: the layout SQL Server
/// itself uses. Never derive layout from syscolpars order: on a database with ALTER
/// history, physical order, offsets and null-bit numbering all differ from declaration
/// order, and dropped columns keep their slots. Record layout (54-byte fixed part):
/// rsid u64@0, rscolid u32@8 (0x04000000 flag = dropped; 0x08000000 flag = internal, a
/// physical column that is no user column), hbcolid u32@12, ti u32@24 (low byte = system
/// type id; decimal prec@+8/scale@+16 bits, time/datetime2 scale@+8, string/binary max
/// length@+8, 0 = MAX), ordkey i16@32, status u32@36 (bit 0x02 = dropped), leaf offset
/// i16@40 (negative = variable-length ordinal), null bit u16@44, bit position u16@48.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysColumn {
    pub col_id: i32,
    pub dropped: bool,
    pub internal: bool,
    pub xtype: u8,
    pub max_length: i16,
    pub precision: u8,
    pub scale: u8,
    pub key_ordinal: i16,
    pub leaf_offset: i16,
    pub null_bit: u16,
    pub bit_pos: u16,
}

impl PhysColumn {
    pub fn is_var(&self) -> bool {
        self.leaf_offset < 0
    }
    pub fn var_ordinal(&self) -> usize {
        (-(self.leaf_offset as i32)) as usize
    }
}

pub struct Catalog {
    pub alloc_units: Vec<AllocUnit>,
    au_index: HashMap<(i64, u8), usize>,
    pub rowsets: Vec<RowSet>,
    rowset_index: HashMap<(i32, i32), usize>,
    /// User tables, in sysschobjs order.
    pub objects: Vec<SysObject>,
    /// Rows in sysschobjs, including the objects `objects` filters out.
    pub total_object_count: usize,
    /// Lookups answered by descending a clustered index rather than scanning it.
    pub clustered_seeks: AtomicUsize,
    /// Lookups that fell back to a scan because the index had an underived shape.
    pub clustered_seek_declines: AtomicUsize,
    meta: Mutex<HashMap<i32, Arc<ObjectMeta>>>,
    rowset_cols: Mutex<HashMap<i64, Arc<Vec<PhysColumn>>>>,
    au_pages: Mutex<HashMap<i64, Arc<HashSet<i32>>>>,
    /// The SQL sequences, when worked out at open ([`Catalog::keep_sequences`]) or kept in the index with the rest of
    /// the catalog: [`Catalog::sequences`] then reads no page.
    kept_sequences: Option<Vec<SqlSequence>>,
}

fn utf16(b: &[u8]) -> String {
    let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&units)
}

pub fn read_page_ptr(b: &[u8], off: usize) -> PagePtr {
    (le::i32(b, off), le::u16(b, off + 4))
}

/// Walk a page chain, calling `f(page, slot offset)` for every live non-ghost record.
fn walk_chain(pf: &PageFile, file_id: u16, page_id: i32, f: &mut dyn FnMut(&[u8], usize) -> Result<()>) -> Result<()> {
    let mut seen: HashSet<(u16, i32)> = HashSet::new();
    let (mut fid, mut pid) = (file_id, page_id);
    while pid != 0 && seen.insert((fid, pid)) {
        let p = pf.page(fid as i32, pid)?;
        for s in 0..header::checked_slot_count(&p)? {
            let Some(so) = live_slot(header::slot_offset(&p, s))? else {
                continue;
            };
            let rt = record_type(&p, so);
            if is_ghost_type(rt) {
                continue; // ghost records: deleted, not yet cleaned up
            }
            if rt != 0 {
                return Err(crate::unsupported!("record type {rt} not supported in catalog walk"));
            }
            f(&p, so)?;
        }
        let (np, nf) = header::next_page(&p);
        pid = np;
        fid = nf;
    }
    Ok(())
}

impl Catalog {
    pub fn open(pf: &PageFile) -> Result<Catalog> {
        let boot = pf.page(1, 9)?;
        if header::page_type(&boot) != 13 {
            return Err(Error::InvalidData("page (1:9) is not the boot page".into()));
        }
        let (fp, ff) = read_page_ptr(&boot, BOOT_PAGE_FIRST_SYSINDEXES_OFFSET);
        drop(boot);
        let mut alloc_units = Vec::new();
        walk_chain(pf, ff, fp, &mut |page, slot| {
            alloc_units.push(parse_alloc_unit(page, slot)?);
            Ok(())
        })?;
        let mut au_index = HashMap::with_capacity(alloc_units.len());
        for (i, au) in alloc_units.iter().enumerate() {
            au_index.entry((au.owner_id, au.ty)).or_insert(i);
        }
        let mut cat = Catalog::assemble(alloc_units, au_index, Vec::new(), Vec::new(), 0);
        let mut rowsets = Vec::new();
        cat.walk_rowset(pf, SYSROWSETS_ROWSET_ID, &mut |page, slot| {
            rowsets.push(parse_rowset(page, slot)?);
            Ok(())
        })?;
        for (i, rs) in rowsets.iter().enumerate() {
            cat.rowset_index.entry((rs.id_major, rs.id_minor)).or_insert(i);
        }
        cat.rowsets = rowsets;
        let (mut objects, mut total) = (Vec::new(), 0usize);
        let mut by_id: HashMap<i32, usize> = HashMap::new();
        cat.walk_table(pf, SYSSCHOBJS_ID, &mut |page, slot| {
            total += 1;
            let r = FixedVar::parse(page, slot)?;
            let fx = r.fixed;
            // Type is the two-byte field at fixed offset 13. Only "U " (user table) is ever
            // consumed; "UQ" is a unique constraint, not a table.
            if fx.len() < 15 || fx[13] != b'U' || fx[14] != b' ' {
                return Ok(());
            }
            let id = le::i32(fx, 0);
            let (name, _) = if r.var_count() > 0 { r.var(0)? } else { (&[][..], false) };
            let obj = SysObject {
                object_id: id,
                name: utf16(name),
            };
            match by_id.get(&id) {
                Some(&i) => objects[i] = obj,
                None => {
                    by_id.insert(id, objects.len());
                    objects.push(obj);
                }
            }
            Ok(())
        })?;
        cat.objects = objects;
        cat.total_object_count = total;
        Ok(cat)
    }

    fn assemble(
        alloc_units: Vec<AllocUnit>,
        au_index: HashMap<(i64, u8), usize>,
        rowsets: Vec<RowSet>,
        objects: Vec<SysObject>,
        total_object_count: usize,
    ) -> Catalog {
        let mut rowset_index = HashMap::with_capacity(rowsets.len());
        for (i, rs) in rowsets.iter().enumerate() {
            rowset_index.entry((rs.id_major, rs.id_minor)).or_insert(i);
        }
        Catalog {
            alloc_units,
            au_index,
            rowsets,
            rowset_index,
            objects,
            total_object_count,
            clustered_seeks: AtomicUsize::new(0),
            clustered_seek_declines: AtomicUsize::new(0),
            meta: Mutex::new(HashMap::new()),
            rowset_cols: Mutex::new(HashMap::new()),
            au_pages: Mutex::new(HashMap::new()),
            kept_sequences: None,
        }
    }

    /// Works out the SQL sequences now, so that [`to_index_bytes`](Catalog::to_index_bytes) keeps them and
    /// [`sequences`](Catalog::sequences) reads no page. A walk that fails keeps nothing: `sequences` walks again and
    /// fails as before. Whether they are kept now.
    pub fn keep_sequences(&mut self, pf: &PageFile) -> bool {
        if self.kept_sequences.is_none() {
            self.kept_sequences = self.walk_sequences(pf).ok();
        }
        self.kept_sequences.is_some()
    }

    /// Whether the SQL sequences are kept ([`keep_sequences`](Catalog::keep_sequences)).
    pub fn has_kept_sequences(&self) -> bool {
        self.kept_sequences.is_some()
    }

    /// The part of the catalog `open` reads from the system tables, as bytes a caller can keep:
    /// allocation units, rowsets and the user tables' names. Column layouts are read on demand
    /// and are not part of it.
    pub fn to_index_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(32 + self.alloc_units.len() * 40 + self.rowsets.len() * 25);
        let put = |b: &mut Vec<u8>, v: u64| b.extend_from_slice(&v.to_le_bytes());
        put(&mut b, self.alloc_units.len() as u64);
        put(&mut b, self.rowsets.len() as u64);
        put(&mut b, self.objects.len() as u64);
        put(&mut b, self.total_object_count as u64);
        for a in &self.alloc_units {
            put(&mut b, a.auid as u64);
            b.push(a.ty);
            put(&mut b, a.owner_id as u64);
            for (p, f) in [a.first_page, a.root_page, a.first_iam_page] {
                b.extend_from_slice(&p.to_le_bytes());
                b.extend_from_slice(&f.to_le_bytes());
            }
        }
        for r in &self.rowsets {
            put(&mut b, r.rowset_id as u64);
            b.extend_from_slice(&r.id_major.to_le_bytes());
            b.extend_from_slice(&r.id_minor.to_le_bytes());
            put(&mut b, r.rows as u64);
            b.push(r.compression_level);
        }
        for o in &self.objects {
            b.extend_from_slice(&o.object_id.to_le_bytes());
            b.extend_from_slice(&(o.name.len() as u32).to_le_bytes());
            b.extend_from_slice(o.name.as_bytes());
        }
        // The kept sequences follow the objects; without them the bytes end there (the layout before they were kept).
        if let Some(seqs) = &self.kept_sequences {
            put(&mut b, seqs.len() as u64);
            for q in seqs {
                b.extend_from_slice(&(q.name.len() as u32).to_le_bytes());
                b.extend_from_slice(q.name.as_bytes());
                for v in [q.start, q.increment, q.current] {
                    put(&mut b, v as u64);
                }
            }
        }
        b
    }

    /// The catalog from [`to_index_bytes`](Catalog::to_index_bytes); `None` when the bytes do not
    /// parse exactly.
    pub fn from_index_bytes(bytes: &[u8]) -> Option<Catalog> {
        let at = std::cell::Cell::new(0usize);
        let take = |n: usize| -> Option<&[u8]> {
            let from = at.get();
            let s = bytes.get(from..from.checked_add(n)?)?;
            at.set(from + n);
            Some(s)
        };
        let u64_ = || Some(u64::from_le_bytes(take(8)?.try_into().ok()?));
        let i32_ = || Some(i32::from_le_bytes(take(4)?.try_into().ok()?));
        let u16_ = || Some(u16::from_le_bytes(take(2)?.try_into().ok()?));
        let (na, nr, no) = (u64_()? as usize, u64_()? as usize, u64_()? as usize);
        let total = u64_()? as usize;
        // Every record is at least 8 bytes: a count the bytes cannot hold is damage, caught before allocating.
        if na.checked_add(nr)?.checked_add(no)?.checked_mul(8)? > bytes.len() {
            return None;
        }
        let mut alloc_units = Vec::with_capacity(na);
        let mut au_index = HashMap::with_capacity(na);
        for i in 0..na {
            let auid = u64_()? as i64;
            let ty = take(1)?[0];
            let owner_id = u64_()? as i64;
            let ptr = || Some((i32_()?, u16_()?));
            let au = AllocUnit {
                auid,
                ty,
                owner_id,
                first_page: ptr()?,
                root_page: ptr()?,
                first_iam_page: ptr()?,
            };
            au_index.entry((au.owner_id, au.ty)).or_insert(i);
            alloc_units.push(au);
        }
        let mut rowsets = Vec::with_capacity(nr);
        for _ in 0..nr {
            rowsets.push(RowSet {
                rowset_id: u64_()? as i64,
                id_major: i32_()?,
                id_minor: i32_()?,
                rows: u64_()? as i64,
                compression_level: take(1)?[0],
            });
        }
        let mut objects = Vec::with_capacity(no);
        for _ in 0..no {
            let object_id = i32_()?;
            let n = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
            let name = String::from_utf8(take(n)?.to_vec()).ok()?;
            objects.push(SysObject { object_id, name });
        }
        let kept_sequences = if at.get() == bytes.len() {
            None
        } else {
            let n = u64_()? as usize;
            // Every sequence is at least 28 bytes: a count the bytes cannot hold is damage.
            if n.checked_mul(28)? > bytes.len() {
                return None;
            }
            let mut seqs = Vec::with_capacity(n);
            for _ in 0..n {
                let len = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
                let name = String::from_utf8(take(len)?.to_vec()).ok()?;
                seqs.push(SqlSequence {
                    name,
                    start: u64_()? as i64,
                    increment: u64_()? as i64,
                    current: u64_()? as i64,
                });
            }
            Some(seqs)
        };
        (at.get() == bytes.len()).then(|| {
            let mut c = Catalog::assemble(alloc_units, au_index, rowsets, objects, total);
            c.kept_sequences = kept_sequences;
            c
        })
    }

    pub fn au_for_rowset(&self, rowset_id: i64) -> Result<&AllocUnit> {
        self.au_index
            .get(&(rowset_id, 1))
            .map(|&i| &self.alloc_units[i])
            .ok_or_else(|| crate::invalid!("no allocation unit for rowset {rowset_id}"))
    }

    /// The rowset of an object, preferring the first of `minors` that exists.
    pub fn rowset_for(&self, id_major: i32, minors: &[i32]) -> Result<&RowSet> {
        for m in minors {
            if let Some(&i) = self.rowset_index.get(&(id_major, *m)) {
                return Ok(&self.rowsets[i]);
            }
        }
        Err(crate::invalid!("no rowset for object {id_major}"))
    }

    fn walk_rowset(&self, pf: &PageFile, rowset_id: i64, f: &mut dyn FnMut(&[u8], usize) -> Result<()>) -> Result<()> {
        let au = self.au_for_rowset(rowset_id)?;
        let (pid, fid) = au.first_page;
        walk_chain(pf, fid, pid, f)
    }

    /// The SQL sequences of the database: name, start, increment and current value.
    ///
    /// A sequence is a `sysschobjs` row of type `SO`; its state is the row of `sysobjvalues` (object 60) with value
    /// class 0x3f for that object id. The value is 45 bytes, observed on a BC 29 W1 backup and checked against the
    /// demo data (the contact series holds 24 contacts and its sequence says 24): magic `a5 01 40 1f`, then five
    /// little-endian 64-bit integers (start, increment, minimum, maximum, current) and a flag byte. A row of another
    /// length or magic is skipped, never guessed.
    pub fn sequences(&self, pf: &PageFile) -> Result<Vec<SqlSequence>> {
        match &self.kept_sequences {
            Some(kept) => Ok(kept.clone()),
            None => self.walk_sequences(pf),
        }
    }

    /// [`sequences`](Catalog::sequences) read from the system tables.
    fn walk_sequences(&self, pf: &PageFile) -> Result<Vec<SqlSequence>> {
        const SYSOBJVALUES_ID: i32 = 60;
        const SEQUENCE_VALUE_CLASS: u8 = 0x3f;
        let mut names: HashMap<i32, String> = HashMap::new();
        self.walk_table(pf, SYSSCHOBJS_ID, &mut |page, slot| {
            let r = FixedVar::parse(page, slot)?;
            let fx = r.fixed;
            if fx.len() >= 15 && fx[13] == b'S' && fx[14] == b'O' && r.var_count() > 0 {
                let (name, _) = r.var(0)?;
                names.insert(le::i32(fx, 0), utf16(name));
            }
            Ok(())
        })?;
        let mut out = Vec::new();
        self.walk_table(pf, SYSOBJVALUES_ID, &mut |page, slot| {
            let r = FixedVar::parse(page, slot)?;
            let fx = r.fixed;
            if fx.len() < 5 || fx[0] != SEQUENCE_VALUE_CLASS || r.var_count() == 0 {
                return Ok(());
            }
            let Some(name) = names.get(&le::i32(fx, 1)) else {
                return Ok(());
            };
            let (v, _) = r.var(0)?;
            if v.len() != 45 || v[..4] != [0xa5, 0x01, 0x40, 0x1f] {
                return Ok(());
            }
            let i = |o: usize| i64::from_le_bytes(v[o..o + 8].try_into().expect("8 bytes"));
            out.push(SqlSequence {
                name: name.clone(),
                start: i(4),
                increment: i(12),
                current: i(36),
            });
            Ok(())
        })?;
        Ok(out)
    }

    fn walk_table(&self, pf: &PageFile, obj_id: i32, f: &mut dyn FnMut(&[u8], usize) -> Result<()>) -> Result<()> {
        self.walk_rowset(pf, self.rowset_for(obj_id, &[1, 0])?.rowset_id, f)
    }

    /// Every page an allocation unit's IAM chain claims, derived without reading any of
    /// the pages themselves: a superset of the unit's data pages. A chain it cannot make
    /// sense of ends the enumeration instead of throwing.
    pub fn alloc_unit_pages(&self, pf: &PageFile, au: &AllocUnit) -> Result<Vec<i32>> {
        let mut pages = Vec::new();
        let (mut iam_pid, mut iam_fid) = au.first_iam_page;
        let mut seen: HashSet<(u16, i32)> = HashSet::new();
        while iam_pid != 0 && seen.insert((iam_fid, iam_pid)) {
            if iam_fid != 1 {
                break;
            }
            let Some(iam) = pf.try_page(iam_fid as i32, iam_pid)? else {
                break;
            };
            if header::page_type(&iam) != 10 || header::slot_count(&iam) != 2 {
                break;
            }
            let (s0, s1) = (header::slot_offset(&iam, 0), header::slot_offset(&iam, 1));
            if s0 + 94 > PAGE_SIZE || s1 + 4 > PAGE_SIZE {
                break;
            }
            let (base_pid, base_fid) = read_page_ptr(&iam, s0 + 40);
            if !(base_fid == 0 || base_fid == 1) || base_pid < 0 || base_pid as usize % GAM_INTERVAL_PAGES != 0 {
                break;
            }
            for sp in 0..8 {
                let (sp_pid, sp_fid) = read_page_ptr(&iam, s0 + 46 + 6 * sp);
                if sp_fid == 1 && sp_pid != 0 {
                    pages.push(sp_pid);
                }
            }
            let bitmap_len = le::u16(&iam, s1 + 2) as usize;
            if s1 + 4 + bitmap_len > PAGE_SIZE {
                break;
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
                    if extent >= GAM_INTERVAL_EXTENTS {
                        continue; // bitmap overhang
                    }
                    let first = base_pid + (extent * PAGES_PER_EXTENT) as i32;
                    pages.extend(first..first + PAGES_PER_EXTENT as i32);
                }
            }
            let (np, nf) = header::next_page(&iam);
            iam_pid = np;
            iam_fid = nf;
        }
        Ok(pages)
    }

    fn au_page_set(&self, pf: &PageFile, au: &AllocUnit) -> Result<Arc<HashSet<i32>>> {
        if let Some(s) = self.au_pages.lock().unwrap().get(&au.auid) {
            return Ok(s.clone());
        }
        let set: Arc<HashSet<i32>> = Arc::new(self.alloc_unit_pages(pf, au)?.into_iter().collect());
        self.au_pages.lock().unwrap().insert(au.auid, set.clone());
        Ok(set)
    }

    // ---- clustered index descent ------------------------------------------------------

    /// The rows of a catalog base table whose leading clustered-key column equals `key`,
    /// found by descending the index; falls back to scanning every row (callers filter)
    /// when the index has a shape the descent has not been derived for.
    fn walk_keyed(
        &self,
        pf: &PageFile,
        catalog_object_id: i32,
        key: i64,
        lead_width: usize,
        what: &str,
        f: &mut dyn FnMut(&[u8], usize) -> Result<()>,
    ) -> Result<()> {
        let au = self.au_for_rowset(self.rowset_for(catalog_object_id, &[1, 0])?.rowset_id)?;
        let set = self.au_page_set(pf, au)?;
        match find_leaf(pf, au, &set, key, lead_width, what)? {
            Some(leaf) => {
                self.clustered_seeks.fetch_add(1, Ordering::Relaxed);
                walk_leaf_run(pf, leaf, key, lead_width, what, f)
            }
            None => {
                self.clustered_seek_declines.fetch_add(1, Ordering::Relaxed);
                self.walk_table(pf, catalog_object_id, f)
            }
        }
    }

    // ---- column metadata ----------------------------------------------------------------

    /// Columns and index columns of one object (syscolpars, sysiscols), by clustered-index
    /// descent. An object with none yields empty metadata.
    pub fn load_column_metadata(&self, pf: &PageFile, object_id: i32) -> Result<Arc<ObjectMeta>> {
        if let Some(m) = self.meta.lock().unwrap().get(&object_id) {
            return Ok(m.clone());
        }
        let mut meta = ObjectMeta::default();
        self.walk_keyed(
            pf,
            SYSCOLPARS_ID,
            object_id as i64,
            4,
            "syscolpars",
            &mut |page, slot| {
                if le::i32(page, slot + 4) != object_id {
                    return Ok(());
                }
                if let Some(c) = parse_colpar(page, slot, object_id)? {
                    meta.columns.push(c);
                }
                Ok(())
            },
        )?;
        meta.columns.sort_by_key(|c| c.col_id);
        self.walk_keyed(pf, SYSISCOLS_ID, object_id as i64, 4, "sysiscols", &mut |page, slot| {
            if le::i32(page, slot + 4) != object_id {
                return Ok(());
            }
            meta.index_columns.push(parse_iscol(page, slot)?);
            Ok(())
        })?;
        let m = Arc::new(meta);
        self.meta.lock().unwrap().insert(object_id, m.clone());
        Ok(m)
    }

    /// Every object's metadata by scanning syscolpars and sysiscols end to end: the
    /// reference the descent is checked against.
    pub fn column_metadata_by_scan(&self, pf: &PageFile) -> Result<HashMap<i32, ObjectMeta>> {
        let mut all: HashMap<i32, ObjectMeta> = HashMap::new();
        self.walk_table(pf, SYSCOLPARS_ID, &mut |page, slot| {
            let obj = le::i32(page, slot + 4);
            if let Some(c) = parse_colpar(page, slot, obj)? {
                all.entry(obj).or_default().columns.push(c);
            }
            Ok(())
        })?;
        for m in all.values_mut() {
            m.columns.sort_by_key(|c| c.col_id);
        }
        self.walk_table(pf, SYSISCOLS_ID, &mut |page, slot| {
            let obj = le::i32(page, slot + 4);
            all.entry(obj).or_default().index_columns.push(parse_iscol(page, slot)?);
            Ok(())
        })?;
        Ok(all)
    }

    /// Physical leaf layout of a rowset in null-bit (physical) order, by descent.
    pub fn rowset_columns(&self, pf: &PageFile, rowset_id: i64) -> Result<Arc<Vec<PhysColumn>>> {
        if let Some(c) = self.rowset_cols.lock().unwrap().get(&rowset_id) {
            return Ok(c.clone());
        }
        let mut list: Vec<PhysColumn> = Vec::new();
        self.walk_keyed(pf, SYSRSCOLS_ID, rowset_id, 8, "sysrscols", &mut |page, slot| {
            if le::i64(page, slot + 4) != rowset_id {
                return Ok(());
            }
            list.push(parse_rowset_column(page, slot)?.1);
            Ok(())
        })?;
        if list.is_empty() {
            return Err(crate::invalid!("no sysrscols layout for rowset {rowset_id}"));
        }
        list.sort_by_key(|c| c.null_bit);
        let l = Arc::new(list);
        self.rowset_cols.lock().unwrap().insert(rowset_id, l.clone());
        Ok(l)
    }

    /// The same layout reached by scanning every sysrscols row; the descent is checked against this.
    pub fn rowset_columns_by_scan(&self, pf: &PageFile, rowset_id: i64) -> Result<Vec<PhysColumn>> {
        let mut list: Vec<PhysColumn> = Vec::new();
        self.walk_table(pf, SYSRSCOLS_ID, &mut |page, slot| {
            let (rsid, col) = parse_rowset_column(page, slot)?;
            if rsid == rowset_id {
                list.push(col);
            }
            Ok(())
        })?;
        if list.is_empty() {
            return Err(crate::invalid!("no sysrscols layout for rowset {rowset_id}"));
        }
        list.sort_by_key(|c| c.null_bit);
        Ok(list)
    }
}

fn parse_alloc_unit(page: &[u8], slot: usize) -> Result<AllocUnit> {
    let r = FixedVar::parse(page, slot)?;
    let fx = r.fixed;
    if fx.len() < 41 {
        return Err(crate::invalid!(
            "sysallocunits record of {} fixed bytes — refusing to guess",
            fx.len()
        ));
    }
    Ok(AllocUnit {
        auid: le::i64(fx, 0),
        ty: fx[8],
        owner_id: le::i64(fx, 9),
        first_page: read_page_ptr(fx, 23),
        root_page: read_page_ptr(fx, 29),
        first_iam_page: read_page_ptr(fx, 35),
    })
}

fn parse_rowset(page: &[u8], slot: usize) -> Result<RowSet> {
    let r = FixedVar::parse(page, slot)?;
    let fx = r.fixed;
    if fx.len() < 36 {
        return Err(crate::invalid!(
            "sysrowsets record of {} fixed bytes — refusing to guess",
            fx.len()
        ));
    }
    Ok(RowSet {
        rowset_id: le::i64(fx, 0),
        id_major: le::i32(fx, 9),
        id_minor: le::i32(fx, 13),
        rows: le::i64(fx, 27),
        compression_level: fx[35],
    })
}

/// One syscolpars row; `None` for procedure parameters and the like (number != 0).
fn parse_colpar(page: &[u8], slot: usize, obj_id: i32) -> Result<Option<SysColumn>> {
    let r = FixedVar::parse(page, slot)?;
    let fx = r.fixed;
    if fx.len() < 19 {
        return Err(crate::invalid!(
            "syscolpars record for object {obj_id} is {} fixed bytes — refusing to guess",
            fx.len()
        ));
    }
    let number = le::i16(fx, 4);
    if number != 0 {
        return Ok(None);
    }
    let col_id = le::i32(fx, 6);
    let xtype = fx[10];
    let max_len = le::i16(fx, 15);
    let (prec, scale) = (fx[17], fx[18]);
    // status u32@23: bit 0x01 = NOT NULL. Derived from the probe database (every type NULL
    // in `probe`, NOT NULL in `probe_notnull`) and validated against sys.columns.is_nullable
    // for all 383 columns of it (bcdb fixtures/typeprobe-nullability.tsv).
    if fx.len() < 27 {
        return Err(crate::invalid!(
            "syscolpars record for object {obj_id} column {col_id} is {} fixed bytes, too short to carry the status word — refusing to guess nullability",
            fx.len()
        ));
    }
    let status = le::u32(fx, 23);
    let name_bytes = if r.var_count() > 0 { r.var(0)?.0 } else { &[][..] };
    let name = if name_bytes.is_empty() {
        format!("col{col_id}")
    } else {
        utf16(name_bytes)
    };
    Ok(Some(SysColumn {
        col_id,
        name,
        xtype,
        max_length: max_len,
        precision: prec,
        scale,
        is_nullable: status & 0x01 == 0,
    }))
}

fn parse_iscol(page: &[u8], slot: usize) -> Result<SysIndexCol> {
    let r = FixedVar::parse(page, slot)?;
    let fx = r.fixed;
    if fx.len() < 20 {
        return Err(crate::invalid!(
            "sysiscols record of {} fixed bytes — refusing to guess",
            fx.len()
        ));
    }
    Ok(SysIndexCol {
        index_id: le::i32(fx, 4),
        key_ordinal: le::i32(fx, 8),
        col_id: le::i32(fx, 16),
    })
}

fn decimal_storage_bytes(precision: u8) -> i16 {
    crate::bcp::decimal_storage_bytes(precision) as i16
}

fn fixed_width(xtype: u8, ti_len: u8) -> i16 {
    let time_w = if ti_len <= 2 {
        3
    } else if ti_len <= 4 {
        4
    } else {
        5
    };
    match xtype {
        48 | 104 => 1,
        52 => 2,
        56 | 59 => 4,
        127 | 62 | 61 | 189 => 8,
        36 => 16,
        58 => 4,
        122 => 4,
        60 => 8,
        40 => 3,
        41 => time_w,
        42 => time_w + 3,
        _ => ti_len as i16,
    }
}

fn parse_rowset_column(page: &[u8], slot: usize) -> Result<(i64, PhysColumn)> {
    let r = FixedVar::parse(page, slot)?;
    let fx = r.fixed;
    if fx.len() < 50 {
        return Err(crate::invalid!(
            "sysrscols record of {} fixed bytes — refusing to guess",
            fx.len()
        ));
    }
    let rsid = le::i64(fx, 0);
    let rscolid = le::u32(fx, 8);
    let ti = le::u32(fx, 24);
    let ordkey = le::i16(fx, 32);
    let status = le::u32(fx, 36);
    let offset = le::i16(fx, 40);
    let nullbit = le::u16(fx, 44);
    let bitpos = le::u16(fx, 48);
    let xtype = (ti & 0xff) as u8;
    let b1 = ((ti >> 8) & 0xff) as u8;
    let b2 = ((ti >> 16) & 0xff) as u8;
    let str_len = ((ti >> 8) & 0xffff) as i32;
    let max_len = match xtype {
        106 | 108 => decimal_storage_bytes(b1),
        231 | 239 | 167 | 175 | 165 | 173 | 34 | 35 | 99 | 241 => {
            if str_len == 0 {
                -1
            } else {
                str_len as i16
            }
        }
        _ => fixed_width(xtype, b1),
    };
    let (prec, scale) = match xtype {
        106 | 108 => (b1, b2),
        41..=43 => (0, b1),
        _ => (0, 0),
    };
    let dropped = status & 0x02 != 0 || rscolid & 0x0400_0000 != 0;
    let internal = rscolid & 0x0800_0000 != 0;
    Ok((
        rsid,
        PhysColumn {
            col_id: (rscolid & 0x00FF_FFFF) as i32,
            dropped,
            internal,
            xtype,
            max_length: max_len,
            precision: prec,
            scale,
            key_ordinal: ordkey,
            leaf_offset: offset,
            null_bit: nullbit,
            bit_pos: bitpos,
        },
    ))
}

// ---- clustered index descent (ClusteredSeek.cs) ---------------------------------------
//
// Non-leaf record layout (derived from DBCC PAGE dumps of the bc281 clustered index roots
// of syscolpars, sysrscols and sysiscols, bcdb PROVENANCE.md "Clustered index descent"):
//
//   [status byte][key columns, packed, in key order, each at its storage width][child]
//
// with the child a 6-byte page pointer (u32 page id, u16 file id) at the end of the
// record. Records on an index page are fixed width and that width is the page header's
// pminlen at offset 14, so the child pointer sits at pminlen - 6. A type-2 page at level 0
// is the lowest index level and its children are the type-1 leaf pages: the descent stops
// on page type, never on a level count. Slot 0 of every index page is the leftmost
// pointer, whose key bytes are not a key.

const CHILD_POINTER_BYTES: usize = 6;
const MAX_DEPTH: usize = 16;

/// The first leaf page that can hold `key`, or `None` when the index has a shape this
/// derivation does not cover (the caller scans instead). A structure that contradicts the
/// derivation throws.
fn find_leaf(
    pf: &PageFile,
    au: &AllocUnit,
    au_pages: &HashSet<i32>,
    key: i64,
    lead_width: usize,
    what: &str,
) -> Result<Option<i32>> {
    let (mut pid, fid) = au.root_page;
    // root_page is one of the pointers the stale-metadata rule distrusts: on a shrunk
    // database it can name a page another object now owns. The IAM page set is the
    // trustworthy one, so a root outside it is treated as stale and scanned instead.
    if fid != 1 || pid <= 0 || !au_pages.contains(&pid) {
        return Ok(None);
    }
    let mut depth = 0;
    loop {
        let page = pf.page(1, pid)?;
        let ty = header::page_type(&page);
        if ty == 1 {
            return Ok(Some(pid)); // leaf level
        }
        if ty != 2 {
            return Err(crate::invalid!(
                "{what}: page 1:{pid} reached by descending the clustered index is type {ty}, not an index or data page — refusing to guess"
            ));
        }
        if depth > MAX_DEPTH {
            return Err(crate::invalid!(
                "{what}: clustered index deeper than {MAX_DEPTH} levels — refusing to guess"
            ));
        }
        // Ghosted index records are a shape this has not been derived for.
        if header::ghost_records(&page) != 0 {
            return Ok(None);
        }
        let pminlen = header::pminlen(&page);
        if pminlen < 1 + lead_width + CHILD_POINTER_BYTES || pminlen > PAGE_SIZE - 96 {
            return Ok(None);
        }
        pid = child_for(&page, pminlen, key, lead_width, what)?;
        depth += 1;
    }
}

/// The child holding the largest key strictly below `key`. Strictly below, not "at most":
/// only the leading column of a composite key is compared, and rows sharing a leading
/// value straddle page boundaries.
fn child_for(page: &[u8], pminlen: usize, key: i64, lead_width: usize, what: &str) -> Result<i32> {
    let slots = header::checked_slot_count(page)?;
    if slots < 1 {
        return Err(crate::invalid!("{what}: index page with no slots — refusing to guess"));
    }
    let mut chosen = 0usize;
    let mut previous: Option<i64> = None;
    for s in 0..slots {
        let off = header::slot_offset(page, s);
        if off < 96 || off + pminlen > PAGE_SIZE {
            return Err(crate::invalid!(
                "{what}: index slot offset {off} does not hold a {pminlen}-byte record — refusing to guess"
            ));
        }
        if s == 0 {
            chosen = off; // leftmost pointer: no key
            continue;
        }
        let slot_key = if lead_width == 8 {
            le::i64(page, off + 1)
        } else {
            le::i32(page, off + 1) as i64
        };
        // Keys ascend with the slot array. If they do not, this is not reading keys where
        // the keys are, and every answer built on it would be wrong.
        if let Some(p) = previous {
            if slot_key < p {
                return Err(crate::invalid!(
                    "{what}: clustered index keys are not ascending across the slot array ({slot_key} after {p}) — refusing to guess"
                ));
            }
        }
        previous = Some(slot_key);
        if slot_key >= key {
            break;
        }
        chosen = off;
    }
    let child_off = chosen + pminlen - CHILD_POINTER_BYTES;
    let (cpid, cfid) = read_page_ptr(page, child_off);
    if cfid != 1 || cpid <= 0 {
        return Err(crate::invalid!(
            "{what}: clustered index child pointer ({cfid}:{cpid}) is not a page of this file — refusing to guess"
        ));
    }
    Ok(cpid)
}

/// Leaf rows from the leaf `first` forward whose leading key equals `key`, following the
/// leaf chain because one object's rows span pages. The leading key column of every
/// catalog base table sits at record offset 4.
fn walk_leaf_run(
    pf: &PageFile,
    first: i32,
    key: i64,
    lead_width: usize,
    what: &str,
    f: &mut dyn FnMut(&[u8], usize) -> Result<()>,
) -> Result<()> {
    let mut pid = first;
    let mut seen: HashSet<i32> = HashSet::new();
    loop {
        let page = pf.page(1, pid)?;
        let mut previous: Option<i64> = None;
        for s in 0..header::checked_slot_count(&page)? {
            let Some(so) = live_slot(header::slot_offset(&page, s))? else {
                continue;
            };
            let rt = record_type(&page, so);
            if is_ghost_type(rt) {
                continue;
            }
            if rt != 0 {
                return Err(crate::unsupported!(
                    "{what}: record type {rt} not supported in a catalog leaf walk"
                ));
            }
            let k = if lead_width == 8 {
                le::i64(&page, so + 4)
            } else {
                le::i32(&page, so + 4) as i64
            };
            if let Some(p) = previous {
                if k < p {
                    return Err(crate::invalid!(
                        "{what}: leaf keys are not ascending ({k} after {p}) — refusing to guess"
                    ));
                }
            }
            previous = Some(k);
            if k < key {
                continue; // still before the run
            }
            if k > key {
                return Ok(()); // past it: the run is over
            }
            f(&page, so)?;
        }
        let (np, nf) = header::next_page(&page);
        if np == 0 {
            return Ok(());
        }
        if nf != 1 {
            return Err(crate::unsupported!(
                "{what}: leaf chain continues into file {nf} — only single-data-file databases are supported"
            ));
        }
        if !seen.insert(np) {
            return Err(crate::invalid!(
                "{what}: leaf chain revisits page 1:{np} — refusing to loop"
            ));
        }
        pid = np;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(seqs: Option<Vec<SqlSequence>>) -> Catalog {
        let objects = vec![SysObject {
            object_id: 7,
            name: "CRONUS$Customer".into(),
        }];
        let mut c = Catalog::assemble(Vec::new(), HashMap::new(), Vec::new(), objects, 1);
        c.kept_sequences = seqs;
        c
    }

    /// Kept sequences come back from the index bytes as they went in; bytes without them (the layout before they were
    /// kept) read as a catalog that walks for them; a cut in the sequence part is damage.
    #[test]
    fn kept_sequences_round_trip_through_the_index_bytes() {
        let seqs = vec![
            SqlSequence {
                name: "CRONUS International Ltd_$NoSeries$CUST".into(),
                start: 1,
                increment: 1,
                current: 24,
            },
            SqlSequence {
                name: "Straße".into(),
                start: -5,
                increment: -1,
                current: i64::MIN,
            },
        ];
        let with = catalog(Some(seqs.clone())).to_index_bytes();
        assert_eq!(Catalog::from_index_bytes(&with).unwrap().kept_sequences, Some(seqs));
        let without = catalog(None).to_index_bytes();
        assert_eq!(Catalog::from_index_bytes(&without).unwrap().kept_sequences, None);
        assert_eq!(
            catalog(Some(Vec::new())).to_index_bytes().len(),
            without.len() + 8,
            "an empty list is kept too"
        );
        for cut in (without.len() + 1)..with.len() {
            assert!(Catalog::from_index_bytes(&with[..cut]).is_none(), "cut at {cut}");
        }
    }
}
