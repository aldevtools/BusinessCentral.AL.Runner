codeunit 51230 "FP Inputs Y3 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Inputs T3";
    begin
        R.Code := 'Y3';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
