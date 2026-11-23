codeunit 52120 "RP Mixed X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Src: Record "RP Mixed Source";
        Out: Record "RP Mixed Differs";
    begin
        Out.Code := 'X';
        if Src.IsEmpty() then
            Out.Value := 'EMPTY'
        else
            Out.Value := 'SEEN';
        if not Out.Insert() then Out.Modify();
    end;
}
