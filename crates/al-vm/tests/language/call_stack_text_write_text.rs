//! `OutStream.WriteText` and `OutStream.Write` of the call stack text of a collected error, and how it reads back
//! (corpus tests/al-language/errors/WdCallStackWriteText.al, BC 27, 28, 29; al-runner issue 6853). The runtime's call stack text
//! is the tracked frames, one per line, and the mark (`CALL_STACK_MARK`) for the untracked frames below them, whose text, length
//! and line breaks are not known. What the corpus pins and the runtime can answer is answered; the rest is refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{boolean, text};

const SRC: &str = r#"
table 50101 "Log"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Call Stack"; Blob) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

codeunit 50100 C
{
    var
        Stack: Text;

    procedure Collected(): Text
    begin
        exit("Hop-Collected"());
    end;

    // The hyphen in the name is a frame format the corpus has not measured: the call stack of the collected error ends
    // with the untracked mark below this frame.
    local procedure "Hop-Collected"(): Text
    begin
        exit(CollectedMid());
    end;

    local procedure CollectedMid(): Text
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

    local procedure Open(var Log: Record "Log" temporary; var OutStr: OutStream; Mode: Integer)
    begin
        Log."Entry No." += 1;
        case Mode of
            0: Log."Call Stack".CreateOutStream(OutStr);
            1: Log."Call Stack".CreateOutStream(OutStr, TextEncoding::UTF8);
            2: Log."Call Stack".CreateOutStream(OutStr, TextEncoding::UTF16);
            3: Log."Call Stack".CreateOutStream(OutStr, TextEncoding::Windows);
            4: Log."Call Stack".CreateOutStream(OutStr, TextEncoding::MSDos);
        end;
    end;

    local procedure OpenIn(var Log: Record "Log" temporary; var InStr: InStream; Mode: Integer)
    begin
        case Mode of
            0: Log."Call Stack".CreateInStream(InStr);
            1: Log."Call Stack".CreateInStream(InStr, TextEncoding::UTF8);
            2: Log."Call Stack".CreateInStream(InStr, TextEncoding::UTF16);
            3: Log."Call Stack".CreateInStream(InStr, TextEncoding::Windows);
            4: Log."Call Stack".CreateInStream(InStr, TextEncoding::MSDos);
        end;
    end;

    // Method 1 WriteText, 2 Write.
    local procedure Written(var Log: Record "Log" temporary; Mode: Integer; Method: Integer)
    var
        OutStr: OutStream;
    begin
        Stack := Collected();
        Open(Log, OutStr, Mode);
        case Method of
            1: OutStr.WriteText(Stack);
            2: OutStr.Write(Stack);
        end;
        Log.Insert();
        Log.CalcFields("Call Stack");
    end;

    procedure WriteThenRead(Mode: Integer; Method: Integer; var Orig: Text): Text
    var
        Log: Record "Log" temporary;
        InStr: InStream;
        ReadBack: Text;
    begin
        Written(Log, Mode, Method);
        OpenIn(Log, InStr, Mode);
        InStr.Read(ReadBack);
        Orig := Stack;
        exit(ReadBack);
    end;

    procedure FirstLine(Mode: Integer): Text
    var
        Log: Record "Log" temporary;
        InStr: InStream;
        Line: Text;
    begin
        Written(Log, Mode, 1);
        OpenIn(Log, InStr, Mode);
        InStr.ReadText(Line);
        exit(Line);
    end;

    procedure EosAfterFirstLine(Mode: Integer): Boolean
    var
        Log: Record "Log" temporary;
        InStr: InStream;
        Line: Text;
    begin
        Written(Log, Mode, 1);
        OpenIn(Log, InStr, Mode);
        InStr.ReadText(Line);
        exit(InStr.EOS());
    end;

    procedure ReadAllLines(Mode: Integer): Integer
    var
        Log: Record "Log" temporary;
        InStr: InStream;
        Line: Text;
        Lines: Integer;
    begin
        Written(Log, Mode, 1);
        OpenIn(Log, InStr, Mode);
        while not InStr.EOS() do begin
            InStr.ReadText(Line);
            Lines += 1;
        end;
        exit(Lines);
    end;

    procedure ReadPastTheMark(Mode: Integer): Text
    var
        Log: Record "Log" temporary;
        InStr: InStream;
        Line: Text;
    begin
        Written(Log, Mode, 1);
        OpenIn(Log, InStr, Mode);
        InStr.ReadText(Line);
        InStr.ReadText(Line);
        InStr.ReadText(Line);
        InStr.ReadText(Line);
        InStr.ReadText(Line);
        exit(Line);
    end;

    procedure BlobLength(Mode: Integer): Integer
    var
        Log: Record "Log" temporary;
    begin
        Written(Log, Mode, 1);
        exit(Log."Call Stack".Length());
    end;

    procedure BlobHasLength(Mode: Integer): Boolean
    var
        Log: Record "Log" temporary;
    begin
        Written(Log, Mode, 1);
        exit(Log."Call Stack".Length() > 0);
    end;

    procedure InStreamLength(Mode: Integer): Integer
    var
        Log: Record "Log" temporary;
        InStr: InStream;
    begin
        Written(Log, Mode, 1);
        OpenIn(Log, InStr, Mode);
        exit(InStr.Length());
    end;

    procedure WriteTextCount(Mode: Integer): Integer
    var
        Log: Record "Log" temporary;
        OutStr: OutStream;
    begin
        Open(Log, OutStr, Mode);
        exit(OutStr.WriteText(Collected()));
    end;

    procedure WriteCount(Mode: Integer): Integer
    var
        Log: Record "Log" temporary;
        OutStr: OutStream;
    begin
        Open(Log, OutStr, Mode);
        exit(OutStr.Write(Collected()));
    end;

    procedure ReadTextCount(Mode: Integer): Integer
    var
        Log: Record "Log" temporary;
        InStr: InStream;
        Line: Text;
    begin
        Written(Log, Mode, 1);
        OpenIn(Log, InStr, Mode);
        InStr.ReadText(Line);
        InStr.ReadText(Line);
        InStr.ReadText(Line);
        exit(InStr.ReadText(Line));
    end;

    procedure ReadCount(Mode: Integer): Integer
    var
        Log: Record "Log" temporary;
        InStr: InStream;
        ReadBack: Text;
    begin
        Written(Log, Mode, 2);
        OpenIn(Log, InStr, Mode);
        exit(InStr.Read(ReadBack));
    end;

    procedure JsonShape(var JsonText: Text): Text
    var
        Log: Record "Log" temporary;
        Entry: JsonObject;
        Arr: JsonArray;
        ReadBack: Text;
        OutStr: OutStream;
        InStr: InStream;
    begin
        Entry.Add('Callstack', Collected());
        Arr.Add(Entry);
        Arr.WriteTo(JsonText);
        Log."Call Stack".CreateOutStream(OutStr);
        OutStr.WriteText(JsonText);
        Log.Insert();
        Log.CalcFields("Call Stack");
        Log."Call Stack".CreateInStream(InStr);
        InStr.ReadText(ReadBack);
        exit(ReadBack);
    end;
}
"#;

