// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-enqueuebackgroundtask-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
// Scope: in-scope
// local object: the reads of a page background task's worker and of the test session, and what the test writes, commits and
//   rolls back between them, are what is under test
// Fixtures used: shared Assert (60021); own tables 97500, 97501, 97514; mode codeunit 97502; codeunits 97503, 97504, 97509, 97517,
//   97590; pages 97505, 97518, 97519; test codeunits 97506, 97507, 97508, 97544, 97545 (neighbors after 97507: see comments)
// Probe (al-runner issue 8467): a read the worker of a page background task repeats gets the answer of the first read, also after
//   the test session changed the rows without committing. The tests read lines of table 97500 (key Group, Line No.) in the worker,
//   either from the page's OnAfterGetCurrRecord (CurrPage.EnqueueBackgroundTask, the page shows the answer) or with
//   TestPage.RunPageBackgroundTask, and change, commit and roll back the lines between the reads. Every test uses a group of its
//   own (a new Guid), except the AutoRollback pairs, which use a fixed key and never commit it.
// Readings: what each worker read answered; what the page shows; what the test session's own Get answers.
// Cells:
//   - the same read again after the test changed the row, not committed: Get (:367, :566, after a Delete :604), FindFirst on both
//     key fields (:389, :586; Get and it answer alike), FindFirst and Count on the group (:641, :658, :675), a Get that found
//     nothing and an empty group (:624, :692), another worker codeunit and SetLoadFields (:566), a first read with SetLoadFields
//     and a later full read (:714), the task of a new page (:477);
//     FindFirst and Count with a SetRange on a field outside the key (:870, :891; a commit of the table ends it), and Count with that filter
//     set by SetRange to the value twice, by SetFilter of a range from the value to itself and by FieldRef.SetRange (:917);
//   - a first read: it sees the test's change (:411, :522), unless the test session read the row before its change while it had
//     not written the table since its last commit (:500, also when it had written another table :541, and when the change was to
//     another row :938); a read the test session made after it wrote the table does not count (:959);
//   - a commit: of the change of the row, the next read sees it (:430, ModifyAll :804); of another table (:451), or of another row of the
//     table (Modify :741, FindFirst on both key fields :762, an insert :784), the next read does not see a later
//     change of the row; of any change of the table, the next FindFirst and Count on a group see it (:823, another group :845);
//   - a rollback: asserterror (:983) and the end of an AutoRollback test (:1016, :1029) do not change what the next read answers;
//     the test session's own Get answers like a worker read then, until it writes the table (:983);
//   - another test codeunit: its read sees its own row (:1044, :1067).
// Not pinned here: whether a commit of a ModifyAll of another row of the table ends a row's entry. BC 27.5, 28.5 and 29.0 end
//   it, a local BC 29.1 container kept it.
// BC versions: 29.1 (local W1 container).

table 97500 "BQ Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "Group"; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Description; Text[50]) { }
        field(4; Note; Text[50]) { }
    }

    keys
    {
        key(PK; "Group", "Line No.") { Clustered = true; }
    }
}

table 97501 "BQ Other"
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

// The read the page's background task makes: set by the test, read by the page in the test session.
codeunit 97502 "BQ Mode"
{
    SingleInstance = true;

    var
        Mode: Text;

    procedure Set(NewMode: Text)
    begin
        Mode := NewMode;
    end;

    procedure GetMode(): Text
    begin
        exit(Mode);
    end;
}

// The worker: one read of BQ Row, chosen by the Mode parameter.
codeunit 97503 "BQ Reader"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        RowRef: RecordRef;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        LineText: Text;
        LineNo: Integer;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Line', LineText);
        Evaluate(LineNo, LineText);
        case Parameters.Get('Mode') of
            'get':
                if Row.Get(Group, LineNo) then
                    Seen := Row.Description
                else
                    Seen := '-';
            'getlf':
                begin
                    Row.SetLoadFields(Description);
                    if Row.Get(Group, LineNo) then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'getboth':
                if Row.Get(Group, LineNo) then
                    Seen := Row.Description + '+' + Row.Note
                else
                    Seen := '-';
            'getlfboth':
                begin
                    Row.SetLoadFields(Description);
                    if Row.Get(Group, LineNo) then
                        Seen := Row.Description + '+' + Row.Note
                    else
                        Seen := '-';
                end;
            'first':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange("Line No.", LineNo);
                    if Row.FindFirst() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'groupfirst':
                begin
                    Row.SetRange("Group", Group);
                    if Row.FindFirst() then
                        Seen := Format(Row."Line No.") + '/' + Row.Description
                    else
                        Seen := '-';
                end;
            'count':
                begin
                    Row.SetRange("Group", Group);
                    Seen := Format(Row.Count());
                end;
            'groupdesccount':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Description, Parameters.Get('Desc'));
                    Seen := Format(Row.Count());
                end;
            'rangecount':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Description, Parameters.Get('Desc'), Parameters.Get('Desc'));
                    Seen := Format(Row.Count());
                end;
            'filtercount':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetFilter(Description, '%1..%1', Parameters.Get('Desc'));
                    Seen := Format(Row.Count());
                end;
            'refcount':
                begin
                    RowRef.Open(Database::"BQ Row");
                    RowRef.Field(Row.FieldNo("Group")).SetRange(Group);
                    RowRef.Field(Row.FieldNo(Description)).SetRange(Parameters.Get('Desc'));
                    Seen := Format(RowRef.Count());
                end;
            'descfirst':
                begin
                    Row.SetRange(Description, Parameters.Get('Desc'));
                    if Row.FindFirst() then
                        Seen := Row."Group" + '/' + Format(Row."Line No.") + '/' + Row.Note
                    else
                        Seen := '-';
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

// A second worker codeunit with the same Get.
codeunit 97504 "BQ Reader 2"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        LineText: Text;
        LineNo: Integer;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Line', LineText);
        Evaluate(LineNo, LineText);
        if Row.Get(Group, LineNo) then
            Results.Add('Seen', Row.Description)
        else
            Results.Add('Seen', '-');
        Page.SetBackgroundTaskResult(Results);
    end;
}

page 97505 "BQ Rows"
{
    PageType = List;
    SourceTable = "BQ Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Group"; Rec."Group") { }
                field("Line No."; Rec."Line No.") { }
                field(Description; Rec.Description) { }
                field(Seen; Seen)
                {
                    Editable = false;
                }
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        BqMode: Codeunit "BQ Mode";
        Parameters: Dictionary of [Text, Text];
        TaskId: Integer;
    begin
        Seen := '';
        if BqMode.GetMode() = '' then
            exit;
        Parameters.Add('Group', Rec."Group");
        Parameters.Add('Line', Format(Rec."Line No."));
        Parameters.Add('Mode', BqMode.GetMode());
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"BQ Reader", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        Seen := CopyStr(Results.Get('Seen'), 1, MaxStrLen(Seen));
    end;

    var
        Seen: Text[50];
}

