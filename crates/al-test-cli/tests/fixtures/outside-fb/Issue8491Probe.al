// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-insert-method
// Scope: in-scope
// local object: the rows the test session inserts and what a page background task's worker reads of them are what is under test
// Fixtures used: shared Assert (60021); own tables 97520 to 97523 and 97527, worker codeunit 97524, page 97525, test codeunit 97526
// Probe (al-runner issue 8491): the test session's statement-form Insert() into a table without a Blob or AutoIncrement field is
//   held back (the insert queue that record/duplicate-insert/Y7407DuplicateInsertBuffer.al pins for the session's own errors);
//   a page background task's worker, run with TestPage.RunPageBackgroundTask, does not see a row that is held back. Every test
//   uses keys of its own (a new Guid), and the worker never reads a key twice.
// Readings: what the worker's Get, Count, IsEmpty, FindFirst, FindSet and a FlowField that counts the rows answer.
// Cells: held back until the test reads the table (:234, :247), a read of another table does not (:262); four inserts are held
//   back, the fifth lets all five through, a sixth is held back again (:277, :294, :312); a Commit, an Insert whose result is
//   used, a table with a Blob field, a Modify of another row of the table and a FlowField of the test session that counts the
//   rows let the row through (:331, :345, :377, :393, :481); Insert(true) into a table with an OnInsert trigger is held back
//   (:361); the worker's own reads do not let rows through (:412); Count, IsEmpty, FindFirst, FindSet and a FlowField leave held-
//   back rows out (:427, :443, :465).
// BC versions: 29.1 (local W1 container).

table 97520 "IQ Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97521 "IQ Other"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97522 "IQ Trigger Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnInsert()
    begin
        Description := Description + '!';
    end;
}

