codeunit 50210 "Tdd App Tests"
{
    Subtype = Test;

    [Test]
    procedure StubOnTheOtherApp()
    var
        Target: Codeunit "Tdd App Target";
    begin
        if Target.Twice(2) <> 0 then
            Error('Twice is not 0');
    end;

    [Test]
    procedure AppUnderTestStaysAnError()
    var
        Uses: Codeunit "Tdd Uses Missing";
    begin
        Uses.CallsMissing();
    end;

    [Test]
    procedure StubFieldOnATableOfTheOtherApp()
    var
        Item: Record "Tdd App Item";
    begin
        Item."No." := 'A';
        Item.Weight := 2;
        Item.Insert();
        Item.Get('A');
        if Item.Weight <> 2 then
            Error('Weight is %1', Item.Weight);
    end;

    [Test]
    procedure StubValueOnAnEnumOfTheOtherApp()
    var
        Kind: Enum "Tdd App Kind";
    begin
        Kind := Enum::"Tdd App Kind"::Medium;
        if Kind.AsInteger() <> 4 then
            Error('Medium is %1', Kind.AsInteger());
    end;
}
