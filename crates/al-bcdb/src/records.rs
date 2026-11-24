//! Record parsers (port of `Records.cs`): the uncompressed FixedVar layout and the
//! compressed "column descriptor" (CD) layout of row- and page-compressed data.
//!
//! Entry points: `record_type`, `is_ghost_type`, `live_slot`, `FixedVar`.

use crate::error::{Error, Result};
use crate::io::le;
use crate::mtf::PAGE_SIZE;
use crate::types::Cell;

/// Record type from status bits A (bits 1-3): 0 = primary, 5/6/7 = ghost variants.
pub fn record_type(p: &[u8], off: usize) -> u8 {
    (p[off] >> 1) & 7
}

pub fn is_ghost_type(rt: u8) -> bool {
    matches!(rt, 5..=7)
}

/// Check a slot offset the way every walk does: 0 is an empty slot, below the 96-byte
/// header is corruption. Returns `None` for an empty slot.
pub fn live_slot(so: usize) -> Result<Option<usize>> {
    if so == 0 {
        return Ok(None); // empty slot (deleted / never completed): SQL Server's own scan skips it
    }
    if so < 96 {
        return Err(crate::invalid!(
            "slot offset {so} inside the page header — page corrupt?"
        ));
    }
    if so + 4 > PAGE_SIZE {
        return Err(crate::invalid!(
            "slot offset {so} leaves no room for a record — page corrupt?"
        ));
    }
    Ok(Some(so))
}

/// An uncompressed FixedVar record: status bits, fixed data, column count, null bitmap,
/// variable-length offset array. Layout is public knowledge from SQL Server internals
/// literature; every offset used was verified against a live SQL Server via DBCC PAGE.
/// Nothing is copied.
pub struct FixedVar<'a> {
    page: &'a [u8],
    off: usize,
    /// The fixed data, past the 4-byte record header.
    pub fixed: &'a [u8],
    /// Column count the record was written with.
    pub ncols: usize,
    /// Empty when the record has no null bitmap.
    pub null_bmp: &'a [u8],
    nvar: usize,
    var_offsets_at: usize,
    data_start: usize,
}

