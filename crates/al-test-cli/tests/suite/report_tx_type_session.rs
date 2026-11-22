//! A TransactionType = Update report run with no request page, by the moment it runs at. The corpus measures the platform's
//! transaction-world decision at the start of the test method (record/TestTxModelReportRun.al:177-211 Test07/Test08;
//! record/TestTxModelReportExec.al:137-171, :213-257 Test06/07/10/11/12), and the open probe of al-runner issue 8324
//! (corpus pull request 1488, record/TxTypeUpdateReport.al, copied to fixtures/outside-fb/Issue8324Probe.al) measures it
//! after a Commit and after a CurrentTransactionType set: no world while the session's type is Update, a world for every
//! other type. Every other moment (a rollback, a Codeunit.Run, a read and a write, a TryFunction, a lock, TransactionModel
//! None) stays refused. The reporter's repro is `UpdateReportAfterTypeSetToUpdate`; the review probes of pull request 8376
//! and the neighbors of the probe's cells are the other shapes.

use al_sema::workspace::AppKind;
use al_testkit::outcomes_in;
use al_testrun::Outcome;

const PROBE_APP_ID: &str = "3c1f2b7e-8324-4a8f-9b2e-5d6c7a8b9c01";

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

codeunit 50304 "Repro Committer"
{
    trigger OnRun()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
    end;
}

codeunit 50305 "Repro Publisher"
{
    procedure Raise()
    begin
        OnCommitPoint();
    end;

    [IntegrationEvent(false, false)]
    local procedure OnCommitPoint()
    begin
    end;
}

codeunit 50306 "Repro Subscriber"
{
    // A Commit, a set and the run in an event subscriber.
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Repro Publisher", 'OnCommitPoint', '', false, false)]
    local procedure CommitSetUpdateAndRun()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        ReproUpdateReport.Run();
    end;
}

