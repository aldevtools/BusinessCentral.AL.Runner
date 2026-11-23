codeunit 51360 "FP Data Q1 Install"
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
