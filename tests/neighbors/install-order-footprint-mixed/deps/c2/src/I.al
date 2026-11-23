codeunit 52140 "RP Mixed C2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "RP Mixed Collide";
    begin
        R.Code := 'ROW';
        R.Value := 'C2';
        if not R.Insert() then R.Modify();
    end;
}
