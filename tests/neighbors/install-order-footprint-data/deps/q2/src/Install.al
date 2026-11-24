codeunit 51370 "FP Data Q2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Data Equal";
    begin
        R.Code := 'ROW';
        R.Value := 'SAME';
        if not R.Insert() then R.Modify();
    end;
}
