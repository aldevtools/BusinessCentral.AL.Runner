// Issue 9244 neighbors: install code that renames a temporary Company record and a company other than the session
// company. The refusal reads only a database row of the session company; both keep main's behavior.
codeunit 50930 "NB SCR Other Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        TempCo: Record Company temporary;
        Co: Record Company;
    begin
        TempCo.Name := CompanyName();
        TempCo.Insert();
        TempCo.Rename('NB SCR TEMP');
        if Co.Get('NB SCR OTHER') then
            Co.Rename('NB SCR OTHER2');
    end;
}
