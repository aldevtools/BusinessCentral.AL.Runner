//! `docs/codemap.md` is the map agents read before opening a source file. It must not rot
//! silently: every module file it lists has to exist. Line numbers in it may drift; files may not.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The `(crate, file)` pairs of the "- `file.rs` ..." bullets under each "### al-..." heading.
fn listed(text: &str) -> Vec<(String, String)> {
    let mut krate: Option<&str> = None;
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("### ") {
            krate = (h.trim().starts_with("al-") || h.trim().starts_with("alrunner-")).then(|| h.trim());
        } else if let (Some(k), Some(rest)) = (krate, line.strip_prefix("- `")) {
            if let Some(file) = rest.split('`').next().filter(|f| f.ends_with(".rs")) {
                out.push((k.to_string(), file.to_string()));
            }
        }
    }
    out
}

#[test]
fn every_module_file_the_codemap_lists_exists() {
    let text = std::fs::read_to_string(root().join("docs/codemap.md")).expect("docs/codemap.md");
    let files = listed(&text);
    assert!(
        files.len() > 100,
        "the codemap lists {} files; its format changed?",
        files.len()
    );
    let missing: Vec<String> = files
        .iter()
        .filter(|(k, f)| !root().join("crates").join(k).join("src").join(f).is_file())
        .map(|(k, f)| format!("crates/{k}/src/{f}"))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/codemap.md lists files that no longer exist; update the map:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn the_parser_reads_crate_headings_and_file_bullets_only() {
    let text = "### al-vm\n- `lib.rs` (10): x\n- `pages/areas.rs` (5): y\n### Corpus run\n- `not_a_crate.rs`\n";
    assert_eq!(
        listed(text),
        [
            ("al-vm".to_string(), "lib.rs".to_string()),
            ("al-vm".to_string(), "pages/areas.rs".to_string())
        ]
    );
}
