codeunit 53010 "RQ hidden D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ hidden Events";
        R: Record "RQ hidden Row";
    begin
        if Events.AskTwo() then;
        R.Code := 'D';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}
