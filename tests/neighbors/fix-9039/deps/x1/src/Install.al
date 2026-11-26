codeunit 53810 "IO9039 X1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "IO9039 Emptied Same";
    begin
        R.Code := 'ROW';
        R.Value := 'SAME';
        if not R.Insert() then R.Modify();
    end;
}
