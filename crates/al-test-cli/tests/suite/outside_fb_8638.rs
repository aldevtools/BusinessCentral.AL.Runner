//! The corpus probe of al-runner issue 8638 (corpus pull request 1498, open: tests/al-language/pages/background-tasks/
//! BgTaskReadIsolationLock.al, measured on BC 29.1, 15 of 15 green) and the reporter's repro, run here until a bump pins the
//! probe: a page background task's read under ReadCommitted, RepeatableRead or UpdLock of a row the test codeunit wrote waits
//! on the test session's lock and fails (`al-vm` bg_readisolation_lock.rs). Delete this file and the fixtures when the pin is
//! in the corpus.

use al_builtins::assumptions::Assumption::{self, BgTaskReadIsolationLock};
use al_test_cli::report::Outcome;
use al_testrun::isolation::TestIsolation;

const PROBE: &str = include_str!("../fixtures/outside-fb/Issue8638Probe.al");
const NEIGHBORS: &str = include_str!("../fixtures/outside-fb/Issue8638Neighbors.al");

fn outcomes(src: &str, codeunit: &str, isolation: TestIsolation) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = format!("{src}\n{}", include_str!("../fixtures/outside-sc/Assert.al"));
    let l = al_testkit::compile_apps(vec![al_testkit::app(&src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        isolation,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(!outcomes.is_empty());
    outcomes
        .into_iter()
        .map(|(n, o)| {
            let marks = hits
                .iter()
                .find(|(h, _)| *h == n)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (n, o, marks)
        })
        .collect()
}

/// The probe's cells that read the row through the lock (BgTaskReadIsolationLock.al:175-205, :220-339).
const CLEAN: [&str; 4] = [
    "ReadUncommittedGet_RowTheTestModified_SeesTheChange",
    "DefaultGet_RowTheTestModified_SeesTheChange",
    "ReadCommittedGet_MissingKey_NotFound",
    "DefaultGet_RowTheTestInsertedAndCommitted_ReadsTheRow",
];

/// Every probe test passes: the eleven lock cells through the assumption, the four others clean.
#[test]
fn issue_8638_probe_cells() {
    let o = outcomes(PROBE, "RL Tests", TestIsolation::Codeunit);
    assert_eq!(o.len(), 15);
    for (name, r, marks) in &o {
        let short = name.trim_start_matches("BgTaskReadIsolation_");
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BgTaskReadIsolationLock),
            !CLEAN.contains(&short),
            "{name}: {marks:?}"
        );
    }
}

/// Under TestIsolation = Function the rows' locks are not measured: the reads under ReadCommitted, RepeatableRead and UpdLock
/// refuse, the others keep their answer.
#[test]
fn issue_8638_probe_under_function_isolation() {
    let o = outcomes(PROBE, "RL Tests", TestIsolation::Function);
    assert_eq!(o.len(), 15);
    for (name, r, _) in &o {
        let short = name.trim_start_matches("BgTaskReadIsolation_");
        if short.starts_with("ReadUncommitted") || short.starts_with("Default") {
            assert_eq!(*r, Outcome::Pass, "{name}");
        } else {
            assert!(
                matches!(r, Outcome::NotRun(w) if w.starts_with("page background task read under ReadIsolation")
                    && w.contains("TestIsolation other than Codeunit")),
                "{name}: {r:?}"
            );
        }
    }
}

/// The reporter's repro passes, flagged.
#[test]
fn issue_8638_repro() {
    let o = outcomes(
        include_str!("../fixtures/outside-fb/Issue8638.al"),
        "Repro Lock Read Tests",
        TestIsolation::Codeunit,
    );
    assert_eq!(o.len(), 1);
    let (name, r, marks) = &o[0];
    assert_eq!(*r, Outcome::Pass, "{name}");
    assert!(marks.contains(&BgTaskReadIsolationLock), "{marks:?}");
}

/// The shapes next to the cells, one refused shape per test codeunit (tests/neighbors/8638-bg-readisolation-lock): each
/// refuses with its own reason.
#[test]
fn issue_8638_refused_neighbors() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let refused = [
        ("RL NB Uncaught", "that leaves the worker"),
        ("RL NB FindSet", "FindSet"),
        ("RL NB IsEmpty", "IsEmpty"),
        ("RL NB RepeatableReadFirst", "FindFirst under RepeatableRead"),
        ("RL NB KeyFilter", "a filter on a key field"),
        ("RL NB StatementFirst", "FindFirst as a statement"),
        ("RL NB Delete", "a Delete"),
        ("RL NB RolledBack", "a write rolled back"),
        ("RL NB LockTable", "a LockTable"),
        ("RL NB LockingRead", "a locking read of the test session"),
        ("RL NB Caption", "a Caption other than its name"),
        ("RL NB SecondaryKey", "a key other than the primary key"),
        ("RL NB RecordRef", "through a RecordRef"),
        ("RL NB CodeunitRun", "a write inside a Codeunit.Run"),
    ];
    for (codeunit, why) in refused {
        let o = outcomes(&src, codeunit, TestIsolation::Codeunit);
        assert_eq!(o.len(), 1, "{codeunit}");
        let (name, r, _) = &o[0];
        assert!(
            matches!(r, Outcome::NotRun(w) if w.starts_with("page background task read under ReadIsolation") && w.contains(why)),
            "{codeunit}::{name}: {r:?}"
        );
    }
}

/// A table the test codeunit did not write, and a read under ReadUncommitted, keep main's answer, clean.
#[test]
fn issue_8638_kept_neighbors() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "RL NB Kept", TestIsolation::Codeunit);
    assert_eq!(o.len(), 3);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(!marks.contains(&BgTaskReadIsolationLock), "{name}: {marks:?}");
    }
}
