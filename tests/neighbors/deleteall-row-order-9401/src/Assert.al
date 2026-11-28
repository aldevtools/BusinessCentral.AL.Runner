// A stand-in for the Assert codeunit of Microsoft's test library.
codeunit 94010 "DRN Assert"
{
    procedure AreEqual(Expected: Variant; Actual: Variant; Msg: Text)
    begin
        if Format(Expected) <> Format(Actual) then
            Error('Assert.AreEqual failed. Expected:<%1>. Actual:<%2>. %3.', Expected, Actual, Msg);
    end;
}