codeunit 97506 "BQ Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        BqMode: Codeunit "BQ Mode";

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    // Inserts a line, then reads the group: a read of the table by the test session after its insert, so the insert reaches the
    // worker's session (without a read in between, a worker read right after the insert does not see it).
    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Insert();
        Flush(Group);
    end;

    local procedure Flush(Group: Code[20])
    var
        Row: Record "BQ Row";
    begin
        Row.SetRange("Group", Group);
        if Row.FindFirst() then;
    end;

    // Changes the line after reading it (Get, Modify).
    local procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Modify();
    end;

    // Changes the line without reading it (ModifyAll).
    local procedure WriteOnly(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.SetRange("Group", Group);
        Row.SetRange("Line No.", LineNo);
        Row.ModifyAll(Description, Description);
    end;

    local procedure OpenOn(var Rows: TestPage "BQ Rows"; Group: Code[20])
    begin
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", Group);
        Rows.First();
    end;

    // The worker run by the test on an open page, without its completion trigger.
    local procedure Run(var Rows: TestPage "BQ Rows"; Worker: Integer; Mode: Text; Group: Code[20]; LineNo: Integer): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Line', Format(LineNo));
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Worker, Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // As Run, with the Description the worker filters on.
    local procedure RunDesc(var Rows: TestPage "BQ Rows"; Mode: Text; Group: Code[20]; Description: Text): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Line', '0');
        Parameters.Add('Mode', Mode);
        Parameters.Add('Desc', Description);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure Reader(): Integer
    begin
        exit(Codeunit::"BQ Reader");
    end;

    // The page's task reads line 1 by Get; the test changes it; the page moves to line 2 and back: the page shows the change,
    // the task answers what it read first.
    [Test]
    procedure BgQueryCache_Page_Get_ChangedRow_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('get');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Rows.Seen.Value(), 'first read of line 1');
        SetRow(G, 1, 'second');
        Rows.Next();
        Assert.AreEqual('other', Rows.Seen.Value(), 'line 2');
        Rows.Previous();
        Assert.AreEqual('second', Rows.Description.Value(), 'line 1 on the page');
        Assert.AreEqual('first', Rows.Seen.Value(), 'second read of line 1');
        Rows.Close();
    end;

    // As above, the task reads by SetRange on both key fields and FindFirst.
    [Test]
    procedure BgQueryCache_Page_FullKeyFindFirst_ChangedRow_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('first');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Rows.Seen.Value(), 'first read of line 1');
        SetRow(G, 1, 'second');
        Rows.Next();
        Assert.AreEqual('other', Rows.Seen.Value(), 'line 2');
        Rows.Previous();
        Assert.AreEqual('second', Rows.Description.Value(), 'line 1 on the page');
        Assert.AreEqual('first', Rows.Seen.Value(), 'second read of line 1');
        Rows.Close();
    end;

    // A line the test changed before the task first read it: the task sees the change.
    [Test]
    procedure BgQueryCache_Page_FirstRead_SeesTheChange()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('get');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Rows.Seen.Value(), 'line 1');
        SetRow(G, 2, 'other2');
        Rows.Next();
        Assert.AreEqual('other2', Rows.Seen.Value(), 'first read of line 2');
        Rows.Close();
    end;

    // The test commits its change of line 1 before the task reads it again: the task sees it.
    [Test]
    procedure BgQueryCache_Page_Get_ChangeCommitted_SeesTheChange()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('get');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Rows.Seen.Value(), 'first read of line 1');
        SetRow(G, 1, 'second');
        Commit();
        Rows.Next();
        Rows.Previous();
        Assert.AreEqual('second', Rows.Seen.Value(), 'second read of line 1');
        Rows.Close();
    end;

    // After the first read, the test commits a write of another table, then changes line 1 without reading it.
    [Test]
    procedure BgQueryCache_Page_Get_OtherTableCommitted_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        Other: Record "BQ Other";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('get');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Rows.Seen.Value(), 'first read of line 1');
        Other."No." := G;
        Other.Insert();
        Commit();
        WriteOnly(G, 1, 'second');
        Rows.Next();
        Rows.Previous();
        Assert.AreEqual('second', Rows.Description.Value(), 'line 1 on the page');
        Assert.AreEqual('first', Rows.Seen.Value(), 'second read of line 1');
        Rows.Close();
    end;

    // The page closes; the test changes line 1; a new page's task reads it.
    [Test]
    procedure BgQueryCache_Page_Get_NewPage_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        Rows2: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('get');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Rows.Seen.Value(), 'first read of line 1');
        Rows.Close();
        SetRow(G, 1, 'second');
        OpenOn(Rows2, G);
        Assert.AreEqual('second', Rows2.Description.Value(), 'line 1 on the new page');
        Assert.AreEqual('first', Rows2.Seen.Value(), 'read of line 1 by the new page');
        Rows2.Close();
    end;

    // Committed lines; the test reads line 1 itself (it has not written the table since the commit), then changes it: the
    // task's first read answers what the test read.
    [Test]
    procedure BgQueryCache_Page_TestReadFirst_FirstReadAnswersIt()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('get');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        Commit();
        Row.Get(G, 1);
        Row.Description := 'second';
        Row.Modify();
        OpenOn(Rows, G);
        Assert.AreEqual('second', Rows.Description.Value(), 'line 1 on the page');
        Assert.AreEqual('first', Rows.Seen.Value(), 'first read of line 1');
        Rows.Close();
    end;

    // As above, the test changes line 1 without reading it (ModifyAll): the task sees the change.
    [Test]
    procedure BgQueryCache_Page_TestWriteOnly_FirstReadSeesIt()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('get');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        Commit();
        WriteOnly(G, 1, 'second');
        OpenOn(Rows, G);
        Assert.AreEqual('second', Rows.Seen.Value(), 'first read of line 1');
        Rows.Close();
    end;

    // The test writes another table (not committed), then reads line 1 and changes it: the task's first read answers what the
    // test read.
    [Test]
    procedure BgQueryCache_Page_OtherTableWritten_TestReadFirst_FirstReadAnswersIt()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        Other: Record "BQ Other";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('get');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        Commit();
        Other."No." := G;
        Other.Insert();
        Row.Get(G, 1);
        Row.Description := 'second';
        Row.Modify();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Rows.Seen.Value(), 'first read of line 1');
        Rows.Close();
    end;

    // The test runs the worker itself: Get, change, then Get, FindFirst on both key fields, the other worker's Get and a Get with
    // SetLoadFields all answer the first read.
    [Test]
    procedure BgQueryCache_Run_Get_ChangedRow_EveryFormFirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get');
        SetRow(G, 1, 'second');
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get again');
        Assert.AreEqual('first', Run(Rows, Reader(), 'first', G, 1), 'FindFirst on both key fields');
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader 2", 'get', G, 1), 'Get of the other worker');
        Assert.AreEqual('first', Run(Rows, Reader(), 'getlf', G, 1), 'Get with SetLoadFields');
        Rows.Close();
    end;

    // FindFirst on both key fields first, change, then Get and FindFirst.
    [Test]
    procedure BgQueryCache_Run_FullKeyFindFirst_ChangedRow_GetFirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Reader(), 'first', G, 1), 'FindFirst');
        SetRow(G, 1, 'second');
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get');
        Assert.AreEqual('first', Run(Rows, Reader(), 'first', G, 1), 'FindFirst again');
        Rows.Close();
    end;

    // Get, the test deletes the line, Get: found.
    [Test]
    procedure BgQueryCache_Run_Get_DeletedRow_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get');
        Row.Get(G, 1);
        Row.Delete();
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get after the delete');
        Rows.Close();
    end;

    // Get of a missing line, the test inserts it, Get: not found.
    [Test]
    procedure BgQueryCache_Run_GetMissing_InsertedRow_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 2, 'other');
        OpenOn(Rows, G);
        Assert.AreEqual('-', Run(Rows, Reader(), 'get', G, 1), 'Get');
        AddRow(G, 1, 'new');
        Assert.AreEqual('-', Run(Rows, Reader(), 'get', G, 1), 'Get after the insert');
        Rows.Close();
    end;

    // FindFirst on the group, the test inserts a line that sorts first, FindFirst: the first line read first.
    [Test]
    procedure BgQueryCache_Run_GroupFindFirst_InsertBefore_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 2, 'b');
        OpenOn(Rows, G);
        Assert.AreEqual('2/b', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst');
        AddRow(G, 1, 'a');
        Assert.AreEqual('2/b', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst after the insert');
        Rows.Close();
    end;

    // FindFirst on the group, the test changes the line it found, FindFirst: the line as read first.
    [Test]
    procedure BgQueryCache_Run_GroupFindFirst_ChangedRow_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'a');
        OpenOn(Rows, G);
        Assert.AreEqual('1/a', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst');
        SetRow(G, 1, 'a2');
        Assert.AreEqual('1/a', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst after the change');
        Rows.Close();
    end;

    // Count on the group, the test inserts, Count.
    [Test]
    procedure BgQueryCache_Run_GroupCount_Insert_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'a');
        OpenOn(Rows, G);
        Assert.AreEqual('1', Run(Rows, Reader(), 'count', G, 0), 'Count');
        AddRow(G, 2, 'b');
        Assert.AreEqual('1', Run(Rows, Reader(), 'count', G, 0), 'Count after the insert');
        Rows.Close();
    end;

    // FindFirst and Count on an empty group, the test inserts into it: nothing found, 0.
    [Test]
    procedure BgQueryCache_Run_EmptyGroup_Insert_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
        H: Code[20];
    begin
        G := NewGroup();
        H := NewGroup();
        BqMode.Set('');
        AddRow(H, 1, 'h');
        OpenOn(Rows, H);
        Assert.AreEqual('-', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst');
        Assert.AreEqual('0', Run(Rows, Reader(), 'count', G, 0), 'Count');
        AddRow(G, 1, 'g');
        Assert.AreEqual('-', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst after the insert');
        Assert.AreEqual('0', Run(Rows, Reader(), 'count', G, 0), 'Count after the insert');
        Rows.Close();
    end;

    // The first read loads Description only; the test changes Description and Note; a full Get and a partial Get answer the
    // line as it was, Note too.
    [Test]
    procedure BgQueryCache_Run_PartialFirst_FullLater_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        Row."Group" := G;
        Row."Line No." := 1;
        Row.Description := 'first';
        Row.Note := 'n1';
        Row.Insert();
        Flush(G);
        OpenOn(Rows, G);
        Assert.AreEqual('first+n1', Run(Rows, Reader(), 'getlfboth', G, 1), 'partial Get');
        Row.Description := 'second';
        Row.Note := 'n2';
        Row.Modify();
        Assert.AreEqual('first+n1', Run(Rows, Reader(), 'getboth', G, 1), 'full Get');
        Assert.AreEqual('first+n1', Run(Rows, Reader(), 'getlfboth', G, 1), 'partial Get again');
        Rows.Close();
    end;

    // Committed lines 1 and 2; Get of line 1; the test changes line 2 (Get and Modify) and commits, then changes line 1
    // without reading it: Get answers the first read.
    [Test]
    procedure BgQueryCache_Run_Get_OtherRowModifyCommitted_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get');
        SetRow(G, 2, 'other2');
        Commit();
        WriteOnly(G, 1, 'second');
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get after the commit');
        Rows.Close();
    end;

    // As above, the reads by FindFirst on both key fields.
    [Test]
    procedure BgQueryCache_Run_FullKeyFindFirst_OtherRowCommitted_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Reader(), 'first', G, 1), 'FindFirst');
        SetRow(G, 2, 'other2');
        Commit();
        WriteOnly(G, 1, 'second');
        Assert.AreEqual('first', Run(Rows, Reader(), 'first', G, 1), 'FindFirst after the commit');
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get after the commit');
        Rows.Close();
    end;

    // Get of line 1; the test inserts line 3 and commits, then changes line 1 without reading it: Get answers the first read.
    [Test]
    procedure BgQueryCache_Run_Get_InsertCommitted_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get');
        AddRow(G, 3, 'third');
        Commit();
        WriteOnly(G, 1, 'second');
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get after the commit');
        Rows.Close();
    end;

    // Get of line 1; the test changes it by ModifyAll and commits: Get sees the change.
    [Test]
    procedure BgQueryCache_Run_Get_ModifyAllCommitted_SeesTheChange()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'Get');
        WriteOnly(G, 1, 'second');
        Commit();
        Assert.AreEqual('second', Run(Rows, Reader(), 'get', G, 1), 'Get after the commit');
        Rows.Close();
    end;

    // Count and FindFirst on the group; the test inserts a line into the group and commits: both see it.
    [Test]
    procedure BgQueryCache_Run_GroupReads_InsertCommitted_SeeTheChange()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 2, 'b');
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('1', Run(Rows, Reader(), 'count', G, 0), 'Count');
        Assert.AreEqual('2/b', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst');
        AddRow(G, 1, 'a');
        Commit();
        Assert.AreEqual('2', Run(Rows, Reader(), 'count', G, 0), 'Count after the commit');
        Assert.AreEqual('1/a', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst after the commit');
        Rows.Close();
    end;

    // Count and FindFirst on group G; the test inserts into another group and commits, then inserts into G without committing:
    // both see the insert into G.
    [Test]
    procedure BgQueryCache_Run_GroupReads_OtherGroupCommitted_SeeTheChange()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
        H: Code[20];
    begin
        G := NewGroup();
        H := NewGroup();
        BqMode.Set('');
        AddRow(G, 2, 'b');
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('1', Run(Rows, Reader(), 'count', G, 0), 'Count');
        Assert.AreEqual('2/b', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst');
        AddRow(H, 1, 'h');
        Commit();
        AddRow(G, 1, 'a');
        Assert.AreEqual('2', Run(Rows, Reader(), 'count', G, 0), 'Count after the commit');
        Assert.AreEqual('1/a', Run(Rows, Reader(), 'groupfirst', G, 0), 'FindFirst after the commit');
        Rows.Close();
    end;

    // Count with a SetRange on the group and on Description (not a key field); the test inserts a matching line: the first
    // answer. Then the test commits: Count sees it.
    [Test]
    procedure BgQueryCache_Run_NonKeyFilterCount_Insert_FirstAnswerUntilCommit()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'x');
        AddRow(G, 2, 'y');
        OpenOn(Rows, G);
        Assert.AreEqual('1', RunDesc(Rows, 'groupdesccount', G, 'x'), 'Count');
        AddRow(G, 3, 'x');
        Assert.AreEqual('1', RunDesc(Rows, 'groupdesccount', G, 'x'), 'Count after the insert');
        Commit();
        Assert.AreEqual('2', RunDesc(Rows, 'groupdesccount', G, 'x'), 'Count after the commit');
        Rows.Close();
    end;

    // FindFirst with a SetRange on Description only; the test changes the line it found and inserts a line with that
    // Description that sorts first: the line as read first. Then the test commits: FindFirst sees the new first line.
    [Test]
    procedure BgQueryCache_Run_NonKeyFilterFindFirst_FirstAnswerUntilCommit()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        G: Code[20];
        Desc: Text[50];
    begin
        G := NewGroup();
        Desc := CopyStr('d' + G, 1, 50);
        BqMode.Set('');
        AddRow(G, 2, Desc);
        OpenOn(Rows, G);
        Assert.AreEqual(G + '/2/', RunDesc(Rows, 'descfirst', G, Desc), 'FindFirst');
        Row.Get(G, 2);
        Row.Note := 'n';
        Row.Modify();
        AddRow(G, 1, Desc);
        Assert.AreEqual(G + '/2/', RunDesc(Rows, 'descfirst', G, Desc), 'FindFirst after the changes');
        Commit();
        Assert.AreEqual(G + '/1/', RunDesc(Rows, 'descfirst', G, Desc), 'FindFirst after the commit');
        Rows.Close();
    end;

    // Count with the Description filter set three other ways: SetRange to the same value twice, SetFilter of a range from a
    // value to itself, and FieldRef.SetRange on a RecordRef; the test inserts a matching line: each answers its first answer.
    [Test]
    procedure BgQueryCache_Run_OtherFilterForms_Insert_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'x');
        OpenOn(Rows, G);
        Assert.AreEqual('1', RunDesc(Rows, 'rangecount', G, 'x'), 'SetRange from and to');
        Assert.AreEqual('1', RunDesc(Rows, 'filtercount', G, 'x'), 'SetFilter range');
        Assert.AreEqual('1', RunDesc(Rows, 'refcount', G, 'x'), 'FieldRef.SetRange');
        AddRow(G, 2, 'x');
        Assert.AreEqual('1', RunDesc(Rows, 'rangecount', G, 'x'), 'SetRange from and to after the insert');
        Assert.AreEqual('1', RunDesc(Rows, 'filtercount', G, 'x'), 'SetFilter range after the insert');
        Assert.AreEqual('1', RunDesc(Rows, 'refcount', G, 'x'), 'FieldRef.SetRange after the insert');
        Rows.Close();
    end;

    // Committed lines; the test reads line 1, then changes line 2: the task's first read of line 1 answers line 1.
    [Test]
    procedure BgQueryCache_Run_TestReadFirst_OtherRowChanged_FirstRead()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        Commit();
        Row.Get(G, 1);
        WriteOnly(G, 2, 'other2');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Reader(), 'get', G, 1), 'first read of line 1');
        Rows.Close();
    end;

    // Committed lines; the test changes line 2, then reads line 1, then changes line 1 without reading it: the task's first read
    // sees the change (the test's read came after it wrote the table).
    [Test]
    procedure BgQueryCache_Run_TableWritten_TestRead_FirstReadSeesTheChange()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        BqMode.Set('');
        AddRow(G, 1, 'first');
        AddRow(G, 2, 'other');
        Commit();
        WriteOnly(G, 2, 'other2');
        Row.Get(G, 1);
        Assert.AreEqual('first', Row.Description, 'the test''s read');
        WriteOnly(G, 1, 'second');
        OpenOn(Rows, G);
        Assert.AreEqual('second', Run(Rows, Reader(), 'get', G, 1), 'first read of line 1');
        Rows.Close();
    end;

    // Inside asserterror the test inserts a line and the task reads it, then an error rolls the insert back. The task's Get
    // still finds it, and so does the test's own Get (it has not written the table since the rollback) until the test writes
    // the table.
    [Test]
    procedure BgQueryCache_AssertError_Rollback_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
        G: Code[20];
        H: Code[20];
    begin
        G := NewGroup();
        H := NewGroup();
        BqMode.Set('');
        AddRow(H, 1, 'h');
        Commit();
        OpenOn(Rows, H);
        asserterror begin
            AddRow(G, 1, 'gone');
            Assert.AreEqual('gone', Run(Rows, Reader(), 'get', G, 1), 'Get of the inserted line');
            Error('roll back');
        end;
        Assert.AreEqual('gone', Run(Rows, Reader(), 'get', G, 1), 'Get after the rollback');
        Assert.IsTrue(Row.Get(G, 1), 'the test''s Get after the rollback');
        Assert.AreEqual('gone', Row.Description, 'the line the test''s Get found');
        Row.Reset();
        Row.SetRange("Group", G);
        Assert.AreEqual(0, Row.Count(), 'the group holds no line');
        AddRow(H, 2, 'h2');
        Assert.IsFalse(Row.Get(G, 1), 'the test''s Get after it wrote the table');
        Rows.Close();
    end;

    // An AutoRollback test inserts a fixed key and the task reads it; the test passes, its insert is rolled back. The next test
    // inserts the same key with another Description: the task answers the first.
    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure BgQueryCache_AutoRollback_1_ReadsTheKey()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddRow('BQZ', 1, 'z1');
        OpenOn(Rows, 'BQZ');
        Assert.AreEqual('z1', Run(Rows, Reader(), 'get', 'BQZ', 1), 'Get');
        Rows.Close();
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure BgQueryCache_AutoRollback_2_SameKey_FirstAnswer()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddRow('BQZ', 1, 'z2');
        OpenOn(Rows, 'BQZ');
        Assert.AreEqual('z1', Run(Rows, Reader(), 'get', 'BQZ', 1), 'Get of the key the previous test read');
        Rows.Close();
    end;

    // The first test of a pair whose second test is in codeunit 97507: it inserts a fixed key and the task reads it, and its
    // insert is rolled back.
    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure BgQueryCache_OtherCodeunit_1_ReadsTheKey()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddRow('BQW', 1, 'w1');
        OpenOn(Rows, 'BQW');
        Assert.AreEqual('w1', Run(Rows, Reader(), 'get', 'BQW', 1), 'Get');
        Rows.Close();
    end;
}

