//! A call stack text (it carries the untracked-frame mark) is moved, never read, by the builtins that only store or write
//! their text argument (al-runner issue 6853, a follow-up of 6310 and PR 6393). The corpus pins that the call stack text of a
//! collected error round-trips through `OutStream.Write` into a Windows and a UTF-8 blob (error-handling/lt-wd/
//! WdCallStackToBlob.al:96-102, WD_CallStack_Collected_ToWindowsBlob) and is not empty (:92). `WriteText`, a Notification
//! and an ErrorInfo carry it the same way. A builtin that reads the text (StrLen, CopyStr, Evaluate, a Message template)
//! still refuses it.

use al_builtins::native::Signal;
use al_testkit::{boolean, run_in};

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
    // The hyphen in the name is a frame format the corpus has not measured: the call stack of the collected error ends
    // with the untracked mark below this frame.
    procedure "Hop-Collect"(var Info: ErrorInfo)
    begin
        Collect(Info);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    procedure Collect(var Info: ErrorInfo)
    var
        E: ErrorInfo;
    begin
        E := ErrorInfo.Create('boom');
        E.Collectible := true;
        Error(E);
        foreach E in GetCollectedErrors(true) do
            Info := E;
    end;

    procedure WriteTextTo(Encoding: TextEncoding; UseEncoding: Boolean): Boolean
    var
        Info: ErrorInfo;
        Log: Record "Log" temporary;
        OutStr: OutStream;
        InStr: InStream;
        Line: Text;
    begin
        "Hop-Collect"(Info);
        Log."Entry No." := 1;
        if UseEncoding then
            Log."Call Stack".CreateOutStream(OutStr, Encoding)
        else
            Log."Call Stack".CreateOutStream(OutStr);
        OutStr.WriteText(Info.Callstack());
        Log.Insert();
        Log.CalcFields("Call Stack");
        if UseEncoding then
            Log."Call Stack".CreateInStream(InStr, Encoding)
        else
            Log."Call Stack".CreateInStream(InStr);
        InStr.ReadText(Line);
        exit(Line <> '');
    end;

    procedure WriteTextDefault(): Boolean
    begin
        exit(WriteTextTo(TextEncoding::MSDos, false));
    end;

    procedure WriteTextUtf8(): Boolean
    begin
        exit(WriteTextTo(TextEncoding::UTF8, true));
    end;

    procedure WriteTextWindows(): Boolean
    begin
        exit(WriteTextTo(TextEncoding::Windows, true));
    end;

    procedure WriteTextUtf16(): Boolean
    begin
        exit(WriteTextTo(TextEncoding::UTF16, true));
    end;

    procedure WriteTextMsDos(): Boolean
    begin
        exit(WriteTextTo(TextEncoding::MSDos, true));
    end;

    procedure JsonThenWriteText(): Boolean
    var
        Info: ErrorInfo;
        Log: Record "Log" temporary;
        Entry: JsonObject;
        Arr: JsonArray;
        JsonText: Text;
        OutStr: OutStream;
    begin
        "Hop-Collect"(Info);
        Entry.Add('Callstack', Info.Callstack());
        Arr.Add(Entry);
        Arr.WriteTo(JsonText);
        Log."Entry No." := 1;
        Log."Call Stack".CreateOutStream(OutStr);
        OutStr.WriteText(JsonText);
        Log.Insert();
        exit(JsonText <> '');
    end;

    procedure NotificationCarries(): Boolean
    var
        Info: ErrorInfo;
        N: Notification;
    begin
        "Hop-Collect"(Info);
        N.Message := Info.Callstack();
        N.SetData('k', Info.Callstack());
        exit((N.Message <> '') and (N.GetData('k') <> ''));
    end;

    procedure ErrorInfoCarries(): Boolean
    var
        Info: ErrorInfo;
        Other: ErrorInfo;
    begin
        "Hop-Collect"(Info);
        Other := ErrorInfo.Create(Info.Callstack());
        Other.DetailedMessage := Info.Callstack();
        exit((Other.Message <> '') and (Other.DetailedMessage <> ''));
    end;

    procedure LengthIsRefused(): Integer
    var
        Info: ErrorInfo;
    begin
        "Hop-Collect"(Info);
        exit(StrLen(Info.Callstack()));
    end;
}
"#;

#[test]
fn write_text_of_a_call_stack_to_a_stream_of_the_default_encoding() {
    assert!(boolean(run_in(SRC, "C", "WriteTextDefault")));
}

#[test]
fn write_text_of_a_call_stack_to_a_utf8_stream() {
    assert!(boolean(run_in(SRC, "C", "WriteTextUtf8")));
}

#[test]
fn write_text_of_a_call_stack_to_a_windows_stream() {
    assert!(boolean(run_in(SRC, "C", "WriteTextWindows")));
}

#[test]
fn write_text_of_a_call_stack_to_a_utf16_stream() {
    assert!(boolean(run_in(SRC, "C", "WriteTextUtf16")));
}

#[test]
fn write_text_of_a_call_stack_to_an_ms_dos_stream() {
    assert!(boolean(run_in(SRC, "C", "WriteTextMsDos")));
}

#[test]
fn a_json_text_holding_a_call_stack_is_written_with_write_text() {
    assert!(boolean(run_in(SRC, "C", "JsonThenWriteText")));
}

#[test]
fn a_notification_keeps_a_call_stack_as_its_message_and_data() {
    assert!(boolean(run_in(SRC, "C", "NotificationCarries")));
}

#[test]
fn an_error_info_keeps_a_call_stack_as_its_message() {
    assert!(boolean(run_in(SRC, "C", "ErrorInfoCarries")));
}

#[test]
fn a_function_that_reads_the_call_stack_text_still_refuses_it() {
    match run_in(SRC, "C", "LengthIsRefused") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("text argument"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
