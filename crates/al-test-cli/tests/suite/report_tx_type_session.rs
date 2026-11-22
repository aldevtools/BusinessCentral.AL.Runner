//! A TransactionType = Update report run with no request page anywhere but at the start of the test method. The corpus
//! measures the platform's transaction-world decision for such a report only there: nothing before the run but, in some
//! tests, one Insert (record/TestTxModelReportRun.al:177-211 Test07/Test08; record/TestTxModelReportExec.al:
//! 137-171, :213-257 Test06/07/10/11/12). After a Commit, a rollback, a read call, a CurrentTransactionType set, a
//! Codeunit.Run or another report run, and under TransactionModel None, whether the report enters a world is not
//! pinned, so the run is refused (al-runner issue 8324, the reporter's repro is `UpdateReportAfterTypeSetToUpdate`; the
//! other shapes are the review probes of pull request 8376).

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

codeunit 50303 "Repro Writer"
{
    trigger OnRun()
    var
        ReproRow: Record "Repro Row";
    begin
        ReproRow."Entry No." := 3;
        ReproRow.Insert();
    end;
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

    // A Commit, then a set back to UpdateNoLocks: the type reads UpdateNoLocks again, but the moment is not the test's
    // start (review probe N02).
    [Test]
    procedure CommitThenSetUpdateNoLocks()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // A second run, after the first run's world committed (review probe N04).
    [Test]
    procedure SecondRunAfterWorldCommit()
    var
        First: Report "Repro Update Report";
        Second: Report "Repro Update Report";
    begin
        First.UseRequestPage(false);
        First.Run();
        Second.UseRequestPage(false);
        Second.Run();
    end;

    // A read call before the run, at the test's start (review probe N06).
    [Test]
    procedure ReadBeforeRun()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        if ReproRow.FindFirst() then;
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // After a trapped error rolled the transaction back (review probe N09).
    [Test]
    procedure AfterTrappedRollback()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        asserterror Error('trapped');
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // After a Codeunit.Run that wrote, at the test's start (review probe N08).
    [Test]
    procedure AfterCodeunitRun()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Codeunit.Run(Codeunit::"Repro Writer");
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // The session type set to Snapshot at the test's start (review probe N07).
    [Test]
    procedure SetSnapshotAtTestStart()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        CurrentTransactionType(TransactionType::Snapshot);
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // Under AutoRollback, the session type set to Update at the test's start (review probe N11).
    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure AutoRollbackSetUpdateAtStart()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        CurrentTransactionType(TransactionType::Update);
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // Under TransactionModel None at the test's start (review probe N12).
    [Test]
    [TransactionModel(TransactionModel::None)]
    procedure ModelNoneAtTestStart()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // Under TransactionModel None after a Commit.
    [Test]
    [TransactionModel(TransactionModel::None)]
    procedure ModelNoneAfterCommit()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        ReproUpdateReport.UseRequestPage(false);
        ReproUpdateReport.Run();
    end;

    // Under AutoRollback at the test's start with no write: the report runs
    // (record/TestTxModelReportExec.al:241-257 Test12).
    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure AutoRollbackAtTestStart()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        ReproUpdateReport.UseRequestPage(false);
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
    let mut wrong = Vec::new();
    for t in [
        "UpdateReportAfterTypeSetToUpdate",
        "AfterCommitWithoutWrite",
        "CommitThenSetUpdateNoLocks",
        "SecondRunAfterWorldCommit",
        "ReadBeforeRun",
        "AfterTrappedRollback",
        "AfterCodeunitRun",
        "SetSnapshotAtTestStart",
        "AutoRollbackSetUpdateAtStart",
        "ModelNoneAtTestStart",
        "ModelNoneAfterCommit",
    ] {
        match get(t) {
            Outcome::NotRun(r) if r.contains("run without its request page after the start of the test method") => {}
            other => wrong.push(format!("{t}: expected a refusal, got {other:?}")),
        }
    }
    for t in [
        "AtTestStartWithPendingWrite",
        "AtTestStartWithoutWrite",
        "AfterCommitWithRequestPage",
        "AutoRollbackAtTestStart",
    ] {
        if get(t) != Outcome::Pass {
            wrong.push(format!("{t}: expected a pass, got {:?}", get(t)));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
