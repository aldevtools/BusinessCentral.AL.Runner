table 54500 "RQ tg Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 54501 "RQ tg Events"
{
    procedure AskOne(): Integer
    var
        Count: Integer;
    begin
        OnOne(Count);
        exit(Count);
    end;

    procedure AskTwo(): Integer
    var
        Count: Integer;
    begin
        OnTwo(Count);
        exit(Count);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnOne(var Count: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnTwo(var Count: Integer)
    begin
    end;
}
