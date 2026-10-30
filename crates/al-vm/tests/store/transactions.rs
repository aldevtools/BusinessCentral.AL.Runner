//! Codeunit.Run and the write-transaction rules around it, end to end.
//!
//! Each test cites the corpus test that pins the behaviour.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;
use al_vm::{Engine, Session};

const FIXTURES: &str = r#"
table 50900 "TX T"
{
    fields { field(1; K; Code[10]) { } field(2; V; Text[30]) { } }
    keys { key(PK; K) { } }
}

codeunit 50910 Erroring
{
    trigger OnRun() begin Error('BOOM'); end;
}

// corpus codeunit/ALTRunTxInserter.Codeunit.al: inserts a marker and commits.
codeunit 50911 Inserter
{
    trigger OnRun() var R: Record "TX T"; begin R.K := 'M'; R.Insert(); Commit(); end;
}

// corpus codeunit/ALTRunTxDirtyInserter.Codeunit.al: inserts and does not commit.
codeunit 50912 DirtyInserter
{
    trigger OnRun() var R: Record "TX T"; begin R.K := 'D'; R.Insert(); end;
}

// corpus codeunit/ALTRunTxWriteThenError.Codeunit.al
codeunit 50913 WriteThenError
{
    trigger OnRun() var R: Record "TX T"; begin R.K := 'W'; R.Insert(); Error('BOOM-W'); end;
}

// corpus codeunit/ALTRunTxCommitThenError.Codeunit.al
codeunit 50914 CommitThenError
{
    trigger OnRun() var R: Record "TX T"; begin
        R.K := 'C1'; R.Insert(); Commit();
        R.K := 'C2'; R.Insert();
        Error('BOOM-C');
    end;
}

codeunit 50915 NoOnRun
{
    procedure P(): Integer begin exit(7); end;
}

codeunit 50916 Probe
{
    var Runs: Integer; Calls: Integer;
    trigger OnRun() begin Runs += 1; end;
    procedure Triple(N: Integer): Integer begin Calls += 1; exit(3 * N); end;
    procedure GetRuns(): Integer begin exit(Runs); end;
    procedure GetCalls(): Integer begin exit(Calls); end;
}

codeunit 50917 Single
{
    SingleInstance = true;
    var Bumps: Integer;
    trigger OnRun() begin Bumps += 1; end;
    procedure GetBumps(): Integer begin exit(Bumps); end;
}

// corpus handlers/TestPageActionRunObjectKinds_Objects.al:254-268: TableNo gives OnRun a Rec.
codeunit 50919 RecReader
{
    TableNo = "TX T";
    trigger OnRun() var L: Record "TX T"; begin L.K := 'LOG'; L.V := Rec.V; L.Insert(); end;
}

codeunit 50920 RecWriter
{
    TableNo = "TX T";
    trigger OnRun() begin Rec.V := 'changed'; end;
}

codeunit 50918 Outer
{
    trigger OnRun() var Ok: Boolean; R: Record "TX T"; begin
        Ok := Codeunit.Run(Codeunit::Erroring);
        R.K := 'O'; R.Insert();
    end;
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    run_in("", body, |_| {})
}

