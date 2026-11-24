// Neighbors of al-runner issue 8945, the runner's row write: Microsoft's test runner (130450) reads its tests from rows the
// run writes before the first test, and that write asks the Global Triggers setup for tables 130450 and 130451. BC asks for
// 130450 once or not at all, depending on the test runner, and for 130451 never (corpus PR 1567, cell D). A global a setup
// subscriber changes on those asks is not BC's value; the other globals are.
codeunit 60970 "LS RA Ask Memo"
{
    SingleInstance = true;

    var
        Asks: Integer;
        Kept: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure OnSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        Asks += 1;
    end;

    procedure AskCount(): Integer
    begin
        exit(Asks);
    end;

    procedure RawKept(): Integer
    begin
        exit(Kept);
    end;

    procedure SetKept(N: Integer)
    begin
        Kept := N;
    end;
}

table 60971 "LS RA Row"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

codeunit 60972 "LS RA Tests"
{
    Subtype = Test;

    // Reads the count the runner's asks changed (main counts them; BC's count depends on the runner).
    [Test]
    procedure ReadsTheAskCount()
    var
        Memo: Codeunit "LS RA Ask Memo";
    begin
        if Memo.AskCount() > 100 then
            Error('asks %1', Memo.AskCount());
    end;

    // A test's own write asks for its table and changes the count; reading it is still a read of the runner's part.
    [Test]
    procedure WritesThenReadsTheAskCount()
    var
        Memo: Codeunit "LS RA Ask Memo";
        Row: Record "LS RA Row";
    begin
        Row.No := 1;
        Row.Insert();
        if Memo.AskCount() > 100 then
            Error('asks %1', Memo.AskCount());
    end;

    // Reads a global the asks did not change.
    [Test]
    procedure ReadsAnotherGlobal()
    var
        Memo: Codeunit "LS RA Ask Memo";
    begin
        if Memo.RawKept() <> 0 then
            Error('kept %1', Memo.RawKept());
    end;

    // Writes the global the asks did not change, then reads it.
    [Test]
    procedure WritesThenReadsAnotherGlobal()
    var
        Memo: Codeunit "LS RA Ask Memo";
    begin
        Memo.SetKept(5);
        if Memo.RawKept() <> 5 then
            Error('kept %1', Memo.RawKept());
    end;

    [Test]
    procedure ReadsNothing()
    begin
    end;
}
