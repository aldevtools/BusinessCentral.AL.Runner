codeunit 53020 "RQ hidden X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ hidden Events";
        R: Record "RQ hidden Row";
    begin
        if Events.AskOne() then;
        R.Code := 'X';
        if Events.AskTwo() then
            R.Value := 'SEEN'
        else
            R.Value := 'EMPTY';
        if not R.Insert() then R.Modify();
    end;
}
