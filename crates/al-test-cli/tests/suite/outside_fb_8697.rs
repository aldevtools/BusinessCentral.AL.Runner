//! Al-runner issue 8697, through the neighbor app tests/neighbors/8697-bg-worker-last-error (a copy of corpus PR 1516,
//! tests/al-language/pages/background-tasks/BgTaskWorkerLastError.al, and the shapes next to it): a page background task's
//! worker has a last error and an error collection of its own (`al-vm` pages/background.rs `ChildParked`).

use al_builtins::assumptions::Assumption::{self, BgTaskWorkerLastError};
use al_test_cli::report::Outcome;
use al_testrun::isolation::TestIsolation;

fn outcomes(codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = [
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/Probe.al"),
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/Neighbors.al"),
        include_str!("../../../../tests/neighbors/8697-bg-worker-last-error/src/Assert.al"),
    ]
    .join("\n");
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
        TestIsolation::Codeunit,
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

/// Every cell of the probe passes. The cells whose worker raises an error it does not catch, caught by the test session through
/// a procedure of its own (as on main), pass clean; every other cell runs through the assumption.
#[test]
fn issue_8697_probe_cells() {
    let o = outcomes("LEW Tests");
    assert_eq!(o.len(), 26);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let clean = name == "WorkerUnhandledError_TestSessionCatchesIt"
            || name == "ErrorTrigger_AfterAnUnhandledWorkerError_ReadsTheTestSessionsLastError"
            || name.ends_with("_AssertErrorOnAProcedure_TheWorkersError");
        assert_eq!(marks.contains(&BgTaskWorkerLastError), !clean, "{name}: {marks:?}");
    }
}

/// The shapes next to the cells (an enqueued task, a test session error from a TryFunction, a worker's guarded Codeunit.Run)
/// pass through the assumption; a TryFunction around the call records the worker's error, as on main, clean.
#[test]
fn issue_8697_neighbors() {
    let o = outcomes("LEN Tests");
    assert_eq!(o.len(), 4);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let clean = name == "WorkerUnhandledError_TryFunctionAroundTheCall_TheWorkersError";
        assert_eq!(marks.contains(&BgTaskWorkerLastError), !clean, "{name}: {marks:?}");
    }
}
