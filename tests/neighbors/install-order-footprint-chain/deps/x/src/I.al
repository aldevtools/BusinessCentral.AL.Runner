codeunit 52020 "RP Chain X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RP Chain Events";
        R: Record "RP Chain Row";
    begin
        Events.InitializeMappings();
        R.Code := 'X';
        if R.Insert() then;
    end;
}
