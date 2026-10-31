//! The corpus tells us when a gate is missing (ADR-015 3.4 point 4): every corpus file with a
//! `#if BCnnPLUS` guard above BC 27 must be cited by a gate, and every gate's evidence must
//! really contain a guard at or below the gate's major.

use al_version::{Gate, MIN_BC_MAJOR};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn corpus_app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language")
}

fn al_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            al_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "al") {
            out.push(p);
        }
    }
}

/// The BC majors named by `BC<n>PLUS` symbols on preprocessor lines (`#if`, `#elif`) of `text`.
/// Comments that mention a symbol do not count.
fn guard_majors(text: &str) -> Vec<u16> {
    let mut out = Vec::new();
    for line in text.lines().map(str::trim_start) {
        if !(line.starts_with("#if") || line.starts_with("#elif")) {
            continue;
        }
        let mut rest = line;
        while let Some(at) = rest.find("BC") {
            rest = &rest[at + 2..];
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            if digits > 0 && rest[digits..].starts_with("PLUS") {
                if let Ok(n) = rest[..digits].parse() {
                    out.push(n);
                }
            }
        }
    }
    out
}

#[test]
fn every_corpus_guard_above_the_floor_is_cited_by_a_gate() {
    let root = corpus_app();
    assert!(root.join("app.json").exists(), "corpus submodule not checked out");
    let mut files = Vec::new();
    al_files(&root, &mut files);
    let cited: BTreeMap<&str, Vec<Gate>> = Gate::ALL.iter().fold(BTreeMap::new(), |mut m, g| {
        for e in g.evidence() {
            m.entry(e.file).or_default().push(*g);
        }
        m
    });
    let mut missing = Vec::new();
    for f in &files {
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(f).unwrap();
        // A guard above the floor, other than a symbol that is not a BC major (BC143PLUS).
        let above: Vec<u16> = guard_majors(&text)
            .into_iter()
            .filter(|n| *n > MIN_BC_MAJOR && *n < 100)
            .collect();
        if !above.is_empty() && !cited.contains_key(rel.as_str()) {
            missing.push(format!("{rel} (BC{}PLUS)", above[0]));
        }
    }
    assert!(
        missing.is_empty(),
        "corpus guards with no gate in al-version/src/gates.rs:\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_gate_evidence_file_has_a_guard_for_the_gate_major() {
    let root = corpus_app();
    for g in Gate::ALL {
        for e in g.evidence() {
            let text = std::fs::read_to_string(root.join(e.file))
                .unwrap_or_else(|err| panic!("{g:?}: evidence {} unreadable: {err}", e.file));
            assert!(
                guard_majors(&text).contains(&g.since_bc()),
                "{g:?}: {} has no BC{}PLUS guard",
                e.file,
                g.since_bc()
            );
        }
    }
}
