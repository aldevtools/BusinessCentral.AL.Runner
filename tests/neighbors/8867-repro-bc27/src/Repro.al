table 50100 "Repro Row"
{
    fields
    {
        field(1; "Key"; Integer) { }
        field(2; Cod; Code[20]) { }
        field(3; Txt; Text[50]) { }
    }
    keys { key(PK; "Key") { Clustered = true; } }
}

// Generic RecordRef code filters a Code or a Text field with a number it holds as an Integer, a BigInteger or a Decimal.
codeunit 50101 "SetRange Number Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SetRangeWithIntegerOnCodeField()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        RecRef.Open(Database::"Repro Row");
        FR := RecRef.Field(2);
        I := 5;
        FR.SetRange(I);
        // BC 29: the filter is '5'
        if FR.GetFilter() <> '5' then
            Error('got [%1], BC gives [5]', FR.GetFilter());
    end;

    [Test]
    procedure SetRangeWithDecimalOnTextField()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        D: Decimal;
    begin
        RecRef.Open(Database::"Repro Row");
        FR := RecRef.Field(3);
        D := 1.5;
        FR.SetRange(D);
        // BC 29 (an en-US session): the filter is '1.5'
        if FR.GetFilter() <> '1.5' then
            Error('got [%1], BC gives [1.5]', FR.GetFilter());
    end;
}