codeunit 97507 "BQ Tests 2"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    // In another test codeunit, the test inserts the key BgQueryCache_OtherCodeunit_1_ReadsTheKey read: the task sees this
    // test's row.
    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure BgQueryCache_OtherCodeunit_2_SameKey_SeesItsRow()
    var
        Row: Record "BQ Row";
        Rows: TestPage "BQ Rows";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Row."Group" := 'BQW';
        Row."Line No." := 1;
        Row.Description := 'w2';
        Row.Insert();
        Row.SetRange("Group", 'BQW');
        if Row.FindFirst() then;
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", 'BQW');
        Rows.First();
        Parameters.Add('Group', 'BQW');
        Parameters.Add('Line', '1');
        Parameters.Add('Mode', 'get');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        Rows.Close();
        Assert.AreEqual('w2', Seen, 'Get of the key the other codeunit read');
    end;
}

// Neighbors of the cells above (al-runner issue 8467, review of al-runner PR 8490): a row's entry after commits that wrote the
// row while no commit held it before or after; the test session's group read before it wrote the table; a copy of a record a
// partial Get answered from the cache; and read forms the cells above do not use (FindLast, IsEmpty, FindFirst after a
// SetFilter) repeated after the test changed the rows. Measured on a local BC 29.1 OnPrem w1 container.
codeunit 97509 "BQ Reader 3"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Copy: Record "BQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        case Parameters.Get('Mode') of
            'last':
                begin
                    Row.SetRange("Group", Group);
                    if Row.FindLast() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'empty':
                begin
                    Row.SetRange("Group", Group);
                    Seen := Format(Row.IsEmpty());
                end;
            'setfilter':
                begin
                    Row.SetFilter("Group", Group);
                    if Row.FindFirst() then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'copynote':
                begin
                    Row.SetLoadFields(Description);
                    Row.Get(Group, 1);
                    Copy := Row;
                    Seen := Copy.Description + '+' + Copy.Note;
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97508 "BQ Tests 3"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        // No BQ Mode (a SingleInstance of BQ Tests): page 97519 starts no task of its own.

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    // Inserts a line, then reads the group (as AddRow of BQ Tests).
    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50]; Note: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Note := Note;
        Row.Insert();
        Row.SetRange("Group", Group);
        if Row.FindFirst() then;
    end;

    local procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50]; Note: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Note := Note;
        Row.Modify();
    end;

    local procedure OpenOn(var Rows: TestPage "BQ Plain Rows"; Group: Code[20])
    begin
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", Group);
        Rows.First();
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Worker: Integer; Mode: Text; Group: Code[20]; LineNo: Integer): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Line', Format(LineNo));
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Worker, Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // A Get inside an asserterror finds the line the test inserted; the error rolls it back. The test inserts the line again
    // and commits, deletes it and commits: the same Get then sees the table.
    [Test]
    procedure BgQueryCache_Run_Get_RolledBack_InsertDeleteCommitted_SeesTheTable()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
        H: Code[20];
        InRollback: Text;
    begin
        G := NewGroup();
        H := NewGroup();
        // Page 97519 starts no task.
        AddRow(H, 1, 'h', '');
        Commit();
        OpenOn(Rows, H);
        asserterror begin
            AddRow(G, 1, 'gone', '');
            InRollback := Run(Rows, Codeunit::"BQ Reader", 'get', G, 1);
            Error('roll back');
        end;
        Assert.AreEqual('gone', InRollback, 'Get in the rolled-back transaction');
        AddRow(G, 1, 'back', '');
        Commit();
        Row.Get(G, 1);
        Row.Delete();
        Commit();
        Assert.AreEqual('-', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'Get after the commits');
        Rows.Close();
    end;

    // The test session counts the group while it has not written the table since its last commit, then inserts a line of it:
    // the worker's first Count of the group.
    [Test]
    procedure BgQueryCache_Run_GroupCount_TestCountThenInsert_FirstRead()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        // Page 97519 starts no task.
        AddRow(G, 1, 'a', '');
        Commit();
        OpenOn(Rows, G);
        Row.SetRange("Group", G);
        Assert.AreEqual(1, Row.Count(), 'the test''s Count');
        AddRow(G, 2, 'b', '');
        Assert.AreEqual('1', Run(Rows, Codeunit::"BQ Reader", 'count', G, 0), 'the worker''s Count');
        Rows.Close();
    end;

    // A Get of line 1; the test changes it; a Get with SetLoadFields(Description) and a copy of that record: the fields of the copy.
    [Test]
    procedure BgQueryCache_Run_PartialGet_Copy_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        // Page 97519 starts no task.
        AddRow(G, 1, 'first', 'n1');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'Get');
        SetRow(G, 1, 'second', 'n9');
        Assert.AreEqual('first+n1', Run(Rows, Codeunit::"BQ Reader 3", 'copynote', G, 1), 'the copy');
        Rows.Close();
    end;

    // A FindLast on the group; the test changes the line: the same FindLast again.
    [Test]
    procedure BgQueryCache_Run_FindLast_ChangedRow_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        // Page 97519 starts no task.
        AddRow(G, 1, 'a', '');
        OpenOn(Rows, G);
        Assert.AreEqual('a', Run(Rows, Codeunit::"BQ Reader 3", 'last', G, 0), 'FindLast');
        SetRow(G, 1, 'b', '');
        Assert.AreEqual('a', Run(Rows, Codeunit::"BQ Reader 3", 'last', G, 0), 'FindLast again');
        Rows.Close();
    end;

    // IsEmpty on an empty group; the test inserts a line of it: the same IsEmpty again.
    [Test]
    procedure BgQueryCache_Run_IsEmpty_Insert_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        E: Code[20];
    begin
        G := NewGroup();
        E := NewGroup();
        // Page 97519 starts no task.
        AddRow(G, 1, 'a', '');
        OpenOn(Rows, G);
        Assert.AreEqual('Yes', Run(Rows, Codeunit::"BQ Reader 3", 'empty', E, 0), 'IsEmpty');
        AddRow(E, 1, 'c', '');
        Assert.AreEqual('Yes', Run(Rows, Codeunit::"BQ Reader 3", 'empty', E, 0), 'IsEmpty again');
        Rows.Close();
    end;

    // FindFirst after SetFilter on the group to one value; the test changes the line: the same read again.
    [Test]
    procedure BgQueryCache_Run_SetFilterFindFirst_ChangedRow_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        // Page 97519 starts no task.
        AddRow(G, 1, 'a', '');
        OpenOn(Rows, G);
        Assert.AreEqual('a', Run(Rows, Codeunit::"BQ Reader 3", 'setfilter', G, 0), 'FindFirst');
        SetRow(G, 1, 'b', '');
        Assert.AreEqual('a', Run(Rows, Codeunit::"BQ Reader 3", 'setfilter', G, 0), 'FindFirst again');
        Rows.Close();
    end;
}

