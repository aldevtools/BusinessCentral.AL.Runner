//! Collects this crate's refusal keys (ADR-071), one file per key in `src/refusals/<key>.rs`, into one
//! `refusal_keys! { ... }` invocation that `src/refusal.rs` includes (as `al-builtins/build.rs` does for its list).

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let dir = Path::new("src/refusals");
    // Git keeps no empty folder: a list whose last entry was deleted has no folder in a clean checkout, and is empty.
    let watched = if dir.is_dir() { dir } else { Path::new("src") };
    println!("cargo:rerun-if-changed={}", watched.display());
    let mut files: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .map(|e| e.expect("refusal key entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => panic!("{}: {e}", dir.display()),
    };
    // By file name, which is the key.
    files.sort();
    let mut out = String::from("refusal_keys! {\n");
    for p in &files {
        let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let entry = text.trim_end().trim_end_matches(',');
        writeln!(out, "// {}\n{entry},", p.display()).unwrap();
    }
    out.push_str("}\n");
    let dest = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("refusal_registry.rs");
    std::fs::write(dest, out).expect("write the refusal key list");
}
