// Review round 1 of the issue 9322 change: the shapes the reviewer named. Each keeps main's refusal: the reporter's repro
// measured one buffer, opened and read in the [Test] method, holding 100 ASCII bytes, on a record not declared temporary.
// A test passes when it gives the repro's result (a CR LF at position 77, 138 characters).
table 91661 "PZR Holder"
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

codeunit 91662 "PZR Report"
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

    // Opens the field's OutStream and writes 100 'A' through it, in a routine that is no [Test] method.
    procedure Write(var Holder: Record "PZR Holder")
    var
        OutS: OutStream;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
    end;
}

codeunit 91663 "PZR N RefillAfterClear"
{
    Subtype = Test;

    [Test]
    procedure FieldRefilledInHelperAfterClear()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        Clear(Holder);
        Rep.Write(Holder);
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91664 "PZR N RefillAfterInit"
{
    Subtype = Test;

    [Test]
    procedure FieldRefilledInHelperAfterInit()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        Holder.Init();
        Rep.Write(Holder);
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91665 "PZR N RefillByAssign"
{
    Subtype = Test;

    [Test]
    procedure FieldRefilledAfterRecordAssign()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder";
        Other: Record "PZR Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        Rep.Write(Other);
        Holder := Other;
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91666 "PZR N Length57"
{
    Subtype = Test;

    [Test]
    procedure OtherLength57()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 57, 'A'));
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91667 "PZR N Length120"
{
    Subtype = Test;

    [Test]
    procedure OtherLength120()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 120, 'A'));
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91668 "PZR N NonAscii"
{
    Subtype = Test;

    [Test]
    procedure NonAsciiBytes()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 57, 'é'));
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91669 "PZR N DeclaredTemporary"
{
    Subtype = Test;

    // The repro's table (TableType = Temporary) with the record also declared temporary.
    [Test]
    procedure DeclaredTemporaryOnTemporaryTable()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder" temporary;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91670 "PZR N Repro"
{
    Subtype = Test;

    // Control: the reporter's repro still runs (BC 28, assumption base-convert-line-breaks-temporary-table-blob).
    [Test]
    procedure ReproStillServed()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91671 "PZR N ReopenAfterClear"
{
    Subtype = Test;

    // A second CreateOutStream of the method on the field, after a Clear: the repro opened the field once.
    [Test]
    procedure SecondCreateOutStreamAfterClear()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        Holder: Record "PZR Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        Clear(Holder);
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}

codeunit 91672 "PZR N TempBlobControl"
{
    Subtype = Test;

    // Control on every major: the Temp Blob stream, pinned by tests/al-language/streams/Base64LineBreaks.al:120-134.
    [Test]
    procedure TempBlobControl()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZR Report";
        OutS: OutStream;
        InS: InStream;
        TempBlob: Codeunit "Temp Blob";
    begin
        TempBlob.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        TempBlob.CreateInStream(InS);
        Rep.Measured('x', Converter.ToBase64(InS, true));
    end;
}