// The page of BQ Tests 3: page 97505 without its OnAfterGetCurrRecord task, so the tests need no BQ Mode.
page 97519 "BQ Plain Rows"
{
    PageType = List;
    SourceTable = "BQ Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Group"; Rec."Group") { }
                field("Line No."; Rec."Line No.") { }
                field(Description; Rec.Description) { }
            }
        }
    }
}

// Neighbors of the FindLast, IsEmpty and SetFilter cells of BQ Tests 3 (al-runner issue 8637; measured on a local BC 29.1
// OnPrem w1 container): which read shapes the query cache answers. The InsertAfterFirstRead cells read FindFirst, FindLast,
// Count and IsEmpty of an empty set in one worker run, insert a line the filters keep, and read the same four again: the cache
// answers all four with the first answers for every filter shape here (two, three and four filters, a filter outside the key
// alone, filter groups 2, 4 and 6, a range, a descending sort, a RecordRef, SetFilter of one value, of a placeholder, of two
// values, of a wildcard with and without @, the other comparison operators, several of these at once, a table whose key has
// one field). A FindFirst by SetRange and one by SetFilter of the same value share their answer, as do a Get and a FindFirst
// by SetFilter on every key field. The test session's own FindLast and IsEmpty before its change answer the worker's first
// read, as :500 measures for Get; one after its change does not, as :959 measures for Get. A commit of the table ends a
// FindLast's answer. Find('+'), Find('-'), FindSet and FindFirst or FindLast after SetLoadFields read the rows as they are, as
// does the worker's first read after the test page read the group through its filter.

