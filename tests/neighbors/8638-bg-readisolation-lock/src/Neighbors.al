// The shapes next to the cells of corpus PR 1498 (pages/background-tasks/BgTaskReadIsolationLock.al, in Probe.al): a page
// background task's read under a ReadIsolation of a table the test codeunit wrote. Each refused shape is a test codeunit of its
// own. The refused tests expect the lock timeout, what BC gives in the measured cells next to them; it is not measured for them.
// The tests that keep main's answer expect it.

table 97544 "RL Captioned Row"
{
    Caption = 'RL Captioned';
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Group"; Code[20]) { }
        field(3; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97545 "RL Keyed Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Group"; Code[20]) { }
        field(3; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
        key(ByGroup; "Group") { }
    }
}

// The worker: one read under ReadCommitted (RepeatableRead for 'rrfirst', ReadUncommitted for 'rufindset'), inside a
// TryFunction unless the parameter Caught is 'no'; an error caught is returned by its code.
codeunit 97546 "RL Neighbor Reader"
{
    trigger OnRun()
    var
        Row: Record "RL Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        if Parameters.Get('Caught') = 'no' then begin
            Row.ReadIsolation := IsolationLevel::ReadCommitted;
            if Row.Get(Parameters.Get('Key')) then
                Seen := Row.Description
            else
                Seen := '-';
        end else
            if not TryRead(Parameters, Seen) then
                Seen := 'error: ' + GetLastErrorCode();
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;

    [TryFunction]
    local procedure TryRead(Parameters: Dictionary of [Text, Text]; var Seen: Text)
    var
        Row: Record "RL Row";
        Captioned: Record "RL Captioned Row";
        Keyed: Record "RL Keyed Row";
        Ref: RecordRef;
        GroupRef: FieldRef;
        KeyText: Text;
    begin
        KeyText := Parameters.Get('Key');
        Row.ReadIsolation := IsolationLevel::ReadCommitted;
        Captioned.ReadIsolation := IsolationLevel::ReadCommitted;
        Keyed.ReadIsolation := IsolationLevel::ReadCommitted;
        case Parameters.Get('Method') of
            'get':
                if Row.Get(KeyText) then
                    Seen := Row.Description
                else
                    Seen := '-';
            'findset':
                begin
                    Row.SetRange("Group", KeyText);
                    if Row.FindSet() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'isempty':
                begin
                    Row.SetRange("Group", KeyText);
                    Seen := Format(Row.IsEmpty());
                end;
            'rrfirst':
                begin
                    Row.ReadIsolation := IsolationLevel::RepeatableRead;
                    Row.SetRange("Group", KeyText);
                    if Row.FindFirst() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'keyfirst':
                begin
                    Row.SetRange("No.", KeyText);
                    if Row.FindFirst() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'stmtfirst':
                begin
                    Row.SetRange("Group", KeyText);
                    Row.FindFirst();
                    Seen := Row.Description;
                end;
            'rufindset':
                begin
                    Row.ReadIsolation := IsolationLevel::ReadUncommitted;
                    Row.SetRange("Group", KeyText);
                    if Row.FindSet() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'captioned':
                if Captioned.Get(KeyText) then
                    Seen := Captioned.Description
                else
                    Seen := '-';
            'keyedfirst':
                begin
                    Keyed.SetRange("Group", KeyText);
                    if Keyed.FindFirst() then
                        Seen := Keyed.Description
                    else
                        Seen := '-';
                end;
            'keyedcount':
                begin
                    Keyed.SetRange("Group", KeyText);
                    Seen := Format(Keyed.Count());
                end;
            'recref':
                begin
                    Ref.Open(Database::"RL Row");
                    Ref.ReadIsolation := IsolationLevel::ReadCommitted;
                    GroupRef := Ref.Field(2);
                    GroupRef.SetRange(KeyText);
                    if Ref.FindFirst() then
                        Seen := Format(Ref.Field(3).Value)
                    else
                        Seen := '-';
                end;
        end;
    end;
}

// What the neighbor tests share: keys, the rows they write, and the worker's parameters and result.
codeunit 97547 "RL NB Lib"
{
    procedure NewKey(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    // Inserts a row, commits it.
    procedure Seed(No: Code[20]; Group: Code[20]; Description: Text[50])
    var
        Row: Record "RL Row";
    begin
        Row."No." := No;
        Row."Group" := Group;
        Row.Description := Description;
        Row.Insert();
        Commit();
    end;

    // Changes the row without reading it first, and does not commit.
    procedure Change(No: Code[20]; Description: Text[50])
    var
        Row: Record "RL Row";
    begin
        Row.SetRange("No.", No);
        Row.ModifyAll(Description, Description);
    end;

    procedure Parameters(Method: Text; KeyText: Text; Caught: Text): Dictionary of [Text, Text]
    var
        Result: Dictionary of [Text, Text];
    begin
        Result.Add('Method', Method);
        Result.Add('Key', KeyText);
        Result.Add('Caught', Caught);
        exit(Result);
    end;

    procedure Seen(Results: Dictionary of [Text, Text]): Text
    var
        Value: Text;
    begin
        Results.Get('Seen', Value);
        exit(Value);
    end;
}

codeunit 97548 "RL NB Uncaught"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedGet_OutsideATryFunction()
    var
        Rows: TestPage "RL Rows";
        Results: Dictionary of [Text, Text];
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('get', A, 'no');
        asserterror Results := Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false);
        // BC: the test gets an error with no code after the lock wait (measured on a local BC 29.1 container; its text is not
        // measured).
        Assert.AreEqual('', GetLastErrorCode(), 'the error the worker raised reaches the test');
    end;
}

