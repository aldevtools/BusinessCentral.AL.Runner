//! A TransactionType = Update report run with no request page, in a session whose transaction type is not UpdateNoLocks.
//! The corpus measures the platform's transaction-world decision only at a test's start, where the type is UpdateNoLocks
//! (record/TestTxModelReportRun.al:177-211 Test07/Test08; record/TestTxModelReportExec.al:137-150 Test06). After
//! `Commit()` the type is Update (session-system/ProbeSystemEnumMembers.al:94-112): whether the report then still enters
//! a world is not pinned, so the run is refused (al-runner issue 8324, the reporter's repro is `UpdateReportAfterTypeSetToUpdate`).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50300 "Repro Row"
{
    DataClassification = CustomerContent;
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Touched; Boolean) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

report 50301 "Repro Update Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    TransactionType = Update;

    dataset
    {
        dataitem(ReproRow; "Repro Row")
        {
            trigger OnAfterGetRecord()
            begin
                Touched := true;
                Modify();
            end;
        }
    }
}

codeunit 50302 "Repro Report Tx Tests"
{
    Subtype = Test;

    [Test]
    procedure UpdateReportAfterTypeSetToUpdate()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        // [GIVEN] the next transaction is an Update transaction, and it has a pending write
        Commit();
        CurrentTransactionType(TransactionType::Update);
        ReproRow."Entry No." := 1;
        ReproRow.Insert();
        // [WHEN] a report with TransactionType = Update and no request page runs
        ReproRow.SetRange("Entry No.", 1);
        ReproUpdateReport.SetTableView(ReproRow);
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
        // [THEN] it ran and touched the row
        ReproRow.Get(1);
        if not ReproRow.Touched then
            Error('The report did not touch the row');
    end;

    // After a Commit with no write pending: the session's type is Update as well.
    [Test]
    procedure AfterCommitWithoutWrite()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // At the test's start (UpdateNoLocks) with a pending write: main's pinned refusal
    // (record/TestTxModelReportRun.al:199-211 Test08).
    [Test]
    procedure AtTestStartWithPendingWrite()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        ReproRow."Entry No." := 2;
        ReproRow.Insert();
        ReproUpdateReport.UseRequestPage(false);
        asserterror ReproUpdateReport.Run();
        if GetLastErrorText() <> 'An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.' then
            Error('text %1', GetLastErrorText());
    end;

    // At the test's start with no write: the report runs in its own world (record/TestTxModelReportRun.al:177-196 Test07).
    [Test]
    procedure AtTestStartWithoutWrite()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // After a Commit with the request page shown: the page enters the world whatever the type, main's result
    // (record/TestTxModelReportRun.al:152-174 Test06).
    [Test]
    [HandlerFunctions('RP')]
    procedure AfterCommitWithRequestPage()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        ReproUpdateReport.UseRequestPage(true);
        ReproUpdateReport.Run();
    end;

    [RequestPageHandler]
    procedure RP(var Page: TestRequestPage "Repro Update Report")
    begin
        Page.OK().Invoke();
    end;
}
"#;

#[test]
fn an_update_report_after_the_session_type_became_update_is_refused_and_its_neighbors_keep_their_result() {
    let o = outcomes_in(SRC, "Repro Report Tx Tests");
    let get = |t: &str| {
        o.iter()
            .find(|(n, _)| n == t)
            .map(|(_, o)| o.clone())
            .unwrap_or_else(|| panic!("{t}: {o:?}"))
    };
    for t in ["UpdateReportAfterTypeSetToUpdate", "AfterCommitWithoutWrite"] {
        match get(t) {
            Outcome::NotRun(r) => assert!(r.contains("not pinned"), "{t}: {r}"),
            other => panic!("{t}: expected a refusal, got {other:?}"),
        }
    }
    for t in [
        "AtTestStartWithPendingWrite",
        "AtTestStartWithoutWrite",
        "AfterCommitWithRequestPage",
    ] {
        assert_eq!(get(t), Outcome::Pass, "{t}");
    }
}