const MODES: [i32; 5] = [0, 1, 2, 3, 4];

fn call(routine: &str, args: Vec<Value>) -> Result<Value, Signal> {
    al_testkit::run_with(SRC, "C", routine, args).map(|(v, _)| v)
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Text(t) => t.as_str().to_string(),
        other => panic!("expected a Text, got {other:?}"),
    }
}

/// Runs `routine`, whose last argument is a `var Text`, and returns its result, that argument and the ids of the
/// assumptions the run marked. Two call stack texts are compared here, not in AL: AL refuses to compare them.
fn clean(routine: &str, mut args: Vec<Value>) -> (String, String, Vec<&'static str>) {
    args.push(Value::Text(String::new().into()));
    al_builtins::assumptions::clear_hits();
    let (got, out) = al_testkit::run_with(SRC, "C", routine, args).expect("runs");
    let hits = al_builtins::assumptions::take_hits()
        .iter()
        .map(|a| a.info().id)
        .collect();
    (text_of(&got), text_of(out.last().expect("the var argument")), hits)
}

fn refused(r: Result<Value, Signal>, what: &str) {
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains(what), "{m}"),
        other => panic!("expected a refusal naming {what:?}, got {other:?}"),
    }
}

/// WriteText and Write of the call stack text are never refused, on every encoding, and Read gives the whole text back
/// (WdCallStackWriteText.al:126-133, :148-149 `read equal` Yes for WriteText and Write on all five encodings). Nothing is
/// assumed on the way.
#[test]
fn write_text_and_write_of_a_call_stack_read_back_with_read_on_every_encoding() {
    for mode in MODES {
        for method in [1, 2] {
            let (got, orig, hits) = clean("WriteThenRead", vec![Value::Integer(mode), Value::Integer(method)]);
            assert!(orig.contains(al_builtins::native::CALL_STACK_MARK), "{orig:?}");
            assert_eq!(got, orig, "mode {mode} method {method}");
            assert!(hits.is_empty(), "mode {mode} method {method}: {hits:?}");
        }
    }
}