codeunit 97549 "RL NB FindSet"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedFindSet_GroupOfTheModifiedRow()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        G := Lib.NewKey();
        Lib.Seed(A, G, 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('findset', G, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'FindSet');
    end;
}

codeunit 97550 "RL NB IsEmpty"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedIsEmpty_GroupOfTheModifiedRow()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        G := Lib.NewKey();
        Lib.Seed(A, G, 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('isempty', G, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'IsEmpty');
    end;
}

codeunit 97551 "RL NB RepeatableReadFirst"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure RepeatableReadFindFirst_GroupOfTheModifiedRow()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        G := Lib.NewKey();
        Lib.Seed(A, G, 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('rrfirst', G, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'RepeatableRead FindFirst');
    end;
}

codeunit 97552 "RL NB KeyFilter"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedFindFirst_FilterOnTheKey()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('keyfirst', A, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'FindFirst on the key');
    end;
}

codeunit 97553 "RL NB StatementFirst"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedFindFirst_AsAStatement()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        G := Lib.NewKey();
        Lib.Seed(A, G, 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('stmtfirst', G, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'FindFirst as a statement');
    end;
}

codeunit 97554 "RL NB Delete"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedGet_AfterTheTestDeletedAnotherRow()
    var
        Rows: TestPage "RL Rows";
        Row: Record "RL Row";
        A: Code[20];
        B: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        B := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Lib.Seed(B, '', 'other');
        Row.Get(B);
        Row.Delete();
        Rows.OpenView();
        Parameters := Lib.Parameters('get', A, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'Get after a Delete');
    end;
}

codeunit 97555 "RL NB RolledBack"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedGet_AfterARolledBackInsert()
    var
        Rows: TestPage "RL Rows";
        Row: Record "RL Row";
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        asserterror begin
            Row."No." := Lib.NewKey();
            Row.Insert();
            Error('roll back');
        end;
        Rows.OpenView();
        Parameters := Lib.Parameters('get', A, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'Get after a rollback');
    end;
}

codeunit 97556 "RL NB LockTable"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedGet_AfterTheTestLockedTheTable()
    var
        Rows: TestPage "RL Rows";
        Row: Record "RL Row";
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Row.LockTable();
        Row.Get(A);
        Rows.OpenView();
        Parameters := Lib.Parameters('get', A, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'Get after LockTable');
    end;
}

codeunit 97557 "RL NB LockingRead"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedGet_AfterAnUpdLockReadOfTheTest()
    var
        Rows: TestPage "RL Rows";
        Row: Record "RL Row";
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Row.ReadIsolation := IsolationLevel::UpdLock;
        Row.SetRange("No.", A);
        Row.FindFirst();
        Rows.OpenView();
        Parameters := Lib.Parameters('get', A, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'Get after an UpdLock read');
    end;
}

codeunit 97558 "RL NB Caption"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedGet_TableWithACaption()
    var
        Rows: TestPage "RL Rows";
        Captioned: Record "RL Captioned Row";
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Captioned."No." := A;
        Captioned.Description := 'first';
        Captioned.Insert();
        Commit();
        Rows.OpenView();
        Parameters := Lib.Parameters('captioned', A, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'Get of a captioned table');
    end;
}

codeunit 97559 "RL NB SecondaryKey"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedFindFirst_TableWithASecondaryKey()
    var
        Rows: TestPage "RL Rows";
        Keyed: Record "RL Keyed Row";
        G: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        G := Lib.NewKey();
        Keyed."No." := Lib.NewKey();
        Keyed."Group" := G;
        Keyed.Description := 'first';
        Keyed.Insert();
        Commit();
        Rows.OpenView();
        Parameters := Lib.Parameters('keyedfirst', G, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'FindFirst of a table with a secondary key');
    end;
}

codeunit 97560 "RL NB RecordRef"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedRecordRefFindFirst_GroupOfTheModifiedRow()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        G := Lib.NewKey();
        Lib.Seed(A, G, 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('recref', G, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'RecordRef FindFirst');
    end;
}

// Modifies the row it is run on.
codeunit 97562 "RL NB Writer"
{
    TableNo = "RL Row";

    trigger OnRun()
    begin
        Rec.Description := 'second';
        Rec.Modify();
    end;
}

