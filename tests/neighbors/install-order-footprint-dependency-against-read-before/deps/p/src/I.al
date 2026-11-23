codeunit 54110 "RQ dc2 P Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ dc2 Events";
        R: Record "RQ dc2 Row";
    begin
        R.Code := 'P';
        if Events.AskOne() then
            R.Value := 'SEEN'
        else
            R.Value := 'EMPTY';
        if not R.Insert() then R.Modify();
    end;
}
