codeunit 52410 "RP Nav2 N1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "RP Nav2 Row";
    begin
        if not NavApp.IsInstalling() then
            exit;
        R.Code := 'N1';
        if R.Insert() then;
    end;
}
