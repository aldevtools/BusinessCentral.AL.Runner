//! Resolves off-row values: LOB columns (image/text/ntext, varbinary(max)/nvarchar(max))
//! and row-overflow columns (port of `Lob.cs`). All byte layouts derived from probe
//! tables on a scratch SQL Server, cross-annotated with DBCC PAGE and validated by
//! comparing assembled values with SELECT (bcdb PROVENANCE.md "Off-row storage").
//!
//! In-row forms (the column's stored cell, marked "complex" by the record format):
//!  * 16-byte text pointer (image/text/ntext): `[u64 timestamp][6-byte page ptr][u16 slot]`.
//!  * 12+12n-byte inline root (MAX types / row-overflow):
//!    `[u8 type (2 = row-overflow, 4 = LOB root)][u8][u8 level][u8][u32 updateSeq][u32 timestamp]`
//!    then n links of `[u32 cumulative size][6-byte page ptr][u16 slot]`.
//!
//! Records on LOB pages (page types 3/4), addressed by (page, slot):
//! `[u16 statusA][u16 record length][u64 blobId][u16 type]`, then per type:
//!  * 0 SMALL: `[u16 length][u16 x][u16 0]` then the data. The word at +16 is 0 on freshly
//!    written values and 1 after the value was rewritten.
//!  * 2 INTERNAL: `[u16 maxLinks][u16 curLinks][u16 level]` then curLinks x
//!    `[u64 cumulative size][6-byte page ptr][u16 slot]`.
//!  * 3 DATA: data up to record length.
//!  * 5 LARGE_ROOT_YUKON: `[u16 maxLinks][u16 curLinks][u16 level][u32]` then curLinks x
//!    `[u32 cumulative size][6-byte page ptr][u16 slot]`.
//!  * 8 NULL: the value is SQL NULL. Updating a legacy text/image value to NULL keeps the
//!    in-row text pointer and turns the root record into this type. Only valid as the root.
//!
//! Entry points: `LobReader`.

use crate::error::Result;
use crate::io::le;
use crate::pagefile::{PageFile, header};
use crate::types::LobResolver;

pub struct LobReader<'a> {
    pf: &'a PageFile,
}

impl<'a> LobReader<'a> {
    pub fn new(pf: &'a PageFile) -> Self {
        LobReader { pf }
    }
}

fn row_id(b: &[u8], o: usize) -> (i32, i32, usize) {
    (le::i32(b, o), le::u16(b, o + 4) as i32, le::u16(b, o + 6) as usize)
}

impl LobResolver for LobReader<'_> {
    /// Resolve a complex in-row cell (text pointer or inline root) to the full value;
    /// `None` = the value is SQL NULL (a type-8 NULL root).
    fn resolve(&self, pointer: &[u8], column: &str) -> Result<Option<Vec<u8>>> {
        if pointer.len() == 16 {
            // text pointer: timestamp, then row id of the root/data record
            let (page, file_id, slot) = row_id(pointer, 8);
            let mut out = Vec::new();
            if !self.append_record(&mut out, file_id, page, slot, 0, column)? {
                return Ok(None);
            }
            return Ok(Some(out));
        }
        if pointer.len() >= 24 && (pointer.len() - 12) % 12 == 0 && (pointer[0] == 2 || pointer[0] == 4) {
            let links = (pointer.len() - 12) / 12;
            let mut out = Vec::new();
            let mut expected = 0u64;
            for i in 0..links {
                let o = 12 + 12 * i;
                let cum = le::u32(pointer, o) as u64;
                let (page, file_id, slot) = row_id(pointer, o + 4);
                if !self.append_record(&mut out, file_id, page, slot, 0, column)? {
                    return Err(crate::invalid!(
                        "off-row value of {column}: NULL root record inside a multi-link inline root — never observed, refusing to guess"
                    ));
                }
                if out.len() as u64 != cum {
                    return Err(crate::invalid!(
                        "off-row value of {column}: assembled {} bytes where the root link records {cum}",
                        out.len()
                    ));
                }
                expected = cum;
            }
            if out.len() as u64 != expected {
                return Err(crate::invalid!(
                    "off-row value of {column}: assembled {} bytes, root says {expected}",
                    out.len()
                ));
            }
            return Ok(Some(out));
        }
        Err(crate::unsupported!(
            "column {column}: unrecognized {}-byte off-row pointer (first byte 0x{:02x}) — layout not derived, refusing to guess",
            pointer.len(),
            pointer.first().copied().unwrap_or(0)
        ))
    }
}

