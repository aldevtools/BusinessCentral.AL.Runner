codeunit 51310 "FP Data Y Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Data Source";
    begin
        R.Code := 'Y';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
