table 56100 "RQ iti Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 56101 "RQ iti Events"
{
    procedure AskX(): Integer
    var
        Count: Integer;
    begin
        OnX(Count);
        exit(Count);
    end;

    procedure AskI1(): Integer
    var
        Count: Integer;
    begin
        OnI1(Count);
        exit(Count);
    end;

    procedure AskI2(): Integer
    var
        Count: Integer;
    begin
        OnI2(Count);
        exit(Count);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnX(var Count: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnI1(var Count: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnI2(var Count: Integer)
    begin
    end;
}
