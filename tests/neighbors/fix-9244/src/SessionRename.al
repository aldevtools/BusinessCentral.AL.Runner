// Issue 9244: install code that renames the session company is refused; a test that renames it keeps main's behavior.
codeunit 50920 "NB SCR Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        Co: Record Company;
    begin
        if Co.Get(CompanyName()) then
            Co.Rename('NB SCR RENAMED');
    end;
}

codeunit 50921 "NB SCR Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure InstallRename_RunWithoutSessionCompanyRow()
    var
        Co: Record Company;
    begin
        // No test data, so no database row of the session company: the install Rename has nothing to refuse.
        if Co.Get('NB SCR RENAMED') then
            Error('the session company was renamed');
    end;

    [Test]
    procedure TestRename_CompanyTheTestCreated()
    var
        Co: Record Company;
    begin
        Co.Init();
        Co.Name := 'NB SCR TC';
        Co.Insert();
        Co.Rename('NB SCR TC2');
        if not Co.Get('NB SCR TC2') then
            Error('the rename did not happen');
        if Co.Get('NB SCR TC') then
            Error('the old name is still there');
    end;
}
