// Neighbors of al-runner issue 8945, review round 1 of al-runner PR 9028: Microsoft caches the partner login changed
// outside any Global Triggers raise, where the measured cells change them inside one.
codeunit 61070 "LD Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
    end;
}

codeunit 61071 "LD Login Tests"
{
    Subtype = Test;

    // One global changed by a direct call, the other on the insert event: not measured.
    [Test]
    procedure ReadsChangeLogChangedDirectlyThenOnInsert()
    var
        C: Codeunit "Change Log Management";
    begin
        if not C.IsRead() then
            Error('change log not read');
    end;

    // Asked by a direct call, outside any raise: not measured.
    [Test]
    procedure ReadsApiCacheAskedOutsideRaise()
    var
        W: Codeunit "API Webhook Notification Mgt.";
    begin
        if not W.Asked() then
            Error('api cache');
    end;

    // A test that reads neither cache runs on both binaries (so the two runs are compared).
    [Test]
    procedure RunsWithoutReadingTheCaches()
    var
        N: Integer;
    begin
        N := 2;
        if N <> 2 then
            Error('control');
    end;
}
