codeunit 50210 "Debug Stepping Tests"
{
    Subtype = Test;

    [Test]
    procedure CallsAHelper()
    var
        Result: Integer;
    begin
        Result := 1;
        Result := Twice(Result);
        Result := Result + 10;
        if Result <> 12 then
            Error('Result is %1', Result);
    end;

    local procedure Twice(X: Integer): Integer
    var
        Y: Integer;
    begin
        Y := X * 2;
        exit(Y);
    end;
}
