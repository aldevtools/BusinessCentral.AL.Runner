// Real code an argument of a stub call runs: each Next adds one.
codeunit 50101 "Tdd Counter"
{
    var
        Count: Integer;

    procedure Next(): Integer
    begin
        Count += 1;
        exit(Count);
    end;

    procedure Current(): Integer
    begin
        exit(Count);
    end;
}
