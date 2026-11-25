codeunit 55620 "RQ ti T2 Install"
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
        if Events.AskTwo() > 0 then
            if R.Value = '0' then
                R.Value := '1'
            else
                R.Value := '0';
        R.Modify();
    end;
}

