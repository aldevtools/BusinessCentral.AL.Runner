// Neighbors of the issue 9322 shape: one fact of the reporter's repro flipped per codeunit (one test each, so a refusal
// does not cascade into the next test). A test passes when it gives the repro's result.
table 91630 "PZC Normal Holder"
{
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

codeunit 91650 "PZC Report"
{
    procedure Measured(What: Text; Result: Text)
    var
        T: Text[2];
    begin
        T[1] := 13;
        T[2] := 10;
        // BC 28.4 for the repro's holder, and Base64LineBreaks.al:120-134 for a Temp Blob: crlfAt=77 length=138.
        if (StrPos(Result, T) <> 77) or (StrLen(Result) <> 138) then
            Error('MEASURED %1: crlfAt=%2 length=%3', What, StrPos(Result, T), StrLen(Result));
    end;

    procedure Fill(var Holder: Record "PZC Holder"; var InS: InStream)
    var
        OutS: OutStream;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
    end;
}

codeunit 91631 "PZC N GlobalHolder"
{
    Subtype = Test;

    var
        Holder: Record "PZC Holder";

    [Test]
    procedure GlobalHolder()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91632 "PZC N DeclaredTemporary"
{
    Subtype = Test;

    [Test]
    procedure DeclaredTemporary()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Normal Holder" temporary;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91633 "PZC N NormalNotInserted"
{
    Subtype = Test;

    [Test]
    procedure NormalNotInserted()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Normal Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91634 "PZC N TempTableInserted"
{
    Subtype = Test;

    [Test]
    procedure TempTableInserted()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder."No." := 1;
        Holder.Insert();
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91635 "PZC N TwoWrites"
{
    Subtype = Test;

    [Test]
    procedure TwoWrites()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 50, 'A'));
        OutS.WriteText(PadStr('', 50, 'A'));
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91636 "PZC N ReadTextBefore"
{
    Subtype = Test;

    [Test]
    procedure ReadTextBefore()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
        T: Text;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        InS.ReadText(T, 10);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91637 "PZC N OutStreamForm"
{
    Subtype = Test;

    [Test]
    procedure OutStreamForm()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
        Target: Codeunit "Temp Blob";
        TargetOut: OutStream;
        TargetIn: InStream;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Target.CreateOutStream(TargetOut);
        Converter.ToBase64(InS, true, TargetOut);
        Target.CreateInStream(TargetIn);
        TargetIn.Read(Result);
        Rep.Measured('x', Result);
    end;
}

codeunit 91638 "PZC N Utf8OutStream"
{
    Subtype = Test;

    [Test]
    procedure Utf8OutStream()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
    begin
        Holder.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91639 "PZC N WrittenElsewhere"
{
    Subtype = Test;

    [Test]
    procedure WrittenInOtherCodeunit()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
    begin
        Rep.Fill(Holder, InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91640 "PZC N ClearedBeforeCall"
{
    Subtype = Test;

    [Test]
    procedure HolderClearedBeforeCall()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Clear(Holder);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91641 "PZC N CopiedBeforeInStream"
{
    Subtype = Test;

    [Test]
    procedure CopiedBeforeInStream()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
        Holder2: Record "PZC Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder2 := Holder;
        Holder2.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91642 "PZC N TempBlobControl"
{
    Subtype = Test;

    [Test]
    procedure TempBlobControl()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        TempBlob: Codeunit "Temp Blob";
    begin
        TempBlob.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        TempBlob.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91643 "PZC N Utf8InStream"
{
    Subtype = Test;

    [Test]
    procedure Utf8InStream()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS, TextEncoding::UTF8);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91644 "PZC N SecondInStream"
{
    Subtype = Test;

    [Test]
    procedure SecondInStream()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
        InS1: InStream;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS1);
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91645 "PZC N AssignedStream"
{
    Subtype = Test;

    [Test]
    procedure AssignedStream()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
        InS2: InStream;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS2);
        InS := InS2;
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91646 "PZC N BinaryWrite"
{
    Subtype = Test;

    [Test]
    procedure BinaryWrite()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
        OutS: OutStream;
        InS: InStream;
        Result: Text;
        Holder: Record "PZC Holder";
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.Write(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
        Result := Converter.ToBase64(InS, true);
        Rep.Measured('x', Result);
    end;
}

codeunit 91647 "PZC N FilledInLocalProc"
{
    Subtype = Test;

    [Test]
    procedure FilledInLocalProcedure()
    var
        Converter: Codeunit "Base64 Convert";
        Rep: Codeunit "PZC Report";
    begin
        Rep.Measured('x', Converter.ToBase64(LocalFill(), true));
    end;

    local procedure LocalFill() InS: InStream
    var
        Holder: Record "PZC Holder";
        OutS: OutStream;
    begin
        Holder.Data.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', 100, 'A'));
        Holder.Data.CreateInStream(InS);
    end;
}
