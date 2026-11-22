// An excerpt of the corpus probe BgTaskQueryCache.al of corpus pull requests 1457 and 1495 at 37bffb8a (al-runner issue 8467):
// the file up to BQ Tests 3 and page 97519, then BQ Tests 3b, BQ Tests 11, BQ Media Row and BQ Reader 12. Left out: the cells
// of al-runner issues 8637 and 8677 between them, and BQ Tests 12 and BQ Version, which need Base Application.
// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-enqueuebackgroundtask-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
// Scope: in-scope
// local object: the reads of a page background task's worker and of the test session, and what the test writes, commits and
//   rolls back between them, are what is under test
// Fixtures used: Assert (60021); tables 97500, 97501, 97514, 97546, 97593, 97594, 97597, 97598; mode codeunit 97502; codeunits 97503,
//   97504, 97509, 97517, 97590, 97591, 97595, 97597; pages 97505, 97518, 97519; test codeunits 97506-97508, 97544, 97545, 97592, 97596, 97598
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
// A commit of a ModifyAll or DeleteAll of another row of the table (BQ Tests 12), SelectLatestVersion and a table with a Blob
//   field (BQ Tests 11): the end of the file. GroupCount and PartialGet of BQ Tests 3 are in BQ Tests 3b.
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

// Two cells of BQ Tests 3 in a test codeunit of their own (review of al-runner PR 8490, LOW 9): the runner refuses both, and a
// refused test stops the trust in the later failures of its codeunit, so the FindLast, IsEmpty and SetFilter cells of BQ Tests 3
// now run before them there. Measured on a local BC 29.1 OnPrem w1 container.
codeunit 97577 "BQ Tests 3b"
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
}

// What ends a query cache entry besides a commit that wrote the row (al-runner issue 8467, review of al-runner PR 8490, findings 1
// to 3; measured on a local BC 29.1 OnPrem w1 container, 3 runs):
//   - Database.SelectLatestVersion in the worker, with no table or with the row's table, before its Get: the read of the row (a row
//     it read, a Get that found nothing, a row the test session read first) goes to the table, and the next Get answers what it
//     read; with another table it changes nothing. (A Count or a FindFirst of a group after it: with the first read in BQ Reader
//     and the read after SelectLatestVersion in BQ Reader 11 they read the table, with all three reads in BQ Reader 11 they
//     answered the first read; no cell.)
//   - Database.SelectLatestVersion in the test session changes no answer of a worker read: a row, a row the test session read
//     first (before or after its change), a Count and a FindFirst of a group.
//   - A table with a Blob field, and one with a Media field (BQ Media Row and BQ Reader 12, at the end of the file): a repeated
//     Get answers the first read, as on BQ Row. (CalcFields of the Blob in a worker waits for the rows the test codeunit wrote,
//     as the locking reads of corpus PR 1498 do, so no cell reads it here.)
//   - A commit of a ModifyAll or a DeleteAll of another row of the table, then a change of the row the test does not read: BC
//     27.5, 28.5 and 29.0 end the row's entry (for ModifyAll: the corpus CI of PR 1457), BC 29.1 keeps it; BQ Version tells
//     them apart. These cells are in BQ Tests 12, so BQ Tests 11 needs no Base Application. (After a Truncate of another row
//     the local 29.1 container answered the first read when the test changed the row by Get and Modify, which the Get alone
//     answers (:500), and waited on a lock when it changed the row by ModifyAll; no cell.)
table 97575 "BQ Blob Row"
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

// The worker of BQ Tests 11: SelectLatestVersion before a Get of BQ Row, or a Get of BQ Blob Row.
codeunit 97575 "BQ Reader 11"
{
    trigger OnRun()
    var
        BlobRow: Record "BQ Blob Row";
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
            'slvget':
                begin
                    Database.SelectLatestVersion();
                    Seen := RowSeen(Group, LineNo);
                end;
            'slvtableget':
                begin
                    Database.SelectLatestVersion(Database::"BQ Row");
                    Seen := RowSeen(Group, LineNo);
                end;
            'slvotherget':
                begin
                    Database.SelectLatestVersion(Database::"BQ Other");
                    Seen := RowSeen(Group, LineNo);
                end;
            'blobdesc':
                if BlobRow.Get(Group) then
                    Seen := BlobRow.Description
                else
                    Seen := '-';
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;

    local procedure RowSeen(Group: Text; LineNo: Integer): Text
    var
        Row: Record "BQ Row";
    begin
        if Row.Get(Group, LineNo) then
            exit(Row.Description);
        exit('-');
    end;
}

