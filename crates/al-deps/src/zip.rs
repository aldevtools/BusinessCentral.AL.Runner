//! A small ZIP reader over a byte slice, and the NAVX wrapper of `.app` files.
//!
//! Every entry read is checked against the CRC-32 and the size in the central
//! directory (ADR-013 decision 1, integrity). Only what Business Central
//! packages use is supported: stored and deflate entries, no ZIP64, no
//! encryption. Anything else is a named error, never a guess.
//!
//! Entry points: `ZipError`, `Entry`, `Zip`, `EndRecord`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZipError {
    /// The bytes are not a ZIP archive (no end-of-central-directory record).
    NotAZip(String),
    Truncated(String),
    Unsupported(String),
    Crc {
        name: String,
        expected: u32,
        actual: u32,
    },
    Inflate {
        name: String,
        message: String,
    },
    Size {
        name: String,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for ZipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZipError::NotAZip(m) => write!(f, "not a ZIP archive: {m}"),
            ZipError::Truncated(m) => write!(f, "truncated ZIP archive: {m}"),
            ZipError::Unsupported(m) => write!(f, "unsupported ZIP feature: {m}"),
            ZipError::Crc { name, expected, actual } => {
                write!(
                    f,
                    "entry `{name}`: CRC-32 {actual:08x} does not match the recorded {expected:08x}"
                )
            }
            ZipError::Inflate { name, message } => write!(f, "entry `{name}`: {message}"),
            ZipError::Size { name, expected, actual } => {
                write!(f, "entry `{name}`: {actual} bytes after inflating, {expected} recorded")
            }
        }
    }
}

impl std::error::Error for ZipError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub size: usize,
    pub crc: u32,
    method: u16,
    compressed: usize,
    local_offset: usize,
}

pub struct Zip<'a> {
    data: &'a [u8],
    /// Bytes in front of the archive proper (the NAVX header when parsed whole).
    base: usize,
    pub entries: Vec<Entry>,
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

const EOCD: u32 = 0x0605_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;

/// How many bytes at the end of a ZIP file can hold its end record: the record is 22 bytes
/// and its comment is at most 64 KiB.
pub const END_SEARCH: usize = 22 + 65535;

/// The end-of-central-directory record, located in the tail of an archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndRecord {
    /// Where the record starts in the slice searched.
    pub at: usize,
    pub count: usize,
    pub cd_size: usize,
    /// Offset of the central directory from the start of the archive.
    pub cd_offset: usize,
}

/// Finds the end record in `tail`, the last bytes of an archive (or all of it).
pub fn find_end_record(tail: &[u8]) -> Result<EndRecord, ZipError> {
    let from = tail.len().saturating_sub(END_SEARCH);
    let at = (from..tail.len().saturating_sub(21))
        .rev()
        .find(|&i| u32_at(tail, i) == Some(EOCD))
        .ok_or_else(|| ZipError::NotAZip("no end-of-central-directory record".into()))?;
    let short = || ZipError::Truncated("end-of-central-directory record".into());
    let count = u16_at(tail, at + 10).ok_or_else(short)? as usize;
    let cd_size = u32_at(tail, at + 12).ok_or_else(short)? as usize;
    let cd_offset = u32_at(tail, at + 16).ok_or_else(short)? as usize;
    if count == 0xFFFF || cd_size == 0xFFFF_FFFF || cd_offset == 0xFFFF_FFFF {
        return Err(ZipError::Unsupported("ZIP64".into()));
    }
    Ok(EndRecord {
        at,
        count,
        cd_size,
        cd_offset,
    })
}

