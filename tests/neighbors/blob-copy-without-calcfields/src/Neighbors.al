// Neighbors of issue 9286: an Insert of a database record whose Blob holds stored content that was never loaded (a Get
// without CalcFields, or a copy of such a record) under a new key. Each test checks BC's answer, so a changed answer and a
// refusal both show as a difference. One test per codeunit: a refused test makes the later tests of its codeunit not run.
// Invented names only.
table 50123 "NB BI Other"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Body; Blob) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

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

    // Raises when the answer is not BC's (corpus pull request 1631, BlobInsertUnloaded.al, or a pin).
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

// The shapes of codeunit 73782 of BlobInsertUnloaded.al (corpus pull request 1631), named in the review of the pull request.
codeunit 50143 "NB BI Case 14"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A stale buffer: another variable changed the row's Blob, then the unloaded record is modified on its own key.
    [Test]
    procedure Stale_NoCalc_Modify()
    var
        Src: Record "NB BI Row";
        X: Record "NB BI Row";
    begin
        L.Seed('F1', 'hello');
        Src.Get('F1');
        X.Get('F1');
        L.W(X, 'zz');
        X.Modify();
        Src.Info := 'n';
        Src.Modify();
        L.Expect(L.Stored('F1'), 'hv:zz');
    end;
}

codeunit 50144 "NB BI Case 15"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // Insert of the copy (no Blob stored, C01), then Modify of the same variable.
    [Test]
    procedure Assign_NoCalc_Insert_Modify()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('F2', 'hello');
        Src.Get('F2');
        C := Src;
        C."No." := 'F2C';
        C.Insert();
        C.Info := 'n';
        C.Modify();
        L.Expect(L.Stored('F2C'), 'nohv:');
    end;
}

codeunit 50145 "NB BI Case 16"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A database row copied into a temporary record.
    [Test]
    procedure DatabaseIntoTemporary_Insert()
    var
        Src: Record "NB BI Row";
        T: Record "NB BI Row" temporary;
        InS: InStream;
        Txt: Text;
    begin
        L.Seed('F3', 'hello');
        Src.Get('F3');
        T := Src;
        T."No." := 'F3C';
        T.Insert();
        T.Get('F3C');
        T.CalcFields(Body);
        T.Body.CreateInStream(InS);
        InS.ReadText(Txt);
        L.Expect(StrSubstNo('%1 [%2]', T.Body.HasValue(), Txt), 'No []');
    end;
}

codeunit 50146 "NB BI Case 17"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A temporary row read with Get copied into a database record.
    [Test]
    procedure TemporaryIntoDatabase_Insert()
    var
        T: Record "NB BI Row" temporary;
        C: Record "NB BI Row";
    begin
        T."No." := 'F4';
        L.W(T, 'hello');
        T.Insert();
        T.Get('F4');
        C := T;
        C."No." := 'F4C';
        C.Insert();
        L.Expect(L.Stored('F4C'), 'nohv:');
    end;
}

codeunit 50147 "NB BI Case 18"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A Blob field assignment from a record read without CalcFields.
    [Test]
    procedure FieldAssign_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('F5', 'hello');
        Src.Get('F5');
        C."No." := 'F5C';
        C.Body := Src.Body;
        C.Insert();
        L.Expect(L.Stored('F5C'), 'nohv:');
    end;
}

codeunit 50148 "NB BI Case 19"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A source read after ChangeCompany (the current company), copied with :=.
    [Test]
    procedure ChangeCompany_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('F6', 'hello');
        Src.ChangeCompany(CompanyName());
        Src.Get('F6');
        C := Src;
        C."No." := 'F6C';
        C.Insert();
        L.Expect(L.Stored('F6C'), 'nohv:');
    end;
}

codeunit 50149 "NB BI Case 20"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // The inserting record's own CalcFields right after the Insert of the copy.
    [Test]
    procedure Assign_NoCalc_Insert_OwnCalcFields()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
        InS: InStream;
        Txt: Text;
    begin
        L.Seed('F7', 'hello');
        Src.Get('F7');
        C := Src;
        C."No." := 'F7C';
        C.Insert();
        C.CalcFields(Body);
        C.Body.CreateInStream(InS);
        InS.ReadText(Txt);
        L.Expect(StrSubstNo('%1 [%2]', C.Body.HasValue(), Txt), 'Yes []');
    end;
}

// R1 to R3 of codeunit 73782 (corpus pull request 1631).
codeunit 50150 "NB BI Case 21"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // Another variable cleared the row's Blob and modified it; then the first, unloaded buffer is modified on the same key.
    [Test]
    procedure ClearedRow_NoCalc_Modify()
    var
        Src: Record "NB BI Row";
        X: Record "NB BI Row";
    begin
        L.Seed('H1', 'hello');
        Src.Get('H1');
        X.Get('H1');
        Clear(X.Body);
        X.Modify();
        Src.Info := 'n';
        Src.Modify();
        L.Expect(L.Stored('H1'), 'nohv:');
    end;
}

codeunit 50151 "NB BI Case 22"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A Blob field assignment into another table, then Insert.
    [Test]
    procedure FieldAssignOtherTable_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
        B: Record "NB BI Other";
        R: Record "NB BI Other";
    begin
        L.Seed('H2', 'hello');
        Src.Get('H2');
        B."No." := 'H2C';
        B.Body := Src.Body;
        B.Insert();
        R.Get('H2C');
        R.CalcFields(Body);
        L.Expect(Format(R.Body.HasValue()), 'No');
    end;
}

codeunit 50152 "NB BI Case 23"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // A source read in another company, copied with := and inserted without switching back: the row lands in this company.
    [Test]
    procedure OtherCompany_NoSwitch_Insert()
    var
        O: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        O.ChangeCompany('NB Other Co');
        O.DeleteAll();
        O.Init();
        O."No." := 'H3';
        L.W(O, 'there');
        O.Insert();
        Clear(O);
        O.ChangeCompany('NB Other Co');
        O.Get('H3');
        C := O;
        C."No." := 'H3C';
        C.Insert();
        L.Expect(L.Stored('H3C'), 'nohv:');
    end;
}
