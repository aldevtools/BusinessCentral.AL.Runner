table 50300 "Nb Ae Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }

table 50301 "Nb Accent Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }

table 50302 "Nb Plain Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }

codeunit 50399 "Nb Run Flag"
{
    SingleInstance = true;
    var
        Seen: Integer;

    procedure Set(V: Integer)
    begin
        Seen := V;
    end;

    procedure Get(): Integer
    begin
        exit(Seen);
    end;
}