table 97514 "BQ Single"
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

// The worker of BQ Tests 4: FindFirst, FindLast, Count and IsEmpty of the filters a Mode names, or the one read a Mode names.
codeunit 97590 "BQ Reader 4"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Single: Record "BQ Single";
        RowRef: RecordRef;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        Seen := '-';
        case Mode of
            'ref':
                begin
                    RowRef.Open(Database::"BQ Row");
                    RowRef.Field(Row.FieldNo("Group")).SetRange(Group);
                    Seen := RefForms(RowRef);
                end;
            'single':
                begin
                    Single.SetRange("No.", Group);
                    Seen := SingleForms(Single);
                end;
            'singlefilter':
                begin
                    Single.SetFilter("No.", Group);
                    Seen := SingleForms(Single);
                end;
            'singleget':
                if Single.Get(Group) then
                    Seen := Single.Description;
            'singlefilterfirst':
                begin
                    Single.SetFilter("No.", Group);
                    if Single.FindFirst() then
                        Seen := Single.Description;
                end;
            'get':
                if Row.Get(Group, 1) then
                    Seen := Row.Description;
            'keyfilterfirst':
                begin
                    Row.SetFilter("Group", Group);
                    Row.SetFilter("Line No.", '1');
                    if Row.FindFirst() then
                        Seen := Row.Description;
                end;
            'rangefirst':
                begin
                    Row.SetRange("Group", Group);
                    if Row.FindFirst() then
                        Seen := Row.Description;
                end;
            'setfilterfirst':
                begin
                    Row.SetFilter("Group", Group);
                    if Row.FindFirst() then
                        Seen := Row.Description;
                end;
            'last':
                begin
                    Row.SetRange("Group", Group);
                    if Row.FindLast() then
                        Seen := Row.Description;
                end;
            'empty':
                begin
                    Row.SetRange("Group", Group);
                    Seen := Format(Row.IsEmpty());
                end;
            'findplus':
                begin
                    Row.SetRange("Group", Group);
                    if Row.Find('+') then
                        Seen := Row.Description;
                end;
            'findminus':
                begin
                    Row.SetRange("Group", Group);
                    if Row.Find('-') then
                        Seen := Row.Description;
                end;
            'findset':
                begin
                    Row.SetRange("Group", Group);
                    if Row.FindSet() then
                        Seen := Row.Description;
                end;
            'firstload':
                begin
                    Row.SetLoadFields(Description);
                    Row.SetRange("Group", Group);
                    if Row.FindFirst() then
                        Seen := Row.Description;
                end;
            'lastload':
                begin
                    Row.SetLoadFields(Description);
                    Row.SetRange("Group", Group);
                    if Row.FindLast() then
                        Seen := Row.Description;
                end;
            else begin
                SetShape(Row, Mode, Group);
                Seen := Forms(Row);
            end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;

    // The filters of a Mode; any other Mode is a filter on Description next to the group's.
    local procedure SetShape(var Row: Record "BQ Row"; Mode: Text; Group: Text)
    begin
        case Mode of
            'group':
                Row.SetRange("Group", Group);
            'second':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                end;
            'nonkey':
                Row.SetRange(Note, Group);
            'group2':
                begin
                    Row.FilterGroup(2);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                end;
            'group4':
                begin
                    Row.FilterGroup(4);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                end;
            'range':
                Row.SetRange("Group", Group, Group + 'Z');
            'desc':
                begin
                    Row.Ascending(false);
                    Row.SetRange("Group", Group);
                end;
            'setfilter':
                Row.SetFilter("Group", Group);
            'holder':
                Row.SetFilter("Group", '%1', Group);
            'or':
                Row.SetFilter("Group", '%1|%2', Group, 'NOGROUP');
            'like':
                Row.SetFilter("Group", Group + '*');
            'likeci':
                Row.SetFilter("Group", '@' + LowerCase(Group) + '*');
            'setfiltersecond':
                begin
                    Row.SetFilter("Group", Group);
                    Row.SetRange(Note, '');
                end;
            'three':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                    Row.FilterGroup(2);
                    Row.SetFilter(Description, '<>%1', 'zz');
                    Row.FilterGroup(0);
                end;
            'four':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                    Row.SetFilter(Description, '<>%1', 'zz');
                    Row.SetRange("Line No.", 1, 9);
                end;
            'group6':
                begin
                    Row.FilterGroup(6);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                end;
            'combined':
                begin
                    Row.Ascending(false);
                    Row.FilterGroup(2);
                    Row.SetFilter("Group", '%1|%2', Group, 'NOGROUP');
                    Row.FilterGroup(0);
                    Row.SetFilter(Description, '>%1', '0');
                end;
            else begin
                Row.SetRange("Group", Group);
                Row.SetFilter(Description, Mode);
            end;
        end;
    end;

    local procedure Forms(var Row: Record "BQ Row"): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Row.FindFirst() then
            First := Row.Description;
        if Row.FindLast() then
            Last := Row.Description;
        exit(First + '|' + Last + '|' + Format(Row.Count()) + '|' + Format(Row.IsEmpty()));
    end;

    local procedure RefForms(var RowRef: RecordRef): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if RowRef.FindFirst() then
            First := Format(RowRef.Field(3).Value());
        if RowRef.FindLast() then
            Last := Format(RowRef.Field(3).Value());
        exit(First + '|' + Last + '|' + Format(RowRef.Count()) + '|' + Format(RowRef.IsEmpty()));
    end;

    local procedure SingleForms(var Single: Record "BQ Single"): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Single.FindFirst() then
            First := Single.Description;
        if Single.FindLast() then
            Last := Single.Description;
        exit(First + '|' + Last + '|' + Format(Single.Count()) + '|' + Format(Single.IsEmpty()));
    end;
}