table 97523 "IQ Blob Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; Data; Blob) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97527 "IQ Counter"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Prefix Filter"; Code[20])
        {
            FieldClass = FlowFilter;
        }
        field(3; Rows; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("IQ Row" where("No." = field("Prefix Filter")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The worker: Get of a row of the table the Table parameter names, or Count of IQ Row rows with a key prefix.
codeunit 97524 "IQ Reader"
{
    trigger OnRun()
    var
        Row: Record "IQ Row";
        TriggerRow: Record "IQ Trigger Row";
        BlobRow: Record "IQ Blob Row";
        Counter: Record "IQ Counter";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        No: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('No', No);
        Seen := '-';
        case Parameters.Get('Mode') of
            'get':
                if Row.Get(No) then
                    Seen := Row.Description;
            'gettrigger':
                if TriggerRow.Get(No) then
                    Seen := TriggerRow.Description;
            'getblob':
                if BlobRow.Get(No) then
                    Seen := BlobRow.Description;
            'count':
                begin
                    Row.SetFilter("No.", No + '*');
                    Seen := Format(Row.Count());
                end;
            'flow':
                begin
                    Counter.SetFilter("Prefix Filter", No + '*');
                    Counter.CalcFields(Rows);
                    Seen := Format(Counter.Rows);
                end;
            'isempty':
                begin
                    Row.SetFilter("No.", No + '*');
                    Seen := Format(Row.IsEmpty());
                end;
            'first':
                begin
                    Row.SetFilter("No.", No + '*');
                    if Row.FindFirst() then
                        Seen := Row.Description;
                end;
            'set':
                begin
                    Row.SetFilter("No.", No + '*');
                    Seen := '';
                    if Row.FindSet() then
                        repeat
                            Seen += Row.Description;
                        until Row.Next() = 0;
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

page 97525 "IQ Host"
{
    PageType = Card;
    SourceTable = "IQ Other";
    ApplicationArea = All;
    UsageCategory = Administration;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
        }
    }
}

codeunit 97526 "IQ Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewKey(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    local procedure AddRow(No: Code[20]; Description: Text[50])
    var
        Row: Record "IQ Row";
    begin
        Row."No." := No;
        Row.Description := Description;
        Row.Insert();
    end;

    local procedure Run(var Host: TestPage "IQ Host"; Mode: Text; No: Text): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('No', No);
        Parameters.Add('Mode', Mode);
        Results := Host.RunPageBackgroundTask(Codeunit::"IQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The test inserts a row; the worker's Get does not find it.
    [Test]
    procedure BgTaskInsertQueue_Insert_TaskDoesNotFindIt()
    var
        Host: TestPage "IQ Host";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        Assert.AreEqual('-', Run(Host, 'get', K + 'A'), 'r');
    end;

    // The test inserts a row and reads it (Get): the worker finds it.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestGet_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        Row.Get(K + 'A');
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'r');
    end;

    // A read of another table by the test does not let the row through.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestReadsOtherTable_TaskDoesNotFindIt()
    var
        Host: TestPage "IQ Host";
        Other: Record "IQ Other";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        if Other.FindFirst() then;
        Assert.AreEqual('-', Run(Host, 'get', K + 'A'), 'r');
    end;

    // Four inserts: the worker finds none of them.
    [Test]
    procedure BgTaskInsertQueue_FourInserts_TaskFindsNone()
    var
        Host: TestPage "IQ Host";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        AddRow(K + 'B', 'b');
        AddRow(K + 'C', 'c');
        AddRow(K + 'D', 'd');
        Assert.AreEqual('-', Run(Host, 'get', K + 'A'), 'a');
        Assert.AreEqual('-', Run(Host, 'get', K + 'D'), 'd');
    end;

    // Five inserts: the fifth lets all five through.
    [Test]
    procedure BgTaskInsertQueue_FiveInserts_TaskFindsAll()
    var
        Host: TestPage "IQ Host";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        AddRow(K + 'B', 'b');
        AddRow(K + 'C', 'c');
        AddRow(K + 'D', 'd');
        AddRow(K + 'E', 'e');
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'a');
        Assert.AreEqual('e', Run(Host, 'get', K + 'E'), 'e');
    end;

    // Six inserts: the first five are through, the sixth is held back.
    [Test]
    procedure BgTaskInsertQueue_SixInserts_TaskFindsTheFirstFive()
    var
        Host: TestPage "IQ Host";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        AddRow(K + 'B', 'b');
        AddRow(K + 'C', 'c');
        AddRow(K + 'D', 'd');
        AddRow(K + 'E', 'e');
        AddRow(K + 'F', 'f');
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'a');
        Assert.AreEqual('-', Run(Host, 'get', K + 'F'), 'f');
    end;

    // The test commits its insert: the worker finds it.
    [Test]
    procedure BgTaskInsertQueue_Insert_Commit_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        Commit();
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'r');
    end;

    // An Insert whose result the test uses: the worker finds the row.
    [Test]
    procedure BgTaskInsertQueue_InsertResultUsed_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        Row."No." := K + 'A';
        Row.Description := 'a';
        if Row.Insert() then;
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'r');
    end;

    // Insert(true) into a table with an OnInsert trigger: the worker does not find the row.
    [Test]
    procedure BgTaskInsertQueue_TriggerTable_InsertTrue_TaskDoesNotFindIt()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Trigger Row";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        Row."No." := K + 'A';
        Row.Description := 'a';
        Row.Insert(true);
        Assert.AreEqual('-', Run(Host, 'gettrigger', K + 'A'), 'r');
    end;

    // An Insert into a table with a Blob field: the worker finds the row.
    [Test]
    procedure BgTaskInsertQueue_BlobTable_Insert_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Blob Row";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        Row."No." := K + 'A';
        Row.Description := 'a';
        Row.Insert();
        Assert.AreEqual('a', Run(Host, 'getblob', K + 'A'), 'r');
    end;

    // The test inserts a row, then modifies another row of the table: the worker finds the row.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestModifiesOtherRow_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        AddRow(K + 'Z', 'z');
        Commit();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        Row."No." := K + 'Z';
        Row.Description := 'z2';
        Row.Modify();
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'r');
    end;

    // Two inserts; the worker's read of the first does not let the second through.
    [Test]
    procedure BgTaskInsertQueue_TwoInserts_TaskReadsDoNotLetThemThrough()
    var
        Host: TestPage "IQ Host";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        AddRow(K + 'B', 'b');
        Assert.AreEqual('-', Run(Host, 'get', K + 'A'), 'a');
        Assert.AreEqual('-', Run(Host, 'get', K + 'B'), 'b');
    end;

    // A row the test read, and a new insert: the worker's Count counts the first only.
    [Test]
    procedure BgTaskInsertQueue_Insert_TaskCountLeavesItOut()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        AddRow(K + 'A', 'a');
        Row.Get(K + 'A');
        Host.OpenView();
        AddRow(K + 'B', 'b');
        Assert.AreEqual('1', Run(Host, 'count', K), 'c');
    end;

    // A row the test read, and new inserts before and after it and in another key range: the worker's IsEmpty, FindFirst and FindSet leave the new ones out.
    [Test]
    procedure BgTaskInsertQueue_Inserts_TaskIsEmptyFindFirstFindSetLeaveThemOut()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
        L: Code[20];
    begin
        K := NewKey();
        L := NewKey();
        AddRow(K + 'B', 'b');
        Row.Get(K + 'B');
        Host.OpenView();
        AddRow(K + 'A', 'a');
        AddRow(K + 'C', 'c');
        AddRow(L + 'A', 'l');
        Assert.AreEqual('Yes', Run(Host, 'isempty', L), 'empty');
        Assert.AreEqual('b', Run(Host, 'first', K), 'first');
        Assert.AreEqual('b', Run(Host, 'set', K), 'set');
    end;

    // A row the test read, and a new insert: a FlowField of the worker that counts the rows leaves the new one out.
    [Test]
    procedure BgTaskInsertQueue_Insert_TaskFlowFieldLeavesItOut()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        AddRow(K + 'A', 'a');
        Row.Get(K + 'A');
        Host.OpenView();
        AddRow(K + 'B', 'b');
        Assert.AreEqual('1', Run(Host, 'flow', K), 'FlowField count');
    end;

    // The test inserts a row, then reads a FlowField that counts the rows of the table: the worker finds the row.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestFlowField_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        Counter: Record "IQ Counter";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        Counter.SetFilter("Prefix Filter", K + '*');
        Counter.CalcFields(Rows);
        Assert.AreEqual(1, Counter.Rows, 'the test''s FlowField');
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'Get');
    end;
    // The cells from here on (al-runner pull request 8498, review round 1; objects after this codeunit). Readings as above.
    // - A worker that page code starts (CurrPage.EnqueueBackgroundTask) does not see a held-back row either (:548, :566).
    // - The worker's FindLast, Find('-') with Next, SetLoadFields with Get, RecordRef.Count, CountApprox, GetBySystemId, a Query,
    //   CalcSums and a Sum FlowField leave held-back rows out (:587, :606, :624, :640, :657).
    // - An Insert into a table with an OnAfterInsertEvent subscriber is held back (:675); a read of the table inside the
    //   subscriber lets it through (:691).
    // - A FlowField of the test session lets the rows through and the count of five starts again (:708); so do the test
    //   session's CountApprox, Query, GetBySystemId and the TableRelation check of a Validate (:730, :747, :765, :783).
    local procedure AddAmount(No: Code[20]; Amount: Decimal)
    var
        AmountRow: Record "IQ Amount Row";
    begin
        AmountRow."No." := No;
        AmountRow.Amount := Amount;
        AmountRow.Insert();
    end;

    local procedure AddSubRow(No: Code[20]; Description: Text[50])
    var
        SubRow: Record "IQ Sub Row";
    begin
        SubRow."No." := No;
        SubRow.Description := Description;
        SubRow.Insert();
    end;

    // Inserts an IQ Other row and commits, so page 97528 opens on a row; nothing of IQ Row is held back yet.
    local procedure AddHostRow(No: Code[20])
    var
        Other: Record "IQ Other";
    begin
        Other."No." := No;
        Other.Insert();
        Commit();
    end;

    // The worker codeunit 97537 run by the test, as Run does with 97524.
    local procedure RunMore(var Host: TestPage "IQ Host"; Mode: Text; No: Text): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('No', No);
        Parameters.Add('Mode', Mode);
        Results := Host.RunPageBackgroundTask(Codeunit::"IQ Reader More", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // Page 97528's own code starts the worker (CurrPage.EnqueueBackgroundTask in OnAfterGetCurrRecord) after the test inserted a
    // row: the worker's Get does not find it.
    [Test]
    procedure BgTaskInsertQueue_Insert_PageEnqueuedTaskDoesNotFindIt()
    var
        TaskHost: TestPage "IQ Task Host";
        IqMode: Codeunit "IQ Mode";
        K: Code[20];
    begin
        K := NewKey();
        AddHostRow(K);
        AddRow(K + 'A', 'a');
        IqMode.SetMode('get', K + 'A');
        TaskHost.OpenView();
        TaskHost.GoToKey(K);
        IqMode.SetMode('', '');
        Assert.AreEqual('-', TaskHost.Seen.Value(), 'page task');
    end;

    // The same after the test read the row: the page's worker finds it.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestGet_PageEnqueuedTaskFindsIt()
    var
        TaskHost: TestPage "IQ Task Host";
        IqMode: Codeunit "IQ Mode";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        AddHostRow(K);
        AddRow(K + 'A', 'a');
        Row.Get(K + 'A');
        IqMode.SetMode('get', K + 'A');
        TaskHost.OpenView();
        TaskHost.GoToKey(K);
        IqMode.SetMode('', '');
        Assert.AreEqual('a', TaskHost.Seen.Value(), 'page task');
    end;

    // A row the test read and new inserts before and after it: the worker's FindLast, and Find('-') with Next, leave the new ones
    // out.
    [Test]
    procedure BgTaskInsertQueue_Inserts_TaskFindLastFindNextLeaveThemOut()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        AddRow(K + 'B', 'b');
        Row.Get(K + 'B');
        Host.OpenView();
        AddRow(K + 'A', 'a');
        AddRow(K + 'C', 'c');
        Assert.AreEqual('b', RunMore(Host, 'last', K), 'FindLast');
        Assert.AreEqual('b', RunMore(Host, 'findnext', K), 'Find and Next');
    end;

    // A row the test read and a new insert: the worker's RecordRef.Count counts the first only, its SetLoadFields and Get does not
    // find the new one, and its CountApprox answers 1.
    [Test]
    procedure BgTaskInsertQueue_Insert_TaskRecordRefLoadFieldsCountApproxLeaveItOut()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        AddRow(K + 'A', 'a');
        Row.Get(K + 'A');
        Host.OpenView();
        AddRow(K + 'B', 'b');
        Assert.AreEqual('1', RunMore(Host, 'recref', K), 'RecordRef.Count');
        Assert.AreEqual('-', RunMore(Host, 'loadfields', K + 'B'), 'SetLoadFields and Get');
        Assert.AreEqual('1', RunMore(Host, 'countapprox', K), 'CountApprox');
    end;

    // The test inserts a row: the worker's GetBySystemId of it does not find it.
    [Test]
    procedure BgTaskInsertQueue_Insert_TaskGetBySystemIdDoesNotFindIt()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        Row."No." := K + 'A';
        Row.Description := 'a';
        Row.Insert();
        Assert.AreEqual('-', RunMore(Host, 'sysid', Format(Row.SystemId)), 'GetBySystemId');
    end;

    // A row the test read and a new insert: a Query the worker reads returns the first only.
    [Test]
    procedure BgTaskInsertQueue_Insert_TaskQueryLeavesItOut()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        AddRow(K + 'A', 'a');
        Row.Get(K + 'A');
        Host.OpenView();
        AddRow(K + 'B', 'b');
        Assert.AreEqual('1', RunMore(Host, 'query', K), 'Query');
    end;

    // A row the test read and a new insert into a table with a SumIndexField: the worker's CalcSums and a Sum FlowField leave the
    // new one out.
    [Test]
    procedure BgTaskInsertQueue_Insert_TaskCalcSumsSumFlowFieldLeaveItOut()
    var
        Host: TestPage "IQ Host";
        AmountRow: Record "IQ Amount Row";
        K: Code[20];
    begin
        K := NewKey();
        AddAmount(K + 'A', 1);
        AmountRow.Get(K + 'A');
        Host.OpenView();
        AddAmount(K + 'B', 10);
        Assert.AreEqual('1', RunMore(Host, 'sums', K), 'CalcSums');
        Assert.AreEqual('1', RunMore(Host, 'sumflow', K), 'Sum FlowField');
    end;

    // An Insert into a table with an OnAfterInsertEvent subscriber that reads nothing of the table: the worker does not find the
    // row.
    [Test]
    procedure BgTaskInsertQueue_SubscriberTable_Insert_TaskDoesNotFindIt()
    var
        Host: TestPage "IQ Host";
        SubEvents: Codeunit "IQ Sub Events";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        SubEvents.Reset(false);
        AddSubRow(K + 'A', 'a');
        Assert.AreEqual(1, SubEvents.GetCalls(), 'the subscriber ran');
        Assert.AreEqual('-', RunMore(Host, 'getsub', K + 'A'), 'r');
    end;

    // The same subscriber reads the row the Insert stored (Get): the worker finds it.
    [Test]
    procedure BgTaskInsertQueue_SubscriberTable_SubscriberReadsRow_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        SubEvents: Codeunit "IQ Sub Events";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        SubEvents.Reset(true);
        AddSubRow(K + 'A', 'a');
        Assert.AreEqual(1, SubEvents.GetCalls(), 'the subscriber ran');
        Assert.AreEqual('a', RunMore(Host, 'getsub', K + 'A'), 'r');
    end;

    // Three inserts, a FlowField of the test session that counts the rows, then two more inserts: the worker finds the first three
    // and neither of the last two (the FlowField sent the queue and its count starts again).
    [Test]
    procedure BgTaskInsertQueue_ThreeInserts_TestFlowField_TwoInserts_TaskFindsTheFirstThree()
    var
        Host: TestPage "IQ Host";
        Counter: Record "IQ Counter";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        AddRow(K + 'B', 'b');
        AddRow(K + 'C', 'c');
        Counter.SetFilter("Prefix Filter", K + '*');
        Counter.CalcFields(Rows);
        AddRow(K + 'D', 'd');
        AddRow(K + 'E', 'e');
        Assert.AreEqual('c', Run(Host, 'get', K + 'C'), 'C');
        Assert.AreEqual('-', Run(Host, 'get', K + 'D'), 'D');
        Assert.AreEqual('-', Run(Host, 'get', K + 'E'), 'E');
    end;

    // The test inserts a row, then reads CountApprox of the table: the worker finds the row.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestCountApprox_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        Row.SetFilter("No.", K + '*');
        if Row.CountApprox() < 0 then
            Error('negative');
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'r');
    end;

    // The test inserts a row, then reads a Query over the table: the worker finds the row.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestQuery_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        IqRows: Query "IQ Rows";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        IqRows.SetFilter(No, K + '*');
        IqRows.Open();
        Assert.IsTrue(IqRows.Read(), 'the test''s Query');
        IqRows.Close();
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'r');
    end;

    // The test inserts a row, then reads it with GetBySystemId: the worker finds the row.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestGetBySystemId_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Other: Record "IQ Row";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        Row."No." := K + 'A';
        Row.Description := 'a';
        Row.Insert();
        Assert.IsTrue(Other.GetBySystemId(Row.SystemId), 'the test''s GetBySystemId');
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'r');
    end;

    // The test inserts a row, then validates a field whose TableRelation reads the table: the worker finds the row.
    [Test]
    procedure BgTaskInsertQueue_Insert_TestTableRelation_TaskFindsIt()
    var
        Host: TestPage "IQ Host";
        Link: Record "IQ Link";
        K: Code[20];
    begin
        K := NewKey();
        Host.OpenView();
        AddRow(K + 'A', 'a');
        Link.Validate("Row No.", K + 'A');
        Assert.AreEqual('a', Run(Host, 'get', K + 'A'), 'r');
    end;

    // The test inserts row A and reads it, then inserts row B and A again (a duplicate key: its error is deferred), and the
    // worker reads B; a Commit ends the asserterror. Readings: what the worker's Get of B answers, whether the statement after
    // it is reached, and the error the asserterror caught.
    [Test]
    procedure BgTaskInsertQueue_DuplicateInsertDeferred_TaskDoesNotFindTheOther_CommitRaises()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        K: Code[20];
        Seen: Text;
        Reached: Text;
    begin
        K := NewKey();
        Host.OpenView();
        asserterror begin
            AddRow(K + 'A', 'a');
            Row.Get(K + 'A');
            AddRow(K + 'B', 'b');
            AddRow(K + 'A', 'a2');
            Seen := Run(Host, 'get', K + 'B');
            Reached := 'yes';
            Commit();
        end;
        Assert.AreEqual('-', Seen, 'the worker''s Get of B');
        Assert.AreEqual('yes', Reached, 'the statement after the worker');
        Assert.IsTrue(StrPos(GetLastErrorText(), 'already exists') > 0, GetLastErrorText());
    end;
}

