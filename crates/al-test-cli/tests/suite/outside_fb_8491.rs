//! The corpus probe of al-runner issue 8491 (corpus pull request 1459, merged, not pinned yet, and the cells from :548 on in
//! corpus pull request 1489, open: tests/al-language/pages/background-tasks/BgTaskInsertQueue.al, measured on BC 29.1, 30 of
//! 30 green) and the reporter's repro, run here until a bump pins the probe: a
//! page background task's worker does not see the rows the test session's insert queue holds (`al-vm`
//! records/insert_buffer.rs, `hide_queued_from_worker`). Delete this file and the fixtures when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{self, BgTaskInsertQueue};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../fixtures/outside-fb/Issue8491Probe.al");

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

/// Every probe test passes; those whose worker runs while the queue holds rows are flagged.
#[test]
fn issue_8491_probe_cells() {
    let o = outcomes(PROBE, "IQ Tests");
    assert_eq!(o.len(), 30);
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
    let o = outcomes(include_str!("../fixtures/outside-fb/Issue8491.al"), "Repro Buf Tests");
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert_eq!(
            marks.contains(&BgTaskInsertQueue),
            name.starts_with("InsertThenTask"),
            "{name}: {marks:?}"
        );
    }
}

/// Not measured, refused: a worker's read of a table whose duplicate-key Insert is deferred in the test session, and a worker's
/// TableRelation check of a table whose queue holds rows. (Rows queued under another company value: al-testrun
/// `companies_opt_in.rs`, which runs with two companies.)
const NEIGHBORS: &str = r#"
codeunit 97590 "IQ Neighbor Tests"
{
    Subtype = Test;

    [Test]
    procedure DeferredDuplicate_TaskRead()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Parameters: Dictionary of [Text, Text];
    begin
        Row."No." := 'IQNB1';
        Row.Insert();
        Row.Get('IQNB1');
        Host.OpenView();
        Row."No." := 'IQNB2';
        Row.Insert();
        Row."No." := 'IQNB1';
        Row.Insert();
        Parameters.Add('No', 'IQNB2');
        Parameters.Add('Mode', 'get');
        Host.RunPageBackgroundTask(Codeunit::"IQ Reader", Parameters, false);
    end;

    [Test]
    procedure Queued_TaskTableRelation()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Parameters: Dictionary of [Text, Text];
    begin
        Host.OpenView();
        Row."No." := 'IQNB3';
        Row.Insert();
        Parameters.Add('No', 'IQNB3');
        Host.RunPageBackgroundTask(Codeunit::"IQ Relation Reader", Parameters, false);
    end;
}

codeunit 97591 "IQ Relation Reader"
{
    trigger OnRun()
    var
        Link: Record "IQ Link";
        Results: Dictionary of [Text, Text];
        No: Text;
    begin
        Page.GetBackgroundParameters().Get('No', No);
        Link.Validate("Row No.", No);
        Results.Add('Seen', Link."Row No.");
        Page.SetBackgroundTaskResult(Results);
    end;
}
"#;

#[test]
fn issue_8491_unmeasured_neighbors() {
    let o = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "IQ Neighbor Tests");
    for (name, why) in [
        (
            "DeferredDuplicate_TaskRead",
            "a read of a page background task's worker of IQ Row while the error of an Insert into it is deferred",
        ),
        (
            "Queued_TaskTableRelation",
            "the TableRelation check of IQ Row by a page background task's worker",
        ),
    ] {
        let (_, r, _) = o.iter().find(|(n, _, _)| n == name).expect("the test");
        assert!(matches!(r, Outcome::NotRun(w) if w.contains(why)), "{name}: {r:?}");
    }
}
