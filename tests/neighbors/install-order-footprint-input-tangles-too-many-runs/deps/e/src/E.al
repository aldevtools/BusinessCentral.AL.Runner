table 56200 "RQ itm Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 56201 "RQ itm Events"
{
    procedure AskX(): Integer
    var
        Count: Integer;
    begin
        OnX(Count);
        exit(Count);
    end;

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

    procedure AskA3(): Integer
    var
        Count: Integer;
    begin
        OnA3(Count);
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

    procedure AskB3(): Integer
    var
        Count: Integer;
    begin
        OnB3(Count);
        exit(Count);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnX(var Count: Integer)
    begin
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
    local procedure OnA3(var Count: Integer)
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

    [IntegrationEvent(false, false)]
    local procedure OnB3(var Count: Integer)
    begin
    end;
}
