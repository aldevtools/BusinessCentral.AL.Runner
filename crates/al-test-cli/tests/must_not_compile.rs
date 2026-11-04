//! The must-not-compile corpus (`al_test_cli::mnc`): every case is AL the real compiler rejects, and
//! the front end must report the same ALnnnn codes. The cases come from `AL_MNC_DIR`, else from the
//! corpus submodule (`corpus/tests/must-not-compile`, there since the bump to 345627d). Without the
//! submodule checked out the test is skipped with a message; with it checked out but without the folder it
//! fails. The cases are never faked.

use al_test_cli::mnc::{Verdict, read_cases, render, run_case};
use std::path::PathBuf;

fn cases_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("AL_MNC_DIR") {
        return Some(PathBuf::from(d));
    }
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/must-not-compile");
    d.is_dir().then_some(d)
}

#[test]
fn every_must_not_compile_case_gets_the_expected_codes() {
    let Some(dir) = cases_dir() else {
        let submodule = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests");
        assert!(
            !submodule.is_dir(),
            "the corpus submodule is checked out but has no tests/must-not-compile folder"
        );
        eprintln!(
            "SKIPPED: no must-not-compile corpus. Set AL_MNC_DIR to .../tests/must-not-compile, or bump the \
             corpus submodule to a commit that has tests/must-not-compile."
        );
        return;
    };
    let cases = read_cases(&dir).expect("cases parse");
    assert!(!cases.is_empty(), "{} holds no case", dir.display());
    // The cases the front end does not decide yet are listed in `must-not-compile-known-gaps.txt`
    // (`<major> <case name>` per line), a ratchet like `metrics/<major>/not-passing.txt`: a mismatch outside
    // the list fails, and so does a listed case that matches now (delete its line). The cases are never
    // edited or skipped.
    let gaps_text = include_str!("must-not-compile-known-gaps.txt");
    let gaps: std::collections::BTreeSet<(u16, String)> = gaps_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (major, name) = l.split_once(' ').expect("`<major> <case name>`");
            (major.parse().expect("a BC major"), name.trim().to_string())
        })
        .collect();
    let mut failures = Vec::new();
    let mut mismatched: std::collections::BTreeSet<(u16, String)> = Default::default();
    for major in [27u16, 28, 29] {
        let outcomes: Vec<_> = cases.iter().map(|c| run_case(c, major)).collect();
        let report = render(&outcomes, major);
        eprintln!("{report}");
        for o in &outcomes {
            if matches!(o.verdict, Verdict::Mismatch(_)) {
                mismatched.insert((major, o.name.clone()));
            }
            // A case is never checked silently: every one has a verdict with its reason.
            assert!(
                !matches!(&o.verdict, Verdict::NotChecked(w) | Verdict::Skipped(w) if w.is_empty()),
                "{}: a verdict without a reason",
                o.name
            );
        }
        let new: Vec<_> = mismatched
            .iter()
            .filter(|(m, n)| *m == major && !gaps.contains(&(*m, n.clone())))
            .collect();
        if !new.is_empty() {
            failures.push(format!("BC {major}: new mismatches {new:?}\n{report}"));
        }
    }
    let fixed: Vec<_> = gaps.difference(&mismatched).collect();
    assert!(
        fixed.is_empty(),
        "these listed cases match now, delete their lines from must-not-compile-known-gaps.txt: {fixed:?}"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
