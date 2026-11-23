codeunit 51030 "FP Collide C3 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Collide Row";
    begin
        R.Code := 'ROW';
        R.Value := 'C3';
        if not R.Insert() then R.Modify();
    end;
}
