codeunit 50271 "Debug Outcome Lib"
{
    SingleInstance = true;

    var
        Raised: Integer;

    procedure Fib(N: Integer): Integer
    begin
        if N < 2 then
            exit(N);
        exit(Fib(N - 1) + Fib(N - 2));
    end;

    [TryFunction]
    procedure TryDivide(A: Integer; B: Integer; var Result: Integer)
    begin
        Result := A div B;
    end;

    procedure SumRows(): Decimal
    var
        Row: Record "Debug Outcome Row";
        Total: Decimal;
    begin
        if Row.FindSet() then
            repeat
                Total += Row.Amount;
            until Row.Next() = 0;
        OnAfterSum(Total);
        exit(Total);
    end;

    procedure CountRaised(): Integer
    begin
        exit(Raised);
    end;

    procedure NoteRaised()
    begin
        Raised += 1;
    end;

    [IntegrationEvent(false, false)]
    local procedure OnAfterSum(Total: Decimal)
    begin
    end;
}
