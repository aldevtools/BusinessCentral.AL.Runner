codeunit 54010 "RQ dc1 P Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ dc1 Events";
        R: Record "RQ dc1 Row";
    begin
        R.Code := 'P';
        if Events.AskOne() then
            R.Value := 'SEEN'
        else
            R.Value := 'EMPTY';
        if not R.Insert() then R.Modify();
    end;
}
