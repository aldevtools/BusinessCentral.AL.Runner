// What the later codeunit sees of each writer's OnRun. Every test asserts what BC is expected to leave; the delete-only
// rows are not measured yet (al-runner issue 7622), so the runner keeps main's answer there.
codeunit 86910 "OCW Reader"
{
    Subtype = Test;

    [Test]
    procedure SeedInsert_Seen()
    var
        Row: Record "OCW Row";
    begin
        if not Row.Get('DA1') and not Row.Get('D1') then
            Error('seed rows are not there');
    end;

    [Test]
    procedure DeleteAllOnly_RowsGone()
    var
        Row: Record "OCW Row";
    begin
        Row.SetFilter("Key", 'DA*');
        if not Row.IsEmpty() then
            Error('DeleteAll rows are still there');
    end;

    [Test]
    procedure DeleteOneOnly_RowGone()
    var
        Row: Record "OCW Row";
    begin
        if Row.Get('D1') then
            Error('deleted row is still there');
    end;

    [Test]
    procedure RecordRefInsert_Seen()
    var
        Row: Record "OCW Row";
    begin
        Row.Get('RR');
    end;

    [Test]
    procedure SubscriberWrite_Seen()
    var
        Row: Record "OCW Row";
    begin
        Row.Get('SUB');
    end;

    [Test]
    procedure TempOnly_NothingLeft()
    var
        Row: Record "OCW Row";
    begin
        Row.SetFilter("Key", 'TMP*');
        if not Row.IsEmpty() then
            Error('a temporary row reached the table');
    end;

    [Test]
    procedure TableCount()
    var
        Row: Record "OCW Row";
    begin
        if Row.Count() > 5 then
            Error('too many rows: %1', Row.Count());
    end;
}
