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

    procedure Show(var R: Record "NB BI Row"): Text
    var
        InS: InStream;
        Txt: Text;
    begin
        R.Body.CreateInStream(InS);
        if R.Body.HasValue() then
            InS.ReadText(Txt);
        exit(StrSubstNo('%1 [%2]', R.Body.HasValue(), Txt));
    end;

    // A temporary row: HasValue before CalcFields, then HasValue and the text after it.
    procedure StoredTemp(var T: Record "NB BI Row" temporary; No: Code[20]): Text
    var
        Before: Boolean;
    begin
        if not T.Get(No) then
            exit('<no row>');
        Before := T.Body.HasValue();
        T.CalcFields(Body);
        exit(StrSubstNo('%1 %2', Before, Show(T)));
    end;

    procedure CopyInsert(var C: Record "NB BI Row"; From: Code[20]; Into: Code[20])
    var
        Src: Record "NB BI Row";
    begin
        Seed(From, 'hello');
        Src.Get(From);
        C := Src;
        C."No." := Into;
        C.Insert();
    end;

    procedure ModifyByValue(C: Record "NB BI Row"; No: Code[20])
    begin
        C."No." := No;
        C.Modify();
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

// Codeunit 73783 of BlobInsertUnloadedOrigin.al (corpus pull request 1654), the shapes named in the second review.
codeunit 50153 "NB BI Case 24"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // F1: a guarded Insert onto an existing key, then Modify and CalcFields.
    [Test]
    procedure GuardedInsert_Exists_Modify_CalcFields()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('J1', 'hello');
        L.Seed('J1B', 'other');
        Src.Get('J1');
        C := Src;
        C."No." := 'J1B';
        if not C.Insert() then
            C.Modify();
        C.CalcFields(Body);
        L.Expect(L.Show(C), 'Yes [other]');
    end;
}

codeunit 50154 "NB BI Case 25"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // F2: a guarded Insert on the record's own key, then CalcFields.
    [Test]
    procedure GuardedInsert_OwnKey_CalcFields()
    var
        Src: Record "NB BI Row";
    begin
        L.Seed('J2', 'hello');
        Src.Get('J2');
        if not Src.Insert() then;
        Src.CalcFields(Body);
        L.Expect(L.Show(Src), 'Yes [hello]');
    end;
}

codeunit 50155 "NB BI Case 26"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // F3: asserterror around the statement form onto an existing key, the rows written again, then CalcFields.
    [Test]
    procedure AssertErrorInsert_CalcFields()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('J3', 'hello');
        L.Seed('J3B', 'other');
        Src.Get('J3');
        C := Src;
        C."No." := 'J3B';
        asserterror C.Insert();
        L.Expect(GetLastErrorCode(), 'DB:RecordExists');
        L.Seed('J3', 'hello');
        L.Seed('J3B', 'other');
        C.CalcFields(Body);
        L.Expect(L.Show(C), 'Yes [other]');
    end;
}

codeunit 50156 "NB BI Case 27"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // S1: the copy's Insert, then Modify, then CalcFields.
    [Test]
    procedure Insert_Modify_OwnCalcFields()
    var
        C: Record "NB BI Row";
    begin
        L.CopyInsert(C, 'J4', 'J4C');
        C.Info := 'n';
        C.Modify();
        C.CalcFields(Body);
        L.Expect(L.Show(C), 'Yes []');
    end;
}

codeunit 50157 "NB BI Case 28"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // S2: the copy's Insert, then another variable writes 'x' and modifies the row, then CalcFields.
    [Test]
    procedure Insert_OtherWrite_OwnCalcFields()
    var
        C: Record "NB BI Row";
        X: Record "NB BI Row";
    begin
        L.CopyInsert(C, 'J5', 'J5C');
        X.Get('J5C');
        L.W(X, 'x');
        X.Modify();
        C.CalcFields(Body);
        L.Expect(L.Show(C), 'Yes [x]');
    end;
}

codeunit 50158 "NB BI Case 29"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // S4: the copy's Insert, then Delete, then CalcFields.
    [Test]
    procedure Insert_Delete_OwnCalcFields()
    var
        C: Record "NB BI Row";
    begin
        L.CopyInsert(C, 'J6', 'J6C');
        C.Delete();
        C.CalcFields(Body);
        L.Expect(L.Show(C), 'No []');
    end;
}

