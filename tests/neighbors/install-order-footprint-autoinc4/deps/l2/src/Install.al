codeunit 51420 "FP AutoInc L2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Log: Record "FP AutoInc Log";
    begin
        Log.Message := 'L2';
        Log.Insert();
    end;
}
