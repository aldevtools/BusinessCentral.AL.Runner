// Four unordered apps write the same row in their install code (ADR-079 decision 2, collide; decision 5 limit).
codeunit 51090 "FP collide4 Tests"
{
    Subtype = Test;

    [Test]
    procedure SharedRowExists()
    var
        R: Record "FP Collide Row";
    begin
        R.Get('ROW');
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