impl<'a> FixedVar<'a> {
    pub fn parse(page: &'a [u8], off: usize) -> Result<FixedVar<'a>> {
        let a = page[off];
        let has_null_bmp = a & 0x10 != 0;
        let has_var = a & 0x20 != 0;
        let fixed_end = le::u16(page, off + 2) as usize;
        if fixed_end < 4 || off + fixed_end + 2 > PAGE_SIZE {
            return Err(crate::invalid!(
                "record at offset {off}: fixed part ends at {fixed_end} — page corrupt?"
            ));
        }
        let mut pos = off + fixed_end;
        let ncols = le::u16(page, pos) as usize;
        pos += 2;
        let mut null_bmp: &[u8] = &[];
        if has_null_bmp {
            let nb = ncols.div_ceil(8);
            if pos + nb > PAGE_SIZE {
                return Err(crate::invalid!(
                    "record at offset {off}: null bitmap runs past the page"
                ));
            }
            null_bmp = &page[pos..pos + nb];
            pos += nb;
        }
        let (mut nvar, mut var_offsets_at, mut data_start) = (0, 0, 0);
        if has_var {
            if pos + 2 > PAGE_SIZE {
                return Err(crate::invalid!(
                    "record at offset {off}: variable column count runs past the page"
                ));
            }
            nvar = le::u16(page, pos) as usize;
            pos += 2;
            if pos + 2 * nvar > PAGE_SIZE {
                return Err(crate::invalid!(
                    "record at offset {off}: variable offset array runs past the page"
                ));
            }
            var_offsets_at = pos;
            data_start = pos + 2 * nvar - off;
        }
        Ok(FixedVar {
            page,
            off,
            fixed: &page[off + 4..off + fixed_end],
            ncols,
            null_bmp,
            nvar,
            var_offsets_at,
            data_start,
        })
    }

    pub fn var_count(&self) -> usize {
        self.nvar
    }

    /// Variable column `i` (0-based): its absolute (start, length) in the page and whether
    /// it is a "complex" (off-row pointer) entry.
    pub fn var_span(&self, i: usize) -> Result<(usize, usize, bool)> {
        let raw = le::u16(self.page, self.var_offsets_at + 2 * i) as usize;
        let complex = raw & 0x8000 != 0;
        let end = raw & 0x7fff;
        let prev = if i == 0 {
            self.data_start
        } else {
            le::u16(self.page, self.var_offsets_at + 2 * (i - 1)) as usize & 0x7fff
        };
        if end < prev || self.off + end > PAGE_SIZE {
            return Err(crate::invalid!(
                "variable column {i} of the record at offset {} spans {prev}..{end} — refusing to guess",
                self.off
            ));
        }
        Ok((self.off + prev, end - prev, complex))
    }

    pub fn var(&self, i: usize) -> Result<(&'a [u8], bool)> {
        let (s, l, c) = self.var_span(i)?;
        Ok((&self.page[s..s + l], c))
    }

    pub fn is_null(&self, col0: usize) -> bool {
        !self.null_bmp.is_empty() && self.null_bmp[col0 / 8] & (1 << (col0 % 8)) != 0
    }
}

/// (absolute start, length) of every variable column of the FixedVar record at `off`.
pub fn fixed_var_var_columns(page: &[u8], off: usize) -> Result<Vec<(usize, usize)>> {
    let r = FixedVar::parse(page, off)?;
    (0..r.var_count())
        .map(|i| r.var_span(i).map(|(s, l, _)| (s, l)))
        .collect()
}

// ---- compressed (CD) records ------------------------------------------------------------

/// Where a cell's bytes are: the page itself, the scratch arena (a prefix-expanded value),
/// or the one-byte constant a 0xB code stands for.
#[derive(Clone, Copy, Debug)]
pub struct CellLoc {
    kind: u8,
    pub complex: bool,
    start: u32,
    len: u32,
}

const K_NULL: u8 = 0;
const K_PAGE: u8 = 1;
const K_ARENA: u8 = 2;
const K_ONE: u8 = 3;

impl CellLoc {
    pub const NULL: CellLoc = CellLoc {
        kind: K_NULL,
        complex: false,
        start: 0,
        len: 0,
    };
    fn page(start: usize, len: usize) -> CellLoc {
        CellLoc {
            kind: K_PAGE,
            complex: false,
            start: start as u32,
            len: len as u32,
        }
    }
    fn is_null(&self) -> bool {
        self.kind == K_NULL
    }
    pub fn cell<'a>(&self, page: &'a [u8], arena: &'a [u8]) -> Cell<'a> {
        let bytes: &'a [u8] = match self.kind {
            K_NULL => return Cell::Null,
            K_PAGE => &page[self.start as usize..(self.start + self.len) as usize],
            K_ARENA => &arena[self.start as usize..(self.start + self.len) as usize],
            _ => &[1u8],
        };
        if self.complex {
            Cell::Complex(bytes)
        } else {
            Cell::Value(bytes)
        }
    }
}

/// Compression information of a page (types with flag 0x80): the anchor record and the
/// page dictionary. Both are slices of the page.
#[derive(Default)]
pub struct PageCi {
    pub anchors: Option<Vec<CellLoc>>,
    pub dictionary: Vec<(u32, u32)>,
}

/// Reusable per-record buffers.
#[derive(Default)]
pub struct CdScratch {
    pub cells: Vec<CellLoc>,
    pub arena: Vec<u8>,
    codes: Vec<u8>,
    long_cols: Vec<usize>,
}

pub fn is_cd(p: &[u8], off: usize) -> bool {
    p[off] & 1 != 0
}

