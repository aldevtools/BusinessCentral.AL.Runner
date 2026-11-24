codeunit 51440 "FP AutoInc L4 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Log: Record "FP AutoInc Log";
    begin
        Log.Message := 'L4';
        Log.Insert();
    end;
}