codeunit 50159 "NB BI Case 30"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // S5: the copy's Insert, then Get, then CalcFields.
    [Test]
    procedure Insert_Get_OwnCalcFields()
    var
        C: Record "NB BI Row";
    begin
        L.CopyInsert(C, 'J7', 'J7C');
        C.Get('J7C');
        C.CalcFields(Body);
        L.Expect(L.Show(C), 'No []');
    end;
}

codeunit 50160 "NB BI Case 31"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // S6: the copy's Insert, then a second Insert under another key, then CalcFields and what the second row stores.
    [Test]
    procedure Insert_SecondInsert_OwnCalcFields()
    var
        C: Record "NB BI Row";
    begin
        L.CopyInsert(C, 'J8', 'J8C');
        C."No." := 'J8D';
        C.Insert();
        C.CalcFields(Body);
        L.Expect(L.Show(C) + ' ' + L.Stored('J8D'), 'Yes [] nohv:');
    end;
}

codeunit 50161 "NB BI Case 32"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // S3: the copy's Insert, then Commit, then CalcFields: the unloaded view survives the Commit.
    [Test]
    procedure Insert_Commit_OwnCalcFields()
    var
        C: Record "NB BI Row";
    begin
        L.CopyInsert(C, 'J9', 'J9C');
        Commit();
        C.CalcFields(Body);
        L.Expect(L.Show(C), 'Yes []');
    end;
}

codeunit 50162 "NB BI Case 33"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // O1: the source read with FindLast.
    [Test]
    procedure FindLast_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('K1', 'hello');
        Src.SetRange("No.", 'K1');
        Src.FindLast();
        C := Src;
        C."No." := 'K1C';
        C.Insert();
        L.Expect(L.Stored('K1C'), 'nohv:');
    end;
}

codeunit 50163 "NB BI Case 34"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // O5: the second row of a FindSet loop (Next).
    [Test]
    procedure Next_NoCalc_Insert()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('K2A', 'first');
        L.Seed('K2B', 'hello');
        Src.SetFilter("No.", 'K2A|K2B');
        Src.FindSet();
        Src.Next();
        C := Src;
        C."No." := 'K2C';
        C.Insert();
        L.Expect(L.Stored('K2C'), 'nohv:');
    end;
}

codeunit 50164 "NB BI Case 35"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // O6: a RecordRef opened with Open and read with FindFirst, a new key, RecordRef.Insert.
    [Test]
    procedure RecordRefOpen_FindFirst_Insert()
    var
        RR: RecordRef;
        FR: FieldRef;
    begin
        L.Seed('K3', 'hello');
        RR.Open(Database::"NB BI Row");
        FR := RR.Field(1);
        FR.SetRange('K3');
        RR.FindFirst();
        FR.SetRange();
        FR.Value := 'K3C';
        RR.Insert();
        L.Expect(L.Stored('K3C'), 'nohv:');
    end;
}

codeunit 50165 "NB BI Case 36"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // M4: Modify(true) onto another stored row.
    [Test]
    procedure ModifyTrue_OntoOther()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('K4', 'hello');
        L.Seed('K4B', 'other');
        Src.Get('K4');
        C := Src;
        C."No." := 'K4B';
        C.Modify(true);
        L.Expect(L.Stored('K4B'), 'hv:other');
    end;
}

codeunit 50166 "NB BI Case 37"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // M5: RecordRef.GetTable, the key of another stored row, RecordRef.Modify.
    [Test]
    procedure RecordRefModify_OntoOther()
    var
        Src: Record "NB BI Row";
        RR: RecordRef;
        FR: FieldRef;
    begin
        L.Seed('K5', 'hello');
        L.Seed('K5B', 'other');
        Src.Get('K5');
        RR.GetTable(Src);
        FR := RR.Field(1);
        FR.Value := 'K5B';
        RR.Modify();
        L.Expect(L.Stored('K5B'), 'hv:other');
    end;
}

codeunit 50167 "NB BI Case 38"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // M6: a by-value copy modified in the callee under the key of another stored row.
    [Test]
    procedure ByValueModify_OntoOther()
    var
        Src: Record "NB BI Row";
    begin
        L.Seed('K6', 'hello');
        L.Seed('K6B', 'other');
        Src.Get('K6');
        L.ModifyByValue(Src, 'K6B');
        L.Expect(L.Stored('K6B'), 'hv:other');
    end;
}

