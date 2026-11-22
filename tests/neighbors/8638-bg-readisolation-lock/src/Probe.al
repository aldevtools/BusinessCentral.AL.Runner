// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-readisolation-property
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
// Scope: in-scope
// local object: the reads of a page background task's worker under a ReadIsolation, and the rows the test session wrote without
//   committing, are what is under test
// Fixtures used: shared Assert (60021); own tables 97540 and 97564, pages 97541 and 97565, worker codeunit 97542, test codeunits 97543 and 97566-97568, helper codeunit 97569
// Probe (al-runner issue 8638): a page background task's worker runs as another session. The worker reads table 97540 with
//   TestPage.RunPageBackgroundTask, under one ReadIsolation (or none), by Get, by FindFirst on a group or by Count of a group,
//   inside a TryFunction, and returns what it read or the error it caught (its text, each line break written back as \, and its code).
//   Every test uses keys of its own (a new Guid); the test inserts and commits a row, then changes it with ModifyAll, so the test
//   session does not read the row before its change.
// Readings: what the worker's read answered, or the text and code of its error.
// Cells: a row the test codeunit wrote stays locked to the worker for the whole codeunit: a Get under ReadCommitted,
//   RepeatableRead or UpdLock of a row the test modified, of a row it only inserted and committed, and of a row it modified and
//   committed, and of a row it inserted without committing (ReadCommitted), waits on the lock and fails with NDBCS:LockTimeout; so do FindFirst and Count under ReadCommitted of a group with
//   a filter on a field outside the key, also of a group that holds no row (the read scans the table). A Get under ReadCommitted
//   of a key no row has answers not found at once. Under ReadUncommitted and without a ReadIsolation the Get sees the change, and
//   reads the row the test inserted and committed.
// BC versions: 29.1 (local W1 container).

table 97540 "RL Row"
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
    }
}

page 97541 "RL Rows"
{
    PageType = List;
    SourceTable = "RL Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field("Group"; Rec."Group") { }
                field(Description; Rec.Description) { }
            }
        }
    }
}