/// Parses `count` central directory entries from `cd`. Offsets in the entries are relative to
/// the start of the archive.
pub fn parse_central_directory(cd: &[u8], count: usize) -> Result<Vec<Entry>, ZipError> {
    let mut entries = Vec::with_capacity(count);
    let mut at = 0;
    for _ in 0..count {
        let bad = || ZipError::Truncated("central directory entry".into());
        if u32_at(cd, at) != Some(CENTRAL) {
            return Err(ZipError::NotAZip("bad central directory entry".into()));
        }
        let flags = u16_at(cd, at + 8).ok_or_else(bad)?;
        let method = u16_at(cd, at + 10).ok_or_else(bad)?;
        let crc = u32_at(cd, at + 16).ok_or_else(bad)?;
        let compressed = u32_at(cd, at + 20).ok_or_else(bad)?;
        let size = u32_at(cd, at + 24).ok_or_else(bad)?;
        let nlen = u16_at(cd, at + 28).ok_or_else(bad)? as usize;
        let xlen = u16_at(cd, at + 30).ok_or_else(bad)? as usize;
        let clen = u16_at(cd, at + 32).ok_or_else(bad)? as usize;
        let local_offset = u32_at(cd, at + 42).ok_or_else(bad)?;
        let name = cd.get(at + 46..at + 46 + nlen).ok_or_else(bad)?;
        if flags & 1 != 0 {
            return Err(ZipError::Unsupported("encrypted entries".into()));
        }
        if compressed == 0xFFFF_FFFF || size == 0xFFFF_FFFF || local_offset == 0xFFFF_FFFF {
            return Err(ZipError::Unsupported("ZIP64 entry".into()));
        }
        entries.push(Entry {
            name: String::from_utf8_lossy(name).into_owned(),
            size: size as usize,
            crc,
            method,
            compressed: compressed as usize,
            local_offset: local_offset as usize,
        });
        at += 46 + nlen + xlen + clen;
    }
    Ok(entries)
}

/// Size of the local header at the start of `head` (at least 30 bytes of it): the fixed part,
/// the name and the extra field. The entry's data follows.
pub fn local_header_len(head: &[u8], name: &str) -> Result<usize, ZipError> {
    if u32_at(head, 0) != Some(LOCAL) {
        return Err(ZipError::NotAZip(format!("entry `{name}` has no local header")));
    }
    let short = || ZipError::Truncated(format!("entry `{name}`"));
    let nlen = u16_at(head, 26).ok_or_else(short)? as usize;
    let xlen = u16_at(head, 28).ok_or_else(short)? as usize;
    Ok(30 + nlen + xlen)
}

impl Entry {
    /// Offset of the local header from the start of the archive.
    pub fn local_offset(&self) -> usize {
        self.local_offset
    }

    /// Size of the entry's data as stored (compressed).
    pub fn compressed_size(&self) -> usize {
        self.compressed
    }

    /// Inflates `raw`, the entry's stored bytes, and checks the recorded size and CRC-32.
    pub fn decode(&self, raw: &[u8]) -> Result<Vec<u8>, ZipError> {
        let e = self;
        let out = match e.method {
            0 => raw.to_vec(),
            8 => {
                use std::io::Read;
                let mut out = Vec::with_capacity(e.size);
                flate2::read::DeflateDecoder::new(raw)
                    .read_to_end(&mut out)
                    .map_err(|err| ZipError::Inflate {
                        name: e.name.clone(),
                        message: err.to_string(),
                    })?;
                out
            }
            m => {
                return Err(ZipError::Unsupported(format!(
                    "compression method {m} (entry `{}`)",
                    e.name
                )));
            }
        };
        if out.len() != e.size {
            return Err(ZipError::Size {
                name: e.name.clone(),
                expected: e.size,
                actual: out.len(),
            });
        }
        let actual = crc32fast::hash(&out);
        if actual != e.crc {
            return Err(ZipError::Crc {
                name: e.name.clone(),
                expected: e.crc,
                actual,
            });
        }
        Ok(out)
    }
}

/// An entry inflated piece by piece, for an entry too big to hold in memory (the demo backup).
/// Feed the stored bytes with [`EntryDecoder::push`] and end with [`EntryDecoder::finish`], which
/// checks the recorded size and CRC-32.
pub struct EntryDecoder {
    name: String,
    expected_size: usize,
    expected_crc: u32,
    inflate: Option<flate2::Decompress>,
    crc: crc32fast::Hasher,
    size: usize,
}

