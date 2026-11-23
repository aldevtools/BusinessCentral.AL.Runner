//! A ZIP archive read by byte ranges: the central directory first, then only the entries wanted
//! (ADR-013 decision 1). The platform artifact is 1.3 GB; the test libraries are about 200 KB of it.
//!
//! The ranges come from a [`RangeSource`]: an HTTP server that honours `Range` (Microsoft's
//! artifact CDN does) or a local file (the tests' fixture). Every entry is checked against the
//! CRC-32 and size in the central directory, exactly as for a local archive.
//!
//! Entry points: `RemoteError`, `RangeSource`, `FileSource`, `HttpSource`.

use crate::zip::{self, Entry, ZipError};
use std::fmt;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteError {
    /// The server has no such file (HTTP 404). `url` names it.
    NotFound {
        url: String,
    },
    /// The server did not answer, or answered with an error status.
    Http {
        url: String,
        message: String,
    },
    /// The server ignored the `Range` header (it answered 200 with the whole file). The runtime
    /// never downloads a gigabyte to read a few kilobytes.
    NoRangeSupport {
        url: String,
    },
    /// The answer was not the bytes asked for.
    Short {
        url: String,
        wanted: u64,
        got: u64,
    },
    Zip(ZipError),
    Io(String),
}

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemoteError::NotFound { url } => write!(f, "{url}: not found (HTTP 404)"),
            RemoteError::Http { url, message } => write!(f, "{url}: {message}"),
            RemoteError::NoRangeSupport { url } => {
                write!(f, "{url}: the server does not support HTTP range requests")
            }
            RemoteError::Short { url, wanted, got } => {
                write!(f, "{url}: asked for {wanted} bytes and got {got}")
            }
            RemoteError::Zip(e) => write!(f, "{e}"),
            RemoteError::Io(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for RemoteError {}

impl From<ZipError> for RemoteError {
    fn from(e: ZipError) -> RemoteError {
        RemoteError::Zip(e)
    }
}

/// Random access to the bytes of one file.
pub trait RangeSource {
    /// The total length of the file and its last `n` bytes (all of it when shorter).
    fn tail(&self, n: u64) -> Result<(u64, Vec<u8>), RemoteError>;
    /// Exactly `len` bytes from `start`. Asking past the end is an error.
    fn range(&self, start: u64, len: u64) -> Result<Vec<u8>, RemoteError>;
    /// Exactly `len` bytes from `start`, handed to `sink` in pieces as they arrive. For an entry
    /// too big to hold in memory. The default reads the pieces with [`RangeSource::range`].
    fn stream(
        &self,
        start: u64,
        len: u64,
        sink: &mut dyn FnMut(&[u8]) -> Result<(), RemoteError>,
    ) -> Result<(), RemoteError> {
        const PIECE: u64 = 8 << 20;
        let mut done = 0;
        while done < len {
            let n = PIECE.min(len - done);
            sink(&self.range(start + done, n)?)?;
            done += n;
        }
        Ok(())
    }
}

/// A local file as a range source.
pub struct FileSource(pub PathBuf);

impl FileSource {
    fn open(&self) -> Result<std::fs::File, RemoteError> {
        std::fs::File::open(&self.0).map_err(|e| RemoteError::Io(format!("{}: {e}", self.0.display())))
    }
}

impl RangeSource for FileSource {
    fn tail(&self, n: u64) -> Result<(u64, Vec<u8>), RemoteError> {
        let mut f = self.open()?;
        let total = f.metadata().map_err(|e| RemoteError::Io(e.to_string()))?.len();
        let start = total.saturating_sub(n);
        let bytes = read_at(&mut f, start, total - start, &self.0.display().to_string())?;
        Ok((total, bytes))
    }

    fn range(&self, start: u64, len: u64) -> Result<Vec<u8>, RemoteError> {
        let mut f = self.open()?;
        read_at(&mut f, start, len, &self.0.display().to_string())
    }
}

fn read_at(f: &mut std::fs::File, start: u64, len: u64, what: &str) -> Result<Vec<u8>, RemoteError> {
    f.seek(SeekFrom::Start(start))
        .map_err(|e| RemoteError::Io(format!("{what}: {e}")))?;
    let mut out = Vec::new();
    f.take(len)
        .read_to_end(&mut out)
        .map_err(|e| RemoteError::Io(format!("{what}: {e}")))?;
    if out.len() as u64 != len {
        return Err(RemoteError::Short {
            url: what.to_string(),
            wanted: len,
            got: out.len() as u64,
        });
    }
    Ok(out)
}

/// A file on an HTTP server that honours `Range`.
pub struct HttpSource {
    agent: ureq::Agent,
    pub url: String,
}

/// The largest single answer accepted (a ranged read of one entry; the biggest wanted is Base
/// Application's 27 MB source archive).
const MAX_BODY: u64 = 512 << 20;
const ATTEMPTS: u32 = 3;

pub fn http_agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(300)))
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .user_agent(concat!("al-runner/", env!("CARGO_PKG_VERSION")))
        .http_status_as_error(false)
        .build();
    ureq::Agent::new_with_config(config)
}

