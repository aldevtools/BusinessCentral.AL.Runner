codeunit 51410 "FP AutoInc L1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Log: Record "FP AutoInc Log";
    begin
        Log.Message := 'L1';
        Log.Insert();
    end;
}
