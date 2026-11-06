//! Lane T13: Codeunit.Run of a TableNo codeunit with a record of another table, nested `[CommitBehavior]` scopes,
//! `Consistent(false)` then `Commit`, writes inside a `[TryFunction]` whose result is used, and the arguments of such a call.
//!
//! Each test cites the corpus test that pins the behavior.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const FIXTURES: &str = r#"
table 51300 "CR Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[50]) { } }
    keys { key(PK; Id) { } }
}

table 51301 "CR Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

codeunit 51305 "CR TableNo Run"
{
    TableNo = "CR Row";
    trigger OnRun() begin Rec.Name := 'set by OnRun'; end;
}
"#;

fn run(body: &str, routine: &str) -> Result<al_builtins::value::Value, Signal> {
    run_in(&format!("{FIXTURES}\ncodeunit 51310 C {{ {body} }}"), "C", routine)
}

/// session-system/isolated-storage/CuWriteTxTests.al:203-233 (CodeunitRun_TableNo_RecordOfAnotherTable and _Guarded),
/// codeunits/TestCodeunitRunErrorTextsAndModels.al:273-300: the error is raised before the run and the Boolean result
/// does not trap it.
#[test]
fn codeunit_run_of_a_tableno_codeunit_with_a_record_of_another_table_fails() {
    let body = r#"
        procedure F(): Text
        var Other: Record "CR Other"; Ok: Boolean; Log: Text;
        begin
            Other.Id := 1;
            asserterror Codeunit.Run(Codeunit::"CR TableNo Run", Other);
            Log := GetLastErrorText() + '|' + GetLastErrorCode() + ';';
            Ok := true;
            asserterror Ok := Codeunit.Run(Codeunit::"CR TableNo Run", Other);
            Log += GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Ok);
            exit(Log);
        end;"#;
    assert_eq!(
        text(run(body, "F")),
        "Record(51301) is not compatible with Codeunit.Run(Record(51300)).|InvalidType;\
         Record(51301) is not compatible with Codeunit.Run(Record(51300)).|InvalidType|Yes"
    );
}

/// errors/CtErrorLeads.al:76-93 and :550-570 (Item27_TryFunction_ArgumentRaises): an argument of a
/// TryFunction call whose result is used is evaluated inside the try; in the statement form the error goes on.
#[test]
fn an_argument_error_of_a_used_try_call_is_trapped_by_the_try() {
    let body = r#"
        [TryFunction]
        local procedure TryFn(Input: Text) begin end;
        local procedure RaiseError(): Text begin Error('argument failed'); end;
        procedure F(): Text
        var Ok: Boolean; Log: Text;
        begin
            if TryFn(RaiseError()) then Log := 'true' else Log := 'false: ' + GetLastErrorText();
            Log += ' continued;';
            ClearLastError();
            Ok := TryFn(RaiseError());
            Log += 'assigned ' + Format(Ok) + ': ' + GetLastErrorText() + ';';
            asserterror TryFn(RaiseError());
            Log += 'bare: ' + GetLastErrorText();
            exit(Log);
        end;"#;
    assert_eq!(
        text(run(body, "F")),
        "false: argument failed continued;assigned No: argument failed;bare: argument failed"
    );
}

/// errors/CuTryTests.al:83-150: an Insert inside a TryFunction whose result is used fails with
/// TryFunctionWrite, which the try does not trap; the statement form writes and fails with the try's own error.
/// The scope named is the caller of the procedure that uses the result: the test method for a helper
/// (:105-121 TryInsertThenError_IfNotForm), the runner's root scope when the test method uses it itself (:236-238).
#[test]
fn an_insert_inside_a_used_try_function_fails_with_try_function_write() {
    let body = r#"
        Subtype = Test;
        [TryFunction]
        local procedure TryInsertThenError(Id: Integer)
        var Row: Record "CR Row";
        begin Row.Id := Id; Row.Insert(); Error('try error after insert'); end;
        local procedure IfNotTry(Id: Integer; var Branch: Text)
        begin
            if not TryInsertThenError(Id) then Branch := 'not ok' else Branch := 'ok';
        end;
        [Test]
        procedure Helper(): Text
        var Row: Record "CR Row"; Branch: Text; Log: Text;
        begin
            asserterror IfNotTry(2, Branch);
            Log := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Branch + '|' + Format(Row.Count()) + ';';
            asserterror TryInsertThenError(3);
            Log += GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Row.Count());
            exit(Log);
        end;
        [Test]
        procedure Direct(): Text
        var Row: Record "CR Row"; Ok: Boolean;
        begin
            asserterror Ok := TryInsertThenError(1);
            exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Ok) + '|' + Format(Row.Count()));
        end;"#;
    assert_eq!(
        text(run(body, "Helper")),
        "Call to the function 'INSERT' is not allowed inside the call to 'Helper' when it is used as a TryFunction.\
         |TryFunctionWrite||0;try error after insert|Dialog|0"
    );
    assert_eq!(
        text(run(body, "Direct")),
        "Call to the function 'INSERT' is not allowed inside the call to 'RootMethodScope' when it is used as a TryFunction.\
         |TryFunctionWrite|No|0"
    );
    // A helper of a procedure that is not a test method: the scope name is not pinned, and reading the text refuses.
    let plain = body.replace("Subtype = Test;", "").replace("[Test]", "");
    match run(&plain, "Helper") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("not pinned"), "{m}"),
        other => panic!("{other:?}"),
    }
}

