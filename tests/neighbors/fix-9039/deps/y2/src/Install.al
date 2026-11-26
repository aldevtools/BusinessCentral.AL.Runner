// Empties the table with a DeleteAll without filters: after X2 the row is gone, before X2 it stays.
codeunit 53825 "IO9039 Y2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "IO9039 Emptied Differs";
    begin
        R.DeleteAll();
    end;
}