impl Entry {
    /// A decoder for this entry's stored bytes (method 0 or 8, like [`Entry::decode`]).
    pub fn decoder(&self) -> Result<EntryDecoder, ZipError> {
        let inflate = match self.method {
            0 => None,
            8 => Some(flate2::Decompress::new(false)),
            m => {
                return Err(ZipError::Unsupported(format!(
                    "compression method {m} (entry `{}`)",
                    self.name
                )));
            }
        };
        Ok(EntryDecoder {
            name: self.name.clone(),
            expected_size: self.size,
            expected_crc: self.crc,
            inflate,
            crc: crc32fast::Hasher::new(),
            size: 0,
        })
    }
}

impl EntryDecoder {
    /// Takes the next stored bytes and hands every inflated piece to `out`.
    pub fn push<E>(&mut self, chunk: &[u8], out: &mut dyn FnMut(&[u8]) -> Result<(), E>) -> Result<(), E>
    where
        E: From<ZipError>,
    {
        let Some(inflate) = self.inflate.as_mut() else {
            self.crc.update(chunk);
            self.size += chunk.len();
            return out(chunk);
        };
        let mut buf = vec![0u8; 256 << 10];
        let mut input = chunk;
        loop {
            let (before_in, before_out) = (inflate.total_in(), inflate.total_out());
            let status = inflate
                .decompress(input, &mut buf, flate2::FlushDecompress::None)
                .map_err(|err| ZipError::Inflate {
                    name: self.name.clone(),
                    message: err.to_string(),
                })?;
            let used = (inflate.total_in() - before_in) as usize;
            let made = (inflate.total_out() - before_out) as usize;
            input = &input[used..];
            if made > 0 {
                self.crc.update(&buf[..made]);
                self.size += made;
                out(&buf[..made])?;
            }
            if matches!(status, flate2::Status::StreamEnd) || (input.is_empty() && made < buf.len()) {
                return Ok(());
            }
            if used == 0 && made == 0 {
                return Ok(());
            }
        }
    }

    /// Checks the size and CRC-32 of everything pushed.
    pub fn finish(self) -> Result<(), ZipError> {
        if self.size != self.expected_size {
            return Err(ZipError::Size {
                name: self.name,
                expected: self.expected_size,
                actual: self.size,
            });
        }
        let actual = self.crc.finalize();
        if actual != self.expected_crc {
            return Err(ZipError::Crc {
                name: self.name,
                expected: self.expected_crc,
                actual,
            });
        }
        Ok(())
    }
}

impl<'a> Zip<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Zip<'a>, ZipError> {
        let end = find_end_record(data)?;
        // Offsets are relative to the start of the archive; bytes may precede it.
        let cd_start = end
            .at
            .checked_sub(end.cd_size)
            .ok_or_else(|| ZipError::Truncated("central directory".into()))?;
        let base = cd_start
            .checked_sub(end.cd_offset)
            .ok_or_else(|| ZipError::NotAZip("central directory offset is past its position".into()))?;
        let entries = parse_central_directory(&data[cd_start..end.at], end.count)?;
        Ok(Zip { data, base, entries })
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// The entry's bytes, inflated and checked against the recorded size and CRC-32.
    pub fn read(&self, e: &Entry) -> Result<Vec<u8>, ZipError> {
        let lh = self.base + e.local_offset;
        let head = self.data.get(lh..).unwrap_or_default();
        let len = local_header_len(head, &e.name)?;
        let start = lh + len;
        let raw = self
            .data
            .get(start..start + e.compressed)
            .ok_or_else(|| ZipError::Truncated(format!("entry `{}`", e.name)))?;
        e.decode(raw)
    }
}

/// The ZIP archive inside an `.app` file. A Business Central package is a
/// 40-byte NAVX header (`NAVX`, header size, version, a GUID, the payload
/// length, `NAVX`) followed by a plain ZIP archive. A plain ZIP passes through.
pub fn navx_payload(data: &[u8]) -> Result<&[u8], ZipError> {
    if !data.starts_with(b"NAVX") {
        return Ok(data);
    }
    let short = || ZipError::Truncated("NAVX header".into());
    let header = u32_at(data, 4).ok_or_else(short)? as usize;
    if header < 12 || header > data.len() {
        return Err(ZipError::NotAZip(format!("NAVX header size {header}")));
    }
    let version = u32_at(data, 8).ok_or_else(short)?;
    if version != 2 {
        return Err(ZipError::Unsupported(format!(
            "NAVX version {version} (only 2 is known)"
        )));
    }
    let payload = &data[header..];
    // The header records the payload length; a shorter file was cut off.
    if header >= 36 {
        let len = u64::from_le_bytes(data[28..36].try_into().expect("8 bytes"));
        if len > payload.len() as u64 {
            return Err(ZipError::Truncated(format!(
                "payload is {} bytes, the header records {len}",
                payload.len()
            )));
        }
    }
    Ok(payload)
}

