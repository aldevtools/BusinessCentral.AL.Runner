codeunit 54410 "RQ fo B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ fo Events";
        R: Record "RQ fo Row";
    begin
        if Events.AskOne() > 0 then;
        R.Code := 'B';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}
