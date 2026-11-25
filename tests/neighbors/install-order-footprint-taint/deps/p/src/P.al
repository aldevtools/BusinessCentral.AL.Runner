table 52500 "RP Taint D Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

table 52501 "RP Taint X Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52502 "RP Taint Events"
{
    procedure Ask(): Boolean
    var
        Seen: Boolean;
    begin
        OnAsk(Seen);
        exit(Seen);
    end;
    [IntegrationEvent(false, false)]
    local procedure OnAsk(var Seen: Boolean)
    begin
    end;
}
