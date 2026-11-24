codeunit 51350 "FP Data T2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Data Tangle";
    begin
        R.Code := 'ROW';
        R.Value := 'T2';
        if not R.Insert() then R.Modify();
    end;
}
