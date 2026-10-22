// Stub fields named only as the field argument of Validate and SetRange (ADR-051 decision 10).
table 50240 "Tdd Arg Item"
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

codeunit 50241 "Tdd Arg Tests"
{
    Subtype = Test;

    [Test]
    procedure ValidateNewField()
    var
        Item: Record "Tdd Arg Item";
    begin
        Item."No." := 'A';
        Item.Validate(Quantity, 5);
        Item.Insert();
        Item.Get('A');
        if Item.Quantity <> 5 then
            Error('Quantity is %1', Item.Quantity);
    end;

    [Test]
    procedure ValidateOnlyNewField()
    var
        Item: Record "Tdd Arg Item";
    begin
        Item."No." := 'C';
        Item.Validate(Size, 3);
        Item.Insert();
    end;

    [Test]
    procedure SetRangeNewField()
    var
        Item: Record "Tdd Arg Item";
    begin
        Item."No." := 'B';
        Item.Insert();
        Item.SetRange(Weight, 1, 10);
        if not Item.IsEmpty() then
            Error('a row with weight 0 is in the range 1..10');
    end;

    [Test]
    procedure Untouched()
    var
        Item: Record "Tdd Arg Item";
    begin
        Item.SetRange("No.", 'X');
        if not Item.IsEmpty() then
            Error('no row X');
    end;
}
