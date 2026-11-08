//! Writing a profile to disk: a folder with one `.alcpuprofile` per test codeunit plus `summary.txt` (the default), or
//! one merged file when the path ends in `.alcpuprofile`.

use super::alcpuprofile::{UrlOf, codeunit_file, merged_file};
use super::{Collected, summary};
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

fn write_json(path: &Path, v: &serde_json::Value) -> Result<(), String> {
    let text = serde_json::to_string(v).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Writes the profile; returns the files written.
pub fn write(path: &Path, c: &Collected, url_of: UrlOf) -> Result<Vec<PathBuf>, String> {
    if is_merged(path) {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        write_json(path, &merged_file(c, url_of))?;
        return Ok(vec![path.to_path_buf()]);
    }
    std::fs::create_dir_all(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for p in &c.codeunits {
        let f = path.join(file_name(p.codeunit, &p.name));
        write_json(&f, &codeunit_file(p, &c.options, url_of))?;
        out.push(f);
    }
    let s = path.join("summary.txt");
    std::fs::write(&s, summary::render(c)).map_err(|e| format!("cannot write {}: {e}", s.display()))?;
    out.push(s);
    Ok(out)
}
