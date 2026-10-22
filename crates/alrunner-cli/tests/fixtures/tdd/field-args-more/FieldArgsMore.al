// Stub fields named only as the field argument of TestField, SetFilter, CalcFields, FieldNo, FieldCaption,
// ModifyAll and SetCurrentKey (ADR-051 decision 10).
table 50260 "Tdd Arg2 Item"
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

codeunit 50261 "Tdd Arg2 Tests"
{
    Subtype = Test;

    [Test]
    procedure TestFieldNewField()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item."No." := 'A';
        Item.TestField(Amount, 0);
    end;

    [Test]
    procedure ModifyAllNewField()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item."No." := 'A';
        Item.Insert();
        Item.ModifyAll(Level, 7);
        Item.Get('A');
        if Item.Level <> 7 then
            Error('Level is %1', Item.Level);
    end;

    [Test]
    procedure SetFilterNewField()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item."No." := 'B';
        Item.Insert();
        Item.SetFilter(Rank, '>%1', 5);
        if not Item.IsEmpty() then
            Error('a row with rank 0 is in the filter >5');
    end;

    [Test]
    procedure FieldNoTypedElsewhere()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item.Memo := 'x';
        if Item.FieldNo(Memo) = 0 then
            Error('no field number');
        if Item.FieldCaption(Memo) <> 'Memo' then
            Error('caption is %1', Item.FieldCaption(Memo));
    end;

    [Test]
    procedure SetCurrentKeyTypedElsewhere()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item.Prio := 1;
        Item.SetCurrentKey(Prio);
    end;

    [Test]
    procedure SetFilterWithoutValue()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item.SetFilter(Color, 'RED');
    end;

    [Test]
    procedure FieldNoOnly()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        if Item.FieldNo(Weight) = 0 then
            Error('no field number');
    end;

    [Test]
    procedure TestFieldWithoutValue()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item.TestField(Flag);
    end;

    [Test]
    procedure CalcFieldsNewField()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item.CalcFields(Total);
    end;

    [Test]
    procedure CalcFieldsTypedElsewhere()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item.Sum := 1;
        Item.CalcFields(Sum);
    end;

    [Test]
    procedure Untouched()
    var
        Item: Record "Tdd Arg2 Item";
    begin
        Item.TestField("No.", '');
    end;
}
