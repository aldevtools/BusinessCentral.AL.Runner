codeunit 52510 "RP Taint D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RP Taint Events";
        R: Record "RP Taint D Row";
    begin
        R.Code := 'D';
        if Events.Ask() then
            R.Value := 'SEEN'
        else
            R.Value := 'EMPTY';
        if not R.Insert() then R.Modify();
    end;
}
