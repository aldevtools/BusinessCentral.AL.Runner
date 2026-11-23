table 52000 "RP Chain Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52001 "RP Chain Events"
{
    SingleInstance = true;
    var
        Prefixes: Dictionary of [Text, Text];
    procedure InitializeMappings()
    begin
        OnRegisterPrefix(Prefixes);
    end;
    [IntegrationEvent(false, false)]
    local procedure OnRegisterPrefix(var Map: Dictionary of [Text, Text])
    begin
    end;
}
