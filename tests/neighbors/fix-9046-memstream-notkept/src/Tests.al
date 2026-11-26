// Issue 9046: as fix-9046-memstream, but MemoryStream(Int32), a constructor that stays on the sidecar and touches
// everything. Three such installs tie and have all their orders tried, on main and on head alike.
codeunit 53190 "FP MemStream Cap Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP MemStream Cap TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP MemStream Cap TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure RowOfCExists()
    var
        R: Record "FP MemStream Cap TC";
    begin
        R.Get('C');
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    begin
        if 1 + 1 <> 2 then
            Error('arithmetic');
    end;
}
