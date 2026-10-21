// Stub sites whose return type cannot be told from the use: each test is not run, with the reason.
codeunit 50111 "Tdd Refused Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit "Tdd Assert";
        Target: Codeunit "Tdd Target";

    [Test]
    procedure BothSidesUnknown()
    begin
        Assert.AreEqual(Target.ExpectedOf(1), Target.ActualOf(1), 'both');
    end;

    [Test]
    procedure ResultUsedAsReceiver()
    begin
        Target.GetRec().Insert();
    end;

    [Test]
    procedure StillRuns()
    begin
    end;
}
