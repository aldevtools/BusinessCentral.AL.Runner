codeunit 56270 "RQ itm X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ itm Events";
        R: Record "RQ itm Row";
    begin
        if Events.AskX() > 0 then;
        R.Code := 'X';
        R.Value := 'CONST';
        R.Insert();
    end;
}
