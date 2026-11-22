//! The corpus probe of al-runner issue 8670 (corpus pull request 1507: tests/al-language/pages/background-tasks/
//! BgTaskWorkerFilterGroup1.al, measured on Business Central 29.1, 17 of 17 green) and the reporter's repro, run here until a
//! bump pins the probe: a page background task's worker starts with none of the test session's filters in filter group 1, and
//! what it sets there ends with it (`al-vm` group1_sessions.rs). Delete this file and the fixtures when the pin is in the
//! corpus.

use al_builtins::assumptions::Assumption::{self, BgTaskWorkerFilterGroupOne};
use al_test_cli::report::Outcome;
use al_testrun::isolation::TestIsolation;

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

/// Every probe test passes, through the assumption, except the cell whose test session holds its filter on another table
/// than the one the worker reads (BgTaskWorkerFilterGroup1.al:336), which main answers the same, clean.
#[test]
fn issue_8670_probe_cells() {
    let o = outcomes(include_str!("../fixtures/outside-fb/Issue8670Probe.al"), "G1W Tests");
    assert_eq!(o.len(), 17);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        let clean = name == "WorkerFreshRecord_Group1FilterOnAnotherTable_EveryRowNoFilter";
        assert_eq!(marks.contains(&BgTaskWorkerFilterGroupOne), !clean, "{name}: {marks:?}");
    }
}

/// The reporter's repro passes, flagged.
#[test]
fn issue_8670_repro() {
    let o = outcomes(
        include_str!("../fixtures/outside-fb/Issue8670.al"),
        "Repro G1 Worker Tests",
    );
    assert_eq!(o.len(), 1);
    let (name, r, marks) = &o[0];
    assert_eq!(*r, Outcome::Pass, "{name}");
    assert!(marks.contains(&BgTaskWorkerFilterGroupOne), "{marks:?}");
}

/// A worker that takes back the test session's filter in filter group 1 (SetRange(Tag) in group 1) does so in its own
/// session: the test session keeps its filter, in the same test and in the next one (tests/neighbors/
/// 8670-bg-worker-filter-group1, codeunit "G1N Worker Clears", measured on Business Central 29.1). Main took the filter back
/// for the test session. Both tests read the table through the assumption, the next one too.
const WORKER_CLEARS: &str = r#"
codeunit 97578 "G1X Clearer"
{
    trigger OnRun()
    var
        Row: Record "G1W Row";
        Results: Dictionary of [Text, Text];
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag);
        Row.FilterGroup(0);
        Results.Add('Seen', Format(Row.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97579 "G1X Tests"
{
    Subtype = Test;

    [Test]
    procedure WorkerClears_TestKeepsItsFilter()
    var
        Row: Record "G1W Row";
        Fresh: Record "G1W Row";
        Hosts: TestPage "G1W Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Row."No." := 'A';
        Row.Insert();
        Row."No." := 'B';
        Row.Tag := 7;
        Row.Insert();
        Row.FilterGroup(1);
        Row.SetRange(Tag, 7);
        Row.FilterGroup(0);
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"G1X Clearer", Parameters, false);
        Hosts.Close();
        if Results.Get('Seen') <> '2' then
            Error('The worker counted %1', Results.Get('Seen'));
        if Fresh.Count() <> 1 then
            Error('After the task the test session counted %1', Fresh.Count());
    end;

    [Test]
    procedure WorkerClears_NextTestFreshCount()
    var
        Fresh: Record "G1W Row";
        Seen: Integer;
    begin
        Seen := Fresh.Count();
        Fresh.FilterGroup(1);
        Fresh.SetRange(Tag);
        Fresh.FilterGroup(0);
        if Seen <> 1 then
            Error('The next test counted %1', Seen);
    end;
}
"#;

#[test]
fn issue_8670_worker_clears_group1_flags_the_later_reads() {
    let src = format!(
        "{}\n{WORKER_CLEARS}",
        include_str!("../fixtures/outside-fb/Issue8670Probe.al")
    );
    let o = outcomes(&src, "G1X Tests");
    assert_eq!(o.len(), 2);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BgTaskWorkerFilterGroupOne), "{name}: {marks:?}");
    }
}
