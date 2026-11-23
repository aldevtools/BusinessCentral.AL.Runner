table 54400 "RQ di Row"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 54401 "RQ di Events"
{
    procedure AskOne(): Integer
    var
        Count: Integer;
    begin
        OnOne(Count);
        exit(Count);
    end;
    [IntegrationEvent(false, false)]
    local procedure OnOne(var Count: Integer)
    begin
    end;
}