codeunit 97563 "RL NB CodeunitRun"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedGet_RowACodeunitRunModified()
    var
        Rows: TestPage "RL Rows";
        Row: Record "RL Row";
        Parameters: Dictionary of [Text, Text];
        A: Code[20];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Row.Get(A);
        Codeunit.Run(Codeunit::"RL NB Writer", Row);
        Rows.OpenView();
        Parameters := Lib.Parameters('get', A, 'yes');
        Assert.AreEqual('error: NDBCS:LockTimeout', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'Get after a Codeunit.Run wrote the row');
    end;
}

// Shapes that keep main's answer: a table the test codeunit did not write, and a read under ReadUncommitted.
codeunit 97561 "RL NB Kept"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";

    [Test]
    procedure ReadCommittedGet_TableTheCodeunitDidNotWrite()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('captioned', A, 'yes');
        Assert.AreEqual('-', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'Get of another table');
    end;

    [Test]
    procedure ReadCommittedCount_TableTheCodeunitDidNotWrite()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, A, 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('keyedcount', A, 'yes');
        Assert.AreEqual('0', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'Count of another table');
    end;

    [Test]
    procedure ReadUncommittedFindSet_GroupOfTheModifiedRow()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        G := Lib.NewKey();
        Lib.Seed(A, G, 'first');
        Lib.Change(A, 'second');
        Rows.OpenView();
        Parameters := Lib.Parameters('rufindset', G, 'yes');
        Assert.AreEqual('second', Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false)), 'ReadUncommitted FindSet');
    end;
}

// A worker that the test codeunit's OnRun starts, before its first test, when the runner has not begun noting the rows the
// codeunit writes: a ReadCommitted Get of a table nobody wrote keeps main's answer (not found); after OnRun wrote a row of the
// table it refuses.
codeunit 97570 "RL NB OnRun Untouched"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";
        Seen: Text;

    trigger OnRun()
    var
        Rows: TestPage "RL Rows";
        Parameters: Dictionary of [Text, Text];
    begin
        Rows.OpenView();
        Parameters := Lib.Parameters('captioned', Lib.NewKey(), 'yes');
        Seen := Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false));
    end;

    [Test]
    procedure ReadCommittedGet_FromOnRun_TableNobodyWrote()
    begin
        Assert.AreEqual('-', Seen, 'Get from OnRun of a table nobody wrote');
    end;
}

codeunit 97571 "RL NB OnRun Written"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL NB Lib";
        Seen: Text;

    trigger OnRun()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        Parameters: Dictionary of [Text, Text];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Rows.OpenView();
        Parameters := Lib.Parameters('get', A, 'yes');
        Seen := Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Neighbor Reader", Parameters, false));
    end;

    [Test]
    procedure ReadCommittedGet_FromOnRun_RowOnRunInserted()
    begin
        Assert.AreEqual('error: NDBCS:LockTimeout', Seen, 'Get from OnRun of the row OnRun inserted');
    end;
}

// A row the test session's insert queue still holds, read under RepeatableRead: refused (corpus PR 1498 measures ReadCommitted,
// :583 and :606, al-runner pull request 8498). Its id comes before RL NB OnRun Written, whose refusal leaves the later
// codeunits not run.
codeunit 97538 "RL NB Queued RepeatableRead"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL More Lib";

    [Test]
    procedure QueuedInsert_RepeatableReadGet()
    var
        Host: TestPage "RL Task Host";
        Parameters: Dictionary of [Text, Text];
        A: Code[20];
    begin
        A := Lib.NewKey();
        Lib.Queue(A, '', 'queued');
        Host.OpenView();
        Parameters := Lib.Parameters('repeatableread', 'get', A);
        Assert.AreEqual('-', Lib.Seen(Host.RunPageBackgroundTask(Codeunit::"RL Reader", Parameters, false)), 'QueuedInsert_RepeatableReadGet');
    end;
}

// A row the queue still holds next to a row the test inserted and committed: the table holds the codeunit's lock on the
// committed row, so the ReadCommitted Get of the queued row is refused (no cell has both).
codeunit 97539 "RL NB Queued And Committed"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL More Lib";

    [Test]
    procedure QueuedInsert_NextToACommittedRow_ReadCommittedGet()
    var
        Host: TestPage "RL Task Host";
        Parameters: Dictionary of [Text, Text];
        A: Code[20];
    begin
        Lib.Queue(Lib.NewKey(), '', 'committed');
        Commit();
        A := Lib.NewKey();
        Lib.Queue(A, '', 'queued');
        Host.OpenView();
        Parameters := Lib.Parameters('readcommitted', 'get', A);
        Assert.AreEqual('-', Lib.Seen(Host.RunPageBackgroundTask(Codeunit::"RL Reader", Parameters, false)), 'QueuedInsert_NextToACommittedRow_ReadCommittedGet');
    end;
}
