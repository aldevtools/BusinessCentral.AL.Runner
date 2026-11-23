codeunit 50111 "NB SPNB Assert"
{
    procedure AreEqual(Expected: Text; Actual: Text; What: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected [%2], got [%3]', What, Expected, Actual);
    end;
}
