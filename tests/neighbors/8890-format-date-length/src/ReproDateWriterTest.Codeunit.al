codeunit 50805 "Repro Date Writer Test"
{
    Subtype = Test;

    var
        Writer: Codeunit "Repro Date Writer";

    [Test]
    procedure CompactDateWithLength8()
    var
        Result: Text;
    begin
        // Format(D, 8, '<Year4><Month,2><Day,2>') of 23.11.2026: the text is 8 characters long, so the length changes nothing
        Result := Writer.CompactDate(DMY2Date(23, 11, 2026));
        if Result <> '20261123' then
            Error('Expected 20261123, got %1', Result);
    end;

    [Test]
    procedure CompactDateWithLength0()
    var
        Result: Text;
    begin
        // Same format string without a length (runs on the runner)
        Result := Writer.CompactDateNoLength(DMY2Date(23, 11, 2026));
        if Result <> '20261123' then
            Error('Expected 20261123, got %1', Result);
    end;

    [Test]
    procedure DashedDateWithLength10()
    var
        Result: Text;
    begin
        // Control: a format string the runner pins with length 10
        Result := Writer.DashedDate(DMY2Date(23, 11, 2026));
        if Result <> '2026-11-23' then
            Error('Expected 2026-11-23, got %1', Result);
    end;
}
