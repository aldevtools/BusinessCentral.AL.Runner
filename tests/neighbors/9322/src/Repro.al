// The reporter's repro of al-runner issue 9322, unchanged.
table 91620 "PZC Holder"
{
    TableType = Temporary;
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Data; Blob) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 91621 "PZC Base64 Line Breaks"
{
    Subtype = Test;

    [Test]
    procedure InStreamToBase64KeepsLineBreaks()
    var
        Holder: Record "PZC Holder";
        Converter: Codeunit "Base64 Convert";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        // BC 28.4 and 5a58a0f16: crlfAt=77 length=138; e5176a9fd: refused
        Error('MEASURED instream: crlfAt=%1 length=%2', StrPos(Result, CRLF()), StrLen(Result));
    end;

    [Test]
    procedure TextToBase64KeepsLineBreaks()
    var
        Converter: Codeunit "Base64 Convert";
        Result: Text;
    begin
        Result := Converter.ToBase64(PadStr('', 100, 'A'), true, TextEncoding::UTF8, 0);
        // BC 28.4 and 5a58a0f16: crlfAt=77 length=138; e5176a9fd: refused as a cascade of the test above
        Error('MEASURED text: crlfAt=%1 length=%2', StrPos(Result, CRLF()), StrLen(Result));
    end;

    local procedure CRLF(): Text[2]
    var
        T: Text[2];
    begin
        T[1] := 13;
        T[2] := 10;
        exit(T);
    end;
}
