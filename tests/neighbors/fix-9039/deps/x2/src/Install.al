codeunit 53820 "IO9039 X2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "IO9039 Emptied Differs";
    begin
        R.Code := 'ROW';
        R.Value := 'X2';
        if not R.Insert() then R.Modify();
    end;
}
