table 54100 "RQ dc2 Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 54101 "RQ dc2 Events"
{
    procedure AskOne(): Boolean
    var
        Seen: Boolean;
    begin
        OnOne(Seen);
        exit(Seen);
    end;
    procedure AskTwo(): Boolean
    var
        Seen: Boolean;
    begin
        OnTwo(Seen);
        exit(Seen);
    end;
    [IntegrationEvent(false, false)]
    local procedure OnOne(var Seen: Boolean)
    begin
    end;
    [IntegrationEvent(false, false)]
    local procedure OnTwo(var Seen: Boolean)
    begin
    end;
}