// The worker: one read of RL Row, under the ReadIsolation the parameter names; an error is caught and returned.
codeunit 97542 "RL Reader"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        if not TryRead(Parameters, Seen) then
            Seen := 'error: ' + OneLine(GetLastErrorText()) + ' | ' + GetLastErrorCode();
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;

    // The error text with each line break (CR LF in a worker) written back as the \ it was in the message, the form
    // GetLastErrorText gives in the test session (session-system/isolated-storage/IsolatedStorageInBackgroundTask.al:326).
    local procedure OneLine(Value: Text): Text
    var
        CR: Char;
        LF: Char;
    begin
        CR := 13;
        LF := 10;
        exit(Value.Replace(Format(CR) + Format(LF), '\'));
    end;

    [TryFunction]
    local procedure TryRead(Parameters: Dictionary of [Text, Text]; var Seen: Text)
    var
        Row: Record "RL Row";
    begin
        case Parameters.Get('Isolation') of
            'readcommitted':
                Row.ReadIsolation := IsolationLevel::ReadCommitted;
            'repeatableread':
                Row.ReadIsolation := IsolationLevel::RepeatableRead;
            'updlock':
                Row.ReadIsolation := IsolationLevel::UpdLock;
            'readuncommitted':
                Row.ReadIsolation := IsolationLevel::ReadUncommitted;
        end;
        case Parameters.Get('Method') of
            'get':
                if Row.Get(Parameters.Get('Key')) then
                    Seen := Row.Description
                else
                    Seen := '-';
            'first':
                begin
                    Row.SetRange("Group", Parameters.Get('Key'));
                    if Row.FindFirst() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'count':
                begin
                    Row.SetRange("Group", Parameters.Get('Key'));
                    Seen := Format(Row.Count());
                end;
        end;
    end;
}

codeunit 97543 "RL Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        LockText: Label 'error: We can''t save your changes right now, because a record in table ''RL Row'' is being updated in a transaction done by another session.\\You''ll have to wait until the other transaction has completed, which may take a while. Please try again later. | NDBCS:LockTimeout', Locked = true;

    local procedure NewKey(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    // Inserts a row, commits it.
    local procedure Seed(No: Code[20]; Group: Code[20]; Description: Text[50])
    var
        Row: Record "RL Row";
    begin
        Row."No." := No;
        Row."Group" := Group;
        Row.Description := Description;
        Row.Insert();
        Commit();
    end;

    // Changes the row without reading it first (ModifyAll), and does not commit.
    local procedure Change(No: Code[20]; Description: Text[50])
    var
        Row: Record "RL Row";
    begin
        Row.SetRange("No.", No);
        Row.ModifyAll(Description, Description);
    end;

    local procedure Run(var Rows: TestPage "RL Rows"; Isolation: Text; Method: Text; KeyText: Text): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Isolation', Isolation);
        Parameters.Add('Method', Method);
        Parameters.Add('Key', KeyText);
        Results := Rows.RunPageBackgroundTask(Codeunit::"RL Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadCommittedGet_RowTheTestModified_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Change(A, 'second');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'readcommitted', 'get', A), 'ReadCommittedGet_RowTheTestModified');
    end;

    [Test]
    procedure BgTaskReadIsolation_RepeatableReadGet_RowTheTestModified_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Change(A, 'second');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'repeatableread', 'get', A), 'RepeatableReadGet_RowTheTestModified');
    end;

    [Test]
    procedure BgTaskReadIsolation_UpdLockGet_RowTheTestModified_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Change(A, 'second');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'updlock', 'get', A), 'UpdLockGet_RowTheTestModified');
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadUncommittedGet_RowTheTestModified_SeesTheChange()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Change(A, 'second');
        Rows.OpenView();
        Assert.AreEqual('second', Run(Rows, 'readuncommitted', 'get', A), 'ReadUncommittedGet_RowTheTestModified');
    end;

    [Test]
    procedure BgTaskReadIsolation_DefaultGet_RowTheTestModified_SeesTheChange()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Change(A, 'second');
        Rows.OpenView();
        Assert.AreEqual('second', Run(Rows, 'default', 'get', A), 'DefaultGet_RowTheTestModified');
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadCommittedGet_RowTheTestInsertedAndCommitted_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'readcommitted', 'get', A), 'ReadCommittedGet_RowTheTestInsertedAndCommitted');
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadCommittedGet_RowTheTestModifiedAndCommitted_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Change(A, 'second');
        Commit();
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'readcommitted', 'get', A), 'ReadCommittedGet_RowTheTestModifiedAndCommitted');
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadCommittedGet_MissingKey_NotFound()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Rows.OpenView();
        Assert.AreEqual('-', Run(Rows, 'readcommitted', 'get', NewKey()), 'ReadCommittedGet_MissingKey');
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadCommittedFindFirst_GroupOfTheModifiedRow_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, G, 'first');
        Change(A, 'second');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'readcommitted', 'first', G), 'ReadCommittedFindFirst_GroupOfTheModifiedRow');
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadCommittedCount_GroupOfTheModifiedRow_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, G, 'first');
        Change(A, 'second');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'readcommitted', 'count', G), 'ReadCommittedCount_GroupOfTheModifiedRow');
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadCommittedCount_EmptyGroup_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, G, 'first');
        Change(A, 'second');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'readcommitted', 'count', NewKey()), 'ReadCommittedCount_EmptyGroup');
    end;

    [Test]
    procedure BgTaskReadIsolation_DefaultGet_RowTheTestInsertedAndCommitted_ReadsTheRow()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
        G: Code[20];
    begin
        A := NewKey();
        G := NewKey();
        Seed(A, '', 'first');
        Rows.OpenView();
        Assert.AreEqual('first', Run(Rows, 'default', 'get', A), 'DefaultGet_RowTheTestInsertedAndCommitted');
    end;
    [Test]
    procedure BgTaskReadIsolation_ReadCommittedGet_RowTheTestInsertedNotCommitted_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        Row: Record "RL Row";
        A: Code[20];
    begin
        A := NewKey();
        Row."No." := A;
        Row.Description := 'new';
        Row.Insert();
        Row.Get(A);
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'readcommitted', 'get', A), 'ReadCommittedGet_RowTheTestInsertedNotCommitted');
    end;

    [Test]
    procedure BgTaskReadIsolation_RepeatableReadGet_RowTheTestInsertedAndCommitted_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
    begin
        A := NewKey();
        Seed(A, '', 'first');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'repeatableread', 'get', A), 'RepeatableReadGet_RowTheTestInsertedAndCommitted');
    end;

    [Test]
    procedure BgTaskReadIsolation_UpdLockGet_RowTheTestInsertedAndCommitted_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        A: Code[20];
    begin
        A := NewKey();
        Seed(A, '', 'first');
        Rows.OpenView();
        Assert.AreEqual(LockText, Run(Rows, 'updlock', 'get', A), 'UpdLockGet_RowTheTestInsertedAndCommitted');
    end;
}

