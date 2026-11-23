codeunit 52130 "RP Mixed C1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "RP Mixed Collide";
    begin
        R.Code := 'ROW';
        R.Value := 'C1';
        if not R.Insert() then R.Modify();
    end;
}
