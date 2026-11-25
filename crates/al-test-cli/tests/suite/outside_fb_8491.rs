//! The corpus probe of al-runner issue 8491 (corpus pull request 1459, merged, not pinned yet, and the cells from :548 on in
//! corpus pull request 1489, open: tests/al-language/pages/background-tasks/BgTaskInsertQueue.al, measured on BC 29.1, 34 of
//! 34 green), the reporter's repro and the neighbors, read from the neighbor app tests/neighbors/8491-bg-insert-queue, run
//! here until a bump pins the probe: a page background task's worker does not see the rows the test session's insert queue
//! holds (`al-vm` records/insert_buffer.rs, `hide_queued_from_worker`). Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{self, BgTaskInsertQueue};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8491-bg-insert-queue/src/Probe.al");
const REPRO: &str = include_str!("../../../../tests/neighbors/8491-bg-insert-queue/src/Repro.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8491-bg-insert-queue/src/Neighbors.al");

fn outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
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
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
        al_testrun::host::RunClock::begin(),
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

/// Every probe test passes; those whose worker runs while the queue holds rows are flagged, also while the error of an Insert
/// is deferred (BgTaskInsertQueue.al:800, :830, :855, :879).
#[test]
fn issue_8491_probe_cells() {
    let o = outcomes(PROBE, "IQ Tests");
    assert_eq!(o.len(), 34);
    // The tests whose worker runs after the queue was sent: nothing is hidden.
    let clean = [
        "Insert_TestGet_TaskFindsIt",
        "FiveInserts_TaskFindsAll",
        "Insert_Commit_TaskFindsIt",
        "InsertResultUsed_TaskFindsIt",
        "BlobTable_Insert_TaskFindsIt",
        "Insert_TestModifiesOtherRow_TaskFindsIt",
        "Insert_TestFlowField_TaskFindsIt",
        "Insert_TestGet_PageEnqueuedTaskFindsIt",
        "SubscriberTable_SubscriberReadsRow_TaskFindsIt",
        "Insert_TestCountApprox_TaskFindsIt",
        "Insert_TestQuery_TaskFindsIt",
        "Insert_TestGetBySystemId_TaskFindsIt",
        "Insert_TestTableRelation_TaskFindsIt",
    ];
    for (name, r, marks) in &o {
        let short = name.trim_start_matches("BgTaskInsertQueue_");
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BgTaskInsertQueue),
            !clean.contains(&short),
            "{name}: {marks:?}"
        );
    }
}

/// The reporter's repro: the task right after the insert does not find the row; after the test read it, it does.
#[test]
fn issue_8491_repro() {
    let o = outcomes(REPRO, "Repro Buf Tests");
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BgTaskInsertQueue),
            name.starts_with("InsertThenTask"),
            "{name}: {marks:?}"
        );
    }
}

/// The neighbor app's tests (BC's answer of each, on a local BC 29.1 container, in its comments). (Rows queued under another
/// company value: al-testrun `companies_opt_in.rs`, which runs with two companies.)
#[test]
fn issue_8491_neighbors() {
    let o = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "IQ Neighbor Tests");
    let get = |name: &str| o.iter().find(|(n, _, _)| n == name).expect("the test");
    let flagged = |name: &str| get(name).2.contains(&BgTaskInsertQueue);
    // A worker's Get while the error is deferred raises nothing and runs on the view; the error comes out at the end of the test.
    for (name, error) in [
        ("DeferredDuplicate_TaskRead", "already exists"),
        ("DeferredDuplicate_TaskGetsTheDuplicatedKey", "already exists"),
        ("DeferredSqlRange_TaskGet", "Cannot write the value 06/15/1753"),
    ] {
        let (_, r, _) = get(name);
        assert!(matches!(r, Outcome::Fail(w) if w.contains(error)), "{name}: {r:?}");
        assert!(flagged(name), "{name}");
    }
    for (name, refusal) in [
        (
            "DeferredDuplicate_TaskFindFirst",
            "a read of a page background task's worker of IQ Row while the error of an Insert into it is deferred",
        ),
        (
            "DeferredSqlRangeClosingDate_TaskGet",
            "a read of a page background task's worker of IQ Date Row while the error of an Insert into it is deferred",
        ),
        (
            "Queued_TaskTableRelation",
            "the TableRelation check of IQ Row by a page background task's worker",
        ),
    ] {
        let (_, r, _) = get(name);
        assert!(
            matches!(r, Outcome::NotRun(w) if w.starts_with(refusal)),
            "{name}: {r:?}"
        );
    }
    // A worker's write raises the read-only error before it reads the store: nothing runs through the assumption.
    assert_eq!(get("Queued_TaskModify").1, Outcome::Pass);
    assert!(!flagged("Queued_TaskModify"));
    // The count of five that a FlowField of the test session started again: the duplicate that is the 5th Insert raises.
    assert_eq!(get("IndirectSend_FifthInsertDuplicateRaises").1, Outcome::Pass);
    assert!(flagged("IndirectSend_FifthInsertDuplicateRaises"));
}
