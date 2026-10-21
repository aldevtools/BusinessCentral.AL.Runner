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
}
