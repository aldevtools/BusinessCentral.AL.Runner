codeunit 52010 "RP Chain D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Events: Codeunit "RP Chain Events";
        R: Record "RP Chain Row";
    begin
        Events.InitializeMappings();
        R.Code := 'D';
        if R.Insert() then;
    end;
}
