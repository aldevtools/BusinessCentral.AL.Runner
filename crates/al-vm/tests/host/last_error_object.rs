//! `GetLastErrorObject` (OnPrem): the last error as a .NET exception object, served by the runtime.
//! Every pinned literal is what the real service tier answered in the corpus:
//! tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObject.al (BC 27, 28, 29; corpus issue 572).
//! The stack text of `ToString` after its first line is not pinned (only that it has `   at ` frames).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
dotnet { assembly("mscorlib") {
    type("System.Exception"; LeoExc) { }
    type("System.Type"; LeoType) { }
} }
table 50160 "LEO Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50161 "LEO Thrower"
{
    var Mode: Integer;
    trigger OnRun()
    var Row: Record "LEO Row"; Zero: Integer; Result: Integer; Info: ErrorInfo;
    begin
        case Mode of
            1: Error('boom from Run');
            2: Row.Get(-987654);
            3: Result := 1 div Zero;
            4: Error('boom with %1 and %2', 'arg', 42);
            5: Error(Lbl);
            6: begin Info.Message := 'info message'; Error(Info); end;
            7: Row.TestField(Id, 5);
            8: begin Row.Id := 1; Row.Insert(); Row.Insert(); end;
            9: Helper();
        end;
    end;
    local procedure Helper() begin Error('boom from helper'); end;
    procedure SetMode(NewMode: Integer) begin Mode := NewMode; end;
    [TryFunction] procedure TryRaise() begin Error('boom from try'); end;
    procedure Raise() begin Error('boom from asserterror'); end;
    var Lbl: Label 'boom from label';
}
codeunit 50100 C
{
    local procedure Describe(): Text
    var Exc: DotNet LeoExc; T: DotNet LeoType; Last: Text; Out: Text;
    begin
        Last := GetLastErrorText();
        Exc := GetLastErrorObject();
        Out := Format(IsNull(Exc));
        if IsNull(Exc) then exit(Out);
        T := Exc.GetType();
        exit(Out + '|' + Exc.Message + '|' + Format(Exc.Message = Last) + '|' + T.FullName + '|' + Format(IsNull(Exc.InnerException)));
    end;
    local procedure FirstLine(Value: Text): Text
    var Lf: Text[1];
    begin
        Lf[1] := 10;
        exit(CopyStr(Value, 1, StrPos(Value + Lf, Lf) - 1));
    end;
    BODY
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = SRC.replace("BODY", body);
    al_testkit::run_in(&src, "C", "F")
}

fn after_run(mode: i32) -> Result<Value, Signal> {
    run(&format!(
        "procedure F(): Text var T: Codeunit \"LEO Thrower\"; begin T.SetMode({mode}); if T.Run() then exit('ran'); exit(Describe()); end;"
    ))
}

const DIALOG: &str = "Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLDialogException";

/// GetLastErrorObject.al:138-160 (Error() caught by Codeunit.Run).
#[test]
fn error_caught_by_codeunit_run_is_a_dialog_exception() {
    assert_eq!(text(after_run(1)), format!("No|boom from Run|Yes|{DIALOG}|Yes"));
}

/// GetLastErrorObject.al:162-185 (Error with substitutions).
#[test]
fn error_with_substitutions_is_a_dialog_exception() {
    assert_eq!(text(after_run(4)), format!("No|boom with arg and 42|Yes|{DIALOG}|Yes"));
}

/// GetLastErrorObject.al:187-209 (TryFunction).
#[test]
fn error_in_a_try_function_is_a_dialog_exception() {
    let r = text(run(
        "procedure F(): Text var T: Codeunit \"LEO Thrower\"; begin if T.TryRaise() then exit('ok'); exit(Describe()); end;",
    ));
    assert_eq!(r, format!("No|boom from try|Yes|{DIALOG}|Yes"));
}

/// GetLastErrorObject.al:211-230 (asserterror).
#[test]
fn error_in_asserterror_is_a_dialog_exception() {
    let r = text(run(
        "procedure F(): Text var T: Codeunit \"LEO Thrower\"; begin asserterror T.Raise(); exit(Describe()); end;",
    ));
    assert_eq!(r, format!("No|boom from asserterror|Yes|{DIALOG}|Yes"));
}

/// GetLastErrorObject.al:232-255 (Get without a row).
#[test]
fn get_without_a_row_is_a_record_not_found_exception() {
    assert_eq!(
        text(after_run(2)),
        "No|The LEO Row does not exist. Identification fields and values: Id='-987654'|Yes|Microsoft.Dynamics.Nav.Types.Exceptions.NavCSideRecordNotFoundException|Yes"
    );
}

