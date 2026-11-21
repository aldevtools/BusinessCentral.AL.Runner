// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-readisolation-property
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
// Scope: in-scope
// local object: the reads of a page background task's worker under a ReadIsolation, and the rows the test session wrote without
//   committing, are what is under test
// Fixtures used: shared Assert (60021); own table 97540, page 97541, worker codeunit 97542, test codeunit 97543
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