// The objects of the cells from :548 on (al-runner pull request 8498, review round 1).

table 97530 "IQ Amount Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; SumIndexFields = Amount; }
    }
}

table 97539 "IQ Sum Counter"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Prefix Filter"; Code[20])
        {
            FieldClass = FlowFilter;
        }
        field(3; Amount; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("IQ Amount Row".Amount where("No." = field("Prefix Filter")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// A table with an OnAfterInsertEvent subscriber (codeunit 97533).
table 97532 "IQ Sub Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// A field whose TableRelation reads IQ Row.
table 97534 "IQ Link"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Row No."; Code[20])
        {
            TableRelation = "IQ Row";
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

query 97531 "IQ Rows"
{
    QueryType = Normal;

    elements
    {
        dataitem(Row; "IQ Row")
        {
            column(No; "No.") { }
            column(Description; Description) { }
        }
    }
}

// The subscriber of IQ Sub Row's OnAfterInsertEvent: it counts its calls and, when asked, reads the row the Insert stored.
codeunit 97533 "IQ Sub Events"
{
    SingleInstance = true;

    var
        Calls: Integer;
        ReadRow: Boolean;

    procedure Reset(NewReadRow: Boolean)
    begin
        Calls := 0;
        ReadRow := NewReadRow;
    end;

    procedure GetCalls(): Integer
    begin
        exit(Calls);
    end;

    [EventSubscriber(ObjectType::Table, Database::"IQ Sub Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertSubRow(var Rec: Record "IQ Sub Row"; RunTrigger: Boolean)
    var
        Other: Record "IQ Sub Row";
    begin
        Calls += 1;
        if ReadRow then
            if Other.Get(Rec."No.") then;
    end;
}

// What the task of page 97528 reads: a mode and a key, set by the test.
codeunit 97529 "IQ Mode"
{
    SingleInstance = true;

    var
        Mode: Text;
        No: Text;

    procedure SetMode(NewMode: Text; NewNo: Text)
    begin
        Mode := NewMode;
        No := NewNo;
    end;

    procedure GetMode(): Text
    begin
        exit(Mode);
    end;

    procedure GetNo(): Text
    begin
        exit(No);
    end;
}

// The worker of the reads from :548 on: FindLast, Find('-') with Next, SetLoadFields with Get, RecordRef.Count, CountApprox,
// GetBySystemId, a Query, CalcSums, a Sum FlowField, and a Get of IQ Sub Row.
codeunit 97537 "IQ Reader More"
{
    trigger OnRun()
    var
        Row: Record "IQ Row";
        AmountRow: Record "IQ Amount Row";
        SumCounter: Record "IQ Sum Counter";
        SubRow: Record "IQ Sub Row";
        IqRows: Query "IQ Rows";
        RecRef: RecordRef;
        FRef: FieldRef;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        No: Text;
        Seen: Text;
        Id: Guid;
        N: Integer;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('No', No);
        Seen := '-';
        case Parameters.Get('Mode') of
            'last':
                begin
                    Row.SetFilter("No.", No + '*');
                    if Row.FindLast() then
                        Seen := Row.Description;
                end;
            'findnext':
                begin
                    Row.SetFilter("No.", No + '*');
                    Seen := '';
                    if Row.Find('-') then
                        repeat
                            Seen += Row.Description;
                        until Row.Next() = 0;
                end;
            'loadfields':
                begin
                    Row.SetLoadFields(Description);
                    if Row.Get(No) then
                        Seen := Row.Description;
                end;
            'recref':
                begin
                    RecRef.Open(Database::"IQ Row");
                    FRef := RecRef.Field(1);
                    FRef.SetFilter(No + '*');
                    Seen := Format(RecRef.Count());
                    RecRef.Close();
                end;
            'countapprox':
                begin
                    Row.SetFilter("No.", No + '*');
                    Seen := Format(Row.CountApprox());
                end;
            'sysid':
                begin
                    Evaluate(Id, No);
                    if Row.GetBySystemId(Id) then
                        Seen := Row.Description;
                end;
            'query':
                begin
                    IqRows.SetFilter(No, No + '*');
                    IqRows.Open();
                    N := 0;
                    while IqRows.Read() do
                        N += 1;
                    IqRows.Close();
                    Seen := Format(N);
                end;
            'sums':
                begin
                    AmountRow.SetFilter("No.", No + '*');
                    AmountRow.CalcSums(Amount);
                    Seen := Format(AmountRow.Amount);
                end;
            'sumflow':
                begin
                    SumCounter.SetFilter("Prefix Filter", No + '*');
                    SumCounter.CalcFields(Amount);
                    Seen := Format(SumCounter.Amount);
                end;
            'getsub':
                if SubRow.Get(No) then
                    Seen := SubRow.Description;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

// A page whose own code starts the worker: OnAfterGetCurrRecord enqueues it with the mode and key of codeunit 97529, and the page
// shows the answer.
page 97528 "IQ Task Host"
{
    PageType = Card;
    SourceTable = "IQ Other";
    ApplicationArea = All;
    UsageCategory = Administration;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(Seen; Seen)
            {
                Editable = false;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        IqMode: Codeunit "IQ Mode";
        Parameters: Dictionary of [Text, Text];
        TaskId: Integer;
    begin
        if IqMode.GetMode() = '' then
            exit;
        Parameters.Add('No', IqMode.GetNo());
        Parameters.Add('Mode', IqMode.GetMode());
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"IQ Reader", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        Seen := CopyStr(Results.Get('Seen'), 1, MaxStrLen(Seen));
    end;

    var
        Seen: Text[50];
}