// More cells, in test codeunits of their own:
// - a worker that the page's own code starts (CurrPage.EnqueueBackgroundTask in OnAfterGetCurrRecord of RL Task Host): its
//   ReadCommitted Get of a row the test modified waits on the lock as the worker of TestPage.RunPageBackgroundTask does;
// - a row an earlier test of the codeunit inserted and committed: a later test's worker meets its lock too;
// - a row the test session's insert queue still holds (a statement-form Insert into RL Row, which has no Blob or AutoIncrement
//   field, with no read of RL Row and no commit after it), in a codeunit whose RL Row holds no row of another test: the worker
//   runs from a host page on another table (RL Host), whose reads do not send the queue, and reads RL Row under ReadCommitted by
//   Count of the row's group, or by Get of its key.

table 97564 "RL Host"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Isolation; Text[30]) { }
        field(3; Method; Text[30]) { }
        field(4; "Read Key"; Code[20]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// A row with a Read Key makes the page's own code start the worker with that row's parameters; the page shows its answer.
page 97565 "RL Task Host"
{
    PageType = Card;
    SourceTable = "RL Host";
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
        Parameters: Dictionary of [Text, Text];
        TaskId: Integer;
    begin
        if Rec."Read Key" = '' then
            exit;
        Parameters.Add('Isolation', Rec.Isolation);
        Parameters.Add('Method', Rec.Method);
        Parameters.Add('Key', Rec."Read Key");
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"RL Reader", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        Seen := Results.Get('Seen');
    end;

    var
        Seen: Text;
}

// What the test codeunits 97566-97568 share.
codeunit 97569 "RL More Lib"
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

    // Changes the row without reading it first (ModifyAll), and does not commit.
    procedure Change(No: Code[20]; Description: Text[50])
    var
        Row: Record "RL Row";
    begin
        Row.SetRange("No.", No);
        Row.ModifyAll(Description, Description);
    end;

    // Inserts a row by the Insert statement, with no read and no commit after it: the insert queue holds it.
    procedure Queue(No: Code[20]; Group: Code[20]; Description: Text[50])
    var
        Row: Record "RL Row";
    begin
        Row."No." := No;
        Row."Group" := Group;
        Row.Description := Description;
        Row.Insert();
    end;

    procedure Parameters(Isolation: Text; Method: Text; KeyText: Text): Dictionary of [Text, Text]
    var
        Result: Dictionary of [Text, Text];
    begin
        Result.Add('Isolation', Isolation);
        Result.Add('Method', Method);
        Result.Add('Key', KeyText);
        exit(Result);
    end;

    procedure Seen(Results: Dictionary of [Text, Text]): Text
    var
        Value: Text;
    begin
        Results.Get('Seen', Value);
        exit(Value);
    end;

    procedure LockText(): Text
    begin
        exit('error: We can''t save your changes right now, because a record in table ''RL Row'' is being updated in a transaction done by another session.\\You''ll have to wait until the other transaction has completed, which may take a while. Please try again later. | NDBCS:LockTimeout');
    end;
}

