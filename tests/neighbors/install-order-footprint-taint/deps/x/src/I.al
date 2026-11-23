codeunit 52520 "RP Taint X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Src: Record "RP Taint D Row";
        Out: Record "RP Taint X Row";
    begin
        Src.Get('D');
        Out.Code := 'X';
        Out.Value := Src.Value;
        if not Out.Insert() then Out.Modify();
    end;
}
