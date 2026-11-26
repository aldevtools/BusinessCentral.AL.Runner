codeunit 51210 "FP Inputs Y1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Inputs T1";
    begin
        R.Code := 'Y1';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
