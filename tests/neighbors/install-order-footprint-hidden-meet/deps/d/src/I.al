codeunit 53210 "RQ hz D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ hz Events";
        R: Record "RQ hz Row";
    begin
        if Events.AskTwo() then;
        R.Code := 'D';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}
