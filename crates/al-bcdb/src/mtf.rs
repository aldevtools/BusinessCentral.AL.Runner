//! Structural parser for the Microsoft Tape Format (MTF) container a SQL Server native
//! backup (`.bak`) is written in (port of `Mtf.cs`).
//!
//! Layout (observed on the BC demo backups; block/stream framing per the historically
//! published Microsoft Tape Format Specification 1.00a, bcdb PROVENANCE.md "Backup
//! container"):
//!
//! ```text
//!   TAPE  descriptor block: media header
//!   SFMB  soft filemark block (no stream chain)
//!   SSET  start-of-set: one backup set
//!   VOLB  volume descriptor
//!   MSCI  SQL Server: database/configuration info (MQCI stream)
//!   MSDA  SQL Server: one per data-copy section; carries an MQDA stream of raw 8192-byte blocks
//!   MSTL  SQL Server: transaction-log section; MQTL stream of log blocks
//!   MSLS  SQL Server: trailing section (not needed for reading data)
//! ```
//!
//! Every descriptor block has a 4-byte type tag, a u32 attribute word at +4 and a u16
//! "offset to first event" at +8 that points at its stream chain. Streams have a 22-byte
//! header: id[4], u16 fs-attributes, u16 media-attributes, u64 length, u16 encryption
//! algorithm, u16 compression algorithm, u16 checksum. Stream data is 4-byte aligned; an
//! SPAD stream pads to the next block boundary and ends the chain.
//!
//! Entry points: `DataRegion`, `MtfFile`.

use crate::error::{Error, Result};
use crate::io::{ByteSource, le};

pub const PAGE_SIZE: usize = 8192;

const KNOWN_DBLKS: [&str; 10] = [
    "TAPE", "SSET", "VOLB", "ESET", "EOTM", "SFMB", "MSCI", "MSDA", "MSTL", "MSLS",
];

/// A data-bearing stream extracted from the container (past its 2-byte lead).
#[derive(Debug, Clone)]
pub struct DataRegion {
    pub dblk: String,
    pub stream_id: String,
    pub data_offset: u64,
    pub block_count: u64,
}

#[derive(Debug)]
pub struct MtfFile {
    /// MQDA regions in file order: raw 8192-byte block sections of the data copy.
    pub mqda_regions: Vec<DataRegion>,
    /// Total bytes of MQTL (transaction log) stream data. The log is NOT replayed.
    pub log_stream_bytes: u64,
}

impl MtfFile {
    pub fn parse(src: &dyn ByteSource) -> Result<MtfFile> {
        let size = src.len();
        let mut regions = Vec::new();
        let mut log_bytes = 0u64;
        let mut hdr = [0u8; 60];
        let mut sh = [0u8; 22];
        let mut lead = [0u8; 2];
        let mut pos: i64 = 0;
        while pos >= 0 && pos as u64 + 60 <= size {
            let p = pos as u64;
            src.read_at(p, &mut hdr)?;
            let tag = String::from_utf8_lossy(&hdr[..4]).into_owned();
            if !KNOWN_DBLKS.contains(&tag.as_str()) {
                return Err(crate::invalid!(
                    "unknown MTF descriptor block '{tag}' at offset 0x{p:x} — not a SQL Server native backup, or a layout this reader has not seen"
                ));
            }
            let first_event = le::u16(&hdr, 8) as u64;
            if tag == "ESET" || tag == "EOTM" {
                break; // end of backup set
            }
            if tag == "SFMB" {
                pos += first_event as i64; // soft filemark: no stream chain
                continue;
            }
            let mut spos = p + first_event;
            let mut next: i64 = -1;
            while spos + 22 <= size {
                src.read_at(spos, &mut sh)?;
                if sh[..4].iter().any(|&c| !(b' '..=b'~').contains(&c)) {
                    return Err(crate::invalid!(
                        "invalid stream id at offset 0x{spos:x} inside {tag} block"
                    ));
                }
                let sid = String::from_utf8_lossy(&sh[..4]).into_owned();
                let slen = le::u64(&sh, 8);
                let enc = le::u16(&sh, 16);
                let comp = le::u16(&sh, 18);
                if enc != 0 {
                    return Err(crate::unsupported!(
                        "stream {sid}: encryption algorithm {enc} — encrypted backups are not supported"
                    ));
                }
                if comp != 0 {
                    return Err(crate::unsupported!(
                        "stream {sid}: compression algorithm {comp} — backups taken WITH COMPRESSION are not supported; take the backup without compression"
                    ));
                }
                if (sid == "MQDA" || sid == "MQTL") && slen > 2 {
                    // Stream data starts with 2 bytes (observed always 0x0000), then whole
                    // 8192-byte blocks. Validated: (len-2) is an exact multiple of 8192 on
                    // every MQDA/MQTL stream of the demo backups.
                    src.read_at(spos + 22, &mut lead)?;
                    if lead != [0, 0] {
                        return Err(crate::invalid!(
                            "stream {sid} at 0x{spos:x}: lead bytes {:02x}{:02x} != 0000 — layout differs from every observed backup, refusing to guess",
                            lead[0],
                            lead[1]
                        ));
                    }
                    if (slen - 2) % PAGE_SIZE as u64 != 0 {
                        return Err(crate::invalid!(
                            "stream {sid}: data length {slen}-2 is not a multiple of 8192"
                        ));
                    }
                    if sid == "MQDA" {
                        regions.push(DataRegion {
                            dblk: tag.clone(),
                            stream_id: sid.clone(),
                            data_offset: spos + 22 + 2,
                            block_count: (slen - 2) / PAGE_SIZE as u64,
                        });
                    } else {
                        log_bytes += slen - 2;
                    }
                }
                let data_end = spos + 22 + slen;
                if sid == "SPAD" {
                    next = data_end as i64; // SPAD pads to the block boundary and ends the chain
                    break;
                }
                spos = (data_end + 3) & !3u64; // 4-byte stream alignment
                if spos >= size {
                    break;
                }
            }
            if next < 0 {
                break;
            }
            pos = next;
        }
        if regions.is_empty() {
            return Err(Error::InvalidData(
                "no MQDA data stream found — not a SQL Server full database backup".into(),
            ));
        }
        Ok(MtfFile {
            mqda_regions: regions,
            log_stream_bytes: log_bytes,
        })
    }
}
