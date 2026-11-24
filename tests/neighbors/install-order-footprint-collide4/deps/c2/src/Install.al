codeunit 51020 "FP Collide C2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Collide Row";
    begin
        R.Code := 'ROW';
        R.Value := 'C2';
        if not R.Insert() then R.Modify();
    end;
}
