// One app's install reads the tables four unordered apps' installs write (ADR-079 decision 5: three inputs).
codeunit 51290 "FP inputs4 Tests"
{
    Subtype = Test;

    [Test]
    procedure ReaderRowExists()
    var
        R: Record "FP Inputs TX";
    begin
        R.Get('X');
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
