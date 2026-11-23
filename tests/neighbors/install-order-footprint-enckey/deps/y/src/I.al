codeunit 52220 "RP Enc Y Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "RP Enc Row";
    begin
        R.Code := 'Y';
        if EncryptionKeyExists() then
            R.Value := 'KEY'
        else
            R.Value := 'NOKEY';
        if not R.Insert() then R.Modify();
    end;
}
