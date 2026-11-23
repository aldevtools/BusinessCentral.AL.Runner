codeunit 52210 "RP Enc X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
    begin
        if EncryptionKeyExists() then
            DeleteEncryptionKey();
    end;
}