/// ReadText returns the first line of the call stack text, and the stream is not at its end after it
/// (WdCallStackWriteText.al:138-141 `readtext is first line` Yes, `eos after readtext` No; :541 the reporter's second shape).
#[test]
fn read_text_returns_the_first_line_and_the_stream_goes_on() {
    for mode in MODES {
        let line = text(call("FirstLine", vec![Value::Integer(mode)]));
        assert!(
            line.starts_with("\"C\"(CodeUnit 50100).CollectedRaw line "),
            "mode {mode}: {line:?}"
        );
        assert!(
            !line.contains('\n') && !line.contains(al_builtins::native::CALL_STACK_MARK),
            "mode {mode}: {line:?}"
        );
        assert!(
            !boolean(call("EosAfterFirstLine", vec![Value::Integer(mode)])),
            "mode {mode}"
        );
    }
}

/// The reporter's first shape: the JSON text of the call stack has no line break of its own, so ReadText reads it back equal
/// (WdCallStackWriteText.al:518-530, `shape1 readtext equal` Yes).
#[test]
fn a_json_text_of_a_call_stack_reads_back_equal_with_read_text() {
    let (got, json, hits) = clean("JsonShape", vec![]);
    assert!(
        json.starts_with("[{\"Callstack\":") && json.contains(al_builtins::native::CALL_STACK_MARK),
        "{json:?}"
    );
    assert_eq!(got, json);
    assert!(hits.is_empty(), "{hits:?}");
}

/// How many lines the untracked frames hold is not known (WdCallStackWriteText.al:156-157: 3 lines on Windows, 9 on Linux), so
/// EOS before them and any read after ReadText went past them are refused.
#[test]
fn the_lines_of_the_untracked_frames_are_not_counted() {
    for mode in MODES {
        refused(call("ReadAllLines", vec![Value::Integer(mode)]), "untracked frames");
        refused(call("ReadPastTheMark", vec![Value::Integer(mode)]), "untracked frames");
    }
}

/// The stream holds StrLen bytes of the call stack text (2 x StrLen on UTF-16, WdCallStackWriteText.al:136), and StrLen of
/// the untracked frames is not known: the length is refused, its sign is not.
#[test]
fn the_length_of_a_stream_holding_a_call_stack_is_refused() {
    for mode in MODES {
        refused(call("BlobLength", vec![Value::Integer(mode)]), "call stack");
        refused(call("InStreamLength", vec![Value::Integer(mode)]), "call stack");
        assert!(
            boolean(call("BlobHasLength", vec![Value::Integer(mode)])),
            "mode {mode}"
        );
    }
}

/// The byte counts WriteText, Write, ReadText and Read return for a text with the untracked frames are not known.
#[test]
fn the_byte_counts_of_a_call_stack_text_are_refused() {
    for mode in MODES {
        refused(call("WriteTextCount", vec![Value::Integer(mode)]), "call stack");
        refused(call("WriteCount", vec![Value::Integer(mode)]), "call stack");
        refused(call("ReadTextCount", vec![Value::Integer(mode)]), "call stack");
    }
    refused(call("ReadCount", vec![Value::Integer(3)]), "call stack");
}
