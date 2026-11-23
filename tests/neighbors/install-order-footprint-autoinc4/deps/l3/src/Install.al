codeunit 51430 "FP AutoInc L3 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Log: Record "FP AutoInc Log";
    begin
        Log.Message := 'L3';
        Log.Insert();
    end;
}
