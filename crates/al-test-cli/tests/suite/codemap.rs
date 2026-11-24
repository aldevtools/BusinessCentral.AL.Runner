//! `docs/codemap.md` is the map agents read before opening a source file. It must not rot
//! silently: every module file it lists has to exist. Line numbers in it may drift; files may not.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The size cap of `docs/codemap.md` in bytes. It is an index, one line per module; detail goes in
/// the module's `//!` header. A cap only goes down: shorten a line, never raise this.
const MAX_BYTES: usize = 40_000;

/// The `(crate, path)` pairs under each "### al-..." heading: the path of each "- `path`" bullet
/// (a file, or a directory ending in `/`), and, for a directory bullet, each `.rs` file named after
/// its "Files:" marker, as `dir/file`.
fn listed(text: &str) -> Vec<(String, String)> {
    let mut krate: Option<&str> = None;
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("### ") {
            krate = (h.trim().starts_with("al-") || h.trim().starts_with("alrunner-")).then(|| h.trim());
        } else if let (Some(k), Some(rest)) = (krate, line.strip_prefix("- `")) {
            let Some(path) = rest.split('`').next() else { continue };
            if !(path.ends_with(".rs") || path.ends_with('/')) {
                continue;
            }
            out.push((k.to_string(), path.to_string()));
            if let (true, Some((_, files))) = (path.ends_with('/'), line.split_once("Files:")) {
                for f in files.split('`').skip(1).step_by(2).filter(|f| f.ends_with(".rs")) {
                    out.push((k.to_string(), format!("{path}{f}")));
                }
            }
        }
    }
    out
}

#[test]
fn every_module_the_codemap_lists_exists() {
    let text = std::fs::read_to_string(root().join("docs/codemap.md")).expect("docs/codemap.md");
    let files = listed(&text);
    assert!(
        files.len() > 100,
        "the codemap lists {} paths; its format changed?",
        files.len()
    );
    let missing: Vec<String> = files
        .iter()
        .filter(|(k, f)| !root().join("crates").join(k).join("src").join(f).exists())
        .map(|(k, f)| format!("crates/{k}/src/{f}"))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/codemap.md lists paths that no longer exist; update the map:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn the_codemap_stays_an_index() {
    let len = std::fs::read_to_string(root().join("docs/codemap.md"))
        .expect("docs/codemap.md")
        .len();
    assert!(
        len <= MAX_BYTES,
        "docs/codemap.md is {len} bytes, cap {MAX_BYTES}. It is one line per module: shorten the \
         line and put the detail in the module's `//!` header. Never raise the cap."
    );
}

#[test]
fn the_parser_reads_crate_headings_file_bullets_and_directory_file_lists() {
    let text = "### al-vm\n- `lib.rs`: x\n- `pages/` (2 files): y. Files: `areas.rs`, `state/close.rs`\n- `pages.rs`: z\n### Corpus run\n- `not_a_crate.rs`\n";
    let want = |k: &str, f: &str| (k.to_string(), f.to_string());
    assert_eq!(
        listed(text),
        [
            want("al-vm", "lib.rs"),
            want("al-vm", "pages/"),
            want("al-vm", "pages/areas.rs"),
            want("al-vm", "pages/state/close.rs"),
            want("al-vm", "pages.rs")
        ]
    );
}