/// Like `run`, with extra objects and a hook that sets the session up the way the test runner
/// does (`tx_model`, `in_test_codeunit`) before F is called.
fn run_in(extra: &str, body: &str, setup: impl FnOnce(&mut Session)) -> Result<Value, Signal> {
    let src = format!("{FIXTURES}\n{extra}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        panic!("F is blocked: {r}");
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    setup(&mut s);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn f(body: &str) -> String {
    text(run(&format!("procedure F(): Text {body}")))
}

// codeunit/TestCodeunitRunGuard.al:GuardedRun_TrapsInnerError_ReturnsFalse
#[test]
fn a_guarded_run_traps_the_inner_error_and_returns_false() {
    assert_eq!(
        f("var Ok: Boolean; begin Ok := Codeunit.Run(Codeunit::Erroring); exit(Format(Ok) + GetLastErrorText()); end;"),
        "NoBOOM"
    );
}

// codeunit/TestCodeunitRunGuard.al:UnguardedRun_PropagatesInnerError
#[test]
fn an_unguarded_run_propagates_the_inner_error() {
    assert_eq!(
        f("begin asserterror Codeunit.Run(Codeunit::Erroring); exit(GetLastErrorText()); end;"),
        "BOOM"
    );
}

// codeunit/TestCodeunitRunGuard.al:GuardedRun_InstanceForm_TrapsInnerError_ReturnsFalse
// and UnguardedRun_InstanceForm_PropagatesInnerError
#[test]
fn the_instance_form_is_guarded_the_same_way() {
    assert_eq!(
        f("var G: Codeunit Erroring; Ok: Boolean; begin Ok := G.Run(); exit(Format(Ok) + GetLastErrorText()); end;"),
        "NoBOOM"
    );
    assert_eq!(
        f("var G: Codeunit Erroring; begin asserterror G.Run(); exit(GetLastErrorText()); end;"),
        "BOOM"
    );
}

// codeunit/TestCodeunitRunGuard.al:GuardedRun_InstanceForm_WriteThenError_RollsBackInsertedRow
#[test]
fn a_failed_guarded_run_rolls_back_its_own_writes() {
    assert_eq!(
        f("var R: Record \"TX T\"; Ok: Boolean; begin
            Ok := Codeunit.Run(Codeunit::WriteThenError);
            exit(Format(Ok) + Format(R.Count()) + GetLastErrorText());
        end;"),
        "No0BOOM-W"
    );
}

// codeunit/TestCodeunitRunGuard.al:GuardedRun_StaticForm_CommitThenError_KeepsCommittedRowOnly
#[test]
fn a_failed_guarded_run_keeps_what_it_committed_and_loses_what_it_wrote_after() {
    assert_eq!(
        f("var R: Record \"TX T\"; Ok: Boolean; begin
            Ok := Codeunit.Run(Codeunit::CommitThenError);
            exit(Format(Ok) + Format(R.Get('C1')) + Format(R.Get('C2')));
        end;"),
        "NoYesNo"
    );
}

// codeunit/TestCodeunitRunGuard.al:GuardedRun_InstanceForm_CommitThenError_KeepsCommittedRowOnly
#[test]
fn the_instance_form_keeps_the_committed_row_too() {
    assert_eq!(
        f("var R: Record \"TX T\"; G: Codeunit CommitThenError; Ok: Boolean; begin
            Ok := G.Run();
            exit(Format(Ok) + Format(R.Get('C1')) + Format(R.Get('C2')));
        end;"),
        "NoYesNo"
    );
}

// codeunit/TestCodeunitRunWriteTransaction.al:GuardedRun_WithUncommittedWrite_IsRefused_AndCodeunitNeverRuns
#[test]
fn a_guarded_run_is_refused_while_the_caller_has_an_uncommitted_write() {
    // The full text is quoted in corpus docs/handoff-2026-05-25-test-failures.md:242; the
    // test pins the fragment `the transaction is stopped`.
    assert_eq!(
        f("var R: Record \"TX T\"; Ok: Boolean; begin
            Commit();
            R.K := 'P'; R.Insert();
            asserterror Ok := Codeunit.Run(Codeunit::Inserter);
            Commit();
            R.Reset();
            exit(GetLastErrorText() + '|' + Format(Ok) + Format(R.Get('M')));
        end;"),
        "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|NoNo"
    );
}

// codeunit/TestCodeunitRunWriteTransaction.al:UnguardedRun_WithUncommittedWrite_IsAllowed_AndCodeunitRuns
#[test]
fn an_unguarded_run_is_allowed_with_an_uncommitted_write() {
    assert_eq!(
        f("var R: Record \"TX T\"; begin
            Commit();
            R.K := 'P'; R.Insert();
            Codeunit.Run(Codeunit::Inserter);
            exit(Format(R.Get('M')));
        end;"),
        "Yes"
    );
}

// codeunit/TestCodeunitRunWriteTransaction.al:GuardedRun_AfterCommit_IsAllowed_AndCodeunitRuns
#[test]
fn a_guarded_run_after_a_commit_runs_and_returns_true() {
    assert_eq!(
        f("var R: Record \"TX T\"; Ok: Boolean; begin
            R.K := 'P'; R.Insert(); Commit();
            Ok := Codeunit.Run(Codeunit::Inserter);
            exit(Format(Ok) + Format(R.Get('M')));
        end;"),
        "YesYes"
    );
}

// codeunit/TestCodeunitRunWriteTransaction.al:GuardedRun_LeavingUncommittedWrite_DoesNotBlockTheNextGuardedRun
#[test]
fn a_write_a_guarded_run_leaves_uncommitted_does_not_block_the_next_guarded_run() {
    assert_eq!(
        f("var R: Record \"TX T\"; A: Boolean; B: Boolean; begin
            Commit();
            A := Codeunit.Run(Codeunit::DirtyInserter);
            B := Codeunit.Run(Codeunit::Inserter);
            exit(Format(A) + Format(B));
        end;"),
        "YesYes"
    );
}

// codeunit/TestCodeunitRunWriteTransaction.al:GuardedRun_LeavingUncommittedWrite_CommitsItWithTheRunsOwnTransaction
#[test]
fn a_successful_guarded_run_commits_its_own_transaction() {
    assert_eq!(
        f("var R: Record \"TX T\"; Ok: Boolean; begin
            Commit();
            Ok := Codeunit.Run(Codeunit::DirtyInserter);
            asserterror Error('later');
            exit(Format(Ok) + Format(R.Get('D')));
        end;"),
        "YesYes"
    );
}

// codeunit/TestCodeunitRunContracts.al:Codeunit_Run_SetsErrorTextOnFailure (statement form, no
// OnRun) and types/TestLanguageEdgeCases.al:CodeunitRun_Success_ReturnsTrue (guarded, no OnRun)
#[test]
fn a_codeunit_without_onrun_runs_as_a_no_op() {
    assert_eq!(
        f(
            "var Ok: Boolean; begin Codeunit.Run(Codeunit::NoOnRun); Ok := Codeunit.Run(Codeunit::NoOnRun); exit(Format(Ok) + GetLastErrorText()); end;"
        ),
        "Yes"
    );
}

// codeunit/TestCodeunitVariableDispatch.al:CodeunitRun_ReachesOnRunAndNotTheProcedures and
// RunOnAVariable_ReachesThatVariablesOwnInstance
#[test]
fn the_static_form_runs_its_own_instance_and_the_instance_form_the_variables() {
    assert_eq!(
        f("var P: Codeunit Probe; begin
            Commit();
            Codeunit.Run(Codeunit::Probe);
            exit(Format(P.GetRuns()) + Format(P.GetCalls()));
        end;"),
        "00"
    );
    assert_eq!(
        f("var P: Codeunit Probe; begin
            Commit();
            P.Triple(4); P.Run();
            exit(Format(P.GetRuns()) + Format(P.GetCalls()));
        end;"),
        "11"
    );
}

// codeunit/TestCodeunitSingleInstanceLeak_Tests.al:TestCodeunit_SingleInstance_SurvivesACodeunitBoundary
#[test]
fn a_single_instance_codeunit_keeps_its_state_across_a_run() {
    assert_eq!(
        f("var S: Codeunit Single; begin
            Commit();
            Codeunit.Run(Codeunit::Single);
            exit(Format(S.GetBumps()));
        end;"),
        "1"
    );
}

// record/TestAutoIncrementGuardedRun.Objects.al: Outer H -- an outer run swallows a failed inner
// run, writes, and succeeds.
#[test]
fn a_nested_guarded_run_that_fails_leaves_the_outer_run_working() {
    assert_eq!(
        f("var R: Record \"TX T\"; Ok: Boolean; begin
            Commit();
            Ok := Codeunit.Run(Codeunit::Outer);
            exit(Format(Ok) + Format(R.Get('O')));
        end;"),
        "YesYes"
    );
}

// codeunit/TestCodeunitRunErrorTextsAndModels.al:162-196: text and code of a run of an id no object declares. The emit version is a number.
#[test]
fn running_a_codeunit_id_that_does_not_exist_raises_the_metadata_error() {
    let r = run("procedure F(): Text begin Codeunit.Run(99990); exit(''); end;");
    let Err(Signal::Error(e)) = r else { panic!("{r:?}") };
    assert!(
        e.message.starts_with("You tried to invoke the CodeUnit object with the ID 99990 from the object C. An object with that ID does not exist in the current application compiled with emit version "),
        "{}",
        e.message
    );
    assert!(
        e.message.ends_with('.')
            && e.message
                .trim_end_matches('.')
                .rsplit(' ')
                .next()
                .unwrap()
                .parse::<u32>()
                .is_ok()
    );
    assert_eq!(e.code, "AL:MetadataNotFound");
}

// ---- [CommitBehavior] (error-handling/TestCommitBehaviorAttribute.al) ----

const CB: &str = r#"
    [CommitBehavior(CommitBehavior::Ignore)]
    local procedure InsertCommitThenErrorIgnoring(K: Code[10]) var R: Record "TX T"; begin R.K := K; R.Insert(); Commit(); Error('unrelated'); end;
    local procedure InsertCommitThenError(K: Code[10]) var R: Record "TX T"; begin R.K := K; R.Insert(); Commit(); Error('unrelated'); end;
    [CommitBehavior(CommitBehavior::Ignore)]
    local procedure ViaCallee(K: Code[10]) begin InsertCommitThenError(K); end;
    [CommitBehavior(CommitBehavior::Ignore)]
    local procedure InsertAndIgnoreCommit(K: Code[10]) var R: Record "TX T"; begin R.K := K; R.Insert(); Commit(); end;
    [CommitBehavior(CommitBehavior::Error)]
    local procedure InsertThenCommitUnderError(K: Code[10]) var R: Record "TX T"; begin R.K := K; R.Insert(); Commit(); end;
"#;

fn g(body: &str) -> String {
    text(run(&format!("{CB}\nprocedure F(): Text {body}")))
}

// :CommitBehavior_Unattributed_CommitBeforeErrorMakesRowDurable (the control twin)
#[test]
fn without_the_attribute_the_commit_before_the_error_is_durable() {
    assert_eq!(
        g("var R: Record \"TX T\"; begin asserterror InsertCommitThenError('1'); exit(Format(R.Get('1'))); end;"),
        "Yes"
    );
}

// :CommitBehavior_Ignore_CommitBeforeErrorDoesNotMakeRowDurable
#[test]
fn an_ignored_commit_does_not_move_the_rollback_boundary() {
    assert_eq!(
        g(
            "var R: Record \"TX T\"; begin asserterror InsertCommitThenErrorIgnoring('1'); exit(Format(R.Get('1')) + GetLastErrorText()); end;"
        ),
        "Nounrelated"
    );
}

// :CommitBehavior_Ignore_GovernsCommitIssuedByAnUnattributedCallee
#[test]
fn the_attribute_governs_the_whole_dynamic_scope() {
    assert_eq!(
        g("var R: Record \"TX T\"; begin asserterror ViaCallee('2'); exit(Format(R.Get('2'))); end;"),
        "No"
    );
}

// :CommitBehavior_Ignore_ScopeIsPoppedOnReturn
#[test]
fn the_scope_ends_when_the_attributed_procedure_returns() {
    assert_eq!(
        g(
            "var R: Record \"TX T\"; begin InsertAndIgnoreCommit('3'); Commit(); asserterror Error('later'); exit(Format(R.Get('3'))); end;"
        ),
        "Yes"
    );
}

// :CommitBehavior_Error_CommitRaisesAndRowIsRolledBack
#[test]
fn an_error_scope_makes_commit_raise() {
    assert_eq!(
        g(
            "var R: Record \"TX T\"; begin asserterror InsertThenCommitUnderError('4'); exit(Format(R.Get('4')) + GetLastErrorText()); end;"
        ),
        "NoCommit is prohibited in the current scope. The operation cannot continue. Contact your system administrator."
    );
}

// The scope also ends when the attributed procedure fails.
#[test]
fn the_scope_ends_when_the_attributed_procedure_fails() {
    assert_eq!(
        g(
            "var R: Record \"TX T\"; begin asserterror InsertCommitThenErrorIgnoring('5'); R.K := 'Z'; R.Insert(); Commit(); asserterror Error('later'); exit(Format(R.Get('Z'))); end;"
        ),
        "Yes"
    );
}

// ---- mutation-testing additions (docs/status.md debt) ----

// An unguarded Codeunit.Run in the default model does not commit the caller's pending write
// (codeunit/TestCodeunitRunWriteTransaction.al:UnguardedRun_WithUncommittedWrite_IsAllowed_AndCodeunitRuns
// shows the write is still there afterwards; only a guarded run commits, see
// a_successful_guarded_run_commits_its_own_transaction).
#[test]
fn an_unguarded_run_under_autocommit_leaves_the_callers_write_uncommitted() {
    assert_eq!(
        f("var R: Record \"TX T\"; begin
            R.K := 'P'; R.Insert();
            Codeunit.Run(Codeunit::Probe);
            exit(Format(Database.IsInWriteTransaction()));
        end;"),
        "Yes"
    );
}

// codeunit/TestWriteTxTestBoundary.al:Test09 (a direct Insert under None is refused) and
// :Test10_ARunCodeunitMayWriteUnderNone (the run opens a transaction of its own, which ends
// with the run -- and the refusal applies again afterwards).
#[test]
fn under_transaction_model_none_a_run_may_write_and_the_refusal_returns_after_it() {
    let r = run_in(
        "",
        "procedure F(): Text
        var R: Record \"TX T\"; Seen: Text;
        begin
            asserterror begin R.K := 'A'; R.Insert(); end;
            Seen := GetLastErrorText();
            Codeunit.Run(Codeunit::DirtyInserter);
            Seen += '|' + Format(Database.IsInWriteTransaction());
            asserterror begin R.K := 'B'; R.Insert(); end;
            exit(Seen + '|' + GetLastErrorText());
        end;",
        |s| s.tx_model = al_vm::TxModel::None,
    );
    let none = "A transaction must be started before changes can be made to the database.";
    assert_eq!(text(r), format!("{none}|No|{none}"));
}

// codeunit/TestCodeunitRunNestedTestCodeunit.al:CodeunitRun_TestCodeunitFromTest_Guarded_StillThrows:
// only a codeunit with Subtype = Test counts as a test codeunit.
#[test]
fn only_a_test_codeunit_is_refused_when_run_from_a_test() {
    let extra = "codeunit 50930 NormalCu { Subtype = Normal; trigger OnRun() begin end; }
                 codeunit 50931 TestCu { Subtype = Test; trigger OnRun() begin end; }";
    let body = "procedure F(): Text
        begin
            Codeunit.Run(Codeunit::NormalCu);
            asserterror Codeunit.Run(Codeunit::TestCu);
            exit(GetLastErrorText());
        end;";
    let r = run_in(extra, body, |s| s.in_test_codeunit = true);
    assert!(
        text(r).starts_with("You cannot nest the execution of test codeunits. Test codeunit 50931 TestCu was called"),
    );
}

// Only the OnRun *trigger* runs: a procedure that happens to be called OnRun is not it, and
// neither is another trigger. The codeunit without a trigger is a no-op that succeeds
// (types/TestLanguageEdgeCases.al:CodeunitRun_Success_ReturnsTrue).
#[test]
fn only_the_on_run_trigger_is_run() {
    let extra = "codeunit 50932 NamedOnRun { procedure OnRun() begin Error('procedure ran'); end; }
                 codeunit 50933 OtherTrigger { Subtype = Install; trigger OnInstallAppPerCompany() begin Error('other trigger ran'); end; }";
    let r = run_in(
        extra,
        "procedure F(): Text
        var Ok: Boolean;
        begin
            Ok := Codeunit.Run(Codeunit::NamedOnRun);
            exit(Format(Ok) + Format(Codeunit.Run(Codeunit::OtherTrigger)));
        end;",
        |_| {},
    );
    assert_eq!(text(r), "YesYes");
}

// handlers/TestPageActionRunObjectKinds_Tests.al:ProbeControlEachTargetRecordsItsOwnExecution
// (:143-150): the record handed to Codeunit.Run is the Rec OnRun reads.
#[test]
fn codeunit_run_hands_the_record_to_a_tableno_codeunit_as_rec() {
    assert_eq!(
        f("var R: Record \"TX T\"; L: Record \"TX T\"; begin
            R.K := 'B'; R.V := 'Bravo'; R.Insert();
            Codeunit.Run(Codeunit::RecReader, R);
            L.Get('LOG');
            exit(L.V);
        end;"),
        "Bravo"
    );
    // The instance form passes it the same way.
    assert_eq!(
        f(
            "var R: Record \"TX T\"; L: Record \"TX T\"; Reader: Codeunit RecReader; begin
            R.K := 'B'; R.V := 'Bravo'; R.Insert();
            Reader.Run(R);
            L.Get('LOG');
            exit(L.V);
        end;"
        ),
        "Bravo"
    );
}

// codeunit/TestCodeunitRunErrorTextsAndModels.al:198-220: the record is passed by reference, so a change OnRun makes to Rec reaches the caller.
#[test]
fn a_tableno_codeunit_that_changes_rec_changes_the_callers_record() {
    assert_eq!(
        f("var R: Record \"TX T\"; begin
        R.K := 'B'; R.V := 'Bravo'; R.Insert();
        Codeunit.Run(Codeunit::RecWriter, R);
        exit(R.V);
    end;"),
        "changed"
    );
}

// ---- al-runner issue 5767: LockTable and a locking ReadIsolation start the write transaction ----

fn refused(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(m)) => m,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// codeunit/TestCodeunitRunWriteTransaction.al:211-235 RN_LockTable_WriteTransactionState and :237-252 (temporary: No).
#[test]
fn locktable_starts_the_write_transaction_until_commit() {
    assert_eq!(
        f("var R: Record \"TX T\"; T: Record \"TX T\" temporary; S: Text; begin
            R.K := 'A'; R.Insert(); Commit();
            S := Format(Database.IsInWriteTransaction());
            T.LockTable(); S += Format(Database.IsInWriteTransaction());
            R.LockTable(); S += Format(Database.IsInWriteTransaction());
            R.FindFirst(); S += Format(Database.IsInWriteTransaction());
            Commit(); S += Format(Database.IsInWriteTransaction());
            R.FindFirst(); S += Format(Database.IsInWriteTransaction());
            exit(S);
        end;"),
        "NoNoYesYesNoNo"
    );
}

// codeunit/TestCodeunitRunWriteTransaction.al:254-278 RN_ReadIsolation_WriteTransactionState.
#[test]
fn updlock_set_and_a_repeatable_read_start_the_write_transaction() {
    assert_eq!(
        f("var U: Record \"TX T\"; P: Record \"TX T\"; C: Record \"TX T\"; S: Text; begin
            U.K := 'A'; U.Insert(); Commit();
            U.ReadIsolation := IsolationLevel::UpdLock; S := Format(Database.IsInWriteTransaction());
            U.FindFirst(); S += Format(Database.IsInWriteTransaction());
            Commit();
            P.ReadIsolation := IsolationLevel::RepeatableRead; P.FindFirst(); S += Format(Database.IsInWriteTransaction());
            Commit();
            C.ReadIsolation := IsolationLevel::ReadCommitted; C.FindFirst(); S += Format(Database.IsInWriteTransaction());
            exit(S);
        end;"),
        "YesYesYesNo"
    );
}

// codeunit/TestCodeunitRunWriteTransaction.al:280-293 RN_GuardedRun_AfterLockTable: DB:Ok, the codeunit does not run.
#[test]
fn a_guarded_run_after_locktable_fails_with_db_ok() {
    assert_eq!(
        f("var R: Record \"TX T\"; Ok: Boolean; begin
            R.LockTable();
            asserterror Ok := Codeunit.Run(Codeunit::Inserter);
            R.Reset(); R.SetRange(K, 'M');
            exit(GetLastErrorCode() + '|' + GetLastErrorText() + '|' + Format(R.Count()));
        end;"),
        "DB:Ok|An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|0"
    );
}

// Not pinned: a RepeatableRead set without a read. What IsInWriteTransaction answers is refused. A RecordRef.LockTable on a
// database table opens the write transaction (recordref/lt-wc/WcRecordRefLockTable.al:28-51).
#[test]
fn an_unpinned_lock_shape_refuses_is_in_write_transaction() {
    let m = refused(run("procedure F(): Text var R: Record \"TX T\"; begin
            R.ReadIsolation := IsolationLevel::RepeatableRead;
            exit(Format(Database.IsInWriteTransaction()));
        end;"));
    assert!(m.contains("not pinned"), "{m}");
    assert_eq!(
        f("var RR: RecordRef; begin
            RR.Open(50900); RR.LockTable();
            exit(Format(Database.IsInWriteTransaction()));
        end;"),
        "Yes"
    );
    // A Commit ends the unpinned state.
    assert_eq!(
        f("var R: Record \"TX T\"; begin
            R.ReadIsolation := IsolationLevel::RepeatableRead; Commit();
            exit(Format(Database.IsInWriteTransaction()));
        end;"),
        "No"
    );
}

// Assumption `readuncommitted-read-opens-no-write-transaction` (Tests-ERM Manual Payment Registration tests).
#[test]
fn a_readuncommitted_read_opens_no_write_transaction() {
    assert_eq!(
        f("var R: Record \"TX T\"; begin
            R.K := 'A'; R.Insert(); Commit();
            R.ReadIsolation := IsolationLevel::ReadUncommitted; R.FindFirst();
            exit(Format(Database.IsInWriteTransaction()));
        end;"),
        "No"
    );
}

// [CommitBehavior] on an event declaration governs the Commit of its subscribers (misc/lt-cu-odd-leads/CuCommitTests.al
// SP_CommitBehaviorIgnore_OnIntegrationEvent_SubscriberCommitIgnored, SP_CommitBehaviorError_OnIntegrationEvent_SubscriberCommitFails).
const EVENT_CB: &str = r#"
codeunit 50150 Pub {
    procedure Raise(Error: Boolean) begin if Error then OnError('E') else OnIgnore('I'); end;
    [IntegrationEvent(false, false)] [CommitBehavior(CommitBehavior::Ignore)] local procedure OnIgnore(K: Code[10]) begin end;
    [IntegrationEvent(false, false)] [CommitBehavior(CommitBehavior::Error)] local procedure OnError(K: Code[10]) begin end;
}
codeunit 50151 Sub {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnIgnore', '', false, false)]
    local procedure Ign(K: Code[10]) var R: Record "TX T"; begin R.K := K; R.Insert(); Commit(); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnError', '', false, false)]
    local procedure Err(K: Code[10]) var R: Record "TX T"; begin R.K := K; R.Insert(); Commit(); end;
}
"#;

#[test]
fn commit_behavior_on_an_event_governs_its_subscribers() {
    let ignore = run_in(
        EVENT_CB,
        "procedure F(): Text var P: Codeunit Pub; R: Record \"TX T\"; begin asserterror begin P.Raise(false); Error('unrelated'); end; exit(Format(R.Get('I'))); end;",
        |_| {},
    );
    assert_eq!(text(ignore), "No");
    let error = run_in(
        EVENT_CB,
        "procedure F(): Text var P: Codeunit Pub; R: Record \"TX T\"; begin asserterror P.Raise(true); exit(Format(R.Get('E')) + GetLastErrorText()); end;",
        |_| {},
    );
    assert_eq!(
        text(error),
        "NoCommit is prohibited in the current scope. The operation cannot continue. Contact your system administrator."
    );
}
