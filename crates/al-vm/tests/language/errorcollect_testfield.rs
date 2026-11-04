//! `Record.TestField(Field, ErrorInfo.Create())` collected under `ErrorBehavior::Collect`: the Message of the
//! collected ErrorInfo when the caller gave none.
//!
//! The own Message of a collectible ErrorInfo is pinned (types/PUTestFieldErrorInfo.al:143-166). An ErrorInfo with
//! no Message is not pinned: the runtime assumes the standard TestField text (assumption
//! `testfield-errorinfo-empty-message-collected`), which Microsoft's tests read (Tests-General_Journal,
//! codeunit 134932 "ERM Gen. Jnl. Error Handling", JournalLineErrorsFactBoxTestFieldNotEmpty and
//! LogTestFieldOptionWithErrorBehaviorCollectError).

use al_builtins::value::Value;

const SRC: &str = r#"
table 50100 TfRow
{
    fields { field(1; Id; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50100 C
{
    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure Collect(var R: Record TfRow; var Info: ErrorInfo): Text
    var Errors: List of [ErrorInfo]; Msg: Text;
    begin
        R.TestField(Name, Info);
        Errors := GetCollectedErrors();
        if Errors.Count() <> 1 then Error('COUNT ' + Format(Errors.Count()));
        Msg := Errors.Get(1).Message();
        ClearCollectedErrors();
        exit(Msg);
    end;

    procedure F(): Text
    var R: Record TfRow; Info: ErrorInfo;
    begin
        R.Id := 'A';
        Info := ErrorInfo.Create();
        exit(Collect(R, Info));
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure Rid(var R: Record TfRow; var Info: ErrorInfo): Text
    var Errors: List of [ErrorInfo]; Msg: Text;
    begin
        R.TestField(Name, Info);
        Errors := GetCollectedErrors();
        Msg := Format(Errors.Get(1).RecordId());
        ClearCollectedErrors();
        exit(Msg);
    end;

    procedure H(): Text
    var R: Record TfRow; Info: ErrorInfo;
    begin
        R.Id := 'A';
        Info := ErrorInfo.Create();
        exit(Rid(R, Info));
    end;

    procedure G(): Text
    var R: Record TfRow; Info: ErrorInfo;
    begin
        R.Id := 'A';
        Info := ErrorInfo.Create('given', true);
        exit(Collect(R, Info));
    end;
}
"#;

fn run(routine: &str) -> String {
    match al_testkit::run(SRC, routine) {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_error_info_without_a_message_is_collected_with_the_standard_testfield_text() {
    // Tests-General_Journal 134932 JournalLineErrorsFactBoxTestFieldNotEmpty: "Document No. must have a value" is
    // a substring of the collected Message. The text is the one the uncollected TestField raises
    // (types/PUTestFieldErrorInfo.al:92).
    assert_eq!(
        run("F"),
        "Name must have a value in TfRow: Id=A. It cannot be zero or empty."
    );
}

#[test]
fn an_error_info_with_a_message_keeps_it() {
    // types/PUTestFieldErrorInfo.al:143-166.
    assert_eq!(run("G"), "given");
}

#[test]
fn the_collected_error_names_the_record_it_was_raised_on() {
    // Assumption `testfield-errorinfo-collected-recordid`: Tests-General_Journal 134932
    // JournalLineErrorsFactBoxTestFieldNotEmpty reads the line's errors by their Context Record ID.
    assert_eq!(run("H"), "TfRow: A");
}
