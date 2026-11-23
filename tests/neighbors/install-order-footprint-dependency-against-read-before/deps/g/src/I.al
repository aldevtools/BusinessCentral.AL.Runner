codeunit 54120 "RQ dc2 G Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ dc2 Events";
        R: Record "RQ dc2 Row";
    begin
        if Events.AskTwo() then;
        R.Code := 'G';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}
