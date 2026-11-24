codeunit 50811 "Fix Round Region Log"
{
    SingleInstance = true;

    var
        Mode: Integer;
        Got: Text;

    procedure SetMode(M: Integer)
    begin
        Mode := M;
        Got := '';
    end;

    procedure GetMode(): Integer
    begin
        exit(Mode);
    end;

    procedure Put(T: Text)
    begin
        Got := T;
    end;

    procedure Take(): Text
    begin
        exit(Got);
    end;
}
