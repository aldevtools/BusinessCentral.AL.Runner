//! Collects the entries of the assumption registry, one file per entry in `src/assumptions/<id>.rs`, into one
//! `registry! { ... }` invocation that `src/assumptions.rs` includes. One file per entry keeps two pull requests
//! that each add an assumption from touching the same lines (`tools/assumption_merge_check.sh`).

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let dir = Path::new("src/assumptions");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("src/assumptions")
        .map(|e| e.expect("src/assumptions entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    // By file name, which is the id: the registry order, the order of the marks and of the generated table.
    files.sort();
    let mut out = String::from("registry! {\n");
    for p in &files {
        let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let entry = text.trim_end().trim_end_matches(',');
        writeln!(out, "// {}\n{entry},", p.display()).unwrap();
    }
    out.push_str("}\n");
    let dest = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("assumption_registry.rs");
    std::fs::write(dest, out).expect("write the assumption registry");
}
