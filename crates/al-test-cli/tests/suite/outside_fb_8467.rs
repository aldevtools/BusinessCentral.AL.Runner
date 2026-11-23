//! The corpus probe of al-runner issue 8467 (corpus pull request 1457, merged, not pinned yet: tests/al-language/pages/background-tasks/
//! BgTaskQueryCache.al, measured on BC 29.1, 34 of 34 green; and the cells of open corpus pull request 1495 after it), the
//! reporter's repro and the neighbors, run here until a bump pins the probe: the query cache page background tasks and the test
//! session read through (`al-vm` query_cache.rs). The AL is the neighbor app's (tests/neighbors/8467-bg-query-cache), one copy.
//! Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{self, BackgroundTaskQueryCache};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/8467-bg-query-cache/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/8467-bg-query-cache/src/Neighbors.al");
const REPRO: &str = include_str!("../../../../tests/neighbors/8467-bg-query-cache/src/Repro.al");

type Outcomes = Vec<(String, Outcome, Vec<Assumption>)>;

fn outcomes(src: &str, codeunit: &str) -> Outcomes {
    outcomes_on(src, codeunit, None)
}

/// The tests of `codeunit`, run as Business Central `bc` (`None`: no target).
fn outcomes_on(src: &str, codeunit: &str, bc: Option<&str>) -> Outcomes {
    let src = format!("{src}\n{}", include_str!("../fixtures/outside-sc/Assert.al"));
    let l = al_testkit::compile_apps(vec![al_testkit::app(&src)]);
    let target = bc.map(|v| al_version::Target::for_bc(al_version::BcVersion::parse(v).unwrap()));
    let engine = al_vm::Engine::with_config(l.program.clone(), al_testkit::blocked(&l), target, Default::default());
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

fn get<'a>(o: &'a Outcomes, name: &str) -> &'a (String, Outcome, Vec<Assumption>) {
    o.iter()
        .find(|(n, _, _)| n == name)
        .unwrap_or_else(|| panic!("{name}: {o:?}"))
}

fn assert_pass(o: &Outcomes, name: &str, flagged: bool) {
    let (_, r, marks) = get(o, name);
    assert_eq!(*r, Outcome::Pass, "{name}");
    assert_eq!(marks.contains(&BackgroundTaskQueryCache), flagged, "{name}: {marks:?}");
}

fn assert_refused(o: &Outcomes, name: &str, why: &str) {
    let (_, r, _) = get(o, name);
    assert!(matches!(r, Outcome::NotRun(w) if w.starts_with(why)), "{name}: {r:?}");
}

/// The probe's tests: those whose answer the cache changes pass, flagged; those it does not change pass clean; the test
/// session's own read the cache answers, and a field load into a record the cache answered, are refused.
#[test]
fn issue_8467_probe_cells() {
    let mut o = outcomes(PROBE, "BQ Tests");
    o.extend(outcomes(PROBE, "BQ Tests 2"));
    assert_eq!(o.len(), 34);
    let clean = [
        "Page_FirstRead_SeesTheChange",
        "Page_Get_ChangeCommitted_SeesTheChange",
        "Page_TestWriteOnly_FirstReadSeesIt",
        "Run_Get_ModifyAllCommitted_SeesTheChange",
        "Run_GroupReads_InsertCommitted_SeeTheChange",
        "Run_GroupReads_OtherGroupCommitted_SeeTheChange",
        "Run_TestReadFirst_OtherRowChanged_FirstRead",
        "Run_TableWritten_TestRead_FirstReadSeesTheChange",
        "AutoRollback_1_ReadsTheKey",
        "OtherCodeunit_1_ReadsTheKey",
        "OtherCodeunit_2_SameKey_SeesItsRow",
    ];
    let refused = [
        (
            "Run_PartialFirst_FullLater_FirstAnswer",
            "a field SetLoadFields left out",
        ),
        ("AssertError_Rollback_FirstAnswer", "a read of the test session"),
    ];
    for (name, r, marks) in &o {
        let short = name.trim_start_matches("BgQueryCache_");
        if let Some((_, why)) = refused.iter().find(|(n, _)| *n == short) {
            assert!(matches!(r, Outcome::NotRun(w) if w.starts_with(why)), "{name}: {r:?}");
        } else {
            assert_eq!(*r, Outcome::Pass, "{name}");
            assert_eq!(
                marks.contains(&BackgroundTaskQueryCache),
                !clean.contains(&short),
                "{name}: {marks:?}"
            );
        }
    }
}

/// BQ Tests 3 and 3b of corpus pull request 1495: a Get after commits that inserted and deleted the row reads the table, clean.
/// FindLast, IsEmpty and FindFirst after a SetFilter, repeated after a change, answer their first answer (:1238, :1255, :1274),
/// and the worker's first Count after the test counted the group and then inserted answers the test's count (:6498), flagged
/// (al-runner issue 8637). A copy of a record a partial Get answered from the cache (:6518) stays refused.
#[test]
fn issue_8467_neighbor_cells() {
    let o = outcomes(PROBE, "BQ Tests 3");
    assert_pass(
        &o,
        "BgQueryCache_Run_Get_RolledBack_InsertDeleteCommitted_SeesTheTable",
        false,
    );
    for name in [
        "BgQueryCache_Run_FindLast_ChangedRow_FirstAnswer",
        "BgQueryCache_Run_IsEmpty_Insert_FirstAnswer",
        "BgQueryCache_Run_SetFilterFindFirst_ChangedRow_FirstAnswer",
    ] {
        assert_pass(&o, name, true);
    }
    let o = outcomes(PROBE, "BQ Tests 3b");
    assert_pass(&o, "BgQueryCache_Run_GroupCount_TestCountThenInsert_FirstRead", true);
    assert_refused(
        &o,
        "BgQueryCache_Run_PartialGet_Copy_FirstAnswer",
        "a field SetLoadFields left out",
    );
}