codeunit 50302 "Repro Report Tx Tests"
{
    Subtype = Test;

    // A test method before the shape: a codeunit's first test method starts in the test runner's transaction.
    [Test]
    procedure Nothing()
    begin
    end;

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

    // After a Commit with no write pending, at the Update the test before left.
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

    // ----- Neighbors of the probe's cells (corpus pull request 1488) that no cell measures -----

    // The set at the test's start, then a read and a write: the probe reads or writes there, not both.
    [Test]
    procedure SetUpdateAtStartReadThenWrite()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        CurrentTransactionType(TransactionType::Update);
        if ReproRow.FindFirst() then;
        ReproRow."Entry No." := 11;
        ReproRow.Insert();
        ReproUpdateReport.Run();
    end;

    // After the Commit and the set, a read and a write, then a run, then another run: the probe runs twice only with
    // nothing before the first run.
    [Test]
    procedure CommitSetUpdateReadWriteTwoRuns()
    var
        ReproRow: Record "Repro Row";
        First: Report "Repro Update Report";
        Second: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        ReproRow."Entry No." := 13;
        ReproRow.Insert();
        First.Run();
        Second.Run();
    end;

    // A Commit that keeps the type Update, with no set after it: the probe commits without a set only at UpdateNoLocks.
    [Test]
    procedure CommitKeepsUpdateNoSet()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        CurrentTransactionType(TransactionType::Update);
        ReproRow."Entry No." := 12;
        ReproRow.Insert();
        Commit();
        ReproUpdateReport.Run();
    end;

    // A Codeunit.Run between the set and the report.
    [Test]
    procedure CommitSetUpdateCodeunitRunBetween()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        Codeunit.Run(Codeunit::"Repro Writer");
        ReproUpdateReport.Run();
    end;

    // Two sets after the Commit.
    [Test]
    procedure CommitSetUpdateTwice()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        CurrentTransactionType(TransactionType::Update);
        ReproUpdateReport.Run();
    end;

    // A LockTable between the set and the report.
    [Test]
    procedure CommitSetUpdateLockTable()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        ReproRow.LockTable();
        ReproUpdateReport.Run();
    end;

    // The run inside a TryFunction.
    [Test]
    procedure CommitSetUpdateRunInTry()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if TryRun() then;
        if GetLastErrorText() <> '' then
            Error(GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryRun()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        ReproUpdateReport.Run();
    end;

    // The run inside an asserterror: the probe runs a report that enters no world only outside one.
    [Test]
    procedure CommitSetUpdateRunInAssertError()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        asserterror begin
            ReproUpdateReport.Run();
            Error('after the run');
        end;
    end;

    // A third run after the set: the probe runs two.
    [Test]
    procedure CommitSetUpdateThreeRuns()
    var
        First: Report "Repro Update Report";
        Second: Report "Repro Update Report";
        Third: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        First.Run();
        Second.Run();
        Third.Run();
    end;

    // Snapshot after the Commit with a pending write: the probe sets Snapshot only with no write.
    [Test]
    procedure CommitSetSnapshotThenRead()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        if ReproRow.FindFirst() then;
        ReproUpdateReport.Run();
    end;

    // The set at the test's start, then a DeleteAll: the probe writes there with one Insert.
    [Test]
    procedure SetUpdateAtStartDeleteAll()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        CurrentTransactionType(TransactionType::Update);
        ReproRow.DeleteAll();
        ReproUpdateReport.Run();
    end;

    // After the Commit and the set, a FindLast: the probe reads with Get, FindFirst, FindSet, Next, Count and IsEmpty.
    [Test]
    procedure CommitSetUpdateFindLast()
    var
        ReproRow: Record "Repro Row";
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if ReproRow.FindLast() then;
        ReproUpdateReport.Run();
    end;

    // An Insert between two runs: the probe runs twice with nothing between.
    [Test]
    procedure CommitSetUpdateRunInsertRun()
    var
        ReproRow: Record "Repro Row";
        First: Report "Repro Update Report";
        Second: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        First.Run();
        ReproRow."Entry No." := 14;
        ReproRow.Insert();
        Second.Run();
    end;

    // A Codeunit.Run whose OnRun commits and sets Update, then the report in the caller.
    [Test]
    procedure CodeunitRunCommitsAndSetsUpdate()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Codeunit.Run(Codeunit::"Repro Committer");
        ReproUpdateReport.Run();
    end;

    // The Commit and the set in a procedure, the report in the test: the probe runs it where it committed and set.
    [Test]
    procedure CommitAndSetInAProcedure()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        CommitAndSetUpdate();
        ReproUpdateReport.Run();
    end;

    local procedure CommitAndSetUpdate()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
    end;

    // An isolated storage write after the Commit: it starts the transaction outside the calls the probe makes.
    [Test]
    procedure CommitIsolatedStorageRun()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        IsolatedStorage.Set('k', 'v', DataScope::Module);
        ReproUpdateReport.Run();
    end;

    // The same after the set to Update.
    [Test]
    procedure CommitSetUpdateIsolatedStorageRun()
    var
        ReproUpdateReport: Report "Repro Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        IsolatedStorage.Set('k', 'v', DataScope::Module);
        ReproUpdateReport.Run();
    end;

    // The Commit, the set and the run in an event subscriber the test raises.
    [Test]
    procedure CommitSetUpdateRunInASubscriber()
    var
        ReproPublisher: Codeunit "Repro Publisher";
    begin
        ReproPublisher.Raise();
    end;

    // An Insert at the Snapshot a set after the Commit gave: Snapshot does not write, so the Insert raises before the set.
    [Test]
    procedure CommitSetSnapshotInsertSetUpdate()
    var
        ReproRow: Record "Repro Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        ReproRow."Entry No." := 15;
        ReproRow.Insert();
        CurrentTransactionType(TransactionType::Update);
    end;

    [RequestPageHandler]
    procedure RP(var Page: TestRequestPage "Repro Update Report")
    begin
        Page.OK().Invoke();
    end;
}
"#;

/// Text of the refusal for an Update report without its request page at a moment no cell measures.
const UNMEASURED: &str = "run without its request page at a moment no corpus cell measures";

/// The test procedure `name` of SRC with its attributes, the text from its `[Test]` line through its `end;`.
fn procedure_of(name: &str) -> &'static str {
    let at = SRC
        .find(&format!("procedure {name}()"))
        .unwrap_or_else(|| panic!("no test {name}"));
    let start = SRC[..at].rfind("    [Test]").expect("its [Test] line");
    let end = at + SRC[at..].find("\n    end;\n").expect("its end") + "\n    end;\n".len();
    &SRC[start..end]
}

