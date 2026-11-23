codeunit 51340 "FP Data T1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Data Tangle";
    begin
        R.Code := 'ROW';
        R.Value := 'T1';
        if not R.Insert() then R.Modify();
    end;
}