/// Ghost (deleted, not yet cleaned up) CD record: header bits 2+3 set (byte 0x0D observed).
/// A header with exactly one of the two bits set has never been observed: throw.
pub fn is_cd_ghost(p: &[u8], off: usize) -> Result<bool> {
    let bits = p[off] & 0x0C;
    if bits != 0 && bits != 0x0C {
        return Err(crate::unsupported!(
            "CD record header 0x{:02x}: ghost-bit pattern 0x{bits:02x} never observed — refusing to guess",
            p[off]
        ));
    }
    Ok(bits == 0x0C)
}

fn bounds(p: &[u8], pos: usize, n: usize) -> Result<()> {
    if pos + n > p.len() {
        return Err(crate::invalid!(
            "compressed record runs past the page (offset {pos}+{n})"
        ));
    }
    Ok(())
}

/// Apply anchor-prefix reconstruction: stored = `[prefixLength byte][suffix]` when the
/// column has a non-null anchor.
fn resolve(
    p: &[u8],
    col: usize,
    start: usize,
    len: usize,
    anchors: Option<&[CellLoc]>,
    arena: &mut Vec<u8>,
) -> Result<CellLoc> {
    let anchor = anchors.and_then(|a| a.get(col)).copied().unwrap_or(CellLoc::NULL);
    if anchor.is_null() || anchor.len == 0 {
        return Ok(CellLoc::page(start, len));
    }
    if len == 0 {
        return Ok(anchor); // exact anchor match
    }
    let plen = p[start] as usize;
    if plen > anchor.len as usize {
        return Err(Error::InvalidData("prefix length exceeds anchor".into()));
    }
    let a_start = anchor.start as usize;
    let at = arena.len();
    arena.extend_from_slice(&p[a_start..a_start + plen]);
    arena.extend_from_slice(&p[start + 1..start + len]);
    Ok(CellLoc {
        kind: K_ARENA,
        complex: false,
        start: at as u32,
        len: (plen + len - 1) as u32,
    })
}

/// Parse one CD record into `scratch.cells` (cleared first). CD codes: 0 = NULL, 1 =
/// empty/zero, 2..9 = (code-1) data bytes, 0xA = value in the long data region, 0xB = a bit
/// with value 1, 0xC = one-byte page-dictionary symbol.
pub fn parse_cd(p: &[u8], off: usize, ci: Option<&PageCi>, scratch: &mut CdScratch) -> Result<()> {
    let hdr = p[off];
    if hdr & 1 == 0 {
        return Err(Error::InvalidData("not a CD record".into()));
    }
    let has_long = hdr & 0x20 != 0;
    // Column count: one byte when < 128; with the high bit set it is a two-byte big-endian
    // count, low 7 bits of the first byte are the high bits (bcdb PROVENANCE.md, 203-column probe).
    bounds(p, off, 3)?;
    let mut ncols = p[off + 1] as usize;
    let mut cd_off = off + 2;
    if ncols & 0x80 != 0 {
        ncols = ((ncols & 0x7f) << 8) | p[off + 2] as usize;
        cd_off = off + 3;
    }
    bounds(p, cd_off, ncols.div_ceil(2))?;
    scratch.codes.clear();
    scratch.codes.extend((0..ncols).map(|i| {
        let b = p[cd_off + i / 2];
        if i % 2 == 0 { b & 0xf } else { b >> 4 }
    }));
    let mut pos = cd_off + ncols.div_ceil(2);
    let clusters = ncols.div_ceil(30);
    if clusters > 1 {
        pos += clusters - 1; // per-cluster short-data byte counts
    }
    let anchors = ci.and_then(|c| c.anchors.as_deref());
    scratch.cells.clear();
    scratch.arena.clear();
    scratch.long_cols.clear();
    for i in 0..ncols {
        let code = scratch.codes[i];
        let cell = match code {
            0 => CellLoc::NULL,
            1 => resolve(p, i, pos, 0, anchors, &mut scratch.arena)?,
            2..=9 => {
                let n = (code - 1) as usize;
                bounds(p, pos, n)?;
                let c = resolve(p, i, pos, n, anchors, &mut scratch.arena)?;
                pos += n;
                c
            }
            0xA => {
                scratch.long_cols.push(i);
                CellLoc::NULL
            }
            0xB => CellLoc {
                kind: K_ONE,
                complex: false,
                start: 0,
                len: 1,
            },
            0xC => {
                bounds(p, pos, 1)?;
                let sym = p[pos] as usize;
                pos += 1;
                let Some(&(ds, dl)) = ci.and_then(|c| c.dictionary.get(sym)) else {
                    return Err(Error::InvalidData("dictionary symbol without dictionary".into()));
                };
                resolve(p, i, ds as usize, dl as usize, anchors, &mut scratch.arena)?
            }
            c => return Err(crate::unsupported!("CD code 0x{c:x} not implemented")),
        };
        scratch.cells.push(cell);
    }
    if has_long {
        bounds(p, pos, 3)?;
        if p[pos] != 1 {
            return Err(crate::unsupported!(
                "long data region header 0x{:02x} not implemented",
                p[pos]
            ));
        }
        let cnt = le::u16(p, pos + 1) as usize;
        if cnt != scratch.long_cols.len() {
            return Err(Error::InvalidData("long region count mismatch".into()));
        }
        let ends_off = pos + 3;
        let data_base = ends_off + 2 * cnt + clusters.saturating_sub(1); // per-cluster long-value counts
        bounds(p, ends_off, 2 * cnt)?;
        let mut prev = 0usize;
        for i in 0..cnt {
            let end_raw = le::u16(p, ends_off + 2 * i) as usize;
            // High bit marks a complex entry: an off-row pointer, not inline data.
            let complex = end_raw & 0x8000 != 0;
            let end = end_raw & 0x7fff;
            if end < prev {
                return Err(Error::InvalidData("long value ends before it starts".into()));
            }
            bounds(p, data_base + prev, end - prev)?;
            let col = scratch.long_cols[i];
            let mut c = resolve(p, col, data_base + prev, end - prev, anchors, &mut scratch.arena)?;
            c.complex = complex;
            scratch.cells[col] = c;
            prev = end;
        }
    }
    Ok(())
}

