codeunit 51240 "FP Inputs Y4 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Inputs T4";
    begin
        R.Code := 'Y4';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
