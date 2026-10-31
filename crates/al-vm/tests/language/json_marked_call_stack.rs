//! A collected error's `Callstack` ends with the untracked mark (`errorcollect_internal.rs`). Microsoft's Error Message
//! Management writes it into a JSON text (`GetJsonText`) and reads it back (`ParseErrorText`: `if NullJSONTxt <> JSON then
//! JObject.ReadFrom(JSON)`; Tests-General Journal, ERM General Journal UT, GeneralJournalNumberOfJournalLines and 65 more).
//! The text before the mark is known and the mark sits inside a JSON string, so the comparison and the parse are exact;
//! the string read back keeps the mark.

const SRC: &str = r#"
codeunit 70551 "LT Marked Json"
{
    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure Collect(var Info: ErrorInfo)
    var
        Errors: List of [ErrorInfo];
    begin
        Error(ErrorInfo.Create('Collected', true));
        Errors := GetCollectedErrors(true);
        Errors.Get(1, Info);
    end;

    local procedure JsonOf(var Info: ErrorInfo): Text
    var
        JObject: JsonObject;
        Json: Text;
    begin
        JObject.Add('Description', 'Desc');
        JObject.Add('CallStack', Info.Callstack);
        JObject.WriteTo(Json);
        exit(Json);
    end;

    procedure NotNull(): Boolean
    var
        Info: ErrorInfo;
    begin
        Collect(Info);
        exit('null' <> JsonOf(Info));
    end;

    procedure ReadBack(): Text
    var
        Info: ErrorInfo;
        JObject: JsonObject;
        Token: JsonToken;
    begin
        Collect(Info);
        JObject.ReadFrom(JsonOf(Info));
        JObject.Get('Description', Token);
        exit(Token.AsValue().AsText());
    end;

    procedure StackReadBackNotEmpty(): Boolean
    var
        Info: ErrorInfo;
        JObject: JsonObject;
        Token: JsonToken;
    begin
        Collect(Info);
        JObject.ReadFrom(JsonOf(Info));
        JObject.Get('CallStack', Token);
        exit(Token.AsValue().AsText() <> '');
    end;

    procedure MarkAsKey(): Boolean
    var
        Info: ErrorInfo;
        JObject: JsonObject;
    begin
        Collect(Info);
        exit(JObject.ReadFrom('{"' + Info.Callstack + '":1}'));
    end;

    procedure MarkOutsideString(): Boolean
    var
        Info: ErrorInfo;
        JObject: JsonObject;
    begin
        Collect(Info);
        exit(JObject.ReadFrom('{"a":' + Info.Callstack + '}'));
    end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_in(SRC, "LT Marked Json", routine)
}

#[test]
fn a_json_text_with_a_marked_call_stack_is_not_the_null_text() {
    assert!(al_testkit::boolean(run("NotNull")));
}

#[test]
fn a_json_text_with_a_marked_call_stack_reads_back_with_the_mark_kept() {
    assert_eq!(al_testkit::text(run("ReadBack")), "Desc");
    assert!(al_testkit::boolean(run("StackReadBackNotEmpty")));
}

#[test]
fn a_mark_outside_a_json_string_value_is_refused() {
    for r in ["MarkAsKey", "MarkOutsideString"] {
        let out = run(r);
        assert!(
            matches!(out, Err(al_builtins::native::Signal::Unsupported(_))),
            "{r}: {out:?}"
        );
    }
}