codeunit 97576 "BQ Tests 11"
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

    // Changes the line after reading it (Get, Modify).
    local procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Modify();
    end;

    // Inserts a row of BQ Blob Row with its Blob, then reads the table (as AddRow).
    local procedure AddBlob(No: Code[20]; Description: Text[50]; Content: Text)
    var
        BlobRow: Record "BQ Blob Row";
    begin
        BlobRow."No." := No;
        BlobRow.Description := Description;
        WriteBlob(BlobRow, Content);
        BlobRow.Insert();
        BlobRow.SetRange("No.", No);
        if BlobRow.FindFirst() then;
    end;

    // Changes the row and its Blob after reading it (Get, Modify).
    local procedure SetBlob(No: Code[20]; Description: Text[50]; Content: Text)
    var
        BlobRow: Record "BQ Blob Row";
    begin
        BlobRow.Get(No);
        BlobRow.Description := Description;
        WriteBlob(BlobRow, Content);
        BlobRow.Modify();
    end;

    local procedure WriteBlob(var BlobRow: Record "BQ Blob Row"; Content: Text)
    var
        OutS: OutStream;
    begin
        BlobRow.Data.CreateOutStream(OutS);
        OutS.WriteText(Content);
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

    local procedure SelectInWorker(Mode: Text; AfterSelect: Text; AfterNext: Text)
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'first');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'Get');
        SetRow(G, 1, 'second');
        Assert.AreEqual(AfterSelect, Run(Rows, Codeunit::"BQ Reader 11", Mode, G, 1), 'Get after SelectLatestVersion');
        SetRow(G, 1, 'third');
        Assert.AreEqual(AfterNext, Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'the next Get');
        Rows.Close();
    end;

    // A Get of line 1; the test changes it; the worker calls SelectLatestVersion, then Get: it reads the table; the test changes
    // the line again: the next Get answers what the Get after SelectLatestVersion read.
    [Test]
    procedure BgQueryCache_Run_SelectLatestVersion_InWorker_ThenGet()
    begin
        SelectInWorker('slvget', 'second', 'second');
    end;

    // As above, SelectLatestVersion with the table of the row.
    [Test]
    procedure BgQueryCache_Run_SelectLatestVersionTable_InWorker_ThenGet()
    begin
        SelectInWorker('slvtableget', 'second', 'second');
    end;

    // As above, SelectLatestVersion with another table: every Get answers the first read.
    [Test]
    procedure BgQueryCache_Run_SelectLatestVersionOtherTable_InWorker_ThenGet()
    begin
        SelectInWorker('slvotherget', 'first', 'first');
    end;

    local procedure SelectInTest(Table: Integer)
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'first');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'Get');
        SetRow(G, 1, 'second');
        if Table = 0 then
            Database.SelectLatestVersion()
        else
            Database.SelectLatestVersion(Table);
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'Get after SelectLatestVersion');
        SetRow(G, 1, 'third');
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'the next Get');
        Rows.Close();
    end;

    // A Get of line 1; the test changes it and calls SelectLatestVersion: the worker's Get answers the first read.
    [Test]
    procedure BgQueryCache_Run_SelectLatestVersion_InTest_ThenWorkerGet()
    begin
        SelectInTest(0);
    end;

    // As above, SelectLatestVersion with the table of the row.
    [Test]
    procedure BgQueryCache_Run_SelectLatestVersionTable_InTest_ThenWorkerGet()
    begin
        SelectInTest(Database::"BQ Row");
    end;

    // The test reads line 1 after a commit and changes it; the worker calls SelectLatestVersion, then Get: it reads the table,
    // and the next Get answers what it read.
    [Test]
    procedure BgQueryCache_Run_TestReadFirst_SelectLatestVersionInWorker_ReadsTheTable()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'first');
        Commit();
        Row.Get(G, 1);
        SetRow(G, 1, 'second');
        OpenOn(Rows, G);
        Assert.AreEqual('second', Run(Rows, Codeunit::"BQ Reader 11", 'slvget', G, 1), 'Get after SelectLatestVersion');
        SetRow(G, 1, 'third');
        Assert.AreEqual('second', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'the next Get');
        Rows.Close();
    end;

    // The test reads line 1 after a commit, changes it and calls SelectLatestVersion: the worker's first Get answers what the test
    // read.
    [Test]
    procedure BgQueryCache_Run_TestReadFirst_SelectLatestVersionInTest_FirstReadAnswersIt()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'first');
        Commit();
        Row.Get(G, 1);
        SetRow(G, 1, 'second');
        Database.SelectLatestVersion();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'first Get');
        Rows.Close();
    end;

    // As above, SelectLatestVersion before the change.
    [Test]
    procedure BgQueryCache_Run_TestReadFirst_SelectLatestVersionInTestBeforeChange_FirstReadAnswersIt()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'first');
        Commit();
        Row.Get(G, 1);
        Database.SelectLatestVersion();
        SetRow(G, 1, 'second');
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'first Get');
        Rows.Close();
    end;

    // A Get of a line that is not there; the test inserts it; the worker calls SelectLatestVersion, then Get: it finds the line.
    [Test]
    procedure BgQueryCache_Run_GetMissing_SelectLatestVersionInWorker_SeesTheInsert()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        H: Code[20];
    begin
        G := NewGroup();
        H := NewGroup();
        AddRow(H, 1, 'h');
        OpenOn(Rows, H);
        Assert.AreEqual('-', Run(Rows, Codeunit::"BQ Reader", 'get', G, 1), 'Get');
        AddRow(G, 1, 'new');
        Assert.AreEqual('new', Run(Rows, Codeunit::"BQ Reader 11", 'slvget', G, 1), 'Get after SelectLatestVersion');
        Rows.Close();
    end;

    // Count of the group; the test inserts a line of it and calls SelectLatestVersion: Count answers the first read.
    [Test]
    procedure BgQueryCache_Run_GroupCount_SelectLatestVersionInTest_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a');
        OpenOn(Rows, G);
        Assert.AreEqual('1', Run(Rows, Codeunit::"BQ Reader", 'count', G, 0), 'Count');
        AddRow(G, 2, 'b');
        Database.SelectLatestVersion();
        Assert.AreEqual('1', Run(Rows, Codeunit::"BQ Reader", 'count', G, 0), 'Count after SelectLatestVersion');
        Rows.Close();
    end;

    // FindFirst of the group; the test changes its first line and calls SelectLatestVersion: FindFirst answers the first read.
    [Test]
    procedure BgQueryCache_Run_GroupFindFirst_SelectLatestVersionInTest_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        AddRow(G, 1, 'a');
        OpenOn(Rows, G);
        Assert.AreEqual('1/a', Run(Rows, Codeunit::"BQ Reader", 'groupfirst', G, 0), 'FindFirst');
        SetRow(G, 1, 'b');
        Database.SelectLatestVersion();
        Assert.AreEqual('1/a', Run(Rows, Codeunit::"BQ Reader", 'groupfirst', G, 0), 'FindFirst after SelectLatestVersion');
        Rows.Close();
    end;

    // A committed row of a table with a Blob field; a Get of it; the test changes the row and its Blob: the same Get answers the
    // first read.
    [Test]
    procedure BgQueryCache_Run_BlobTable_GetAgain_AfterChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        K: Code[20];
    begin
        G := NewGroup();
        K := NewGroup();
        AddRow(G, 1, 'h');
        AddBlob(K, 'first', 'b1');
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader 11", 'blobdesc', K, 0), 'Get');
        SetBlob(K, 'second', 'b2');
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader 11", 'blobdesc', K, 0), 'Get again');
        Rows.Close();
    end;

    // As above, a table with a Media field.
    [Test]
    procedure BgQueryCache_Run_MediaTable_GetAgain_AfterChange()
    var
        Rows: TestPage "BQ Plain Rows";
        MediaRow: Record "BQ Media Row";
        G: Code[20];
        K: Code[20];
    begin
        G := NewGroup();
        K := NewGroup();
        AddRow(G, 1, 'h');
        MediaRow."No." := K;
        MediaRow.Description := 'first';
        MediaRow.Insert();
        Commit();
        OpenOn(Rows, G);
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader 12", 'mediadesc', K, 0), 'Get');
        MediaRow.Get(K);
        MediaRow.Description := 'second';
        MediaRow.Modify();
        Assert.AreEqual('first', Run(Rows, Codeunit::"BQ Reader 12", 'mediadesc', K, 0), 'Get again');
        Rows.Close();
    end;
}

// The table and worker of the Media cell of BQ Tests 11.
table 97576 "BQ Media Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; Picture; Media) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 97580 "BQ Reader 12"
{
    trigger OnRun()
    var
        MediaRow: Record "BQ Media Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        if MediaRow.Get(Parameters.Get('Group')) then
            Seen := MediaRow.Description
        else
            Seen := '-';
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}