#[cfg(test)]
pub(crate) mod testutil {
    //! Builds small ZIP and NAVX files for the tests. Nothing here is Microsoft's.

    use std::io::Write;

    pub struct Builder {
        out: Vec<u8>,
        central: Vec<u8>,
        count: u16,
    }

    impl Builder {
        pub fn new() -> Builder {
            Builder {
                out: Vec::new(),
                central: Vec::new(),
                count: 0,
            }
        }

        /// Adds an entry; `deflate` compresses it, otherwise it is stored.
        pub fn add(self, name: &str, data: &[u8], deflate: bool) -> Builder {
            let crc = crc32fast::hash(data);
            self.add_raw(name, data, deflate, crc)
        }

        /// Like `add`, but records a wrong CRC.
        pub fn add_with_crc(self, name: &str, data: &[u8], crc: u32) -> Builder {
            self.add_raw(name, data, false, crc)
        }

        fn add_raw(mut self, name: &str, data: &[u8], deflate: bool, crc: u32) -> Builder {
            let body = if deflate {
                let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(data).unwrap();
                e.finish().unwrap()
            } else {
                data.to_vec()
            };
            let method: u16 = if deflate { 8 } else { 0 };
            let offset = self.out.len() as u32;
            let mut local = Vec::new();
            local.extend(0x0403_4b50u32.to_le_bytes());
            local.extend([20, 0, 0, 0]);
            local.extend(method.to_le_bytes());
            local.extend([0, 0, 0, 0]);
            local.extend(crc.to_le_bytes());
            local.extend((body.len() as u32).to_le_bytes());
            local.extend((data.len() as u32).to_le_bytes());
            local.extend((name.len() as u16).to_le_bytes());
            local.extend([0, 0]);
            local.extend(name.as_bytes());
            self.out.extend(local);
            self.out.extend(&body);
            let c = &mut self.central;
            c.extend(0x0201_4b50u32.to_le_bytes());
            c.extend([20, 0, 20, 0, 0, 0]);
            c.extend(method.to_le_bytes());
            c.extend([0, 0, 0, 0]);
            c.extend(crc.to_le_bytes());
            c.extend((body.len() as u32).to_le_bytes());
            c.extend((data.len() as u32).to_le_bytes());
            c.extend((name.len() as u16).to_le_bytes());
            c.extend([0; 6]); // extra, comment, disk
            c.extend([0; 6]); // internal and external attributes
            c.extend(offset.to_le_bytes());
            c.extend(name.as_bytes());
            self.count += 1;
            self
        }

        pub fn finish(self) -> Vec<u8> {
            let mut out = self.out;
            let cd_offset = out.len() as u32;
            let cd_size = self.central.len() as u32;
            out.extend(self.central);
            out.extend(0x0605_4b50u32.to_le_bytes());
            out.extend([0, 0, 0, 0]);
            out.extend(self.count.to_le_bytes());
            out.extend(self.count.to_le_bytes());
            out.extend(cd_size.to_le_bytes());
            out.extend(cd_offset.to_le_bytes());
            out.extend([0, 0]);
            out
        }
    }