/// Each of `names`, in this order, after the test `Nothing`, in one test codeunit of its own with SRC's objects, helpers
/// and handler; the outcome of the last one. A codeunit runs in a session of its own, so the type an earlier shape leaves
/// reaches no other shape.
fn run_alone(names: &[&str]) -> Outcome {
    run_in_order(&[&["Nothing"], names].concat())
}

/// [`run_alone`] without the test `Nothing` first: `names[0]` is the codeunit's first test method.
fn run_in_order(names: &[&str]) -> Outcome {
    let objects = &SRC[..SRC.find("codeunit 50302").expect("the test codeunit")];
    let helpers: String = [
        "local procedure TryRun()",
        "local procedure CommitAndSetUpdate()",
        "procedure RP(",
    ]
    .iter()
    .map(|h| {
        let at = SRC.find(h).expect("the helper");
        let start = SRC[..at].rfind("\n\n").expect("its start") + 2;
        let end = at + SRC[at..].find("\n    end;\n").expect("its end") + "\n    end;\n".len();
        &SRC[start..end]
    })
    .collect::<Vec<_>>()
    .join("\n");
    let tests: String = names.iter().map(|n| procedure_of(n)).collect::<Vec<_>>().join("\n");
    let src = format!(
        "{objects}codeunit 50302 \"Repro Report Tx Tests\"\n{{\n    Subtype = Test;\n\n{tests}\n{helpers}\n}}\n"
    );
    let o = outcomes_in(&src, "Repro Report Tx Tests");
    let last = names.last().expect("a test");
    o.into_iter()
        .find(|(n, _)| n == last)
        .map(|(_, o)| o)
        .unwrap_or_else(|| panic!("{last}: no outcome"))
}

