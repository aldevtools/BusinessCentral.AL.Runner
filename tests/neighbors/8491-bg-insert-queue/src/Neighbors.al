// Neighbors of al-runner issue 8491. A worker's Get while the error of an Insert is deferred in the test session runs as BC does
// (BgTaskInsertQueue.al:800, :830, :855, :879): it does not find the queued row, it finds the row a duplicate Insert repeated,
// and the error comes out at the end of the test. Not measured, refused: a worker's other reads while the error is deferred
// (FindFirst), a worker's Get while the error of a closing date in 1753 is deferred, and a worker's TableRelation check of a table
// whose queue holds rows (BC does not find the queued row there). A worker's write raises the read-only error, as on main. A
// duplicate that is the 5th Insert after a FlowField of the test session sent the queue raises at once (flagged: no cell has
// it). Every BC answer here: a local BC 29.1 container. (Rows queued under another company value need two companies: al-testrun
// `companies_opt_in.rs`.)

codeunit 97590 "IQ Neighbor Tests"
{
    Subtype = Test;

    local procedure RunTask(var Host: TestPage "IQ Host"; Worker: Integer; Mode: Text; No: Text): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('No', No);
        Parameters.Add('Mode', Mode);
        Results := Host.RunPageBackgroundTask(Worker, Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The worker's Modify of a committed row while the queue holds another row of the table: the read-only error, as on main.
    [Test]
    procedure Queued_TaskModify()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Row."No." := 'IQNB9';
        Row.Insert();
        Commit();
        Host.OpenView();
        Row."No." := 'IQNB10';
        Row.Insert();
        Parameters.Add('No', 'IQNB9');
        // BC: the task fails (the session of a worker is read-only). Its text is not read, as above.
        asserterror Results := Host.RunPageBackgroundTask(Codeunit::"IQ Writer", Parameters, false);
    end;

    [Test]
    procedure DeferredDuplicate_TaskRead()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Row."No." := 'IQNB1';
        Row.Insert();
        Row.Get('IQNB1');
        Host.OpenView();
        Row."No." := 'IQNB2';
        Row.Insert();
        Row."No." := 'IQNB1';
        Row.Insert();
        Parameters.Add('No', 'IQNB2');
        Parameters.Add('Mode', 'get');
        Results := Host.RunPageBackgroundTask(Codeunit::"IQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        // BC: no error at the duplicate Insert, and the worker does not find the queued row (a local BC 29.1 container).
        if Seen <> '-' then
            Error('The worker saw %1.', Seen);
    end;

    // The worker's Get of the key a deferred duplicate Insert repeated: BC finds the row the test read, and the duplicate comes out
    // at the end of the test.
    [Test]
    procedure DeferredDuplicate_TaskGetsTheDuplicatedKey()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Seen: Text;
    begin
        Row."No." := 'IQNB4';
        Row.Description := 'a';
        Row.Insert();
        Row.Get('IQNB4');
        Host.OpenView();
        Row."No." := 'IQNB5';
        Row.Insert();
        Row."No." := 'IQNB4';
        Row.Insert();
        Seen := RunTask(Host, Codeunit::"IQ Reader", 'get', 'IQNB4');
        if Seen <> 'a' then
            Error('The worker saw %1.', Seen);
    end;

    // The worker's Get of the row whose Date its SQL column cannot hold: BC does not find it, and the error comes out at the end of
    // the test.
    [Test]
    procedure DeferredSqlRange_TaskGet()
    var
        Host: TestPage "IQ Host";
        DateRow: Record "IQ Date Row";
        Seen: Text;
    begin
        Host.OpenView();
        DateRow."No." := 'IQNB6';
        DateRow.Description := 'd';
        DateRow.Day := 17530615D;
        DateRow.Insert();
        Seen := RunTask(Host, Codeunit::"IQ Reader More", 'getdate', 'IQNB6');
        if Seen <> '-' then
            Error('The worker saw %1.', Seen);
    end;

    // The worker's FindFirst while the duplicate's error is deferred: refused (BC finds the row the test read, and the duplicate
    // comes out at the end of the test).
    [Test]
    procedure DeferredDuplicate_TaskFindFirst()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Seen: Text;
    begin
        Row."No." := 'IQNB7';
        Row.Description := 'a';
        Row.Insert();
        Row.Get('IQNB7');
        Host.OpenView();
        Row."No." := 'IQNB8';
        Row.Insert();
        Row."No." := 'IQNB7';
        Row.Insert();
        Seen := RunTask(Host, Codeunit::"IQ Reader", 'first', 'IQNB7');
        if Seen <> 'a' then
            Error('The worker saw %1.', Seen);
    end;

    // The worker's Get of the row whose closing date in 1753 its SQL column cannot hold: refused, no cell has a closing date (BC
    // does not find it, and the error comes out at the end of the test).
    [Test]
    procedure DeferredSqlRangeClosingDate_TaskGet()
    var
        Host: TestPage "IQ Host";
        DateRow: Record "IQ Date Row";
        Seen: Text;
    begin
        Host.OpenView();
        DateRow."No." := 'IQNB16';
        DateRow.Description := 'd';
        DateRow.Day := ClosingDate(17530615D);
        DateRow.Insert();
        Seen := RunTask(Host, Codeunit::"IQ Reader More", 'getdate', 'IQNB16');
        if Seen <> '-' then
            Error('The worker saw %1.', Seen);
    end;

    [Test]
    procedure Queued_TaskTableRelation()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Host.OpenView();
        Row."No." := 'IQNB3';
        Row.Insert();
        Parameters.Add('No', 'IQNB3');
        // BC: the worker's TableRelation check does not find the queued row, and the task fails. GetLastErrorText after the
        // asserterror does not give the worker's text on BC (it gives the test session's last error before), so it is not read.
        asserterror Results := Host.RunPageBackgroundTask(Codeunit::"IQ Relation Reader", Parameters, false);
    end;

    // The test session's FlowField sends the queue and its count of five starts again: the 5th Insert after it, a duplicate of
    // the row the FlowField sent, raises at once, on BC too (Y7407DuplicateInsertBuffer.al measures the 5th row after inserts
    // only).
    [Test]
    procedure IndirectSend_FifthInsertDuplicateRaises()
    var
        Row: Record "IQ Row";
        Counter: Record "IQ Counter";
    begin
        Row."No." := 'IQNB11';
        Row.Insert();
        Counter.SetFilter("Prefix Filter", 'IQNB11*');
        Counter.CalcFields(Rows);
        Row."No." := 'IQNB12';
        Row.Insert();
        Row."No." := 'IQNB13';
        Row.Insert();
        Row."No." := 'IQNB14';
        Row.Insert();
        Row."No." := 'IQNB15';
        Row.Insert();
        Row."No." := 'IQNB11';
        asserterror Row.Insert();
        if StrPos(GetLastErrorText(), 'already exists') = 0 then
            Error('Another error: %1', GetLastErrorText());
    end;
}

codeunit 97591 "IQ Relation Reader"
{
    trigger OnRun()
    var
        Link: Record "IQ Link";
        Results: Dictionary of [Text, Text];
        No: Text;
    begin
        Page.GetBackgroundParameters().Get('No', No);
        Link.Validate("Row No.", No);
        Results.Add('Seen', Link."Row No.");
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97592 "IQ Writer"
{
    trigger OnRun()
    var
        Row: Record "IQ Row";
        No: Text;
    begin
        Page.GetBackgroundParameters().Get('No', No);
        Row."No." := CopyStr(No, 1, MaxStrLen(Row."No."));
        Row.Description := 'w';
        Row.Modify();
    end;
}
