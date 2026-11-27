// Issue 9244 neighbors: install code that renames a temporary Company record. The refusal reads only a database row of the session
// company, so this keeps main's behavior. The rename of another company is NOT run here: no test data means no such
// row, and inserting a Company row in install stops on its Media fields (ADR-013 decision 5).
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
