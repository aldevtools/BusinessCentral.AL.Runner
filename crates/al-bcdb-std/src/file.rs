//! A file as a byte source for the readers in `al-bcdb` (the core does no OS access, ADR-006).
//!
//! A file as a byte source (pread), the OS side of `al-bcdb` (ADR-006). Entry points: `FileSource`.

use al_bcdb::io::ByteSource;
use std::fs::File;
use std::path::Path;

/// A file read by positional access (pread). Page reads are scattered (the page map points
/// anywhere in the file), and a memory map pays a page fault per touched page, which cost
/// more than the syscall here: opening the BC 28.1 demo backup took 37 ms mapped and 17 ms
/// with pread.
pub struct FileSource {
    file: File,
    len: u64,
}

impl FileSource {
    pub fn open(path: impl AsRef<Path>) -> Result<FileSource, String> {
        let path = path.as_ref();
        let name = path.display();
        let file = File::open(path).map_err(|e| format!("{name}: {e}"))?;
        let len = file.metadata().map_err(|e| format!("{name}: {e}"))?.len();
        Ok(FileSource { file, len })
    }
}

impl ByteSource for FileSource {
    fn len(&self) -> u64 {
        self.len
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> al_bcdb::Result<()> {
        #[cfg(unix)]
        let r: std::io::Result<()> = {
            use std::os::unix::fs::FileExt;
            self.file.read_exact_at(buf, offset)
        };
        #[cfg(windows)]
        let r: std::io::Result<()> = (|| {
            use std::os::windows::fs::FileExt;
            let mut done = 0;
            while done < buf.len() {
                let n = self.file.seek_read(&mut buf[done..], offset + done as u64)?;
                if n == 0 {
                    return Err(std::io::ErrorKind::UnexpectedEof.into());
                }
                done += n;
            }
            Ok(())
        })();
        r.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                al_bcdb::Error::Io(format!("unexpected end of file at offset 0x{offset:x}"))
            } else {
                al_bcdb::Error::Io(e.to_string())
            }
        })
    }
}