/// GetLastErrorObject.al:257-280 (division by zero): the inner exception is not null.
#[test]
fn division_by_zero_is_an_al_exception_with_an_inner_exception() {
    assert_eq!(
        text(after_run(3)),
        "No|Attempted to divide by zero.|Yes|Microsoft.Dynamics.Nav.Types.Exceptions.NavALException|No"
    );
}

/// GetLastErrorObject.al:84-87: Format(Exc) = Exc.ToString(); the first line is `<type>: <message>`; the text has frames.
#[test]
fn to_string_starts_with_type_and_message_and_has_frames() {
    let r = text(run(
        "procedure F(): Text var T: Codeunit \"LEO Thrower\"; Exc: DotNet LeoExc; begin T.SetMode(1); if T.Run() then exit('ran');
            Exc := GetLastErrorObject();
            exit(Format(Format(Exc) = Exc.ToString()) + '|' + FirstLine(Exc.ToString()) + '|' + Format(StrPos(Exc.ToString(), '   at ') > 0)); end;",
    ));
    assert_eq!(r, format!("Yes|{DIALOG}: boom from Run|Yes"));
}

/// GetLastErrorObject.al:283-301: after ClearLastError the object is null; Format is 'null'; members raise.
#[test]
fn after_clear_last_error_the_object_is_null() {
    let r = text(run(
        "procedure F(): Text var T: Codeunit \"LEO Thrower\"; Exc: DotNet LeoExc; begin T.SetMode(1); if T.Run() then exit('ran');
            ClearLastError(); Exc := GetLastErrorObject();
            exit(Format(IsNull(Exc)) + '|' + Format(Exc)); end;",
    ));
    assert_eq!(r, "Yes|null");
}

#[test]
fn a_member_of_the_null_object_raises_the_not_instantiated_error() {
    let r = text(run(
        "procedure F(): Text var Exc: DotNet LeoExc; M: Text; begin ClearLastError(); Exc := GetLastErrorObject();
            asserterror M := Exc.Message; exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;",
    ));
    assert_eq!(
        r,
        "A DotNet variable has not been instantiated. Attempting to call System.Exception.Message in CodeUnit C: F|DotNetVariableNotCreated"
    );
}

/// GetLastErrorObject.al:303-330: a later successful Codeunit.Run keeps the last error.
#[test]
fn a_successful_run_after_an_error_keeps_the_object() {
    let r = text(run(
        "procedure F(): Text var T: Codeunit \"LEO Thrower\"; begin T.SetMode(1); if T.Run() then exit('ran'); T.SetMode(0); if not T.Run() then exit('failed'); exit(Describe()); end;",
    ));
    assert_eq!(r, format!("No|boom from Run|Yes|{DIALOG}|Yes"));
}

/// GetLastErrorObjectKinds.al pins Error(ErrorInfo) as a dialog exception and a failing TestField as NavTestFieldException.
#[test]
fn error_info_and_test_field_kinds_are_pinned() {
    assert_eq!(text(after_run(6)), format!("No|info message|Yes|{DIALOG}|Yes"));
    assert_eq!(
        text(after_run(7)),
        format!(
            "No|{}|Yes|Microsoft.Dynamics.Nav.Types.NavTestFieldException|Yes",
            "Id must be equal to '5'  in LEO Row: Id=0. Current value is '0'."
        )
    );
}

