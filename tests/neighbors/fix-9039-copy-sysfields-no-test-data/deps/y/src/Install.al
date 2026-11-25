// Writes one row of its own, then copies the company to IO9039S COPY. After X, the copy carries X's setup row with its
// SystemId; before X, X inserts a setup row of its own in the copy.
codeunit 53860 "IO9039S Y Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "IO9039S Copied Alike";
        C: Record Company;
    begin
        if C.Get('IO9039S COPY') then exit;
        R.Code := 'ROW';
        R.Value := 'ALIKE';
        R.Insert();
        Commit();
        Database.CopyCompany(CompanyName(), 'IO9039S COPY');
    end;
}
