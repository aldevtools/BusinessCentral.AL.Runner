// Calls of procedures "Tdd Target" does not declare yet, one test per rule that gives the stub its return type.
codeunit 50110 "Tdd Proc Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit "Tdd Assert";
        Target: Codeunit "Tdd Target";

    [Test]
    procedure VariantSiblingFailsRed()
    begin
        // The other Variant argument is an Integer: the stub returns 0, so the assertion fails.
        Assert.AreEqual(4, Target.DoubleIt(2), 'DoubleIt');
    end;

    [Test]
    procedure DefaultHoldsAndPasses()
    begin
        Assert.AreEqual(0, Target.CountOpen(7), 'CountOpen');
    end;

    [Test]
    procedure AssignTarget()
    var
        D: Decimal;
    begin
        D := 2.5;
        D := Target.Price('A', 3);
        if D <> 0 then
            Error('Price is %1', D);
    end;

    [Test]
    procedure Condition()
    begin
        if Target.IsReady() then
            Error('IsReady is true');
        if not Target.IsDone() then
            exit;
        Error('IsDone is true');
    end;

    [Test]
    procedure KnownParameter()
    begin
        TakesText(Target.Name());
    end;

    [Test]
    procedure ExitValue()
    begin
        if CodeOf() <> '' then
            Error('CodeOf is not empty');
    end;

    [Test]
    procedure Comparison()
    begin
        if Target.Total() <> 0 then
            Error('Total is not 0');
    end;

    [Test]
    procedure CaseSelector()
    begin
        case Target.Kind() of
            1:
                Error('Kind is 1');
        end;
    end;

    [Test]
    procedure FormatIsText()
    begin
        if Format(Target.Label()) <> '' then
            Error('Label is not empty');
    end;

    [Test]
    procedure BareStatement()
    begin
        Target.DoThing();
    end;

    [Test]
    procedure VarArgumentUnchanged()
    var
        I: Integer;
    begin
        I := 5;
        Target.Fill(I);
        if I <> 5 then
            Error('I is %1', I);
    end;

    [Test]
    procedure ArgumentRunsOnce()
    var
        Counter: Codeunit "Tdd Counter";
    begin
        Target.Log(Counter.Next());
        if Counter.Current() <> 1 then
            Error('Next ran %1 times', Counter.Current());
    end;

    [Test]
    procedure RecordIsEmpty()
    var
        Item: Record "Tdd Item";
    begin
        Item."No." := 'X';
        Item := Target.GetItem();
        if Item."No." <> '' then
            Error('the record is not empty');
    end;

    [Test]
    procedure BranchNotRun()
    var
        Run: Boolean;
    begin
        if Run then
            Target.Never();
    end;

    [Test]
    procedure Healthy()
    begin
        Assert.AreEqual(1, Target.Existing(), 'Existing');
    end;

    local procedure TakesText(T: Text)
    begin
        if T <> '' then
            Error('Name is not empty');
    end;

    local procedure CodeOf(): Code[20]
    begin
        exit(Target.CodeOf());
    end;
}
