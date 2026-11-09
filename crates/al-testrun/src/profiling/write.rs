//! Writing a profile to disk: a folder with one `.alcpuprofile` per test codeunit plus `summary.txt` (the default), or
//! one merged file when the path ends in `.alcpuprofile`.

use super::alcpuprofile::UrlOf;
use super::stream::{Stopped, write_codeunit, write_merged};
use super::{Collected, summary};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The file name of a codeunit's profile: `50120-Calib Tests.alcpuprofile`, with characters a file system refuses
/// replaced by `_`.
pub fn file_name(codeunit: u32, name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    format!("{codeunit}-{}.alcpuprofile", safe.trim())
}

/// Whether `path` names one merged file rather than a folder.
pub fn is_merged(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("alcpuprofile"))
}

/// What [`write`] wrote, and the codeunits it left out because the run reached its memory limit while it wrote.
#[derive(Debug, Default)]
pub struct Written {
    pub files: Vec<PathBuf>,
    /// Codeunits whose file was not written (folder), or every codeunit (one merged file that was not written).
    pub left_out: usize,
}

/// Streams one file (`stream.rs`) to `path` through a temporary file next to it, renamed when complete: a write that
/// stops leaves no half file. `Ok(false)`: `stop` said to stop (the memory limit).
fn write_file(path: &Path, body: &dyn Fn(&mut dyn Write) -> std::io::Result<()>) -> Result<bool, String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    let fail = |e: std::io::Error| format!("cannot write {}: {e}", path.display());
    let file = std::fs::File::create(&tmp).map_err(fail)?;
    let mut w = std::io::BufWriter::with_capacity(1 << 20, file);
    match body(&mut w).and_then(|()| w.flush()) {
        Ok(()) => {
            drop(w);
            std::fs::rename(&tmp, path).map_err(fail)?;
            Ok(true)
        }
        Err(e) => {
            drop(w);
            let _ = std::fs::remove_file(&tmp);
            if e.get_ref().is_some_and(|r| r.is::<Stopped>()) {
                Ok(false)
            } else {
                Err(fail(e))
            }
        }
    }
}

/// Writes the profile; returns the files written. `stop` is asked while it writes (every few thousand nodes and
/// before each file): `true` (the run's memory limit is reached) stops the write, and the files not written are
/// counted in [`Written::left_out`]; the summary of a folder is still written.
pub fn write(path: &Path, c: &Collected, url_of: UrlOf, stop: &dyn Fn() -> bool) -> Result<Written, String> {
    let mut out = Written::default();
    if is_merged(path) {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        if !stop() && write_file(path, &|w| write_merged(w, c, url_of, stop))? {
            out.files.push(path.to_path_buf());
        } else {
            out.left_out = c.codeunits.len();
        }
        return Ok(out);
    }
    std::fs::create_dir_all(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    for p in &c.codeunits {
        let f = path.join(file_name(p.codeunit, &p.name));
        if !stop() && write_file(&f, &|w| write_codeunit(w, p, &c.options, url_of, stop))? {
            out.files.push(f);
        } else {
            out.left_out += 1;
        }
    }
    let s = path.join("summary.txt");
    std::fs::write(&s, summary::render(c)).map_err(|e| format!("cannot write {}: {e}", s.display()))?;
    out.files.push(s);
    Ok(out)
}
