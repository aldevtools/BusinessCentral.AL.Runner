// Stub declarations (ADR-051 decision 10): fields, enum values and objects a test names that do not exist yet.
table 50220 "Tdd Decl Item"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(50220; "Taken"; Integer) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    // A bare field name in the table's own code is a stub site too.
    procedure SetSize(NewSize: Integer)
    begin
        Size := NewSize;
    end;

    procedure GetSize(): Integer
    begin
        exit(Size);
    end;
}

tableextension 50222 "Tdd Decl Item Ext" extends "Tdd Decl Item"
{
    procedure SetDepth(NewDepth: Decimal)
    begin
        Depth := NewDepth;
    end;
}

// TransferFields into a stub Text field: the source's field has the id the stub field gets (the first free one).
table 50223 "Tdd Tf Source"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(50220; Memo; Text[4000]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50224 "Tdd Tf Target"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

enum 50221 "Tdd Decl Color"
{
    Extensible = true;
    value(0; Red) { }
    value(5; Green) { }
}

codeunit 50230 "Tdd Decl Tests"
{
    Subtype = Test;

    [Test]
    procedure FieldWrittenStoredAndReadBack()
    var
        Item: Record "Tdd Decl Item";
    begin
        Item."No." := 'A';
        Item.Weight := 7;
        Item.Insert();
        Clear(Item);
        Item.Get('A');
        if Item.Weight <> 7 then
            Error('Weight is %1', Item.Weight);
    end;

    [Test]
    procedure TextFieldTypeFromTheAssignment()
    var
        Item: Record "Tdd Decl Item";
    begin
        Item.Note := 'hello';
        if Item.Note <> 'hello' then
            Error('Note is %1', Item.Note);
    end;

    [Test]
    procedure TextFieldLongerThanTheStubLength()
    var
        Item: Record "Tdd Decl Item";
    begin
        Item.Note := PadStr('', 2049, 'x');
    end;

    [Test]
    procedure ValidateLongerThanTheStubLength()
    var
        Item: Record "Tdd Decl Item";
    begin
        Item.Note := 'short';
        Item.Validate(Note, PadStr('', 2049, 'y'));
    end;

    [Test]
    procedure FieldOnlyRead()
    var
        Item: Record "Tdd Decl Item";
    begin
        if Item.Flag then
            Error('the stub field is not false');
    end;

    [Test]
    procedure EnumValueOnAWorkspaceEnum()
    var
        Color: Enum "Tdd Decl Color";
    begin
        Color := Enum::"Tdd Decl Color"::Blue;
        if Color.AsInteger() <> 6 then
            Error('Blue is %1', Color.AsInteger());
    end;

    [Test]
    procedure NewCodeunit()
    var
        Calc: Codeunit "Tdd New Calc";
    begin
        if Calc.Compute() <> 0 then
            Error('Compute is not 0');
    end;

    [Test]
    procedure NewTable()
    var
        Entry: Record "Tdd New Entry";
    begin
        Entry.Amount := 3;
        Entry.Insert();
        if Entry.Count() <> 1 then
            Error('Count is %1', Entry.Count());
    end;

    [Test]
    procedure NewEnum()
    var
        Kind: Enum "Tdd New Kind";
    begin
        Kind := Enum::"Tdd New Kind"::First;
        if Kind.AsInteger() <> 0 then
            Error('First is %1', Kind.AsInteger());
    end;

    [Test]
    procedure Untouched()
    var
        Item: Record "Tdd Decl Item";
    begin
        Item."No." := 'B';
        Item.Insert();
    end;

    [Test]
    procedure BareFieldInTheTablesOwnCode()
    var
        Item: Record "Tdd Decl Item";
    begin
        Item.SetSize(4);
        if Item.GetSize() <> 4 then
            Error('Size is %1', Item.GetSize());
    end;

    [Test]
    procedure BareFieldInATableExtension()
    var
        Item: Record "Tdd Decl Item";
    begin
        Item.SetDepth(2.5);
    end;

    [Test]
    procedure FieldRefWriteLongerThanTheStubLength()
    var
        Item: Record "Tdd Decl Item";
        RecRef: RecordRef;
        FRef: FieldRef;
    begin
        Item.Note := 'short';
        RecRef.GetTable(Item);
        FRef := RecRef.Field(Item.FieldNo(Note));
        FRef.Value := PadStr('', 2049, 'z');
    end;

    [Test]
    procedure FieldRefValidateLongerThanTheStubLength()
    var
        Item: Record "Tdd Decl Item";
        RecRef: RecordRef;
        FRef: FieldRef;
    begin
        Item.Note := 'short';
        RecRef.GetTable(Item);
        FRef := RecRef.Field(Item.FieldNo(Note));
        FRef.Validate(PadStr('', 2049, 'v'));
    end;

    [Test]
    procedure TransferFieldsLongerThanTheStubLength()
    var
        Source: Record "Tdd Tf Source";
        Target: Record "Tdd Tf Target";
    begin
        Target.Memo := 'short';
        Source."No." := 'S';
        Source.Memo := PadStr('', 2049, 't');
        Target.TransferFields(Source);
    end;
}

// Globals of stub objects (a new table, a new codeunit): every test that touches one is stubbed.
codeunit 50231 "Tdd Decl Global Tests"
{
    Subtype = Test;

    var
        GlobalEntry: Record "Tdd New Global Entry";
        GlobalCalc: Codeunit "Tdd New Global Calc";

    [Test]
    procedure GlobalOfAStubTable()
    begin
        GlobalEntry.DeleteAll();
        if not GlobalEntry.IsEmpty() then
            Error('the stub table is not empty');
    end;

    [Test]
    procedure GlobalOfAStubCodeunit()
    begin
        Clear(GlobalCalc);
    end;
}
