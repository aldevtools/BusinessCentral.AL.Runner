//! The corpus tells us when a gate is missing (ADR-015 3.4 point 4): every corpus file with a
//! `#if BCnnPLUS` guard above BC 27 must be cited by a gate, and every gate's evidence must
//! really contain a guard at or below the gate's major. A guard on a minor symbol (`BC29_1PLUS`,
//! ADR-078 point 3) must be cited by a gate of exactly that minor.

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

/// A guard symbol: `BC<n>PLUS` is `(n, None)`, `BC<n>_<m>PLUS` is `(n, Some(m))`.
type Guard = (u16, Option<u16>);

/// The guards named by `BC<n>PLUS` and `BC<n>_<m>PLUS` symbols on preprocessor lines (`#if`, `#elif`) of `text`.
/// Comments that mention a symbol do not count.
fn guards(text: &str) -> Vec<Guard> {
    let digits = |s: &str| s.bytes().take_while(u8::is_ascii_digit).count();
    let mut out = Vec::new();
    for line in text.lines().map(str::trim_start) {
        if !(line.starts_with("#if") || line.starts_with("#elif")) {
            continue;
        }
        let mut rest = line;
        while let Some(at) = rest.find("BC") {
            rest = &rest[at + 2..];
            let d = digits(rest);
            let Ok(n) = rest[..d].parse() else { continue };
            let after = &rest[d..];
            if after.starts_with("PLUS") {
                out.push((n, None));
            } else if let Some(minor) = after.strip_prefix('_') {
                let m = digits(minor);
                if m > 0 && minor[m..].starts_with("PLUS") {
                    if let Ok(m) = minor[..m].parse() {
                        out.push((n, Some(m)));
                    }
                }
            }
        }
    }
    out
}

/// The version a gate keys on. Gates key on a major only today (`Gate::since_bc`); a gate of a minor needs ADR-078's
/// `(major, minor)` gates first, so a minor guard has no gate yet and is reported.
fn gate_version(g: Gate) -> (u16, u16) {
    (g.since_bc(), 0)
}

/// The files of `files` (path, text) whose guard needs a gate that `cited` (path to the versions of the gates citing it)
/// lacks: a major guard above the floor needs any gate, a minor guard `BC<n>_<m>PLUS` a gate of version `(n, m)`.
fn missing(files: &[(String, String)], cited: &BTreeMap<String, Vec<(u16, u16)>>) -> Vec<String> {
    let mut out = Vec::new();
    for (rel, text) in files {
        let gates = cited.get(rel).map(Vec::as_slice).unwrap_or_default();
        // A guard above the floor, other than a symbol that is not a BC major (BC143PLUS).
        for (n, minor) in guards(text).into_iter().filter(|(n, _)| *n < 100) {
            let lacks = match minor {
                None => n > MIN_BC_MAJOR && gates.is_empty(),
                Some(m) => n >= MIN_BC_MAJOR && !gates.contains(&(n, m)),
            };
            if lacks {
                let name = minor.map_or(format!("BC{n}PLUS"), |m| format!("BC{n}_{m}PLUS"));
                out.push(format!("{rel} ({name})"));
                break;
            }
        }
    }
    out
}

#[test]
fn every_corpus_guard_above_the_floor_is_cited_by_a_gate() {
    let root = corpus_app();
    assert!(root.join("app.json").exists(), "corpus submodule not checked out");
    let mut paths = Vec::new();
    al_files(&root, &mut paths);
    let files: Vec<(String, String)> = paths
        .iter()
        .map(|f| {
            let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            (rel, std::fs::read_to_string(f).unwrap())
        })
        .collect();
    let cited = Gate::ALL
        .iter()
        .fold(BTreeMap::<String, Vec<(u16, u16)>>::new(), |mut m, g| {
            for e in g.evidence() {
                m.entry(e.file.to_string()).or_default().push(gate_version(*g));
            }
            m
        });
    let missing = missing(&files, &cited);
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
                guards(&text).contains(&(g.since_bc(), None)),
                "{g:?}: {} has no BC{}PLUS guard",
                e.file,
                g.since_bc()
            );
        }
    }
}

#[test]
fn the_guard_scanner_reads_major_and_minor_symbols() {
    let text = "#if BC29_1PLUS and not BC29_2PLUS\n#elif BC28PLUS\n// #if BC30PLUS\n#if BC2_91PLUS or BC143PLUS\n";
    assert_eq!(
        guards(text),
        [(29, Some(1)), (29, Some(2)), (28, None), (2, Some(91)), (143, None)]
    );
}

/// ADR-078 conformance: a guard on `BC29_1PLUS` needs a gate of 29.1; a gate of 29.0 does not cover it.
#[test]
fn a_minor_guard_needs_a_gate_of_that_minor() {
    let files = vec![("x/A.al".to_string(), "#if BC29_1PLUS\n#endif\n".to_string())];
    let cited = |v: (u16, u16)| BTreeMap::from([("x/A.al".to_string(), vec![v])]);
    assert_eq!(missing(&files, &BTreeMap::new()), ["x/A.al (BC29_1PLUS)"]);
    assert_eq!(missing(&files, &cited((29, 0))), ["x/A.al (BC29_1PLUS)"]);
    assert!(missing(&files, &cited((29, 1))).is_empty());
    // A major guard needs any gate, as before.
    let major = vec![("x/B.al".to_string(), "#if BC28PLUS\n#endif\n".to_string())];
    assert_eq!(missing(&major, &BTreeMap::new()), ["x/B.al (BC28PLUS)"]);
    assert!(missing(&major, &BTreeMap::from([("x/B.al".to_string(), vec![(28, 0)])])).is_empty());
}
