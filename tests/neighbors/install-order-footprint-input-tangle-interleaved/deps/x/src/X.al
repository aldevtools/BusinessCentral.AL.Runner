codeunit 56130 "RQ iti X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ iti Events";
        R: Record "RQ iti Row";
    begin
        R.Code := 'X';
        if Events.AskX() mod 2 = 0 then
            R.Value := '0'
        else
            R.Value := '1';
        R.Insert();
    end;
}
