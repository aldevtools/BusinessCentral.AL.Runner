codeunit 54020 "RQ dc1 G Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ dc1 Events";
        R: Record "RQ dc1 Row";
    begin
        if Events.AskTwo() then;
        R.Code := 'G';
        R.Value := 'CONST';
        if not R.Insert() then R.Modify();
    end;
}
