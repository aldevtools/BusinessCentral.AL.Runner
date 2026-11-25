// Issue 8954: four unordered apps each build a local System.Text.StringBuilder in their install.
codeunit 52190 "FP Dotnet Local Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP Dotnet Local TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP Dotnet Local TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure RowOfCExists()
    var
        R: Record "FP Dotnet Local TC";
    begin
        R.Get('C');
    end;

    [Test]
    procedure RowOfDExists()
    var
        R: Record "FP Dotnet Local TD";
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