/// GETs `url` with retries on network errors and 5xx/429. Returns the status and body of the
/// last attempt. A non-error status is returned as is; the caller judges it.
fn get(agent: &ureq::Agent, url: &str, range: Option<&str>) -> Result<(u16, Option<String>, Vec<u8>), RemoteError> {
    let mut last = String::new();
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(400 * (1 << attempt)));
        }
        let mut req = agent.get(url);
        if let Some(r) = range {
            req = req.header("Range", r);
        }
        match req.call() {
            Ok(mut resp) => {
                let status = resp.status().as_u16();
                let content_range = resp
                    .headers()
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string);
                if status >= 500 || status == 429 {
                    last = format!("HTTP {status}");
                    continue;
                }
                let body = resp
                    .body_mut()
                    .with_config()
                    .limit(MAX_BODY)
                    .read_to_vec()
                    .map_err(|e| RemoteError::Http {
                        url: url.to_string(),
                        message: format!("reading the answer: {e}"),
                    });
                match body {
                    Ok(b) => return Ok((status, content_range, b)),
                    Err(e) => last = e.to_string(),
                }
            }
            Err(e) => last = e.to_string(),
        }
    }
    Err(RemoteError::Http {
        url: url.to_string(),
        message: format!("no answer after {ATTEMPTS} attempts: {last}"),
    })
}

impl HttpSource {
    pub fn new(agent: ureq::Agent, url: impl Into<String>) -> HttpSource {
        HttpSource { agent, url: url.into() }
    }
}

/// `bytes 100-199/1000` to the total, 1000.
fn total_of(content_range: &str) -> Option<u64> {
    content_range.rsplit('/').next()?.trim().parse().ok()
}

impl RangeSource for HttpSource {
    fn tail(&self, n: u64) -> Result<(u64, Vec<u8>), RemoteError> {
        let (status, cr, body) = get(&self.agent, &self.url, Some(&format!("bytes=-{n}")))?;
        match status {
            404 => Err(RemoteError::NotFound { url: self.url.clone() }),
            206 => {
                let total = cr.as_deref().and_then(total_of).ok_or_else(|| RemoteError::Http {
                    url: self.url.clone(),
                    message: "206 answer without a Content-Range total".into(),
                })?;
                Ok((total, body))
            }
            // A file shorter than the suffix may be answered in full with 200 by some servers; only
            // accept that when it really is short.
            200 if (body.len() as u64) <= n => Ok((body.len() as u64, body)),
            200 => Err(RemoteError::NoRangeSupport { url: self.url.clone() }),
            s => Err(RemoteError::Http {
                url: self.url.clone(),
                message: format!("HTTP {s}"),
            }),
        }
    }

