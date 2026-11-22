// A stand-in for the Assert codeunit of Microsoft's test library, for the copy of the corpus probe in this folder.
codeunit 130000 Assert
{
    procedure AreEqual(Expected: Variant; Actual: Variant; Msg: Text)
    begin
        if Format(Expected) <> Format(Actual) then
            Error('Assert.AreEqual failed. Expected:<%1>. Actual:<%2>. %3.', Expected, Actual, Msg);
    end;
}