codeunit 97544 "BQ Tests 4"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    // Shorter than the other codeunits' groups, so a range to the group plus a letter fits the field.
    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    // Inserts a line, then reads the group (as AddRow of BQ Tests).
    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50]; Note: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Note := Note;
        Row.Insert();
        Row.SetRange("Group", Group);
        if Row.FindFirst() then;
    end;

    // Inserts a row of BQ Single, then reads it.
    local procedure AddSingle(No: Code[20]; Description: Text[50])
    var
        Single: Record "BQ Single";
    begin
        Single."No." := No;
        Single.Description := Description;
        Single.Insert();
        Single.SetRange("No.", No);
        if Single.FindFirst() then;
    end;

    local procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Modify();
    end;

    local procedure SetSingle(No: Code[20]; Description: Text[50])
    var
        Single: Record "BQ Single";
    begin
        Single.Get(No);
        Single.Description := Description;
        Single.Modify();
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader 4", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The four reads of a Mode on an empty set; the test inserts a line the filters keep (Target: 'row', 'note' with the group
    // as its Note, or 'single'); the four reads again.
    local procedure InsertAfterFirstRead(Mode: Text; Target: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Rows.OpenView();
        Assert.AreEqual('-|-|0|Yes', Run(Rows, Mode, G), Mode + ': the first reads');
        case Target of
            'row':
                AddRow(G, 1, 'a', '');
            'note':
                AddRow(G, 1, 'a', G);
            'single':
                AddSingle(G, 'a');
        end;
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_Group_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_SecondFilter_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('second', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_NonKeyFilter_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('nonkey', 'note'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterGroup2_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group2', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterGroup4_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group4', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_Range_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('range', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_Descending_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('desc', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_RecordRef_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('ref', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_SetFilterValue_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('setfilter', 'row'), 'after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('holder', 'row'), 'placeholder, after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_SetFilterTwoValues_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('or', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_SetFilterWildcard_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('like', 'row'), 'after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('likeci', 'row'), '@, after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_SetFilterSecondFilter_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('setfiltersecond', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_ThreeFilters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('three', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FourFilters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('four', 'row'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterGroup6_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group6', 'row'), 'after the insert');
    end;

    // Filter group 2, a SetFilter of two values, a second filter with an operator and a descending sort at once.
    [Test]
    procedure BgQueryCache_Run_AllForms_Combined_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('combined', 'row'), 'after the insert');
    end;

    // The group's filter and one on Description with each other operator.
    [Test]
    procedure BgQueryCache_Run_AllForms_Operators_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('<>zz', 'row'), '<>');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('>0', 'row'), '>');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('>=a', 'row'), '>=');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('<z', 'row'), '<');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('<=a', 'row'), '<=');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('a..', 'row'), 'a..');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('..z', 'row'), '..z');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('>0&<z', 'row'), '&');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('<>z*', 'row'), '<>z*');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_OneFieldKey_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('single', 'single'), 'SetRange, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('singlefilter', 'single'), 'SetFilter, after the insert');
    end;

    // A FindFirst by SetRange of the group; the test changes the line: a FindFirst by SetFilter of the group to the same value.
    [Test]
    procedure BgQueryCache_Run_SetRangeThenSetFilter_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'rangefirst', G), 'SetRange FindFirst');
        SetRow(G, 1, 'b');
        Assert.AreEqual('a', Run(Rows, 'setfilterfirst', G), 'SetFilter FindFirst');
        Rows.Close();
    end;

    // The same the other way round.
    [Test]
    procedure BgQueryCache_Run_SetFilterThenSetRange_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'setfilterfirst', G), 'SetFilter FindFirst');
        SetRow(G, 1, 'b');
        Assert.AreEqual('a', Run(Rows, 'rangefirst', G), 'SetRange FindFirst');
        Rows.Close();
    end;

    // A Get of line 1; the test changes it: a FindFirst by SetFilter on both key fields.
    [Test]
    procedure BgQueryCache_Run_GetThenKeySetFilter_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'get', G), 'Get');
        SetRow(G, 1, 'b');
        Assert.AreEqual('a', Run(Rows, 'keyfilterfirst', G), 'SetFilter FindFirst');
        Rows.Close();
    end;

    // A Get of a row of a table whose key has one field; the test changes it: a FindFirst by SetFilter on the key.
    [Test]
    procedure BgQueryCache_Run_OneFieldKey_GetThenSetFilter_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddSingle(G, 'a');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'singleget', G), 'Get');
        SetSingle(G, 'b');
        Assert.AreEqual('a', Run(Rows, 'singlefilterfirst', G), 'SetFilter FindFirst');
        Rows.Close();
    end;

    // The test session's FindLast of the group after a commit; it changes the line: the worker's first FindLast.
    [Test]
    procedure BgQueryCache_Run_TestFindLastFirst_FirstReadAnswersIt()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Commit();
        Row.SetRange("Group", G);
        Row.FindLast();
        Assert.AreEqual('a', Row.Description, 'the test''s FindLast');
        SetRow(G, 1, 'b');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'last', G), 'the worker''s FindLast');
        Rows.Close();
    end;

    // The test session's IsEmpty of an empty group after a commit; it inserts a line of it: the worker's first IsEmpty.
    [Test]
    procedure BgQueryCache_Run_TestIsEmptyFirst_FirstReadAnswersIt()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
        E: Code[20];
    begin
        G := NewGroup();
        E := NewGroup();
        AddRow(G, 1, 'a', '');
        Commit();
        Row.SetRange("Group", E);
        Assert.IsTrue(Row.IsEmpty(), 'the test''s IsEmpty');
        AddRow(E, 1, 'c', '');
        Rows.OpenView();
        Assert.AreEqual('Yes', Run(Rows, 'empty', E), 'the worker''s IsEmpty');
        Rows.Close();
    end;

    // The worker's FindLast; the test changes the line and commits: the worker's FindLast again.
    [Test]
    procedure BgQueryCache_Run_FindLast_ChangeCommitted_SeesTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'last', G), 'FindLast');
        SetRow(G, 1, 'b');
        Commit();
        Assert.AreEqual('b', Run(Rows, 'last', G), 'FindLast after the commit');
        Rows.Close();
    end;

    // The worker's FindLast; the test changes the line and commits: the test session's own FindLast.
    [Test]
    procedure BgQueryCache_Run_FindLast_ChangeCommitted_TestReadSeesIt()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'last', G), 'FindLast');
        SetRow(G, 1, 'b');
        Commit();
        Row.SetRange("Group", G);
        Row.FindLast();
        Assert.AreEqual('b', Row.Description, 'the test''s FindLast');
        Rows.Close();
    end;

    // The worker's FindLast; the test changes the line, not committed: the test session's own FindLast.
    [Test]
    procedure BgQueryCache_Run_FindLast_TestWrote_TestReadSeesIt()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'last', G), 'FindLast');
        SetRow(G, 1, 'b');
        Row.SetRange("Group", G);
        Row.FindLast();
        Assert.AreEqual('b', Row.Description, 'the test''s FindLast');
        Rows.Close();
    end;

    // Two lines in the group; the test changes the last one: the worker's FindLast again.
    [Test]
    procedure BgQueryCache_Run_FindLast_TwoRows_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        AddRow(G, 2, 'x', '');
        Rows.OpenView();
        Assert.AreEqual('x', Run(Rows, 'last', G), 'FindLast');
        SetRow(G, 2, 'y');
        Assert.AreEqual('x', Run(Rows, 'last', G), 'FindLast again');
        Rows.Close();
    end;

    // The worker's FindLast; a commit of a line of another group; FindLast again, then the test changes the line: FindLast.
    [Test]
    procedure BgQueryCache_Run_FindLast_OtherGroupCommitted_ThenFirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'last', G), 'FindLast');
        AddRow(NewGroup(), 1, 'h', '');
        Commit();
        Assert.AreEqual('a', Run(Rows, 'last', G), 'FindLast after the commit');
        SetRow(G, 1, 'b');
        Assert.AreEqual('a', Run(Rows, 'last', G), 'FindLast after the change');
        Rows.Close();
    end;

    // Find('+') of the group; the test changes the line: Find('+') again.
    [Test]
    procedure BgQueryCache_Run_FindPlus_ChangedRow_SeesTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'findplus', G), 'Find(+)');
        SetRow(G, 1, 'b');
        Assert.AreEqual('b', Run(Rows, 'findplus', G), 'Find(+) again');
        Rows.Close();
    end;

    // Find('-') of the group; the test changes the line: Find('-') again.
    [Test]
    procedure BgQueryCache_Run_FindMinus_ChangedRow_SeesTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'findminus', G), 'Find(-)');
        SetRow(G, 1, 'b');
        Assert.AreEqual('b', Run(Rows, 'findminus', G), 'Find(-) again');
        Rows.Close();
    end;

    // FindSet of the group; the test changes the line: FindSet again.
    [Test]
    procedure BgQueryCache_Run_FindSet_ChangedRow_SeesTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'findset', G), 'FindSet');
        SetRow(G, 1, 'b');
        Assert.AreEqual('b', Run(Rows, 'findset', G), 'FindSet again');
        Rows.Close();
    end;

    // FindFirst after SetLoadFields(Description); the test changes the line: the same read again.
    [Test]
    procedure BgQueryCache_Run_FindFirstLoadFields_ChangedRow_SeesTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'firstload', G), 'FindFirst');
        SetRow(G, 1, 'b');
        Assert.AreEqual('b', Run(Rows, 'firstload', G), 'FindFirst again');
        Rows.Close();
    end;

    // The test changes the line after a commit and reads the group by FindLast itself: the worker's first FindLast.
    [Test]
    procedure BgQueryCache_Run_TestWroteThenFindLast_FirstReadSeesTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Commit();
        SetRow(G, 1, 'b');
        Row.SetRange("Group", G);
        Row.FindLast();
        Assert.AreEqual('b', Row.Description, 'the test''s FindLast');
        Rows.OpenView();
        Assert.AreEqual('b', Run(Rows, 'last', G), 'the worker''s FindLast');
        Rows.Close();
    end;

    // FindLast after SetLoadFields(Description); the test changes the line: the same read again.
    [Test]
    procedure BgQueryCache_Run_FindLastLoadFields_ChangedRow_SeesTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Assert.AreEqual('a', Run(Rows, 'lastload', G), 'FindLast');
        SetRow(G, 1, 'b');
        Assert.AreEqual('b', Run(Rows, 'lastload', G), 'FindLast again');
        Rows.Close();
    end;

    // The test page shows the group through its filter; the test changes the line: the worker's first FindFirst by SetFilter.
    [Test]
    procedure BgQueryCache_Run_PageFilter_ChangedRow_FirstReadSeesTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", G);
        Rows.First();
        SetRow(G, 1, 'b');
        Assert.AreEqual('b', Run(Rows, 'setfilterfirst', G), 'SetFilter FindFirst');
        Rows.Close();
    end;
}