    fn range(&self, start: u64, len: u64) -> Result<Vec<u8>, RemoteError> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let (status, _, body) = get(
            &self.agent,
            &self.url,
            Some(&format!("bytes={start}-{}", start + len - 1)),
        )?;
        match status {
            404 => Err(RemoteError::NotFound { url: self.url.clone() }),
            206 if body.len() as u64 == len => Ok(body),
            206 => Err(RemoteError::Short {
                url: self.url.clone(),
                wanted: len,
                got: body.len() as u64,
            }),
            200 => Err(RemoteError::NoRangeSupport { url: self.url.clone() }),
            s => Err(RemoteError::Http {
                url: self.url.clone(),
                message: format!("HTTP {s}"),
            }),
        }
    }

    /// One request for the whole range, read as it arrives. A broken connection continues from
    /// the byte reached (up to [`ATTEMPTS`] times in a row without progress).
    fn stream(
        &self,
        start: u64,
        len: u64,
        sink: &mut dyn FnMut(&[u8]) -> Result<(), RemoteError>,
    ) -> Result<(), RemoteError> {
        use std::io::Read;
        let mut done = 0u64;
        let mut failures = 0;
        let mut last = String::new();
        let http = |message: String| RemoteError::Http {
            url: self.url.clone(),
            message,
        };
        while done < len {
            if failures >= ATTEMPTS {
                return Err(http(format!(
                    "the download stopped after {done} of {len} bytes: {last}"
                )));
            }
            if failures > 0 {
                std::thread::sleep(std::time::Duration::from_millis(400 * (1 << failures)));
            }
            let from = start + done;
            let resp = self
                .agent
                .get(&self.url)
                .header("Range", &format!("bytes={from}-{}", start + len - 1))
                .call();
            let mut resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    failures += 1;
                    last = e.to_string();
                    continue;
                }
            };
            match resp.status().as_u16() {
                206 => {}
                404 => return Err(RemoteError::NotFound { url: self.url.clone() }),
                200 => return Err(RemoteError::NoRangeSupport { url: self.url.clone() }),
                s if s >= 500 || s == 429 => {
                    failures += 1;
                    last = format!("HTTP {s}");
                    continue;
                }
                s => return Err(http(format!("HTTP {s}"))),
            }
            let mut body = resp.body_mut().as_reader();
            let mut buf = vec![0u8; 256 << 10];
            let before = done;
            loop {
                match body.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let n = n.min((len - done) as usize);
                        sink(&buf[..n])?;
                        done += n as u64;
                        if done >= len {
                            break;
                        }
                    }
                    Err(e) => {
                        last = e.to_string();
                        break;
                    }
                }
            }
            if done == before {
                failures += 1;
            } else {
                failures = 0;
            }
        }
        Ok(())
    }
}

