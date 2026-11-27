// Issue 9260 neighbor: two installs read kept builtins, the third CompanyProperty.UrlName, which still touches everything and ties all three; three tied installs have every order tried.
codeunit 52890 "FP Readonly UrlName Tests"
{
    Subtype = Test;

    [Test]
    procedure RowOfAExists()
    var
        R: Record "FP Readonly TA";
    begin
        R.Get('A');
    end;

    [Test]
    procedure RowOfBExists()
    var
        R: Record "FP Readonly TB";
    begin
        R.Get('B');
    end;

    [Test]
    procedure RowOfCExists()
    var
        R: Record "FP Readonly TC";
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
