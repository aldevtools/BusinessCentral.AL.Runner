codeunit 51040 "FP Collide C4 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Collide Row";
    begin
        R.Code := 'ROW';
        R.Value := 'C4';
        if not R.Insert() then R.Modify();
    end;
}
