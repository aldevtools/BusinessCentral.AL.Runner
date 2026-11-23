//! A zip central-directory reader over a [`ByteSource`]: enough of the format for a
//! `.bacpac` (Open Packaging) and a NAVX `.app` payload. Stored and deflated entries,
//! zip64 sizes and offsets. Sizes come from the central directory only, so entries
//! written with a trailing data descriptor read the same.
//!
//! Entry points: `ZipEntry`, `ZipIndex`.

use crate::error::{Error, Result};
use crate::io::{ByteSource, le};
use flate2::read::DeflateDecoder;
use std::io::{self, BufReader, Read};

#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub method: u16,
    pub compressed_size: u64,
    pub size: u64,
    pub local_offset: u64,
}

#[derive(Debug, Clone)]
pub struct ZipIndex {
    pub entries: Vec<ZipEntry>,
}

const EOCD_SIG: u32 = 0x0605_4b50;
const EOCD64_SIG: u32 = 0x0606_4b50;
const LOCATOR64_SIG: u32 = 0x0706_4b50;
const CENTRAL_SIG: u32 = 0x0201_4b50;
const LOCAL_SIG: u32 = 0x0403_4b50;

fn not_a_zip(why: &str) -> Error {
    crate::invalid!("not a zip container — a .bacpac is an Open Packaging archive ({why})")
}

impl ZipIndex {
    pub fn parse(src: &dyn ByteSource) -> Result<ZipIndex> {
        let len = src.len();
        if len < 22 {
            return Err(not_a_zip("shorter than an end-of-central-directory record"));
        }
        // The end record is within the last 22 + 65535 bytes (a comment may follow it).
        let tail_len = len.min(22 + 65_535) as usize;
        let tail_off = len - tail_len as u64;
        let tail = src.slice(tail_off, tail_len)?;
        let eocd = (0..=tail_len - 22)
            .rev()
            .find(|&i| le::u32(&tail, i) == EOCD_SIG)
            .ok_or_else(|| not_a_zip("no end-of-central-directory record"))?;
        let mut count = le::u16(&tail, eocd + 10) as u64;
        let mut cd_size = le::u32(&tail, eocd + 12) as u64;
        let mut cd_off = le::u32(&tail, eocd + 16) as u64;
        if count == 0xFFFF || cd_size == 0xFFFF_FFFF || cd_off == 0xFFFF_FFFF {
            let loc_pos = tail_off + eocd as u64;
            if loc_pos < 20 {
                return Err(not_a_zip("zip64 record without a locator"));
            }
            let mut loc = [0u8; 20];
            src.read_at(loc_pos - 20, &mut loc)?;
            if le::u32(&loc, 0) != LOCATOR64_SIG {
                return Err(not_a_zip("zip64 end record has no locator"));
            }
            let e64_off = le::u64(&loc, 8);
            let mut e64 = [0u8; 56];
            src.read_at(e64_off, &mut e64)?;
            if le::u32(&e64, 0) != EOCD64_SIG {
                return Err(not_a_zip("bad zip64 end-of-central-directory record"));
            }
            count = le::u64(&e64, 32);
            cd_size = le::u64(&e64, 40);
            cd_off = le::u64(&e64, 48);
        }
        if cd_off.checked_add(cd_size).is_none_or(|e| e > len) {
            return Err(not_a_zip("central directory lies outside the file"));
        }
        let cd = src.slice(cd_off, cd_size as usize)?;
        let mut entries = Vec::with_capacity(count as usize);
        let mut p = 0usize;
        for _ in 0..count {
            if p + 46 > cd.len() || le::u32(&cd, p) != CENTRAL_SIG {
                return Err(not_a_zip("damaged central directory"));
            }
            let method = le::u16(&cd, p + 10);
            let mut csize = le::u32(&cd, p + 20) as u64;
            let mut size = le::u32(&cd, p + 24) as u64;
            let (nlen, xlen, clen) = (
                le::u16(&cd, p + 28) as usize,
                le::u16(&cd, p + 30) as usize,
                le::u16(&cd, p + 32) as usize,
            );
            let mut off = le::u32(&cd, p + 42) as u64;
            if p + 46 + nlen + xlen + clen > cd.len() {
                return Err(not_a_zip("damaged central directory"));
            }
            let name = String::from_utf8_lossy(&cd[p + 46..p + 46 + nlen]).into_owned();
            // zip64 extra field (id 1): the values that did not fit, in this order
            let extra = &cd[p + 46 + nlen..p + 46 + nlen + xlen];
            let mut x = 0usize;
            while x + 4 <= extra.len() {
                let (id, sz) = (le::u16(extra, x), le::u16(extra, x + 2) as usize);
                if id == 1 {
                    let mut q = x + 4;
                    let mut next = |want: bool, out: &mut u64| -> Result<()> {
                        if want {
                            if q + 8 > x + 4 + sz {
                                return Err(not_a_zip("short zip64 extra field"));
                            }
                            *out = le::u64(extra, q);
                            q += 8;
                        }
                        Ok(())
                    };
                    next(size == 0xFFFF_FFFF, &mut size)?;
                    next(csize == 0xFFFF_FFFF, &mut csize)?;
                    next(off == 0xFFFF_FFFF, &mut off)?;
                }
                x += 4 + sz;
            }
            entries.push(ZipEntry {
                name,
                method,
                compressed_size: csize,
                size,
                local_offset: off,
            });
            p += 46 + nlen + xlen + clen;
        }
        Ok(ZipIndex { entries })
    }

    pub fn find(&self, name: &str) -> Option<&ZipEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// A streaming reader over the entry's uncompressed bytes.
    pub fn open<'a>(&self, src: &'a dyn ByteSource, e: &ZipEntry) -> Result<Box<dyn Read + 'a>> {
        let mut h = [0u8; 30];
        src.read_at(e.local_offset, &mut h)?;
        if le::u32(&h, 0) != LOCAL_SIG {
            return Err(crate::invalid!("zip entry {}: bad local header", e.name));
        }
        let data = e.local_offset + 30 + le::u16(&h, 26) as u64 + le::u16(&h, 28) as u64;
        if data.checked_add(e.compressed_size).is_none_or(|end| end > src.len()) {
            return Err(crate::invalid!("zip entry {}: data lies outside the file", e.name));
        }
        let raw = SourceRead {
            src,
            pos: data,
            end: data + e.compressed_size,
        };
        match e.method {
            0 => Ok(Box::new(raw)),
            8 => Ok(Box::new(DeflateDecoder::new(BufReader::with_capacity(64 * 1024, raw)))),
            m => Err(crate::unsupported!("zip entry {}: compression method {m}", e.name)),
        }
    }

    pub fn read(&self, src: &dyn ByteSource, name: &str) -> Result<Vec<u8>> {
        let e = self
            .find(name)
            .ok_or_else(|| crate::invalid!("the package has no {name} — not a bacpac?"))?;
        let mut v = Vec::with_capacity(e.size as usize);
        self.open(src, e)?.read_to_end(&mut v)?;
        Ok(v)
    }
}

struct SourceRead<'a> {
    src: &'a dyn ByteSource,
    pos: u64,
    end: u64,
}

impl Read for SourceRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min((self.end - self.pos) as usize);
        if n == 0 {
            return Ok(0);
        }
        self.src
            .read_at(self.pos, &mut buf[..n])
            .map_err(|e| io::Error::other(e.to_string()))?;
        self.pos += n as u64;
        Ok(n)
    }
}