/// BQ Tests 11 of corpus pull request 1495: SelectLatestVersion in the worker ends the row entries of its table (every table
/// without one), and the read after it is cached again (:6730, :6737, :6786); another table's changes nothing (:6744). In the
/// test session it changes no worker answer (:6771, :6778, :6807, :6826, :6863, :6880). A table with a Blob or a Media field is
/// cached as any other (:6898, :6918).
#[test]
fn issue_8467_select_latest_version_and_blob_cells() {
    let o = outcomes(PROBE, "BQ Tests 11");
    assert_eq!(o.len(), 13);
    for (name, _, _) in &o {
        let flagged = name != "BgQueryCache_Run_GetMissing_SelectLatestVersionInWorker_SeesTheInsert";
        assert_pass(&o, name, flagged);
    }
}

/// The reporter's repro: the second background read of row A answers the first; the control sees the change.
#[test]
fn issue_8467_repro() {
    let o = outcomes(REPRO, "Repro Bg Read Tests");
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BackgroundTaskQueryCache),
            name.starts_with("SecondRead"),
            "{name}: {marks:?}"
        );
    }
}

/// Neighbors the probe does not measure (the neighbor app's Neighbors.al): refused where the cache's answer is not measured,
/// read as on main elsewhere.
#[test]
fn issue_8467_unmeasured_neighbors() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "BQ Neighbor Tests");
    assert_refused(&o, "CopyOfReplayed_LoadsOmittedField", "a field SetLoadFields left out");
    assert_refused(&o, "BlobTable_ReadAgain_AfterChange", "CalcFields of a Blob of a row");
    assert_pass(&o, "FetchAfterReplay_LoadsOmittedField", true);
    assert_pass(&o, "GroupRead_AfterTestReadAndWrite", true);
    for name in ["LongFilterValues_KeepTheirOwnEntries", "RowGoneAgain_AfterCommits"] {
        assert_pass(&o, name, false);
    }
}

/// A worker's Get and CalcFields of a Blob after the test committed its change of a row the cache answered: BC still waits for
/// the row the test codeunit wrote (BQ Neighbor Tests 3, a local BC 29.1 container, 3 runs), so the runner keeps refusing the
/// CalcFields of a row it answered with another row, also once a later read took the row from the table (review of PR 8490,
/// point 3).
#[test]
fn issue_8467_blob_after_commit_neighbor() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "BQ Neighbor Tests 3");
    assert_refused(
        &o,
        "BlobTable_ChangeCommitted_CalcFieldsWaits",
        "CalcFields of a Blob of a row",
    );
}

/// SelectLatestVersion() without a table in a worker ends the entry of a row of another table (BQ Neighbor Tests 4, BC 29.1,
/// 3 runs): the Get after it reads the table, as BC does.
#[test]
fn issue_8467_select_latest_version_without_a_table_neighbor() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "BQ Neighbor Tests 4");
    assert_pass(
        &o,
        "SelectLatestVersionWithoutATable_OtherTableRow_ReadsTheTable",
        false,
    );
}

/// A commit of a ModifyAll, a DeleteAll or a Truncate of another row, then a change of the row: the neighbors assert the
/// answer of BC 27.5, 28.5 and 29.0 for the first two (the cache entry ended, BgTaskQueryCache.al:7035, :7042). On 29.0 the
/// head reads the table, clean; on 29.1 it answers the first read (so the neighbors fail there); with no target, or on a minor
/// no cell measured, it refuses both. A Truncate has no cell: refused on every target.
#[test]
fn issue_8467_bulk_commits_follow_the_target() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let names = [
        "OtherRowModifyAllCommitted_ThenChanged",
        "OtherRowDeleteAllCommitted_ThenChanged",
        "OtherRowTruncateCommitted_ThenChanged",
    ];
    let ended =
        "a page background task's read of a row whose query cache entry a later commit of the table may have ended";
    let o = outcomes_on(&src, "BQ Neighbor Tests 2", Some("29.0"));
    for name in &names[..2] {
        assert_pass(&o, name, false);
    }
    assert_refused(&o, names[2], ended);
    let o = outcomes_on(&src, "BQ Neighbor Tests 2", Some("29.1"));
    for name in &names[..2] {
        let (_, r, _) = get(&o, name);
        assert!(matches!(r, Outcome::Fail(m) if m.contains("first")), "{name}: {r:?}");
    }
    assert_refused(&o, names[2], ended);
    for bc in [None, Some("29.2")] {
        let o = outcomes_on(&src, "BQ Neighbor Tests 2", bc);
        for name in names {
            assert_refused(&o, name, ended);
        }
    }
}
