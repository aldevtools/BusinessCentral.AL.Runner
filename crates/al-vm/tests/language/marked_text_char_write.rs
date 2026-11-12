//! `T[i] := c` on a text that holds a marker (`al_builtins::native::TEXT_MARKS`). The call stack text of a collected error is
//! the tracked frames, then the mark for the untracked frames below them (`call_stack_text_write_text.rs`). A position inside
//! the known characters before the first mark is a known position and keeps its answer; from the mark on the position is
//! not known, and the range error would name a length that is not known, so both refuse.

use al_builtins::native::{CALL_STACK_MARK, Signal};
use al_builtins::value::Value;

const SRC: &str = r#"
codeunit 50100 C
{
    local procedure Collected(): Text
    begin
        exit("Hop-Collected"());
    end;

    // The hyphen in the name is a frame format the corpus has not measured: the call stack ends with the untracked mark.
    local procedure "Hop-Collected"(): Text
    begin
        exit(CollectedRaw());
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectedRaw(): Text
    var
        E: ErrorInfo;
        S: Text;
    begin
        E := ErrorInfo.Create('boom');
        E.Collectible := true;
        Error(E);
        foreach E in GetCollectedErrors(true) do
            S := E.Callstack();
        ClearCollectedErrors();
        exit(S);
    end;

    procedure Stack(): Text begin exit(Collected()); end;
    procedure WriteAt(I: Integer): Text var S: Text; begin S := Collected(); S[I] := 'x'; exit(S); end;
}
"#;

fn call(routine: &str, args: Vec<Value>) -> Result<Value, Signal> {
    al_testkit::run_with(SRC, "C", routine, args).map(|(v, _)| v)
}

fn text_of(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t)) => t.audited_str().to_string(),
        other => panic!("expected a text, got {other:?}"),
    }
}

#[test]
fn a_character_written_before_the_mark_of_a_call_stack_text_keeps_its_answer() {
    let stack = text_of(call("Stack", vec![]));
    let known = stack
        .find(CALL_STACK_MARK)
        .expect("the call stack text ends with the mark");
    assert!(known > 1 && stack.is_char_boundary(1), "{stack:?}");
    let written = text_of(call("WriteAt", vec![Value::Integer(1)]));
    assert_eq!(written, format!("x{}", &stack[1..]));
    let last_known = i32::try_from(stack[..known].encode_utf16().count()).unwrap();
    let written = text_of(call("WriteAt", vec![Value::Integer(last_known)]));
    assert_eq!(&written[known..], &stack[known..], "the mark stays");
}

#[test]
fn a_character_written_at_or_after_the_mark_of_a_call_stack_text_refuses() {
    let stack = text_of(call("Stack", vec![]));
    let known = stack
        .find(CALL_STACK_MARK)
        .expect("the call stack text ends with the mark");
    let on_mark = i32::try_from(stack[..known].encode_utf16().count()).unwrap() + 1;
    // On the mark, after it, far beyond it, and the positions whose range error would name the unknown length.
    for i in [on_mark, on_mark + 1, 100_000, 0, -1] {
        match call("WriteAt", vec![Value::Integer(i)]) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("assigning a character"), "{i}: {why}"),
            other => panic!("{i}: expected a refusal, got {other:?}"),
        }
    }
}
