// Neighbors of issue 9286: an Insert of a database record whose Blob holds stored content that was never loaded (a Get
// without CalcFields, or a copy of such a record) under a new key. Each test checks BC's answer, so a changed answer and a
// refusal both show as a difference. One test per codeunit: a refused test makes the later tests of its codeunit not run.
// Invented names only.
table 50120 "NB BI Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Body; Blob) { }
        field(3; Info; Text[30]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 50121 "NB BI Lib"
{
    procedure Seed(No: Code[20]; Txt: Text)
    var
        R: Record "NB BI Row";
        OutS: OutStream;
    begin
        R.Init();
        R."No." := No;
        if Txt <> '' then begin
            R.Body.CreateOutStream(OutS);
            OutS.WriteText(Txt);
        end;
        R.Insert();
    end;

    procedure W(var R: Record "NB BI Row"; Txt: Text)
    var
        OutS: OutStream;
    begin
        R.Body.CreateOutStream(OutS);
        OutS.WriteText(Txt);
    end;

    procedure Stored(No: Code[20]): Text
    var
        R: Record "NB BI Row";
        InS: InStream;
        Txt: Text;
    begin
        if not R.Get(No) then
            exit('<no row>');
        R.CalcFields(Body);
        if not R.Body.HasValue() then
            exit('nohv:');
        R.Body.CreateInStream(InS);
        InS.ReadText(Txt);
        exit('hv:' + Txt);
    end;

    // Raises when the answer is not the one BC gives (corpus pull request 1631, BlobInsertUnloaded.al, or a pin).
    procedure Expect(Seen: Text; Want: Text)
    begin
        if Seen <> Want then
            Error('seen %1, BC %2', Seen, Want);
    end;

    procedure InsertByValue(R: Record "NB BI Row"; No: Code[20])
    begin
        R."No." := No;
        R.Insert();
    end;
}

codeunit 50130 "NB BI Case 01"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // The shape of issue 9286: Get, :=, a new key, Insert.
    [Test]
    procedure Assign_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('A1', 'hello');
        Src.Get('A1');
        C := Src;
        C."No." := 'A1C';
        C.Insert();
        L.Expect(L.Stored('A1C'), 'nohv:');
    end;
}

codeunit 50131 "NB BI Case 02"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // The control: CalcFields on the source before :=.
    [Test]
    procedure Assign_Calc_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('A2', 'hello');
        Src.Get('A2');
        Src.CalcFields(Body);
        C := Src;
        C."No." := 'A2C';
        C.Insert();
        L.Expect(L.Stored('A2C'), 'hv:hello');
    end;
}

codeunit 50132 "NB BI Case 03"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // CalcFields on the copy, then the new key and Insert.
    [Test]
    procedure Assign_NoCalc_CalcOnCopy_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('A3', 'hello');
        Src.Get('A3');
        C := Src;
        C.CalcFields(Body);
        C."No." := 'A3C';
        C.Insert();
        L.Expect(L.Stored('A3C'), 'hv:hello');
    end;
}

codeunit 50133 "NB BI Case 04"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // The stored Blob is empty: nothing unloaded to lose.
    [Test]
    procedure Assign_NoCalc_StoredEmpty_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('A4', '');
        Src.Get('A4');
        C := Src;
        C."No." := 'A4C';
        C.Insert();
        L.Expect(L.Stored('A4C'), 'nohv:');
    end;
}

codeunit 50134 "NB BI Case 05"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A write on the copy before the Insert.
    [Test]
    procedure Assign_NoCalc_Write_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('A5', 'hello');
        Src.Get('A5');
        C := Src;
        L.W(C, 'zz');
        C."No." := 'A5C';
        C.Insert();
        Error('seen %1', L.Stored('A5C'));
    end;
}

codeunit 50135 "NB BI Case 06"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A Modify of the copy on its own key: the stored Blob stays.
    [Test]
    procedure Assign_NoCalc_Modify()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('A6', 'hello');
        Src.Get('A6');
        C := Src;
        C.Info := 'x';
        C.Modify();
        L.Expect(L.Stored('A6'), 'hv:hello');
    end;
}

codeunit 50136 "NB BI Case 07"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // The same variable: Get, a new key, Insert.
    [Test]
    procedure SameVariable_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
    begin
        L.Seed('B1', 'hello');
        Src.Get('B1');
        Src."No." := 'B1C';
        Src.Insert();
        L.Expect(L.Stored('B1C'), 'nohv:');
    end;
}

codeunit 50137 "NB BI Case 08"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // The same variable: Get, Delete, Insert under the same key.
    [Test]
    procedure SameVariable_NoCalc_DeleteInsert()
    var
        Src: Record "NB BI Row";
    begin
        L.Seed('B2', 'hello');
        Src.Get('B2');
        Src.Delete();
        Src.Insert();
        L.Expect(L.Stored('B2'), 'nohv:');
    end;
}

codeunit 50138 "NB BI Case 09"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // Record.Copy, a new key, Insert.
    [Test]
    procedure Copy_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('C1', 'hello');
        Src.Get('C1');
        C.Copy(Src);
        C."No." := 'C1C';
        C.Insert();
        L.Expect(L.Stored('C1C'), 'nohv:');
    end;
}

codeunit 50139 "NB BI Case 10"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A record passed by value, inserted under a new key by the callee.
    [Test]
    procedure ByValue_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
    begin
        L.Seed('C2', 'hello');
        Src.Get('C2');
        L.InsertByValue(Src, 'C2C');
        L.Expect(L.Stored('C2C'), 'nohv:');
    end;
}

codeunit 50140 "NB BI Case 11"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // TransferFields from a source read without CalcFields (record/crud/RcBTransfer.al:486-492).
    [Test]
    procedure TransferFields_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('C3', 'hello');
        Src.Get('C3');
        C.Init();
        C.TransferFields(Src);
        C."No." := 'C3C';
        C.Insert();
        L.Expect(L.Stored('C3C'), 'nohv:');
    end;
}

codeunit 50141 "NB BI Case 12"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A temporary record: Get of a temporary row without CalcFields, a new key, Insert.
    [Test]
    procedure Temporary_NoCalc_Insert()
    var
        T: Record "NB BI Row" temporary;
        InS: InStream;
        Txt: Text;
    begin
        T."No." := 'D1';
        L.W(T, 'hello');
        T.Insert();
        T.Get('D1');
        T."No." := 'D1C';
        T.Insert();
        T.Get('D1C');
        T.CalcFields(Body);
        T.Body.CreateInStream(InS);
        InS.ReadText(Txt);
        L.Expect(StrSubstNo('%1 [%2]', T.Body.HasValue(), Txt), 'No []');
    end;
}

codeunit 50142 "NB BI Case 13"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A Modify of the copy under the key of another stored row: that row's Blob stays as it was.
    [Test]
    procedure Assign_NoCalc_ModifyOtherRow()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('E1', 'hello');
        L.Seed('E2', 'other');
        Src.Get('E1');
        C := Src;
        C."No." := 'E2';
        C.Modify();
        L.Expect(L.Stored('E2'), 'hv:other');
    end;
}
