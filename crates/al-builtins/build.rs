//! Collects the entries of the assumption registry, one file per entry in `src/assumptions/<id>.rs`, into one
//! `registry! { ... }` invocation that `src/assumptions.rs` includes. One file per entry keeps two pull requests
//! that each add an assumption from touching the same lines (`tools/assumption_merge_check.sh`).
//!
//! The same for this crate's refusal keys (ADR-071): `src/refusals/<key>.rs` into one `refusal_keys! { ... }`
//! invocation that `src/refusal.rs` includes.

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    collect("src/assumptions", "registry", "assumption_registry.rs");
    collect("src/refusals", "refusal_keys", "refusal_registry.rs");
}

/// One `MACRO! { ... }` call with the `.rs` files of `dir` as its entries, written to `OUT_DIR/<out_name>`.
fn collect(dir: &str, mac: &str, out_name: &str) {
    let dir = Path::new(dir);
    // Git keeps no empty folder: a list whose last entry was deleted has no folder in a clean checkout, and is empty.
    let watched = if dir.is_dir() { dir } else { Path::new("src") };
    println!("cargo:rerun-if-changed={}", watched.display());
    let mut files: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .map(|e| e.expect("registry entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => panic!("{}: {e}", dir.display()),
    };
    // By file name, which is the id: the registry order, the order of the marks and of the generated table.
    files.sort();
    let mut out = format!("{mac}! {{\n");
    for p in &files {
        let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let entry = text.trim_end().trim_end_matches(',');
        writeln!(out, "// {}\n{entry},", p.display()).unwrap();
    }
    out.push_str("}\n");
    let dest = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join(out_name);
    std::fs::write(dest, out).expect("write the registry");
}
