codeunit 50900 "Order Probe Events"
{
    SingleInstance = true;

    var
        Prefixes: Dictionary of [Text, Text];

    // called from installs: asks every subscriber for its prefix and keeps them (SingleInstance state, not a row)
    procedure InitializeMappings()
    begin
        OnRegisterPrefix(Prefixes);
    end;

    procedure PrefixCount(): Integer
    begin
        exit(Prefixes.Count());
    end;

    [IntegrationEvent(false, false)]
    local procedure OnRegisterPrefix(var Map: Dictionary of [Text, Text])
    begin
    end;
}
