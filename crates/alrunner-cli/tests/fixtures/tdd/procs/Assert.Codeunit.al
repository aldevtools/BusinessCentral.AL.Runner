// A small stand-in for Library Assert: AreEqual takes two Variants, as Microsoft's does.
codeunit 50102 "Tdd Assert"
{
    procedure AreEqual(Expected: Variant; Actual: Variant; Msg: Text)
    begin
        if Format(Expected) <> Format(Actual) then
            Error('Assert.AreEqual failed. Expected:<%1>. Actual:<%2>. %3.', Expected, Actual, Msg);
    end;
}
