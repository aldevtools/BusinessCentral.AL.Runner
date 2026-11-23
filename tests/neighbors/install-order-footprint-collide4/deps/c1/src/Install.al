codeunit 51010 "FP Collide C1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Collide Row";
    begin
        R.Code := 'ROW';
        R.Value := 'C1';
        if not R.Insert() then R.Modify();
    end;
}
