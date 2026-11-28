// Review round 2 of the issue 9322 change (Copilot): the reporter's repro with an explicit TextEncoding::MSDos on one of
// its two stream opens. The assumption covers both opens without an encoding argument, so each keeps main's refusal.
// A test passes when it gives the repro's result (a CR LF at position 77, 138 characters).
table 91701 "PZS Holder"
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

codeunit 91702 "PZS Report"
{
    procedure Measured(What: Text; Result: Text)
    var
        T: Text[2];
    begin
        T[1] := 13;
        T[2] := 10;
        if (StrPos(Result, T) <> 77) or (StrLen(Result) <> 138) then
            Error('MEASURED %1: crlfAt=%2 length=%3', What, StrPos(Result, T), StrLen(Result));
    end;
}

codeunit 91703 "PZS N Encoded"
{
    Subtype = Test;

    // CreateOutStream with TextEncoding::MSDos given; CreateInStream as in the repro.
    [Test]
    procedure ExplicitMsDosOnCreateOutStream()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZS Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZS Holder";
    begin
        Holder.Data.CreateOutStream(OutS, TextEncoding::MSDos);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Rep.Measured('out', Converter.ToBase64(InS, true));
    end;

    // CreateOutStream as in the repro; CreateInStream with TextEncoding::MSDos given.
    [Test]
    procedure ExplicitMsDosOnCreateInStream()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZS Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZS Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS, TextEncoding::MSDos);
        Rep.Measured('in', Converter.ToBase64(InS, true));
    end;

    // Both opens with TextEncoding::MSDos given.
    [Test]
    procedure ExplicitMsDosOnBothOpens()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZS Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZS Holder";
    begin
        Holder.Data.CreateOutStream(OutS, TextEncoding::MSDos);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS, TextEncoding::MSDos);
        Rep.Measured('both', Converter.ToBase64(InS, true));
    end;
}

codeunit 91704 "PZS N Repro"
{
    Subtype = Test;

    // Control: the reporter's repro, both opens without an encoding, still runs (BC 28, assumption
    // base-convert-line-breaks-temporary-table-blob).
    [Test]
    procedure ReproStillServed()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZS Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZS Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}
