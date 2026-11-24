codeunit 52110 "RP Mixed Y Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "RP Mixed Source";
    begin
        R.Code := 'Y';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
