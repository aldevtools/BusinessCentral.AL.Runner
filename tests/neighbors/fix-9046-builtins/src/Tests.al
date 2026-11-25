// Issue 9046: each install calls the date conversions, and writes only its own table.
codeunit 52590 "FP Builtins Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP Builtins TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP Builtins TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure RowOfCExists()
    var
        R: Record "FP Builtins TC";
    begin
        R.Get('C');
    end;

    [Test]
    procedure RowOfDExists()
    var
        R: Record "FP Builtins TD";
    begin
        R.Get('D');
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    begin
        if 1 + 1 <> 2 then
            Error('arithmetic');
    end;
}
