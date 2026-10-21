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
}