codeunit 50168 "NB BI Case 39"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // T1: a temporary row written with a stream, read with Get, renamed, then a new key and Insert.
    [Test]
    procedure TempRename_Insert()
    var
        T: Record "NB BI Row" temporary;
    begin
        T."No." := 'K7';
        L.W(T, 'mem');
        T.Insert();
        T.Get('K7');
        T.Rename('K7B');
        T."No." := 'K7C';
        T.Insert();
        L.Expect(L.StoredTemp(T, 'K7C'), 'Yes No []');
    end;
}

codeunit 50169 "NB BI Case 40"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // T2: a temporary row written with a stream, read with Get, renamed, then Modify.
    [Test]
    procedure TempRename_Modify()
    var
        T: Record "NB BI Row" temporary;
    begin
        T."No." := 'K8';
        L.W(T, 'mem');
        T.Insert();
        T.Get('K8');
        T.Rename('K8B');
        T.Modify();
        L.Expect(L.StoredTemp(T, 'K8B'), 'Yes No []');
    end;
}

codeunit 50170 "NB BI Case 41"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // F4: a guarded Insert onto an existing key, then CalcFields directly.
    [Test]
    procedure GuardedInsert_Exists_CalcFields()
    var
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        L.Seed('L1', 'hello');
        L.Seed('L1B', 'other');
        Src.Get('L1');
        C := Src;
        C."No." := 'L1B';
        if C.Insert() then
            Error('the Insert went through');
        C.CalcFields(Body);
        L.Expect(L.Show(C), 'Yes [other]');
    end;
}

codeunit 50171 "NB BI Case 42"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // O7: a record filled by RecordRef.SetTable from a FindFirst read, a new key, Insert.
    [Test]
    procedure SetTable_FindFirst_Insert()
    var
        C: Record "NB BI Row";
        RR: RecordRef;
        FR: FieldRef;
    begin
        L.Seed('L2', 'hello');
        RR.Open(Database::"NB BI Row");
        FR := RR.Field(1);
        FR.SetRange('L2');
        RR.FindFirst();
        RR.SetTable(C);
        C."No." := 'L2C';
        C.Insert();
        L.Expect(L.Show(C) + ' ' + L.Stored('L2C'), 'Yes [] nohv:');
    end;
}

codeunit 50172 "NB BI Case 43"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // M7: a copy switched to another company, modified onto that company's row of the same key. BC raises DB:RecordChanged;
    // not served yet, so both binaries give main's answer. Raises what it saw.
    [Test]
    procedure ChangeCompany_Modify()
    var
        O: Record "NB BI Row";
        Src: Record "NB BI Row";
        C: Record "NB BI Row";
    begin
        O.ChangeCompany('NB Other Co');
        O.DeleteAll();
        O.Init();
        O."No." := 'L3';
        L.W(O, 'there');
        O.Insert();
        L.Seed('L3', 'hello');
        Src.Get('L3');
        C := Src;
        C.ChangeCompany('NB Other Co');
        C.Info := 'n';
        Error('seen modify=%1', C.Modify());
    end;
}

codeunit 50173 "NB BI Case 44"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // O6b: after RecordRef.Insert of a row read with RecordRef FindFirst, the RecordRef's own Blob (through SetTable).
    [Test]
    procedure RecordRefInsert_OwnBlob()
    var
        C: Record "NB BI Row";
        RR: RecordRef;
        FR: FieldRef;
    begin
        L.Seed('L4', 'hello');
        RR.Open(Database::"NB BI Row");
        FR := RR.Field(1);
        FR.SetRange('L4');
        RR.FindFirst();
        FR.SetRange();
        FR.Value := 'L4C';
        RR.Insert();
        RR.SetTable(C);
        L.Expect(L.Show(C) + ' ' + L.Stored('L4C'), 'Yes [] nohv:');
    end;
}

codeunit 50174 "NB BI Case 45"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "NB BI Lib";

    // S2b: the copy's Insert, then another variable writes 'x' and modifies the row: what the row stores.
    [Test]
    procedure Insert_OtherWrite_Stored()
    var
        C: Record "NB BI Row";
        X: Record "NB BI Row";
    begin
        L.CopyInsert(C, 'L5', 'L5C');
        X.Get('L5C');
        L.W(X, 'x');
        X.Modify();
        L.Expect(L.Stored('L5C'), 'hv:x');
    end;
}