/// errors/CuTryTests.al:56-70 and :153-171 (TryTemporaryWrites_ExpressionForm): writes to a temporary
/// record are allowed. ModifyAll of a database table raises TryFunctionWrite (errors/lt-r7143-try-write/
/// R7143TryWriteDeny.al:145-160, al-runner issue 7143).
#[test]
fn temporary_writes_are_allowed_and_other_writes_are_refused_in_a_used_try() {
    let body = r#"
        [TryFunction]
        local procedure TryTemp()
        var Tmp: Record "CR Row" temporary;
        begin Tmp.Id := 1; Tmp.Insert(); Tmp.Name := 'x'; Tmp.Modify(); Tmp.DeleteAll(); end;
        [TryFunction]
        local procedure TryModify()
        var Row: Record "CR Row";
        begin Row.ModifyAll(Name, 'x'); end;
        procedure Temp(): Text begin exit(Format(TryTemp())); end;
        procedure Modify(): Boolean begin exit(TryModify()); end;"#;
    assert_eq!(text(run(body, "Temp")), "Yes");
    match run(body, "Modify") {
        Err(Signal::Error(e)) => assert_eq!(e.code, "TryFunctionWrite", "{e:?}"),
        other => panic!("{other:?}"),
    }
}

/// errors/CuCommitTests.al:301-381 (Commit_InTryFunction): Commit inside a TryFunction whose result is
/// used fails with TryFunctionWrite and the try does not trap it; :203-264 (TryFunction_WithCommitBehavior): a
/// `[CommitBehavior]` on the try function decides first.
#[test]
fn a_commit_inside_a_used_try_function_fails_with_try_function_write() {
    let body = r#"
        [TryFunction]
        local procedure TryCommit() begin Commit(); end;
        [TryFunction]
        [CommitBehavior(CommitBehavior::Ignore)]
        local procedure TryCommitUnderIgnore() begin Commit(); end;
        procedure F(): Text
        var Ok: Boolean;
        begin
            asserterror Ok := TryCommit();
            exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Ok) + '|' + Format(TryCommitUnderIgnore()));
        end;"#;
    assert_eq!(
        text(run(body, "F")),
        "Call to the function 'COMMIT' is not allowed inside the call to 'RootMethodScope' when it is used as a TryFunction.\
         |TryFunctionWrite|No|Yes"
    );
}

/// errors/CuCommitTests.al:163-245 (Nested_ErrorOuter_IgnoreInner, Nested_IgnoreOuter_ErrorInner): an
/// inner `[CommitBehavior(Ignore)]` does not lift an outer `Error`.
#[test]
fn an_inner_commit_behavior_ignore_does_not_override_an_outer_error() {
    let body = r#"
        [CommitBehavior(CommitBehavior::Error)]
        local procedure OuterError() begin InnerIgnore(); end;
        [CommitBehavior(CommitBehavior::Ignore)]
        local procedure InnerIgnore() begin Commit(); end;
        [CommitBehavior(CommitBehavior::Ignore)]
        local procedure OuterIgnore() begin InnerError(); end;
        [CommitBehavior(CommitBehavior::Error)]
        local procedure InnerError() begin Commit(); end;
        procedure F(): Text
        var Log: Text;
        begin
            asserterror OuterError();
            Log := GetLastErrorText() + '|' + GetLastErrorCode() + ';';
            asserterror OuterIgnore();
            exit(Log + GetLastErrorCode());
        end;"#;
    assert_eq!(
        text(run(body, "F")),
        "Commit is prohibited in the current scope. The operation cannot continue. Contact your system administrator.|DB:Ok;DB:Ok"
    );
}

/// errors/CuCommitTests.al:382-433 (Consistent_False_ThenCommit, Consistent_FalseThenTrue_ThenCommit).
#[test]
fn a_commit_after_consistent_false_fails_and_consistent_true_clears_it() {
    let body = r#"
        local procedure Inconsistent(Id: Integer)
        var Row: Record "CR Row";
        begin Row.Consistent(false); Row.Id := Id; Row.Insert(); Commit(); end;
        local procedure Repaired(Id: Integer)
        var Row: Record "CR Row";
        begin Row.Consistent(false); Row.Id := Id; Row.Insert(); Row.Consistent(true); Commit(); end;
        procedure F(): Text
        var Row: Record "CR Row"; Log: Text;
        begin
            asserterror Inconsistent(21);
            Log := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Row.Count()) + ';';
            Repaired(22);
            asserterror Error('unrelated');
            exit(Log + Format(Row.Count()));
        end;"#;
    assert_eq!(
        text(run(body, "F")),
        "The transaction cannot be completed because it will cause inconsistencies in the CR Row table. Check where and \
         how the CONSISTENT function is used in the transaction to find the reason for the error.\\\\Contact your system \
         administrator.\\\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents \
         data from being updated incorrectly.|DB:TableInConsistent|0;1"
    );
}

/// Whether the TryFunctionWrite check comes before insert event subscribers is not pinned: such an Insert is refused.
#[test]
fn an_insert_in_a_used_try_on_a_table_with_insert_subscribers_is_refused() {
    let body = r#"
        [TryFunction]
        local procedure TryInsert()
        var Row: Record "CR Row";
        begin Row.Id := 1; Row.Insert(); end;
        procedure F(): Boolean begin exit(TryInsert()); end;
        [EventSubscriber(ObjectType::Table, Database::"CR Row", 'OnBeforeInsertEvent', '', false, false)]
        local procedure OnBeforeInsertRow(var Rec: Record "CR Row"; RunTrigger: Boolean) begin end;"#;
    match run(body, "F") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("subscribers"), "{m}"),
        other => panic!("{other:?}"),
    }
}