    /// Wraps a ZIP in the 40-byte NAVX header.
    pub fn navx(zip: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(b"NAVX");
        out.extend(40u32.to_le_bytes());
        out.extend(2u32.to_le_bytes());
        out.extend([7u8; 16]);
        out.extend((zip.len() as u64).to_le_bytes());
        out.extend(b"NAVX");
        out.extend(zip);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::*;
    use super::*;

    fn sample() -> Vec<u8> {
        Builder::new()
            .add("a.txt", b"hello", false)
            .add("dir/b.al", "codeunit 1 C { }\n".repeat(50).as_bytes(), true)
            .add("empty", b"", false)
            .finish()
    }

    #[test]
    fn reads_stored_and_deflated_entries() {
        let data = sample();
        let z = Zip::parse(&data).unwrap();
        assert_eq!(
            z.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["a.txt", "dir/b.al", "empty"]
        );
        assert_eq!(z.read(z.find("a.txt").unwrap()).unwrap(), b"hello");
        assert_eq!(
            z.read(z.find("dir/b.al").unwrap()).unwrap(),
            "codeunit 1 C { }\n".repeat(50).as_bytes()
        );
        assert_eq!(z.read(z.find("empty").unwrap()).unwrap(), b"");
    }

    #[test]
    fn a_crc_mismatch_is_an_error_naming_the_entry() {
        let data = Builder::new()
            .add_with_crc("bad.al", b"codeunit 1 C { }", 0xdead_beef)
            .finish();
        let z = Zip::parse(&data).unwrap();
        let err = z.read(&z.entries[0]).unwrap_err();
        assert!(
            matches!(&err, ZipError::Crc { name, expected: 0xdead_beef, .. } if name == "bad.al"),
            "{err:?}"
        );
        assert!(err.to_string().contains("bad.al"));
    }

    #[test]
    fn a_flipped_byte_in_the_data_is_caught_by_the_crc() {
        let mut data = Builder::new().add("a.txt", b"hello world", false).finish();
        let at = data.windows(5).position(|w| w == b"hello").unwrap();
        data[at] ^= 1;
        let z = Zip::parse(&data).unwrap();
        assert!(matches!(z.read(&z.entries[0]), Err(ZipError::Crc { .. })));
    }

    #[test]
    fn garbage_and_truncated_archives_are_errors() {
        assert!(matches!(Zip::parse(b"not a zip at all"), Err(ZipError::NotAZip(_))));
        let data = sample();
        assert!(Zip::parse(&data[..data.len() - 30]).is_err());
        // A cut inside an entry's data.
        let z = Zip::parse(&data).unwrap();
        let mut broken = data.clone();
        broken.truncate(10);
        assert!(Zip::parse(&broken).is_err());
        assert!(z.read(&z.entries[0]).is_ok());
    }

    #[test]
    fn offsets_are_relative_to_the_archive_when_bytes_precede_it() {
        let mut data = vec![0xAA; 123];
        data.extend(sample());
        let z = Zip::parse(&data).unwrap();
        assert_eq!(z.read(z.find("a.txt").unwrap()).unwrap(), b"hello");
    }

    #[test]
    fn the_navx_header_is_skipped_and_a_plain_zip_passes_through() {
        let zip = sample();
        let app = navx(&zip);
        assert_eq!(navx_payload(&app).unwrap(), &zip[..]);
        assert_eq!(navx_payload(&zip).unwrap(), &zip[..]);
        let z = Zip::parse(navx_payload(&app).unwrap()).unwrap();
        assert_eq!(z.entries.len(), 3);
    }

    #[test]
    fn a_navx_file_cut_short_is_reported_not_misread() {
        let app = navx(&sample());
        let err = navx_payload(&app[..app.len() - 10]).unwrap_err();
        assert!(matches!(err, ZipError::Truncated(_)), "{err:?}");
        assert!(navx_payload(b"NAVX\x28\x00").is_err());
    }

    #[test]
    fn unsupported_methods_are_named() {
        let mut data = Builder::new().add("a.txt", b"hello", false).finish();
        // Patch the central directory's method field to 12 (bzip2).
        let cd = data
            .windows(4)
            .rposition(|w| w == 0x0201_4b50u32.to_le_bytes())
            .unwrap();
        data[cd + 10] = 12;
        let z = Zip::parse(&data).unwrap();
        let err = z.read(&z.entries[0]).unwrap_err();
        assert!(
            matches!(&err, ZipError::Unsupported(m) if m.contains("method 12")),
            "{err:?}"
        );
    }
}