impl LobReader<'_> {
    /// Append the record's value bytes; false = the record is a type-8 NULL root.
    fn append_record(
        &self,
        out: &mut Vec<u8>,
        file_id: i32,
        page_id: i32,
        slot: usize,
        depth: u32,
        column: &str,
    ) -> Result<bool> {
        if depth > 8 {
            return Err(crate::invalid!("LOB chain of {column} exceeds depth 8 — cycle?"));
        }
        let page = self.pf.page(file_id, page_id)?;
        let pt = header::page_type(&page);
        if pt != 3 && pt != 4 {
            return Err(crate::invalid!(
                "LOB pointer of {column} leads to page 1:{page_id} of type {pt} (expected a LOB page)"
            ));
        }
        let n = header::checked_slot_count(&page)?;
        if slot >= n {
            return Err(crate::invalid!(
                "LOB pointer of {column}: slot {slot} beyond {n} slots on page 1:{page_id}"
            ));
        }
        let r = header::slot_offset(&page, slot);
        if r < 96 {
            return Err(crate::invalid!(
                "LOB pointer of {column} leads to an empty slot ({slot} on page 1:{page_id}) — dangling pointer, refusing to guess"
            ));
        }
        if r + 24 > page.len() {
            return Err(crate::invalid!(
                "LOB record of {column} at offset {r} on page 1:{page_id} runs past the page"
            ));
        }
        let rec_len = le::u16(&page, r + 2) as usize;
        let ty = le::u16(&page, r + 12);
        match ty {
            0 => {
                // SMALL: complete value in one record
                let len = le::u16(&page, r + 14) as usize;
                if 20 + len > rec_len || r + 20 + len > page.len() {
                    return Err(crate::invalid!(
                        "SMALL LOB record of {column} on page 1:{page_id} slot {slot}: size {len} exceeds the {rec_len}-byte record — refusing to guess"
                    ));
                }
                out.extend_from_slice(&page[r + 20..r + 20 + len]);
            }
            3 => {
                // DATA: one chunk
                if rec_len < 14 || r + rec_len > page.len() {
                    return Err(crate::invalid!(
                        "DATA LOB record of {column} on page 1:{page_id} slot {slot}: length {rec_len} out of range"
                    ));
                }
                out.extend_from_slice(&page[r + 14..r + rec_len]);
            }
            8 => {
                // NULL root: the value is SQL NULL
                if depth != 0 {
                    return Err(crate::invalid!(
                        "NULL LOB root record of {column} at depth {depth} (page 1:{page_id} slot {slot}) — never observed below a root, refusing to guess"
                    ));
                }
                let len = le::u16(&page, r + 14);
                if len != 0 {
                    return Err(crate::invalid!(
                        "NULL LOB root record of {column} on page 1:{page_id} slot {slot} carries size {len} — never observed, refusing to guess"
                    ));
                }
                return Ok(false);
            }
            5 => {
                // LARGE_ROOT_YUKON
                let cur = le::u16(&page, r + 16) as usize;
                let start = out.len();
                for i in 0..cur {
                    let o = r + 24 + 12 * i;
                    if o + 12 > page.len() {
                        return Err(crate::invalid!("LOB tree of {column}: link {i} runs past the page"));
                    }
                    let cum = le::u32(&page, o) as u64;
                    let (p2, f2, s2) = row_id(&page, o + 4);
                    self.append_record(out, f2, p2, s2, depth + 1, column)?;
                    if (out.len() - start) as u64 != cum {
                        return Err(crate::invalid!(
                            "LOB tree of {column}: assembled {} bytes where link {i} records {cum}",
                            out.len() - start
                        ));
                    }
                }
            }
            2 => {
                // INTERNAL
                let cur = le::u16(&page, r + 16) as usize;
                let start = out.len();
                for i in 0..cur {
                    let o = r + 20 + 16 * i;
                    if o + 16 > page.len() {
                        return Err(crate::invalid!("LOB tree of {column}: link {i} runs past the page"));
                    }
                    let cum = le::i64(&page, o);
                    let (p2, f2, s2) = row_id(&page, o + 8);
                    self.append_record(out, f2, p2, s2, depth + 1, column)?;
                    if (out.len() - start) as i64 != cum {
                        return Err(crate::invalid!(
                            "LOB tree of {column}: assembled {} bytes where internal link {i} records {cum}",
                            out.len() - start
                        ));
                    }
                }
            }
            t => {
                return Err(crate::unsupported!(
                    "LOB record type {t} on page 1:{page_id} slot {slot} (column {column}) — layout not derived, refusing to guess"
                ));
            }
        }
        Ok(true)
    }
}