codeunit 97566 "RL More Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL More Lib";
        EarlierKey: Code[20];

    [Test]
    procedure BgTaskReadIsolation_PageEnqueued_ReadCommittedGet_RowTheTestModified_WaitsOnTheLock()
    var
        Host: TestPage "RL Task Host";
        HostRow: Record "RL Host";
        A: Code[20];
    begin
        A := Lib.NewKey();
        Lib.Seed(A, '', 'first');
        Lib.Change(A, 'second');
        HostRow."No." := Lib.NewKey();
        HostRow.Isolation := 'readcommitted';
        HostRow.Method := 'get';
        HostRow."Read Key" := A;
        HostRow.Insert();
        Host.OpenView();
        Host.GoToKey(HostRow."No.");
        Assert.AreEqual(Lib.LockText(), Host.Seen.Value(), 'PageEnqueued_ReadCommittedGet_RowTheTestModified');
    end;

    [Test]
    procedure BgTaskReadIsolation_EarlierTest_InsertsAndCommitsARow()
    begin
        EarlierKey := Lib.NewKey();
        Lib.Seed(EarlierKey, '', 'earlier');
        Assert.IsTrue(EarlierKey <> '', 'EarlierTest_InsertsAndCommitsARow');
    end;

    [Test]
    procedure BgTaskReadIsolation_ReadCommittedGet_RowAnEarlierTestInsertedAndCommitted_WaitsOnTheLock()
    var
        Rows: TestPage "RL Rows";
        Parameters: Dictionary of [Text, Text];
    begin
        Rows.OpenView();
        Parameters := Lib.Parameters('readcommitted', 'get', EarlierKey);
        Assert.AreEqual(Lib.LockText(), Lib.Seen(Rows.RunPageBackgroundTask(Codeunit::"RL Reader", Parameters, false)), 'ReadCommittedGet_RowAnEarlierTestInsertedAndCommitted');
    end;
}

codeunit 97567 "RL Queue Count Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL More Lib";

    [Test]
    procedure BgTaskReadIsolation_QueuedInsert_OtherHostPage_ReadCommittedCount_LeavesItOut()
    var
        Host: TestPage "RL Task Host";
        Parameters: Dictionary of [Text, Text];
        G: Code[20];
    begin
        G := Lib.NewKey();
        Lib.Queue(Lib.NewKey(), G, 'queued');
        Host.OpenView();
        Parameters := Lib.Parameters('readcommitted', 'count', G);
        Assert.AreEqual('0', Lib.Seen(Host.RunPageBackgroundTask(Codeunit::"RL Reader", Parameters, false)), 'QueuedInsert_OtherHostPage_ReadCommittedCount');
    end;
}

codeunit 97568 "RL Queue Get Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Lib: Codeunit "RL More Lib";

    [Test]
    procedure BgTaskReadIsolation_QueuedInsert_OtherHostPage_ReadCommittedGet_NotFound()
    var
        Host: TestPage "RL Task Host";
        Parameters: Dictionary of [Text, Text];
        A: Code[20];
    begin
        A := Lib.NewKey();
        Lib.Queue(A, '', 'queued');
        Host.OpenView();
        Parameters := Lib.Parameters('readcommitted', 'get', A);
        Assert.AreEqual('-', Lib.Seen(Host.RunPageBackgroundTask(Codeunit::"RL Reader", Parameters, false)), 'QueuedInsert_OtherHostPage_ReadCommittedGet');
    end;
}