/// Parse the compression information structure at page offset 96 (pages with type flag
/// bits 0x80): `[u8 header (bit1 anchor present, bit2 dictionary present)][u16 pageModCount]`,
/// then one u16 end-offset (relative to the CI start) per present part, then the anchor
/// record, then the dictionary. The offset fields are conditional: anchor-only pages carry
/// no end-of-dictionary slot (bcdb PROVENANCE.md "Compressed (CD) records").
pub fn parse_ci(p: &[u8]) -> Result<PageCi> {
    let o = 96;
    let hdr = p[o];
    if hdr & 1 != 0 {
        return Err(Error::NotSupported("CI version bit set — unknown CI version".into()));
    }
    let has_anchor = hdr & 2 != 0;
    let has_dict = hdr & 4 != 0;
    let mut cursor = o + 3;
    let mut end_of_anchor = 0usize;
    if has_anchor {
        end_of_anchor = le::u16(p, cursor) as usize;
        cursor += 2;
    }
    if has_dict {
        cursor += 2; // end-of-dictionary (not needed: entry offsets delimit it)
    }
    let mut ci = PageCi::default();
    if has_anchor {
        let mut tmp = CdScratch::default();
        parse_cd(p, cursor, None, &mut tmp)?;
        ci.anchors = Some(tmp.cells);
    }
    if has_dict {
        let d = o + if has_anchor { end_of_anchor } else { cursor - o };
        bounds(p, d, 2)?;
        let cnt = le::u16(p, d) as usize;
        bounds(p, d + 2, 2 * cnt)?;
        let mut prev = 2 + 2 * cnt; // entry end offsets are relative to dictionary start
        for i in 0..cnt {
            let end = le::u16(p, d + 2 + 2 * i) as usize;
            if end < prev || d + end > PAGE_SIZE {
                return Err(Error::InvalidData("dictionary entry runs past the page".into()));
            }
            ci.dictionary.push(((d + prev) as u32, (end - prev) as u32));
            prev = end;
        }
    }
    Ok(ci)
}