/// Fetches a small whole file (the artifact index).
pub fn fetch_all(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, RemoteError> {
    let (status, _, body) = get(agent, url, None)?;
    match status {
        200 => Ok(body),
        404 => Err(RemoteError::NotFound { url: url.to_string() }),
        s => Err(RemoteError::Http {
            url: url.to_string(),
            message: format!("HTTP {s}"),
        }),
    }
}

/// An archive whose central directory is in memory and whose entries are read by range.
pub struct RemoteZip<S> {
    src: S,
    total: u64,
    /// The central directory followed by the end record: the last `tail.len()` bytes of the file.
    tail: Vec<u8>,
    /// Bytes in front of the archive proper (zero for the artifacts).
    base: u64,
    pub entries: Vec<Entry>,
}

/// Slack added to the guess of a local header's size so that one request usually covers it.
const HEADER_SLACK: u64 = 256;

impl<S: RangeSource> RemoteZip<S> {
    /// Reads the end record and the central directory: one request for the end, one more when the
    /// central directory is larger than what the first one returned.
    pub fn open(src: S) -> Result<RemoteZip<S>, RemoteError> {
        let (total, mut tail) = src.tail(zip::END_SEARCH as u64)?;
        let end = zip::find_end_record(&tail)?;
        if end.cd_size > end.at {
            let missing = (end.cd_size - end.at) as u64;
            let have = tail.len() as u64;
            let start = total
                .checked_sub(have + missing)
                .ok_or_else(|| ZipError::Truncated("central directory".into()))?;
            let mut more = src.range(start, missing)?;
            more.extend(tail);
            tail = more;
        }
        RemoteZip::from_tail(src, total, tail)
    }

    /// Rebuilds the archive from a saved tail (see [`RemoteZip::tail`]) with no request.
    pub fn from_tail(src: S, total: u64, tail: Vec<u8>) -> Result<RemoteZip<S>, RemoteError> {
        let end = zip::find_end_record(&tail)?;
        let cd_start = end
            .at
            .checked_sub(end.cd_size)
            .ok_or_else(|| ZipError::Truncated("central directory".into()))?;
        let tail_start = total
            .checked_sub(tail.len() as u64)
            .ok_or_else(|| ZipError::Truncated("saved tail is longer than the file".into()))?;
        let base = (tail_start + cd_start as u64)
            .checked_sub(end.cd_offset as u64)
            .ok_or_else(|| ZipError::NotAZip("central directory offset is past its position".into()))?;
        let entries = zip::parse_central_directory(&tail[cd_start..end.at], end.count)?;
        Ok(RemoteZip {
            src,
            total,
            tail,
            base,
            entries,
        })
    }

    /// The file length and the bytes to save so that [`RemoteZip::from_tail`] can reopen it.
    pub fn tail(&self) -> (u64, &[u8]) {
        (self.total, &self.tail)
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// The entry's bytes, fetched by range, inflated and checked against the recorded size and CRC-32.
    pub fn read(&self, e: &Entry) -> Result<Vec<u8>, RemoteError> {
        let at = self.base + e.local_offset() as u64;
        let guess = 30 + e.name.len() as u64 + HEADER_SLACK;
        let want = (guess + e.compressed_size() as u64).min(self.total.saturating_sub(at));
        let buf = self.src.range(at, want)?;
        let header = zip::local_header_len(&buf, &e.name)?;
        let end = header + e.compressed_size();
        let raw = if let Some(raw) = buf.get(header..end) {
            raw.to_vec()
        } else {
            // The local header was longer than guessed: read the data on its own.
            self.src.range(at + header as u64, e.compressed_size() as u64)?
        };
        Ok(e.decode(&raw)?)
    }
}

impl<S: RangeSource> RemoteZip<S> {
    /// Streams the entry's inflated bytes to `sink` without holding them in memory, and checks the
    /// recorded size and CRC-32 at the end. `progress` gets the stored bytes read so far.
    pub fn stream_entry(
        &self,
        e: &Entry,
        sink: &mut dyn FnMut(&[u8]) -> Result<(), RemoteError>,
        progress: &mut dyn FnMut(u64),
    ) -> Result<(), RemoteError> {
        let at = self.base + e.local_offset() as u64;
        let guess = (30 + e.name.len() as u64 + HEADER_SLACK).min(self.total.saturating_sub(at));
        let head = self.src.range(at, guess)?;
        let header = zip::local_header_len(&head, &e.name)? as u64;
        let mut decoder = e.decoder()?;
        let mut read = 0u64;
        self.src.stream(at + header, e.compressed_size() as u64, &mut |chunk| {
            decoder.push::<RemoteError>(chunk, sink)?;
            read += chunk.len() as u64;
            progress(read);
            Ok(())
        })?;
        decoder.finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::testutil::Builder;
    use std::cell::{Cell, RefCell};

    /// A source that counts what it serves.
    struct Counting {
        inner: FileSource,
        bytes: Cell<u64>,
        requests: RefCell<Vec<(u64, u64)>>,
    }

    impl Counting {
        fn new(path: PathBuf) -> Counting {
            Counting {
                inner: FileSource(path),
                bytes: Cell::new(0),
                requests: RefCell::new(Vec::new()),
            }
        }
    }

    impl RangeSource for &Counting {
        fn tail(&self, n: u64) -> Result<(u64, Vec<u8>), RemoteError> {
            let (t, b) = self.inner.tail(n)?;
            self.bytes.set(self.bytes.get() + b.len() as u64);
            self.requests.borrow_mut().push((t - b.len() as u64, b.len() as u64));
            Ok((t, b))
        }
        fn range(&self, start: u64, len: u64) -> Result<Vec<u8>, RemoteError> {
            let b = self.inner.range(start, len)?;
            self.bytes.set(self.bytes.get() + len);
            self.requests.borrow_mut().push((start, len));
            Ok(b)
        }
    }

    /// A multi-megabyte archive with a central directory bigger than the 64 KiB end search:
    /// 3000 filler entries of incompressible bytes around three entries the tests read.
    /// Everything in it is generated here; none of it is Microsoft's.
    fn fixture(dir: &std::path::Path) -> (PathBuf, Vec<(String, Vec<u8>)>) {
        let mut state = 0x1234_5678_u32;
        let mut noise = |n: usize| -> Vec<u8> {
            (0..n)
                .map(|_| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (state >> 24) as u8
                })
                .collect()
        };
        let wanted = vec![
            ("Applications/Lib/Any.Source.zip".to_string(), noise(3000)),
            (
                "Applications/Lib/Big Library.Source.zip".to_string(),
                "codeunit 1 C { }\n".repeat(4000).into_bytes(),
            ),
            ("Applications/Lib/Empty.txt".to_string(), Vec::new()),
        ];
        let mut b = Builder::new();
        for i in 0..1500 {
            b = b.add(&format!("filler/{i:05}-{}.bin", "x".repeat(40)), &noise(1500), false);
        }
        b = b.add(&wanted[0].0, &wanted[0].1, false);
        b = b.add(&wanted[1].0, &wanted[1].1, true);
        b = b.add(&wanted[2].0, &wanted[2].1, false);
        for i in 1500..3000 {
            b = b.add(&format!("filler/{i:05}-{}.bin", "y".repeat(40)), &noise(1500), false);
        }
        let path = dir.join("artifact.zip");
        std::fs::write(&path, b.finish()).unwrap();
        (path, wanted)
    }

    #[test]
    fn opens_a_large_archive_and_reads_only_the_entries_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let (path, wanted) = fixture(dir.path());
        let file_len = std::fs::metadata(&path).unwrap().len();
        assert!(
            file_len > 4_000_000,
            "the fixture should be several megabytes: {file_len}"
        );
        let src = Counting::new(path);
        let z = RemoteZip::open(&src).unwrap();
        assert_eq!(z.entries.len(), 3003);
        // The central directory is bigger than the first 64 KiB read: exactly two requests so far.
        assert_eq!(src.requests.borrow().len(), 2, "{:?}", src.requests.borrow());
        for (name, bytes) in &wanted {
            let e = z.find(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(&z.read(e).unwrap(), bytes, "{name}");
        }
        // One request per entry, and far less than the file.
        assert_eq!(src.requests.borrow().len(), 2 + 3);
        assert!(
            src.bytes.get() < file_len / 10,
            "fetched {} of {file_len} bytes",
            src.bytes.get()
        );
    }

    #[test]
    fn a_small_archive_is_read_whole_by_the_first_request() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.zip");
        std::fs::write(&path, Builder::new().add("a.txt", b"hello", false).finish()).unwrap();
        let src = Counting::new(path);
        let z = RemoteZip::open(&src).unwrap();
        assert_eq!(src.requests.borrow().len(), 1);
        assert_eq!(z.read(z.find("a.txt").unwrap()).unwrap(), b"hello");
    }

    #[test]
    fn the_saved_tail_reopens_the_archive_with_no_request_for_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let (path, wanted) = fixture(dir.path());
        let first = Counting::new(path.clone());
        let z = RemoteZip::open(&first).unwrap();
        let (total, tail) = z.tail();
        let tail = tail.to_vec();
        let second = Counting::new(path);
        let again = RemoteZip::from_tail(&second, total, tail).unwrap();
        assert!(second.requests.borrow().is_empty());
        assert_eq!(again.entries.len(), 3003);
        let e = again.find(&wanted[1].0).unwrap();
        assert_eq!(again.read(e).unwrap(), wanted[1].1);
    }

    #[test]
    fn a_crc_mismatch_in_a_fetched_entry_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.zip");
        std::fs::write(
            &path,
            Builder::new().add_with_crc("x.bin", b"payload", 0xdead_beef).finish(),
        )
        .unwrap();
        let z = RemoteZip::open(FileSource(path)).unwrap();
        let err = z.read(&z.entries[0]).unwrap_err();
        assert!(matches!(err, RemoteError::Zip(ZipError::Crc { .. })), "{err:?}");
    }

    #[test]
    fn a_flipped_byte_in_the_served_data_is_caught() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("flip.zip");
        let mut data = Builder::new().add("x.bin", b"payload-payload", false).finish();
        let at = data.windows(7).position(|w| w == b"payload").unwrap();
        data[at] ^= 0xff;
        std::fs::write(&path, data).unwrap();
        let z = RemoteZip::open(FileSource(path)).unwrap();
        assert!(matches!(
            z.read(&z.entries[0]),
            Err(RemoteError::Zip(ZipError::Crc { .. }))
        ));
    }

    #[test]
    fn a_file_that_is_not_a_zip_or_is_cut_off_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junk");
        std::fs::write(&path, vec![7u8; 5000]).unwrap();
        assert!(matches!(
            RemoteZip::open(FileSource(path)),
            Err(RemoteError::Zip(ZipError::NotAZip(_)))
        ));
        let (full, _) = fixture(dir.path());
        let data = std::fs::read(&full).unwrap();
        let cut = dir.path().join("cut.zip");
        std::fs::write(&cut, &data[..data.len() / 2]).unwrap();
        assert!(RemoteZip::open(FileSource(cut)).is_err());
    }

    #[test]
    fn content_range_totals_are_read() {
        assert_eq!(total_of("bytes 100-199/1000"), Some(1000));
        assert_eq!(total_of("bytes */1315536524"), Some(1_315_536_524));
        assert_eq!(total_of("bytes 1-2/*"), None);
    }
}
