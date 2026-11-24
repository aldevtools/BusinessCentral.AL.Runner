// Four unordered apps whose installs read from each other in a cycle (ADR-079 decision 2; decision 5 limit).
codeunit 51190 "FP cycle4 Tests"
{
    Subtype = Test;

    [Test]
    procedure CycleRowExists()
    var
        R: Record "FP Cycle T1";
    begin
        R.Get('X1');
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
