// Issue 8954: each install reads Length and GetValue of the System.Byte[] that Encoding.GetBytes returns (the
// System.Array members listed in inproc.txt) and writes its own row.
codeunit 52790 "FP Dotnet Array Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP Dotnet Array TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP Dotnet Array TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure RowOfCExists()
    var
        R: Record "FP Dotnet Array TC";
    begin
        R.Get('C');
    end;

    [Test]
    procedure RowOfDExists()
    var
        R: Record "FP Dotnet Array TD";
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