/// Not pinned: the exception type of an error no probe shows (a duplicate insert) is refused, naming the code.
#[test]
fn an_error_kind_the_corpus_does_not_pin_is_refused() {
    match after_run(8) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("GetLastErrorObject"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// GetLastErrorObjectKinds.al:Kind_ErrorWithLabel pins Error(Label) as the same dialog exception.
#[test]
fn error_with_a_label_is_a_dialog_exception() {
    assert_eq!(text(after_run(5)), format!("No|boom from label|Yes|{DIALOG}|Yes"));
}

/// Not pinned: GetLastErrorObject before any error happened in the session is refused. After ClearLastError it is
/// null (GetLastErrorObject.al:283-301).
#[test]
fn before_any_error_is_refused_but_after_clear_is_null() {
    let fresh = run(
        "procedure F(): Text var Exc: DotNet LeoExc; begin Exc := GetLastErrorObject(); exit(Format(IsNull(Exc))); end;",
    );
    match fresh {
        Err(Signal::Unsupported(why)) => assert!(why.contains("before any error"), "{why}"),
        other => panic!("{other:?}"),
    }
    let cleared = text(run(
        "procedure F(): Text var Exc: DotNet LeoExc; begin ClearLastError(); Exc := GetLastErrorObject(); exit(Format(IsNull(Exc))); end;",
    ));
    assert_eq!(cleared, "Yes");
}

/// An error raised in a procedure without parameters of the codeunit whose OnRun calls it: the frames
/// AzLastErrorObject.al:180 ToString_NestedProcedure pins (`Codeunit<id>.Nested()`, OnRunAsync, DoRunAsync; no ALError frame).
#[test]
fn to_string_of_an_error_in_a_helper_of_on_run_has_the_pinned_frames() {
    for body in ["exit(Exc.ToString());", "exit(Format(Exc));"] {
        let r = text(run(&format!(
            "procedure F(): Text var T: Codeunit \"LEO Thrower\"; Exc: DotNet LeoExc; begin T.SetMode(9); if T.Run() then exit('ran'); Exc := GetLastErrorObject(); {body} end;"
        )));
        assert_eq!(
            r,
            format!(
                "{DIALOG}: boom from helper\n   at Microsoft.Dynamics.Nav.BusinessApplication.Codeunit50161.Helper()\n   at Microsoft.Dynamics.Nav.BusinessApplication.Codeunit50161.OnRunAsync(INavRecordHandle \u{3b5}rec)\n   at Microsoft.Dynamics.Nav.Runtime.NavCodeunit.DoRunAsync(DataError errorLevel, NavRecord record)"
            ),
            "{body}"
        );
    }
}

/// A trace no corpus test pins (here an error caught by a TryFunction): BC writes .NET frames, which the runtime does not
/// produce; the AL frames it writes instead mark the assumption `exception-tostring-stack-frames`. The pinned shapes do not.
#[test]
fn to_string_of_a_trace_that_is_not_pinned_marks_the_assumption() {
    use al_builtins::assumptions::{Assumption, take_hits};
    for body in ["exit(Exc.ToString());", "exit(Format(Exc));"] {
        take_hits();
        let r = text(run(&format!(
            "procedure F(): Text var T: Codeunit \"LEO Thrower\"; Exc: DotNet LeoExc; begin if T.TryRaise() then exit('ran'); Exc := GetLastErrorObject(); {body} end;"
        )));
        assert!(r.starts_with(&format!("{DIALOG}: boom from try\n   at ")), "{r}");
        assert!(
            take_hits().contains(&Assumption::ExceptionToStringStackFrames),
            "{body}"
        );
        take_hits();
        let _ = text(run(&format!(
            "procedure F(): Text var T: Codeunit \"LEO Thrower\"; Exc: DotNet LeoExc; begin T.SetMode(9); if T.Run() then exit('ran'); Exc := GetLastErrorObject(); {body} end;"
        )));
        assert!(
            !take_hits().contains(&Assumption::ExceptionToStringStackFrames),
            "{body}"
        );
    }
}

/// An error an asserterror caught shows `NavMethodScope.AssertErrorAsync` in its frames (onprem-dotnet/lt-y76-dotnet/Y76DotNet.al
/// Y76_Asserterror_Error pins that name as a substring; Y89ToStringFrames.al the whole line with its namespace and argument).
#[test]
fn to_string_of_an_asserterror_error_names_the_assert_frame() {
    let r = text(run(
        "procedure F(): Text var T: Codeunit \"LEO Thrower\"; Exc: DotNet LeoExc; begin asserterror T.Raise(); Exc := GetLastErrorObject(); exit(Exc.ToString()); end;",
    ));
    assert!(
        r.ends_with("\n   at Microsoft.Dynamics.Nav.Runtime.NavMethodScope.AssertErrorAsync(Func`1 body)"),
        "{r}"
    );
}

/// GetLastErrorObject.al:Observer.Expect('toStringFirstLine', 'NavALException: Attempted to divide by zero.') ends the first line
/// there; Y76DotNet.al Y76_*_Division reads the text before the first frame, line breaks removed, as
/// `NavALException: Attempted to divide by zero. ---> System.DivideByZeroException: Attempted to divide by zero.` So the inner
/// exception starts on the second line.
#[test]
fn to_string_of_a_division_error_puts_the_inner_exception_on_the_second_line() {
    let r = text(run(
        "procedure F(): Text var T: Codeunit \"LEO Thrower\"; Exc: DotNet LeoExc; begin T.SetMode(3); if T.Run() then exit('ran'); Exc := GetLastErrorObject(); exit(Exc.ToString()); end;",
    ));
    let mut lines = r.split('\n');
    assert_eq!(
        lines.next(),
        Some("Microsoft.Dynamics.Nav.Types.Exceptions.NavALException: Attempted to divide by zero.")
    );
    assert_eq!(
        lines.next(),
        Some(" ---> System.DivideByZeroException: Attempted to divide by zero.")
    );
}
