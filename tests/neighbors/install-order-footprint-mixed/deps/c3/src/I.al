codeunit 52150 "RP Mixed C3 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "RP Mixed Collide";
    begin
        R.Code := 'ROW';
        R.Value := 'C3';
        if not R.Insert() then R.Modify();
    end;
}
