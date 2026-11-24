codeunit 55610 "RQ ti T1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RQ ti Events";
        R: Record "RQ ti Row";
    begin
        if not R.Get('X') then begin
            R.Code := 'X';
            R.Value := '0';
            R.Insert();
        end;
        if Events.AskOne() > 0 then
            if R.Value = '0' then
                R.Value := '1'
            else
                R.Value := '0';
        R.Modify();
    end;
}

