// Issue 8954: A calls StringBuilder.Insert, which is not served in process; B and C call kept members only.
codeunit 52390 "FP Dotnet NotKept Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP Dotnet NotKept TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP Dotnet NotKept TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure RowOfCExists()
    var
        R: Record "FP Dotnet NotKept TC";
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
