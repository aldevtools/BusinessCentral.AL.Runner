codeunit 54410 "RQ di P Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ di Events";
        R: Record "RQ di Row";
    begin
        R.Value := Format(Events.AskOne());
        R.Code := 'P';

        if not R.Insert() then R.Modify();
    end;
}
