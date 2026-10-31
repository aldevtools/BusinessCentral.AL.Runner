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
/// Comments that mention a symbol do not count, nor does a guard that only holds back a suspect test
/// (`suspect_only_guard`).
fn guard_majors(text: &str) -> Vec<u16> {
    let mut out = Vec::new();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    for (i, line) in lines.iter().enumerate() {
        if !(line.starts_with("#if") || line.starts_with("#elif")) {
            continue;
        }
        if suspect_only_guard(&lines[i + 1..]) {
            continue;
        }
        let mut rest: &str = line;
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

/// Whether the guard whose body starts at `body` (the lines after its `#if`) is exactly
/// `[Test]` + `procedure X()` / `#else` / `procedure Suspect_X()` / `#endif`, comments aside: on the
/// majors of the `#else` branch the corpus keeps a Linux container suspect out of the test list
/// (pages/lt-y15/Y15ReapplyMarksSecondaryKey.al:310-318). Such a guard pins no behavior.
fn suspect_only_guard(body: &[&str]) -> bool {
    let code = |l: &&&str| !l.is_empty() && !l.starts_with("//");
    let mut it = body.iter().filter(code);
    let is_procedure = |l: &str| l.starts_with("procedure ") && l.ends_with(')');
    matches!(it.next(), Some(&"[Test]"))
        && it.next().is_some_and(|l| is_procedure(l))
        && matches!(it.next(), Some(&"#else"))
        && it
            .next()
            .is_some_and(|l| is_procedure(l) && l.starts_with("procedure Suspect_"))
        && matches!(it.next(), Some(&"#endif"))
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

/// A guard whose `#else` branch only turns a [Test] into a plain `Suspect_` procedure hides a Linux
/// container suspect from the test list on some majors; it pins no behavior, so it needs no gate
/// (pages/lt-y15/Y15ReapplyMarksSecondaryKey.al:310-318).
#[test]
fn a_guard_that_only_holds_back_a_suspect_test_needs_no_gate() {
    let suspect = "    // Contrast\n#if BC29PLUS or not BC28PLUS\n    [Test]\n    procedure Y15_SK_MovesWithoutRefresh()\n#else\n    // Linux suspect (BC 28.5 only).\n    procedure Suspect_SK_MovesWithoutRefresh()\n#endif\n    var\n";
    assert_eq!(guard_majors(suspect), Vec::<u16>::new());
    // Anything else in either branch is a behavior guard and still counts.
    let behavior = "#if BC29PLUS or not BC28PLUS\n    [Test]\n    procedure A()\n#else\n    procedure Suspect_A()\n    begin\n    end;\n#endif\n";
    assert_eq!(guard_majors(behavior), vec![29, 28]);
    let not_suspect = "#if BC29PLUS\n    [Test]\n    procedure A()\n#else\n    procedure B()\n#endif\n";
    assert_eq!(guard_majors(not_suspect), vec![29]);
}
