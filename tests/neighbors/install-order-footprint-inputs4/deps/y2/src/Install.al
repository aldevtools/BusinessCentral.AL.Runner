codeunit 51220 "FP Inputs Y2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Inputs T2";
    begin
        R.Code := 'Y2';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