#[test]
fn an_update_report_without_its_page_runs_at_the_measured_moments_and_is_refused_at_the_others() {
    let mut wrong = Vec::new();
    for t in [
        "SecondRunAfterWorldCommit",
        "ReadBeforeRun",
        "AfterTrappedRollback",
        "AfterCodeunitRun",
        "SetSnapshotAtTestStart",
        "AutoRollbackSetUpdateAtStart",
        "ModelNoneAtTestStart",
        "ModelNoneAfterCommit",
        "SetUpdateAtStartReadThenWrite",
        "CommitSetUpdateReadWriteTwoRuns",
        "CommitKeepsUpdateNoSet",
        "CommitSetUpdateCodeunitRunBetween",
        "CommitSetUpdateTwice",
        "CommitSetUpdateLockTable",
        "CommitSetUpdateRunInTry",
        "CommitSetUpdateRunInAssertError",
        "CommitSetUpdateThreeRuns",
        "CommitSetSnapshotThenRead",
        "SetUpdateAtStartDeleteAll",
        "CommitSetUpdateFindLast",
        "CommitSetUpdateRunInsertRun",
        "CodeunitRunCommitsAndSetsUpdate",
        "CommitAndSetInAProcedure",
        "CommitIsolatedStorageRun",
        "CommitSetUpdateIsolatedStorageRun",
        "CommitSetUpdateRunInASubscriber",
    ] {
        match run_alone(&[t]) {
            Outcome::NotRun(r) if r.starts_with("TransactionType = Update report") && r.contains(UNMEASURED) => {}
            other => wrong.push(format!("{t}: expected its own refusal, got {other:?}")),
        }
    }
    // The reporter's repro and review probes of pull request 8376 the probe now measures (TxU06, TxU07, TxU11), and the
    // pinned shapes at the test's start.
    for t in [
        "UpdateReportAfterTypeSetToUpdate",
        "AfterCommitWithoutWrite",
        "CommitThenSetUpdateNoLocks",
        "AtTestStartWithPendingWrite",
        "AtTestStartWithoutWrite",
        "AfterCommitWithRequestPage",
        "AutoRollbackAtTestStart",
    ] {
        let o = run_alone(&[t]);
        if o != Outcome::Pass {
            wrong.push(format!("{t}: expected a pass, got {o:?}"));
        }
    }
    // After the Commit, a test before that left Update: no cell commits at Update without a set.
    match run_alone(&["UpdateReportAfterTypeSetToUpdate", "AfterCommitWithoutWrite"]) {
        Outcome::NotRun(r) if r.starts_with("TransactionType = Update report") && r.contains(UNMEASURED) => {}
        other => wrong.push(format!(
            "AfterCommitWithoutWrite after Update: expected its own refusal, got {other:?}"
        )),
    }
    // A test refused while it ran whose reach sets the type leaves a type that is not known: the next test's pinned shape
    // refuses. One whose reach sets none leaves the type it had.
    match run_alone(&["SetSnapshotAtTestStart", "AtTestStartWithoutWrite"]) {
        Outcome::NotRun(r) if r.contains("did not run to its end and could set it") => {}
        other => wrong.push(format!(
            "AtTestStartWithoutWrite after a refused test that sets the type: expected a refusal, got {other:?}"
        )),
    }
    let o = run_alone(&["ReadBeforeRun", "AtTestStartWithoutWrite"]);
    if o != Outcome::Pass {
        wrong.push(format!(
            "AtTestStartWithoutWrite after a refused test that sets no type: expected a pass, got {o:?}"
        ));
    }
    // A set to another type at the start of a codeunit's first test method: whether the test runner's transaction has
    // started there depends on the codeunits that ran before it (session-system/TxTypeFirstTestMethod.al of the open probe).
    for t in ["SetSnapshotAtTestStart", "AutoRollbackSetUpdateAtStart"] {
        match run_in_order(&[t]) {
            Outcome::NotRun(r) if r.contains("at the start of a test codeunit's first test method") => {}
            other => wrong.push(format!(
                "{t} first: expected the first test method's refusal, got {other:?}"
            )),
        }
    }
    // An Insert at Snapshot raises (record/TxTypeSetAfterCall.al TxS16 of the open probe).
    match run_alone(&["CommitSetSnapshotInsertSetUpdate"]) {
        Outcome::Fail(m) if m.starts_with("Write operations in the database") => {}
        other => wrong.push(format!(
            "CommitSetSnapshotInsertSetUpdate: expected DB:TransActionIsReadOnly, got {other:?}"
        )),
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The probe of corpus pull request 1488 (record/TxTypeUpdateReport.al): every test passes. The cells after a Commit or a set
/// carry the mark of the open probe, and a read of a type a Commit kept (other than Update) the mark of al-runner issue 8605.
#[test]
fn the_probe_of_issue_8324_passes_with_its_marks() {
    // Over the System app, for the Integer data items; `run` takes the test codeunit named T.
    let src = format!(
        "{}\n{}",
        include_str!("../fixtures/outside-fb/Issue8324Probe.al")
            .replace("codeunit 97184 \"TxU Update Report Tx\"", "codeunit 97184 T"),
        include_str!("../fixtures/outside-sc/Assert.al")
    );
    let o = crate::field_extension_package_ids::run(&[(PROBE_APP_ID, AppKind::Workspace, &src)]);
    let report = al_builtins::assumptions::by_id("update-report-by-session-type");
    let commit = al_builtins::assumptions::by_id("commit-keeps-transaction-type");
    let mut wrong = Vec::new();
    for (name, outcome, marks) in &o {
        if *outcome != Outcome::Pass {
            wrong.push(format!("{name}: {outcome:?}"));
            continue;
        }
        // Every cell but the request page shown (TxU16, TxU17, main's path) and the type reads around a Commit.
        let cell = [
            "TxU01", "TxU02", "TxU03", "TxU04", "TxU05", "TxU06", "TxU07", "TxU08", "TxU09", "TxU10", "TxU11", "TxU12",
            "TxU13", "TxU14", "TxU15", "TxU18", "TxU19", "TxU20", "TxU27", "TxU28", "TxU29", "TxU30", "TxU31", "TxU32",
            "TxU33",
        ]
        .iter()
        .any(|p| name.starts_with(p));
        let kept = [
            "TxU01", "TxU02", "TxU07", "TxU08", "TxU19", "TxU20", "TxU21", "TxU23", "TxU24", "TxU25", "TxU34", "TxU35",
            "TxU36", "TxU37", "TxU38",
        ]
        .iter()
        .any(|p| name.starts_with(p));
        let mut want = Vec::new();
        if cell {
            want.extend(report);
        }
        if kept {
            want.extend(commit);
        }
        let mut got = marks.clone();
        got.sort();
        want.sort();
        if got != want {
            wrong.push(format!("{name}: marks {got:?}, expected {want:?}"));
        }
    }
    assert_eq!(o.len(), 40, "{o:#?}");
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The probe of al-runner issue 8606 in corpus pull request 1488 (session-system/TxTypeNextTestMethod.al): a test method starts
/// with the type the one before it left. Reading a carried Browse, Snapshot or Report carries the mark of the open probe; an
/// Update report at the start of the test after one that left Update enters no world (the mark of al-runner issue 8324).
#[test]
fn the_probe_of_issue_8606_carries_the_type_into_the_next_test_method() {
    // Over the System app for the probe's report, whose objects come from the 8324 probe; `run` takes the codeunit named T.
    let src = format!(
        "{}\n{}\n{}",
        include_str!("../fixtures/outside-fb/Issue8606Probe.al")
            .replace("codeunit 97185 \"TxU Type Next Test\"", "codeunit 97185 T"),
        include_str!("../fixtures/outside-fb/Issue8324Probe.al"),
        include_str!("../fixtures/outside-sc/Assert.al")
    );
    let o = crate::field_extension_package_ids::run(&[(PROBE_APP_ID, AppKind::Workspace, &src)]);
    let carries = al_builtins::assumptions::by_id("transaction-type-carries-to-next-test").expect("the entry");
    let report = al_builtins::assumptions::by_id("update-report-by-session-type").expect("the entry");
    let commit = al_builtins::assumptions::by_id("commit-keeps-transaction-type").expect("the entry");
    let want = |n: &str| match n {
        "TxN02_StartsWithThePreviousTestsType"
        | "TxN04_StartsWithSnapshot"
        | "TxN06_StartsWithReport"
        | "TxN08_StartsWithBrowseAfterARead" => vec![carries],
        // A carried type rests on every assumption it went through: the Snapshot a Commit kept, and the carried Update.
        "TxN10_StartsWithTheSnapshotACommitKept" => vec![commit, carries],
        "TxN12_StartsWithUpdateAndAnUpdateReportJoinsTheTransaction" => vec![carries, report],
        _ => vec![],
    };
    let wrong: Vec<_> = o
        .iter()
        .filter(|(n, r, m)| *r != Outcome::Pass || *m != want(n))
        .collect();
    assert_eq!(o.len(), 12, "{o:#?}");
    assert!(wrong.is_empty(), "{wrong:#?}");
}