// Neighbors of BQ Tests 4 (al-runner issue 8637, review of the runner change; measured on a local BC 29.1 OnPrem w1 container):
// - a test page's own read of a row: the page shows one line through filters on every key field, the test changes it without
//   reading it, and a worker's first Get of the line answers what the page read, on table 97500 and on table 97514 (a key of
//   one field), and through a filter on the group alone (the test had not written the table since its last commit);
// - the test session's FindFirst, FindLast, Count and IsEmpty of a set after a commit and before its change, then the
//   worker's first four reads of the same set, for SetRange of the group, SetFilter of it to one value, a range, filter group
//   2, a descending sort and a RecordRef.

page 97518 "BQ Single Rows"
{
    PageType = List;
    SourceTable = "BQ Single";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field(Description; Rec.Description) { }
            }
        }
    }
}

// The test session's FindFirst, FindLast, Count and IsEmpty of the filters a Mode names, set as BQ Reader 4 sets them.
codeunit 97517 "BQ Test Reads"
{
    procedure Forms(Mode: Text; Group: Code[20]): Text
    var
        Row: Record "BQ Row";
        RowRef: RecordRef;
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Mode = 'ref' then begin
            RowRef.Open(Database::"BQ Row");
            RowRef.Field(Row.FieldNo("Group")).SetRange(Group);
            if RowRef.FindFirst() then
                First := Format(RowRef.Field(3).Value());
            if RowRef.FindLast() then
                Last := Format(RowRef.Field(3).Value());
            exit(First + '|' + Last + '|' + Format(RowRef.Count()) + '|' + Format(RowRef.IsEmpty()));
        end;
        case Mode of
            'group':
                Row.SetRange("Group", Group);
            'setfilter':
                Row.SetFilter("Group", Group);
            'range':
                Row.SetRange("Group", Group, Group + 'Z');
            'group2':
                begin
                    Row.FilterGroup(2);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                end;
            'desc':
                begin
                    Row.Ascending(false);
                    Row.SetRange("Group", Group);
                end;
        end;
        if Row.FindFirst() then
            First := Row.Description;
        if Row.FindLast() then
            Last := Row.Description;
        exit(First + '|' + Last + '|' + Format(Row.Count()) + '|' + Format(Row.IsEmpty()));
    end;
}

