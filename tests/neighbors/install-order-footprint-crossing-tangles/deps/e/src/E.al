table 55900 "RQ ct Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 55901 "RQ ct Events"
{
    procedure AskA1(): Integer
    var
        Count: Integer;
    begin
        OnA1(Count);
        exit(Count);
    end;

    procedure AskA2(): Integer
    var
        Count: Integer;
    begin
        OnA2(Count);
        exit(Count);
    end;

    procedure AskB1(): Integer
    var
        Count: Integer;
    begin
        OnB1(Count);
        exit(Count);
    end;

    procedure AskB2(): Integer
    var
        Count: Integer;
    begin
        OnB2(Count);
        exit(Count);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnA1(var Count: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnA2(var Count: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnB1(var Count: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnB2(var Count: Integer)
    begin
    end;
}
