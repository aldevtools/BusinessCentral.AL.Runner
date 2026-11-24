codeunit 54410 "RQ di P Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ di Events";
        R: Record "RQ di Row";
    begin
        if Events.AskOne() > 0 then;
        R.Code := 'P';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}