codeunit 97545 "BQ Tests 5"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    // Inserts a line, then reads the group (as AddRow of BQ Tests).
    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Insert();
        Row.SetRange("Group", Group);
        if Row.FindFirst() then;
    end;

    local procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Modify();
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader 4", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The test session reads the four forms of a Mode after a commit; it changes line 1 and inserts line 2: the worker's
    // first four reads of the Mode.
    local procedure TestReadsFirst(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        TestReads: Codeunit "BQ Test Reads";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        AddRow(G, 1, 'a');
        Commit();
        Assert.AreEqual('a|a|1|No', TestReads.Forms(Mode, G), Mode + ': the test''s reads');
        SetRow(G, 1, 'b');
        AddRow(G, 2, 'c');
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // The test page shows line 1 through filters on both key fields; the test changes it without reading it (ModifyAll): the
    // worker's first Get.
    [Test]
    procedure BgQueryCache_Run_PageKeyFilter_WriteOnly_GetAnswersThePageRead()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a');
        Commit();
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", G);
        Rows.Filter.SetFilter("Line No.", '1');
        Rows.First();
        Row.SetRange("Group", G);
        Row.SetRange("Line No.", 1);
        Row.ModifyAll(Description, 'b');
        Assert.AreEqual('a', Run(Rows, 'get', G), 'the worker''s Get');
        Rows.Close();
    end;

    // The same on a table whose key has one field.
    [Test]
    procedure BgQueryCache_Run_OneFieldKey_PageKeyFilter_WriteOnly_GetAnswersThePageRead()
    var
        Singles: TestPage "BQ Single Rows";
        Single: Record "BQ Single";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Single."No." := G;
        Single.Description := 'a';
        Single.Insert();
        Single.SetRange("No.", G);
        if Single.FindFirst() then;
        Commit();
        Singles.OpenView();
        Singles.Filter.SetFilter("No.", G);
        Singles.First();
        Single.Reset();
        Single.SetRange("No.", G);
        Single.ModifyAll(Description, 'b');
        Parameters.Add('Group', G);
        Parameters.Add('Mode', 'singleget');
        Results := Singles.RunPageBackgroundTask(Codeunit::"BQ Reader 4", Parameters, false);
        Results.Get('Seen', Seen);
        Assert.AreEqual('a', Seen, 'the worker''s Get');
        Singles.Close();
    end;

    // The test page shows the group through a filter on it alone; the test changes line 1 without reading it: the worker's
    // first Get.
    [Test]
    procedure BgQueryCache_Run_PageGroupFilter_WriteOnly_GetAnswersThePageRead()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a');
        Commit();
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", G);
        Rows.First();
        Row.SetRange("Group", G);
        Row.SetRange("Line No.", 1);
        Row.ModifyAll(Description, 'b');
        Assert.AreEqual('a', Run(Rows, 'get', G), 'the worker''s Get');
        Rows.Close();
    end;

    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_Group_FirstReadsAnswerThem()
    begin
        Assert.AreEqual('a|a|1|No', TestReadsFirst('group'), 'the worker''s reads');
    end;

    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_SetFilter_FirstReadsAnswerThem()
    begin
        Assert.AreEqual('a|a|1|No', TestReadsFirst('setfilter'), 'the worker''s reads');
    end;

    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_Range_FirstReadsAnswerThem()
    begin
        Assert.AreEqual('a|a|1|No', TestReadsFirst('range'), 'the worker''s reads');
    end;

    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_FilterGroup2_FirstReadsAnswerThem()
    begin
        Assert.AreEqual('a|a|1|No', TestReadsFirst('group2'), 'the worker''s reads');
    end;

    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_Descending_FirstReadsAnswerThem()
    begin
        Assert.AreEqual('a|a|1|No', TestReadsFirst('desc'), 'the worker''s reads');
    end;

    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_RecordRef_FirstReadsAnswerThem()
    begin
        Assert.AreEqual('a|a|1|No', TestReadsFirst('ref'), 'the worker''s reads');
    end;
}
