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
}
