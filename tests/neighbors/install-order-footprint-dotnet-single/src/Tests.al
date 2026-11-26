// Issue 8954: the StringBuilder is a global of a SingleInstance codeunit of each app.
codeunit 52290 "FP Dotnet Single Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP Dotnet Single TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP Dotnet Single TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure RowOfCExists()
    var
        R: Record "FP Dotnet Single TC";
    begin
        R.Get('C');
    end;

    [Test]
    procedure RowOfDExists()
    var
        R: Record "FP Dotnet Single TD";
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
