codeunit 50250 "Debug Dep Subject"
{
    procedure Twice(Value: Integer): Integer
    var
        Result: Integer;
    begin
        Result := Value * 2;
        exit(Result);
    end;

    procedure NotCalled(): Integer
    begin
        exit(7);
    end;
}
