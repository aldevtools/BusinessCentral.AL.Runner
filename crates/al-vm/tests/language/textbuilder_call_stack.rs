//! A call stack text (it carries the untracked-frame mark) goes into a TextBuilder and comes back out of it, as it goes
//! through `+` and a Blob round trip (al-runner issue 6310; corpus error-handling/lt-wd/WdCallStackToBlob.al, codeunit
//! 90742: the call stack text round-trips and is not empty). Its content is not pinned, so no text function reads it.

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

    procedure Direct(): Boolean
    var
        Info: ErrorInfo;
        Tb: TextBuilder;
    begin
        Collect(Info);
        Tb.Append(Info.Callstack());
        Tb.AppendLine(Info.Callstack());
        Tb.Insert(1, Info.Callstack());
        exit((Tb.ToText() <> '') and (Tb.Length() > 0));
    end;

    procedure ThroughBlob(): Boolean
    var
        Info: ErrorInfo;
        Log: Record "Log" temporary;
        OutStr: OutStream;
        InStr: InStream;
        Line: Text;
        Tb: TextBuilder;
    begin
        Collect(Info);
        Log."Entry No." := 1;
        Log."Call Stack".CreateOutStream(OutStr, TextEncoding::Windows);
        OutStr.Write(Info.Callstack());
        Log.Insert();
        Log.CalcFields("Call Stack");
        Log."Call Stack".CreateInStream(InStr, TextEncoding::Windows);
        InStr.ReadText(Line);
        Tb.Append(Line);
        exit(Tb.ToText() <> '');
    end;
}
"#;

#[test]
fn a_call_stack_text_is_appended_to_a_text_builder() {
    assert!(boolean(run_in(SRC, "C", "Direct")));
}

#[test]
fn a_call_stack_text_read_back_from_a_blob_is_appended_to_a_text_builder() {
    assert!(boolean(run_in(SRC, "C", "ThroughBlob")));
}
