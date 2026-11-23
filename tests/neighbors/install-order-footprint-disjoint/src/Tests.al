// Two unordered apps whose installs write disjoint tables and read nothing of the other's (ADR-079 decision 6).
codeunit 51690 "FP disjoint Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP Disjoint TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP Disjoint TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
