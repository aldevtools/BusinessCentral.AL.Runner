// An excerpt of the corpus probe BgTaskQueryCache.al of corpus pull requests 1457 and 1495 at 37bffb8a (al-runner issues 8467
// and 8637): the file up to BQ Tests 9, BQ Tests 10 (al-runner issue 8677), BQ Tests 3b, BQ Tests 11, BQ Media Row and BQ
// Reader 12. Left out: BQ Tests 9b and 9c (the "@ alone" pull request's), and BQ Tests 12 and BQ Version, which need Base
// Application.
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

// Neighbors of BQ Tests 4 and 5 (al-runner issue 8637, the shapes the runner did not cache yet; measured on a local BC 29.1
// OnPrem w1 container): FindFirst, FindLast, Count and IsEmpty of an empty set, read again after the test inserted a line the
// filters keep, for five and eight filters, filter groups 3, 5 and 7, the cross-column OR group -1, an | of a range and a
// value, an & inside an |, @ alone, an empty FILTER() of a view, a pattern not matched with @, and no filter at all (table
// 97546): the cache answers them all.
// Count and IsEmpty after SetLoadFields read the rows as they are. The test session's four reads first, then its change, then
// the worker's first four reads: the worker answers the test's for no filter and every filter shape of BQ Reader 4 and of this
// worker.

table 97546 "BQ Empty"
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

// The worker of BQ Tests 6: the four reads of the filters a Mode names (`SetShape`), or of table 97546 without a filter.
codeunit 97591 "BQ Reader 6"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Empty: Record "BQ Empty";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        if Mode = 'unfiltered' then
            Results.Add('Seen', EmptyForms(Empty))
        else begin
            SetShape(Row, Mode, Group);
            Results.Add('Seen', Forms(Row, Mode));
        end;
        Page.SetBackgroundTaskResult(Results);
    end;

    // The filters of a Mode: those of BQ Reader 4 and the shapes of BQ Tests 6; any other Mode is a filter on Description next
    // to the group's.
    procedure SetShape(var Row: Record "BQ Row"; Mode: Text; Group: Text)
    var
        FilterGroupNo: Integer;
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
            'group2', 'group3', 'group4', 'group5', 'group6', 'group7':
                begin
                    Evaluate(FilterGroupNo, CopyStr(Mode, 6));
                    Row.FilterGroup(FilterGroupNo);
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
            'combined':
                begin
                    Row.Ascending(false);
                    Row.FilterGroup(2);
                    Row.SetFilter("Group", '%1|%2', Group, 'NOGROUP');
                    Row.FilterGroup(0);
                    Row.SetFilter(Description, '>%1', '0');
                end;
            'five':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                    Row.SetFilter(Description, '<>%1', 'zz');
                    Row.SetRange("Line No.", 1, 9);
                    Row.FilterGroup(2);
                    Row.SetRange(Note, '');
                    Row.FilterGroup(0);
                end;
            'eight':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                    Row.SetFilter(Description, '<>%1', 'zz');
                    Row.SetRange("Line No.", 1, 9);
                    Row.FilterGroup(2);
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                    Row.SetFilter(Description, '<>%1', 'zz');
                    Row.SetRange("Line No.", 1, 9);
                    Row.FilterGroup(0);
                end;
            'orgroup':
                begin
                    Row.FilterGroup(-1);
                    Row.SetRange("Group", Group);
                    Row.SetRange(Description, 'NODESCRIPTION');
                    Row.FilterGroup(0);
                end;
            'orrange':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetFilter("Line No.", '1..3|5');
                end;
            'nested':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetFilter(Description, '>0&<z|zz');
                end;
            'atsign':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetFilter(Description, '@');
                end;
            'notlikeci':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetFilter(Description, '<>@zz*');
                end;
            'emptyview':
                Row.SetView(StrSubstNo('WHERE(Group=FILTER(%1),Description=FILTER())', Group));
            'loadfields':
                begin
                    Row.SetLoadFields(Description);
                    Row.SetRange("Group", Group);
                end;
            else begin
                Row.SetRange("Group", Group);
                Row.SetFilter(Description, Mode);
            end;
        end;
    end;

    // FindFirst, FindLast, Count and IsEmpty; after SetLoadFields Count and IsEmpty alone.
    procedure Forms(var Row: Record "BQ Row"; Mode: Text): Text
    var
        First: Text;
        Last: Text;
    begin
        if Mode = 'loadfields' then
            exit(Format(Row.Count()) + '|' + Format(Row.IsEmpty()));
        First := '-';
        Last := '-';
        if Row.FindFirst() then
            First := Row.Description;
        if Row.FindLast() then
            Last := Row.Description;
        exit(First + '|' + Last + '|' + Format(Row.Count()) + '|' + Format(Row.IsEmpty()));
    end;

    procedure EmptyForms(var Empty: Record "BQ Empty"): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Empty.FindFirst() then
            First := Empty.Description;
        if Empty.FindLast() then
            Last := Empty.Description;
        exit(First + '|' + Last + '|' + Format(Empty.Count()) + '|' + Format(Empty.IsEmpty()));
    end;
}

codeunit 97592 "BQ Tests 6"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

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

    local procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Modify();
    end;

    // Inserts a row of table 97546, then reads the table.
    local procedure AddEmpty(No: Code[20]; Description: Text[50])
    var
        Empty: Record "BQ Empty";
    begin
        Empty."No." := No;
        Empty.Description := Description;
        Empty.Insert();
        if Empty.FindFirst() then;
    end;

    // Empties table 97546 and commits.
    local procedure ClearEmpty()
    var
        Empty: Record "BQ Empty";
    begin
        Empty.DeleteAll();
        Commit();
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader 6", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The four reads of a Mode on an empty set; the test inserts a line the filters keep (Description as given); the four
    // reads again.
    local procedure InsertAfterFirstRead(Mode: Text; Description: Text[50]): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Rows.OpenView();
        Assert.AreEqual('-|-|0|Yes', Run(Rows, Mode, G), Mode + ': the first reads');
        AddRow(G, 1, Description, '');
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FiveFilters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('five', 'a'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_EightFilters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('eight', 'a'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterGroups3And5_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group3', 'a'), 'filter group 3, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group5', 'a'), 'filter group 5, after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterGroup7_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group7', 'a'), 'after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_OrGroup_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('orgroup', 'a'), 'after the insert');
    end;

    // An | of a range and a value on Line No., and an & inside an | on Description.
    [Test]
    procedure BgQueryCache_Run_AllForms_CompoundFilters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('orrange', 'a'), '1..3|5, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('nested', 'a'), '>0&<z|zz, after the insert');
    end;

    // @ alone, an empty FILTER() of a view, and a pattern not matched with @.
    [Test]
    procedure BgQueryCache_Run_AllForms_OtherFilterKinds_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('atsign', 'a'), '@, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('emptyview', 'a'), 'FILTER(), after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('notlikeci', 'a'), '<>@zz*, after the insert');
    end;

    // Count and IsEmpty after SetLoadFields(Description): they read the rows as they are.
    [Test]
    procedure BgQueryCache_Run_LoadFields_CountIsEmpty_SeeTheChange()
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
    begin
        G := NewGroup();
        Rows.OpenView();
        Assert.AreEqual('0|Yes', Run(Rows, 'loadfields', G), 'the first reads');
        AddRow(G, 1, 'a', '');
        Assert.AreEqual('1|No', Run(Rows, 'loadfields', G), 'after the insert');
        Rows.Close();
    end;

    // The four reads of table 97546 without a filter: the table is empty, then the test inserts a row.
    [Test]
    procedure BgQueryCache_Run_AllForms_NoFilter_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
    begin
        ClearEmpty();
        Rows.OpenView();
        Assert.AreEqual('-|-|0|Yes', Run(Rows, 'unfiltered', ''), 'the first reads');
        AddEmpty(NewGroup(), 'a');
        Assert.AreEqual('-|-|0|Yes', Run(Rows, 'unfiltered', ''), 'after the insert');
        Rows.Close();
    end;

    // The test session reads table 97546 without a filter after a commit, then inserts a row: the worker's first four reads.
    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_NoFilter_FirstReadsAnswerThem()
    var
        Rows: TestPage "BQ Plain Rows";
        Empty: Record "BQ Empty";
        Reader: Codeunit "BQ Reader 6";
    begin
        ClearEmpty();
        AddEmpty(NewGroup(), 'a');
        Commit();
        Assert.AreEqual('a|a|1|No', Reader.EmptyForms(Empty), 'the test''s reads');
        AddEmpty(NewGroup(), 'b');
        Rows.OpenView();
        Assert.AreEqual('a|a|1|No', Run(Rows, 'unfiltered', ''), 'the worker''s reads');
        Rows.Close();
    end;

    // For every Mode: the test session reads its four reads after a commit, changes line 1 and inserts line 2; the worker's
    // first four reads answer the test's. The Modes that differ are listed.
    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_EveryShape_FirstReadsAnswerThem()
    var
        Modes: List of [Text];
        Mode: Text;
        Differ: Text;
    begin
        Modes.AddRange('group', 'second', 'nonkey', 'group2', 'group3', 'group4', 'group5', 'group6', 'group7', 'range');
        Modes.AddRange('desc', 'setfilter', 'holder', 'or', 'like', 'likeci', 'setfiltersecond', 'three', 'four', 'combined');
        Modes.AddRange('five', 'eight', 'orgroup', 'orrange', 'nested', 'atsign', 'emptyview', 'notlikeci');
        Modes.AddRange('<>zz', '>0', '>=a', '<z', '<=a', 'a..', '..z', '>0&<z', '<>z*');
        foreach Mode in Modes do
            if not TestReadsFirst(Mode) then
                Differ += ' ' + Mode;
        Assert.AreEqual('', Differ, 'the Modes whose worker reads differ from the test''s');
    end;

    local procedure TestReadsFirst(Mode: Text): Boolean
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        Reader: Codeunit "BQ Reader 6";
        G: Code[20];
        Note: Text[50];
        TestSeen: Text;
        Seen: Text;
    begin
        G := NewGroup();
        if Mode = 'nonkey' then
            Note := G;
        AddRow(G, 1, 'a', Note);
        Commit();
        Reader.SetShape(Row, Mode, G);
        TestSeen := Reader.Forms(Row, Mode);
        SetRow(G, 1, 'b');
        AddRow(G, 2, 'c', Note);
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen = TestSeen);
    end;
}

// BQ Tests 7 (al-runner issue 8637, the shapes the runner still read as they are; measured on a local BC 29.1 OnPrem w1
// container): FindFirst, FindLast, Count and IsEmpty in one worker run, read again after the test session changed the rows
// without committing. The cache answers them with the first answers for 9, 12 and 30 filters (one on each stored field of
// table 97593), for 210 and 7650 (every field again in filter groups 2-7 and 2-255), for filter groups 8, 10, 100 and 255, for
// filter group 1 alone and with filter group 0, and for a filter on a FlowFilter field alone or with the key. A worker's
// filter in filter group 1 is not on the test session's records after the task. With a filter on a FlowField, FindFirst and
// FindLast answer from the cache when the other filters set every primary-key field to one value (in filter group 0 or 2,
// next to a FlowFilter filter or not), and a commit of the table the FlowField counts or sums ends the entry, a commit of
// another table does not; Count and IsEmpty read the rows as they are, and so do all four without such a key filter (a
// FlowField filter alone, with a range on the key, with a filter outside the key, with a part of a two-field key). <>@ alone
// on a Text field keeps no line before and after the insert. The test session's four reads first answer the worker's first
// four reads for filter groups 8-255, many filters, a FlowFilter filter, and FindFirst and FindLast with a FlowField filter
// next to the key.

table 97593 "BQ Wide"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; F3; Integer) { }
        field(4; F4; Integer) { }
        field(5; F5; Integer) { }
        field(6; F6; Integer) { }
        field(7; F7; Integer) { }
        field(8; F8; Integer) { }
        field(9; F9; Integer) { }
        field(10; F10; Integer) { }
        field(11; F11; Integer) { }
        field(12; F12; Integer) { }
        field(13; F13; Integer) { }
        field(14; F14; Integer) { }
        field(15; F15; Integer) { }
        field(16; F16; Integer) { }
        field(17; F17; Integer) { }
        field(18; F18; Integer) { }
        field(19; F19; Integer) { }
        field(20; F20; Integer) { }
        field(21; F21; Integer) { }
        field(22; F22; Integer) { }
        field(23; F23; Integer) { }
        field(24; F24; Integer) { }
        field(25; F25; Integer) { }
        field(26; F26; Integer) { }
        field(27; F27; Integer) { }
        field(28; F28; Integer) { }
        field(29; F29; Integer) { }
        field(30; F30; Integer) { }
        field(31; "Child Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Child" where(Parent = field("No.")));
        }
        field(32; "Child Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Child".Amount where(Parent = field("No."), Kind = field("Kind Filter")));
        }
        field(33; "Kind Filter"; Code[10])
        {
            FieldClass = FlowFilter;
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97594 "BQ Child"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Kind; Code[10]) { }
        field(4; Amount; Decimal) { }
        field(5; "Sibling Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Child" where(Parent = field(Parent)));
        }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

// The worker of BQ Tests 7: the four reads of the filters a Mode names: 'wide' and a number of filters on table 97593
// (`SetWide`), a FlowField or FlowFilter shape on table 97593 or 97594 (`SetFlow`, `SetChildFlow`), or a filter group or <>@
// on table 97500 (`SetShape`).
codeunit 97595 "BQ Reader 7"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Wide: Record "BQ Wide";
        Child: Record "BQ Child";
        WideRef: RecordRef;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
        Filters: Integer;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        case true of
            Mode.StartsWith('wide'):
                begin
                    Evaluate(Filters, CopyStr(Mode, 5));
                    WideRef.Open(Database::"BQ Wide");
                    SetWide(WideRef, Filters, Group);
                    Results.Add('Seen', WideForms(WideRef));
                end;
            Mode.StartsWith('flowchild'):
                begin
                    SetChildFlow(Child, Mode, Group);
                    Results.Add('Seen', ChildForms(Child));
                end;
            Mode.StartsWith('flow'):
                begin
                    SetFlow(Wide, Mode, Group);
                    Results.Add('Seen', FlowForms(Wide));
                end;
            else begin
                SetShape(Row, Mode, Group);
                Results.Add('Seen', Forms(Row));
            end;
        end;
        Page.SetBackgroundTaskResult(Results);
    end;

    // Filters filters on table 97593: one on each of its 30 stored fields in field order ("No." the group, Description <>zz,
    // the Integer fields 0) in filter group 0, then again in filter group 2, 3 and on (filter group 1 left out).
    procedure SetWide(var WideRef: RecordRef; Filters: Integer; Group: Text)
    var
        K: Integer;
        FieldNo: Integer;
        FilterGroupNo: Integer;
    begin
        for K := 0 to Filters - 1 do begin
            FieldNo := K mod 30 + 1;
            FilterGroupNo := K div 30;
            if FilterGroupNo > 0 then
                FilterGroupNo += 1;
            WideRef.FilterGroup(FilterGroupNo);
            case FieldNo of
                1:
                    WideRef.Field(1).SetRange(Group);
                2:
                    WideRef.Field(2).SetFilter('<>zz');
                else
                    WideRef.Field(FieldNo).SetRange(0);
            end;
        end;
        WideRef.FilterGroup(0);
    end;

    procedure WideForms(var WideRef: RecordRef): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if WideRef.FindFirst() then
            First := Format(WideRef.Field(2).Value());
        if WideRef.FindLast() then
            Last := Format(WideRef.Field(2).Value());
        exit(First + '|' + Last + '|' + Format(WideRef.Count()) + '|' + Format(WideRef.IsEmpty()));
    end;

    // The FlowField and FlowFilter shapes of table 97593: "Child Count" 1 or 0 alone, with "No." the group (by SetRange, by
    // SetFilter, in filter group 2), with "No." a range or with Description a; "Child Sum" 5 with "Kind Filter" K and "No." the
    // group; "Kind Filter" K or A..K alone, and K with "No." the group.
    procedure SetFlow(var Wide: Record "BQ Wide"; Mode: Text; Group: Text)
    begin
        case Mode of
            'flowcount':
                Wide.SetRange("Child Count", 1);
            'flowzero':
                Wide.SetRange("Child Count", 0);
            'flowcountkey':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Child Count", 1);
                end;
            'flowzerokey':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Child Count", 0);
                end;
            'flowcountsetfilter':
                begin
                    Wide.SetFilter("No.", Group);
                    Wide.SetRange("Child Count", 1);
                end;
            'flowcountgroup2':
                begin
                    Wide.FilterGroup(2);
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(0);
                    Wide.SetRange("Child Count", 1);
                end;
            'flowcountrange':
                begin
                    Wide.SetRange("No.", Group, Group + 'Z');
                    Wide.SetRange("Child Count", 1);
                end;
            'flowcountdesc':
                begin
                    Wide.SetRange(Description, 'a');
                    Wide.SetRange("Child Count", 1);
                end;
            'flowsum':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Kind Filter", 'K');
                    Wide.SetRange("Child Sum", 5);
                end;
            'flowfilter':
                Wide.SetRange("Kind Filter", 'K');
            'flowfilterrange':
                Wide.SetFilter("Kind Filter", 'A..K');
            'flowfilterkey':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Kind Filter", 'K');
                end;
        end;
    end;

    procedure FlowForms(var Wide: Record "BQ Wide"): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Wide.FindFirst() then
            First := Wide.Description;
        if Wide.FindLast() then
            Last := Wide.Description;
        exit(First + '|' + Last + '|' + Format(Wide.Count()) + '|' + Format(Wide.IsEmpty()));
    end;

    // The FlowField shapes of table 97594 (a two-field key): "Sibling Count" 2 with both key fields one value, or with Parent
    // alone.
    procedure SetChildFlow(var Child: Record "BQ Child"; Mode: Text; Group: Text)
    begin
        case Mode of
            'flowchildkey':
                begin
                    Child.SetRange(Parent, Group);
                    Child.SetRange("Line No.", 1);
                    Child.SetRange("Sibling Count", 2);
                end;
            'flowchildpart':
                begin
                    Child.SetRange(Parent, Group);
                    Child.SetRange("Sibling Count", 2);
                end;
        end;
    end;

    procedure ChildForms(var Child: Record "BQ Child"): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Child.FindFirst() then
            First := Child.Kind;
        if Child.FindLast() then
            Last := Child.Kind;
        exit(First + '|' + Last + '|' + Format(Child.Count()) + '|' + Format(Child.IsEmpty()));
    end;

    // The filter on the group in filter group 1, 8, 10, 100 or 255; in filter group 1 next to a filter on Note in filter
    // group 0; or <>@ alone on Description next to the filter on the group.
    procedure SetShape(var Row: Record "BQ Row"; Mode: Text; Group: Text)
    var
        FilterGroupNo: Integer;
    begin
        case Mode of
            'group1', 'group8', 'group10', 'group100', 'group255':
                begin
                    Evaluate(FilterGroupNo, CopyStr(Mode, 6));
                    Row.FilterGroup(FilterGroupNo);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                end;
            'group1and0':
                begin
                    Row.FilterGroup(1);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                    Row.SetRange(Note, '');
                end;
            'notat':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetFilter(Description, '<>@');
                end;
        end;
    end;

    procedure Forms(var Row: Record "BQ Row"): Text
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
}

codeunit 97596 "BQ Tests 7"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

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

    local procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Modify();
    end;

    // Inserts a row of table 97593 (its Integer fields 0), then reads it.
    local procedure AddWide(No: Code[20]; Description: Text[50])
    var
        Wide: Record "BQ Wide";
    begin
        Wide."No." := No;
        Wide.Description := Description;
        Wide.Insert();
        Wide.SetRange("No.", No);
        if Wide.FindFirst() then;
    end;

    local procedure SetWideRow(No: Code[20]; Description: Text[50])
    var
        Wide: Record "BQ Wide";
    begin
        Wide.Get(No);
        Wide.Description := Description;
        Wide.Modify();
    end;

    // Inserts a line of table 97594 (Kind K, Amount 5), then reads the parent's lines.
    local procedure AddChild(Parent: Code[20]; LineNo: Integer)
    var
        Child: Record "BQ Child";
    begin
        Child.Parent := Parent;
        Child."Line No." := LineNo;
        Child.Kind := 'K';
        Child.Amount := 5;
        Child.Insert();
        Child.SetRange(Parent, Parent);
        if Child.FindFirst() then;
    end;

    // Inserts a row of table 97501 and commits.
    local procedure AddOther()
    var
        Other: Record "BQ Other";
    begin
        Other."No." := NewGroup();
        Other.Insert();
        Commit();
    end;

    // Empties tables 97593 and 97594 and commits.
    local procedure ClearWide()
    var
        Wide: Record "BQ Wide";
        Child: Record "BQ Child";
    begin
        Wide.DeleteAll();
        Child.DeleteAll();
        Commit();
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader 7", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The four reads of a Mode of table 97500 on an empty set; the test inserts a line the filters keep (Description a); the
    // four reads again.
    local procedure InsertAfterFirstRead(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Rows.OpenView();
        Assert.AreEqual('-|-|0|Yes', Run(Rows, Mode, G), Mode + ': the first reads');
        AddRow(G, 1, 'a', '');
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // The four reads of a 'wide' Mode on an empty set; the test inserts a row of table 97593 the filters keep (Description a);
    // the four reads again.
    local procedure InsertWideAfterFirstRead(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Rows.OpenView();
        Assert.AreEqual('-|-|0|Yes', Run(Rows, Mode, G), Mode + ': the first reads');
        AddWide(G, 'a');
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // Tables 97593 and 97594 emptied and committed, then the Committed rows: none, 'parent' (a row of table 97593, Description
    // a) or 'child' (the row and its line 1). The four reads of a FlowField or FlowFilter Mode, the test's Change, the four reads
    // again: both answers, as 'first / again'. Changes: 'parent' inserts the row, 'child' inserts line 2, 'childcommit' inserts
    // line 2 and commits, 'othercommitchild' commits a row of table 97501 and then inserts line 2.
    local procedure FlowReads(Mode: Text; Committed: Text; Change: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        CommitRows(G, Committed);
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        case Change of
            'parent':
                AddWide(G, 'a');
            'child':
                AddChild(G, 2);
            'childcommit':
                begin
                    AddChild(G, 2);
                    Commit();
                end;
            'othercommitchild':
                begin
                    AddOther();
                    AddChild(G, 2);
                end;
        end;
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    local procedure CommitRows(G: Code[20]; Committed: Text)
    begin
        ClearWide();
        if Committed in ['parent', 'child'] then
            AddWide(G, 'a');
        if Committed = 'child' then
            AddChild(G, 1);
        Commit();
    end;

    // The Committed rows as for FlowReads; the test session's four reads of a FlowField or FlowFilter Mode of table 97593; its
    // Change ('desc': Description b, 'child': line 2 inserted); the worker's first four reads: both answers, as 'test / worker'.
    local procedure FlowTestReadsFirst(Mode: Text; Committed: Text; Change: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Wide: Record "BQ Wide";
        Reader: Codeunit "BQ Reader 7";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        CommitRows(G, Committed);
        Reader.SetFlow(Wide, Mode, G);
        Seen := Reader.FlowForms(Wide);
        case Change of
            'desc':
                SetWideRow(G, 'b');
            'child':
                AddChild(G, 2);
        end;
        Rows.OpenView();
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_NineAndTwelveFilters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertWideAfterFirstRead('wide9'), '9 filters, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertWideAfterFirstRead('wide12'), '12 filters, after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterOnEveryField_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertWideAfterFirstRead('wide30'), 'after the insert');
    end;

    // Every field again in filter groups 2-7 (210 filters) and in filter groups 2-255 (7650 filters).
    [Test]
    procedure BgQueryCache_Run_AllForms_EveryFieldInManyGroups_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertWideAfterFirstRead('wide210'), '210 filters, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertWideAfterFirstRead('wide7650'), '7650 filters, after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterGroups8And10_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group8'), 'filter group 8, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group10'), 'filter group 10, after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterGroups100And255_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group100'), 'filter group 100, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group255'), 'filter group 255, after the insert');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FilterGroup1_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group1'), 'alone, after the insert');
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group1and0'), 'with filter group 0, after the insert');
    end;

    // After a task whose worker set a filter in filter group 1, the test session's record of the table reads another group's
    // line, and its filter group 1 holds no filter.
    [Test]
    procedure BgQueryCache_Run_FilterGroup1_EndsWithTheWorker()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        G: Code[20];
        Other: Code[20];
    begin
        G := NewGroup();
        Other := NewGroup();
        AddRow(Other, 1, 'o', '');
        Commit();
        Rows.OpenView();
        Assert.AreEqual('-|-|0|Yes', Run(Rows, 'group1', G), 'the worker''s reads');
        Rows.Close();
        Row.SetRange("Group", Other);
        Assert.AreEqual(1, Row.Count(), 'the other group''s lines');
        Row.FilterGroup(1);
        Assert.AreEqual('', Row.GetFilters(), 'filter group 1');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_NotAtSignAlone_KeepsNoLine()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('notat'), 'after the insert');
    end;

    // A FlowField filter alone: all four read the rows as they are, after an insert of the row and of a line.
    [Test]
    procedure BgQueryCache_Run_FlowFieldAlone_SeesTheChange()
    begin
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', FlowReads('flowzero', '', 'parent'), 'Child Count 0, the row inserted');
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', FlowReads('flowcount', 'parent', 'child'), 'Child Count 1, a line inserted');
    end;

    // A FlowField filter next to one value on the key: FindFirst and FindLast answer the first answer, Count and IsEmpty see
    // the change.
    [Test]
    procedure BgQueryCache_Run_FlowFieldWithKey_FindsFirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', FlowReads('flowzerokey', '', 'parent'), 'Child Count 0, the row inserted');
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', FlowReads('flowcountkey', 'parent', 'child'), 'Child Count 1, a line inserted');
        Assert.AreEqual('a|a|1|No / a|a|0|Yes', FlowReads('flowcountkey', 'child', 'child'), 'Child Count 1, a second line');
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', FlowReads('flowsum', 'parent', 'child'), 'Child Sum 5 with Kind Filter K');
    end;

    // The key by SetFilter, the key in filter group 2, and both fields of a two-field key (table 97594): as with SetRange.
    [Test]
    procedure BgQueryCache_Run_FlowFieldWithKeyForms_FindsFirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', FlowReads('flowcountsetfilter', 'parent', 'child'), 'SetFilter of the key');
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', FlowReads('flowcountgroup2', 'parent', 'child'), 'the key in filter group 2');
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', FlowReads('flowchildkey', 'child', 'child'), 'a two-field key');
    end;

    // A range on the key, a filter outside the key, and one field of a two-field key: all four see the change.
    [Test]
    procedure BgQueryCache_Run_FlowFieldWithoutTheKey_SeesTheChange()
    begin
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', FlowReads('flowcountrange', 'parent', 'child'), 'a range on the key');
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', FlowReads('flowcountdesc', 'parent', 'child'), 'Description a');
        Assert.AreEqual('-|-|0|Yes / K|K|2|No', FlowReads('flowchildpart', 'child', 'child'), 'Parent alone');
    end;

    // A commit of the table the FlowField counts ends the answer of FindFirst and FindLast; a commit of another table does not.
    [Test]
    procedure BgQueryCache_Run_FlowFieldWithKey_CommitOfTheCountedTable_SeesTheChange()
    begin
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', FlowReads('flowcountkey', 'parent', 'childcommit'), 'the line committed');
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', FlowReads('flowcountkey', 'parent', 'othercommitchild'), 'another table');
    end;

    [Test]
    procedure BgQueryCache_Run_AllForms_FlowFilter_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', FlowReads('flowfilter', '', 'parent'), 'Kind Filter K');
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', FlowReads('flowfilterrange', '', 'parent'), 'Kind Filter A..K');
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', FlowReads('flowfilterkey', '', 'parent'), 'with the key');
    end;

    // For filter groups 8, 10, 100, 255 and <>@: the test session reads its four reads after a commit, changes line 1 and
    // inserts line 2; the worker's first four reads answer the test's. The Modes that differ are listed.
    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_GroupShapes_FirstReadsAnswerThem()
    var
        Modes: List of [Text];
        Mode: Text;
        Differ: Text;
    begin
        Modes.AddRange('group8', 'group10', 'group100', 'group255', 'notat');
        foreach Mode in Modes do
            if not TestReadsFirst(Mode) then
                Differ += ' ' + Mode;
        Assert.AreEqual('', Differ, 'the Modes whose worker reads differ from the test''s');
    end;

    // For 9, 12, 30, 210 and 7650 filters on table 97593: the test session reads its four reads after a commit and changes the
    // row's Description; the worker's first four reads answer the test's. The Modes that differ are listed.
    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_ManyFilters_FirstReadsAnswerThem()
    var
        Modes: List of [Text];
        Mode: Text;
        Differ: Text;
    begin
        Modes.AddRange('wide9', 'wide12', 'wide30', 'wide210', 'wide7650');
        foreach Mode in Modes do
            if not TestReadsFirstWide(Mode) then
                Differ += ' ' + Mode;
        Assert.AreEqual('', Differ, 'the Modes whose worker reads differ from the test''s');
    end;

    // With the key, FindFirst and FindLast of the worker answer the test's, Count and IsEmpty read the table; alone, all four
    // read the table; a FlowFilter filter answers the test's.
    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_FlowShapes_WorkerReads()
    begin
        Assert.AreEqual('a|a|1|No / a|a|1|No', FlowTestReadsFirst('flowcountkey', 'child', 'desc'), 'with the key, Description b');
        Assert.AreEqual('a|a|1|No / a|a|0|Yes', FlowTestReadsFirst('flowcountkey', 'child', 'child'), 'with the key, a line');
        Assert.AreEqual('a|a|1|No / b|b|1|No', FlowTestReadsFirst('flowcount', 'child', 'desc'), 'alone, Description b');
        Assert.AreEqual('a|a|1|No / a|a|1|No', FlowTestReadsFirst('flowfilterkey', 'parent', 'desc'), 'Kind Filter K');
    end;

    local procedure TestReadsFirst(Mode: Text): Boolean
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
        Reader: Codeunit "BQ Reader 7";
        G: Code[20];
        TestSeen: Text;
        Seen: Text;
    begin
        G := NewGroup();
        AddRow(G, 1, 'a', '');
        Commit();
        Reader.SetShape(Row, Mode, G);
        TestSeen := Reader.Forms(Row);
        SetRow(G, 1, 'b');
        AddRow(G, 2, 'c', '');
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen = TestSeen);
    end;

    local procedure TestReadsFirstWide(Mode: Text): Boolean
    var
        Rows: TestPage "BQ Plain Rows";
        Reader: Codeunit "BQ Reader 7";
        WideRef: RecordRef;
        Filters: Integer;
        G: Code[20];
        TestSeen: Text;
        Seen: Text;
    begin
        G := NewGroup();
        AddWide(G, 'a');
        Commit();
        Evaluate(Filters, CopyStr(Mode, 5));
        WideRef.Open(Database::"BQ Wide");
        Reader.SetWide(WideRef, Filters, G);
        TestSeen := Reader.WideForms(WideRef);
        SetWideRow(G, 'b');
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen = TestSeen);
    end;
}

// BQ Tests 8 (al-runner issue 8637, the shapes past BQ Tests 7; measured on a local BC 29.1 OnPrem w1 container): FindFirst,
// FindLast, Count and IsEmpty in one worker run, read again after the test session changed the rows without committing. The
// cache answers all four with no limit on the number of filters found: 7651 and 7710 filters (every field of table 97593
// in filter groups 0, 2-255, -1 and 1), 2000, 2200 and 7650 filters of values of their own, and 10000 and 26214 filters on
// table 97598 (102 fields in every filter group). With a filter on a FlowField, FindFirst and FindLast answer from the cache
// whenever every primary-key field has one filter of one value, in any filter group but the OR group: next to a filter
// outside the key, with the FlowField filter a range, >, <> or |, with two FlowField filters, with the FlowField filter in
// filter group 2 or 5, with the key in filter groups 1, 3, 5, 7, 8, 100 or 255, the key as a range to itself, descending,
// and for FlowFields that count, sum, take the minimum, maximum or average, check existence, look up a value, sum negated
// or count with a const condition (table 97597); Count and IsEmpty read the rows as they are. With the key in the OR group,
// or as a filter of two values, all four read the rows as they are. A commit of the table a FlowField takes the minimum of
// ends the answer. The test session's four reads first answer the worker's first FindFirst and FindLast for such shapes.

table 97597 "BQ Calc"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Child" where(Parent = field("No.")));
        }
        field(4; "Amount Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Child".Amount where(Parent = field("No.")));
        }
        field(5; "Amount Min"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = min("BQ Child".Amount where(Parent = field("No.")));
        }
        field(6; "Amount Max"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = max("BQ Child".Amount where(Parent = field("No.")));
        }
        field(7; "Amount Average"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = average("BQ Child".Amount where(Parent = field("No.")));
        }
        field(8; "Has Lines"; Boolean)
        {
            FieldClass = FlowField;
            CalcFormula = exist("BQ Child" where(Parent = field("No.")));
        }
        field(9; "Line Kind"; Code[10])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("BQ Child".Kind where(Parent = field("No.")));
        }
        field(10; "Negative Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = - sum("BQ Child".Amount where(Parent = field("No.")));
        }
        field(11; "K Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Child" where(Parent = field("No."), Kind = const('K')));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97598 "BQ Wider"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; F3; Integer) { }
        field(4; F4; Integer) { }
        field(5; F5; Integer) { }
        field(6; F6; Integer) { }
        field(7; F7; Integer) { }
        field(8; F8; Integer) { }
        field(9; F9; Integer) { }
        field(10; F10; Integer) { }
        field(11; F11; Integer) { }
        field(12; F12; Integer) { }
        field(13; F13; Integer) { }
        field(14; F14; Integer) { }
        field(15; F15; Integer) { }
        field(16; F16; Integer) { }
        field(17; F17; Integer) { }
        field(18; F18; Integer) { }
        field(19; F19; Integer) { }
        field(20; F20; Integer) { }
        field(21; F21; Integer) { }
        field(22; F22; Integer) { }
        field(23; F23; Integer) { }
        field(24; F24; Integer) { }
        field(25; F25; Integer) { }
        field(26; F26; Integer) { }
        field(27; F27; Integer) { }
        field(28; F28; Integer) { }
        field(29; F29; Integer) { }
        field(30; F30; Integer) { }
        field(31; F31; Integer) { }
        field(32; F32; Integer) { }
        field(33; F33; Integer) { }
        field(34; F34; Integer) { }
        field(35; F35; Integer) { }
        field(36; F36; Integer) { }
        field(37; F37; Integer) { }
        field(38; F38; Integer) { }
        field(39; F39; Integer) { }
        field(40; F40; Integer) { }
        field(41; F41; Integer) { }
        field(42; F42; Integer) { }
        field(43; F43; Integer) { }
        field(44; F44; Integer) { }
        field(45; F45; Integer) { }
        field(46; F46; Integer) { }
        field(47; F47; Integer) { }
        field(48; F48; Integer) { }
        field(49; F49; Integer) { }
        field(50; F50; Integer) { }
        field(51; F51; Integer) { }
        field(52; F52; Integer) { }
        field(53; F53; Integer) { }
        field(54; F54; Integer) { }
        field(55; F55; Integer) { }
        field(56; F56; Integer) { }
        field(57; F57; Integer) { }
        field(58; F58; Integer) { }
        field(59; F59; Integer) { }
        field(60; F60; Integer) { }
        field(61; F61; Integer) { }
        field(62; F62; Integer) { }
        field(63; F63; Integer) { }
        field(64; F64; Integer) { }
        field(65; F65; Integer) { }
        field(66; F66; Integer) { }
        field(67; F67; Integer) { }
        field(68; F68; Integer) { }
        field(69; F69; Integer) { }
        field(70; F70; Integer) { }
        field(71; F71; Integer) { }
        field(72; F72; Integer) { }
        field(73; F73; Integer) { }
        field(74; F74; Integer) { }
        field(75; F75; Integer) { }
        field(76; F76; Integer) { }
        field(77; F77; Integer) { }
        field(78; F78; Integer) { }
        field(79; F79; Integer) { }
        field(80; F80; Integer) { }
        field(81; F81; Integer) { }
        field(82; F82; Integer) { }
        field(83; F83; Integer) { }
        field(84; F84; Integer) { }
        field(85; F85; Integer) { }
        field(86; F86; Integer) { }
        field(87; F87; Integer) { }
        field(88; F88; Integer) { }
        field(89; F89; Integer) { }
        field(90; F90; Integer) { }
        field(91; F91; Integer) { }
        field(92; F92; Integer) { }
        field(93; F93; Integer) { }
        field(94; F94; Integer) { }
        field(95; F95; Integer) { }
        field(96; F96; Integer) { }
        field(97; F97; Integer) { }
        field(98; F98; Integer) { }
        field(99; F99; Integer) { }
        field(100; F100; Integer) { }
        field(101; F101; Integer) { }
        field(102; F102; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The worker of BQ Tests 8: the four reads of the filters a Mode names: 'many' on table 97593 or 97598 (`SetMany`), a
// FlowField shape on table 97593 (`SetFlow`) or on table 97597 (`SetCalc`).
codeunit 97597 "BQ Reader 8"
{
    trigger OnRun()
    var
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        Ref: RecordRef;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        case true of
            Mode.StartsWith('many'):
                begin
                    SetMany(Ref, Mode, Group);
                    Results.Add('Seen', RefForms(Ref));
                end;
            Mode.StartsWith('calc'):
                begin
                    SetCalc(Calc, Mode, Group);
                    Results.Add('Seen', CalcForms(Calc));
                end;
            else begin
                SetFlow(Wide, Mode, Group);
                Results.Add('Seen', FlowForms(Wide));
            end;
        end;
        Page.SetBackgroundTaskResult(Results);
    end;

    // 'many-<table>-<filters>-<values>', table wide (97593, 30 fields) or wider (97598, 102 fields): one filter on each field in
    // field order, in filter groups 0, 2-255, -1 and 1 in turn. Values 'same': "No." the group, Description <>zz, the Integer
    // fields 0. Values 'diff': "No." the group in filter group 0, then every filter <> a value of its own.
    procedure SetMany(var Ref: RecordRef; Mode: Text; Group: Text)
    var
        Parts: List of [Text];
        Groups: List of [Integer];
        K: Integer;
        N: Integer;
        Fields: Integer;
        FieldNo: Integer;
        G: Integer;
        Diff: Boolean;
    begin
        Parts := Mode.Split('-');
        if Parts.Get(2) = 'wide' then begin
            Ref.Open(Database::"BQ Wide");
            Fields := 30;
        end else begin
            Ref.Open(Database::"BQ Wider");
            Fields := 102;
        end;
        Evaluate(N, Parts.Get(3));
        Diff := Parts.Get(4) = 'diff';
        Groups.Add(0);
        for G := 2 to 255 do
            Groups.Add(G);
        Groups.Add(-1);
        Groups.Add(1);
        for K := 0 to N - 1 do begin
            FieldNo := K mod Fields + 1;
            Ref.FilterGroup(Groups.Get(K div Fields + 1));
            case true of
                FieldNo = 1:
                    if Diff and (K > 0) then
                        Ref.Field(1).SetFilter('<>X' + Format(K))
                    else
                        Ref.Field(1).SetRange(Group);
                FieldNo = 2:
                    if Diff then
                        Ref.Field(2).SetFilter('<>z' + Format(K))
                    else
                        Ref.Field(2).SetFilter('<>zz');
                else
                    if Diff then
                        Ref.Field(FieldNo).SetFilter('<>' + Format(K + 1))
                    else
                        Ref.Field(FieldNo).SetRange(0);
            end;
        end;
        Ref.FilterGroup(0);
    end;

    procedure RefForms(var Ref: RecordRef): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Ref.FindFirst() then
            First := Format(Ref.Field(2).Value());
        if Ref.FindLast() then
            Last := Format(Ref.Field(2).Value());
        exit(First + '|' + Last + '|' + Format(Ref.Count()) + '|' + Format(Ref.IsEmpty()));
    end;

    // The FlowField shapes of table 97593: "Child Count" 1 next to "No." the group and Description a (in filter group 0 or 2)
    // or F3 0; "Child Count" 1..5, >0, <>0 or 1|2 next to "No." the group; "Child Count" 1 and "Child Sum" 5 with "Kind
    // Filter" K; "Child Count" 1 in filter group 2 or 5; "No." the group in filter group 1, 3, 5, 7, 8, 100, 255 or -1; both
    // in filter group 5; "No." a range from the group to itself, or the group | NOROW; descending.
    procedure SetFlow(var Wide: Record "BQ Wide"; Mode: Text; Group: Text)
    var
        G: Integer;
    begin
        case Mode of
            'keydesc':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange(Description, 'a');
                    Wide.SetRange("Child Count", 1);
                end;
            'keyf3':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange(F3, 0);
                    Wide.SetRange("Child Count", 1);
                end;
            'keydescgroup2':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(2);
                    Wide.SetRange(Description, 'a');
                    Wide.FilterGroup(0);
                    Wide.SetRange("Child Count", 1);
                end;
            'flowrange':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Child Count", 1, 5);
                end;
            'flowgt', 'flowne', 'flowor':
                begin
                    Wide.SetRange("No.", Group);
                    case Mode of
                        'flowgt':
                            Wide.SetFilter("Child Count", '>0');
                        'flowne':
                            Wide.SetFilter("Child Count", '<>0');
                        'flowor':
                            Wide.SetFilter("Child Count", '1|2');
                    end;
                end;
            'twoflow':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Kind Filter", 'K');
                    Wide.SetRange("Child Count", 1);
                    Wide.SetRange("Child Sum", 5);
                end;
            'flowgroup2', 'flowgroup5':
                begin
                    Wide.SetRange("No.", Group);
                    Evaluate(G, CopyStr(Mode, 10));
                    Wide.FilterGroup(G);
                    Wide.SetRange("Child Count", 1);
                    Wide.FilterGroup(0);
                end;
            'keygroup1', 'keygroup3', 'keygroup5', 'keygroup7', 'keygroup8', 'keygroup100', 'keygroup255', 'keygroup-1':
                begin
                    Evaluate(G, CopyStr(Mode, 9));
                    Wide.FilterGroup(G);
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(0);
                    Wide.SetRange("Child Count", 1);
                end;
            'bothgroup5':
                begin
                    Wide.FilterGroup(5);
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Child Count", 1);
                    Wide.FilterGroup(0);
                end;
            'keyrange':
                begin
                    Wide.SetRange("No.", Group, Group);
                    Wide.SetRange("Child Count", 1);
                end;
            'keyor':
                begin
                    Wide.SetFilter("No.", '%1|%2', Group, 'NOROW');
                    Wide.SetRange("Child Count", 1);
                end;
            'descending':
                begin
                    Wide.Ascending(false);
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Child Count", 1);
                end;
        end;
    end;

    procedure FlowForms(var Wide: Record "BQ Wide"): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Wide.FindFirst() then
            First := Wide.Description;
        if Wide.FindLast() then
            Last := Wide.Description;
        exit(First + '|' + Last + '|' + Format(Wide.Count()) + '|' + Format(Wide.IsEmpty()));
    end;

    // "No." the group and one FlowField of table 97597 set to what one line of Amount 5 and Kind K gives it.
    procedure SetCalc(var Calc: Record "BQ Calc"; Mode: Text; Group: Text)
    begin
        Calc.SetRange("No.", Group);
        case Mode of
            'calccount':
                Calc.SetRange("Line Count", 1);
            'calcsum':
                Calc.SetRange("Amount Sum", 5);
            'calcmin':
                Calc.SetRange("Amount Min", 5);
            'calcmax':
                Calc.SetRange("Amount Max", 5);
            'calcaverage':
                Calc.SetRange("Amount Average", 5);
            'calcexist':
                Calc.SetRange("Has Lines", true);
            'calclookup':
                Calc.SetRange("Line Kind", 'K');
            'calcnegsum':
                Calc.SetRange("Negative Sum", -5);
            'calcconst':
                Calc.SetRange("K Count", 1);
        end;
    end;

    procedure CalcForms(var Calc: Record "BQ Calc"): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Calc.FindFirst() then
            First := Calc.Description;
        if Calc.FindLast() then
            Last := Calc.Description;
        exit(First + '|' + Last + '|' + Format(Calc.Count()) + '|' + Format(Calc.IsEmpty()));
    end;
}

codeunit 97598 "BQ Tests 8"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader 8", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // Inserts a row of table 97593, 97597 or 97598 (Description a, the Integer fields 0), then reads it.
    local procedure AddRow(Mode: Text; No: Code[20])
    var
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        Wider: Record "BQ Wider";
    begin
        case true of
            Mode.StartsWith('calc'):
                begin
                    Calc."No." := No;
                    Calc.Description := 'a';
                    Calc.Insert();
                    Calc.SetRange("No.", No);
                    if Calc.FindFirst() then;
                end;
            Mode.Contains('-wider-'):
                begin
                    Wider."No." := No;
                    Wider.Description := 'a';
                    Wider.Insert();
                    Wider.SetRange("No.", No);
                    if Wider.FindFirst() then;
                end;
            else begin
                Wide."No." := No;
                Wide.Description := 'a';
                Wide.Insert();
                Wide.SetRange("No.", No);
                if Wide.FindFirst() then;
            end;
        end;
    end;

    // Sets the Description of the row of table 97593 or 97597 to b.
    local procedure SetDescription(Mode: Text; No: Code[20])
    var
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
    begin
        if Mode.StartsWith('calc') then begin
            Calc.Get(No);
            Calc.Description := 'b';
            Calc.Modify();
        end else begin
            Wide.Get(No);
            Wide.Description := 'b';
            Wide.Modify();
        end;
    end;

    // Inserts a line of table 97594 (Kind K, Amount 5), then reads the parent's lines.
    local procedure AddChild(Parent: Code[20]; LineNo: Integer)
    var
        Child: Record "BQ Child";
    begin
        Child.Parent := Parent;
        Child."Line No." := LineNo;
        Child.Kind := 'K';
        Child.Amount := 5;
        Child.Insert();
        Child.SetRange(Parent, Parent);
        if Child.FindFirst() then;
    end;

    // The four reads of a 'many' Mode on an empty set; the test inserts a row the filters keep; the four reads again: both
    // answers, as 'first / again'.
    local procedure ManyReads(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        AddRow(Mode, G);
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // The row of a FlowField Mode committed (Description a, no line); the four reads; the test inserts its line (with 'commit':
    // and commits); the four reads again: both answers, as 'first / again'.
    local procedure FlowReads(Mode: Text; Change: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        AddRow(Mode, G);
        Commit();
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        AddChild(G, 1);
        if Change = 'commit' then
            Commit();
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // The FlowField Modes whose FlowReads answer is not Expected.
    local procedure FlowDiffer(Modes: List of [Text]; Expected: Text): Text
    var
        Mode: Text;
        Seen: Text;
        Differ: Text;
    begin
        foreach Mode in Modes do begin
            Seen := FlowReads(Mode, '');
            if Seen <> Expected then
                Differ += ' ' + Mode + ': ' + Seen;
        end;
        exit(Differ);
    end;

    // The row of a FlowField Mode and its line committed; the test session's four reads; its Description set to b; the
    // worker's first four reads: both answers, as 'test / worker'.
    local procedure FlowTestReadsFirst(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        Reader: Codeunit "BQ Reader 8";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        AddRow(Mode, G);
        AddChild(G, 1);
        Commit();
        if Mode.StartsWith('calc') then begin
            Reader.SetCalc(Calc, Mode, G);
            Seen := Reader.CalcForms(Calc);
        end else begin
            Reader.SetFlow(Wide, Mode, G);
            Seen := Reader.FlowForms(Wide);
        end;
        SetDescription(Mode, G);
        Rows.OpenView();
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // 7651 filters (one more in the OR group) and 7710 (every field of table 97593 in filter groups 0, 2-255, -1 and 1).
    [Test]
    procedure BgQueryCache_Run_AllForms_MoreThan7650Filters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', ManyReads('many-wide-7651-same'), '7651 filters');
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', ManyReads('many-wide-7710-same'), '7710 filters');
    end;

    // Filters of values of their own: 2000, 2200 and 7650.
    [Test]
    procedure BgQueryCache_Run_AllForms_FiltersOfTheirOwnValues_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', ManyReads('many-wide-2000-diff'), '2000 filters');
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', ManyReads('many-wide-2200-diff'), '2200 filters');
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', ManyReads('many-wide-7650-diff'), '7650 filters');
    end;

    // Table 97598 (102 fields): 10000 filters, and 26214 (every field in every filter group), alike and of their own values.
    [Test]
    procedure BgQueryCache_Run_AllForms_WiderTable_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', ManyReads('many-wider-10000-same'), '10000 filters');
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', ManyReads('many-wider-26214-same'), '26214 filters');
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', ManyReads('many-wider-26214-diff'), '26214 filters of their own values');
    end;

    // Next to the key: a filter outside the key (Description in filter group 0 or 2, F3), the FlowField filter as a range, >,
    // <> or |, two FlowField filters. FindFirst and FindLast answer the first answer, Count and IsEmpty see the line.
    [Test]
    procedure BgQueryCache_Run_FlowFieldWithKey_OtherFilters_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('keydesc', 'keyf3', 'keydescgroup2', 'flowrange', 'flowgt', 'flowne', 'flowor', 'twoflow');
        Assert.AreEqual('', FlowDiffer(Modes, '-|-|0|Yes / -|-|1|No'), 'the Modes that differ');
    end;

    // The FlowField filter in filter group 2 or 5, the key in filter group 1, 3, 5, 7, 8, 100 or 255, both in filter group 5,
    // the key as a range to itself, a descending sort.
    [Test]
    procedure BgQueryCache_Run_FlowFieldWithKey_OtherGroups_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('flowgroup2', 'flowgroup5', 'keygroup1', 'keygroup3', 'keygroup5', 'keygroup7', 'keygroup8');
        Modes.AddRange('keygroup100', 'keygroup255', 'bothgroup5', 'keyrange', 'descending');
        Assert.AreEqual('', FlowDiffer(Modes, '-|-|0|Yes / -|-|1|No'), 'the Modes that differ');
    end;

    // The key in the OR group, or as a filter of two values: all four see the line.
    [Test]
    procedure BgQueryCache_Run_FlowFieldWithoutOneKeyValue_SeesTheChange()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('keygroup-1', 'keyor');
        Assert.AreEqual('', FlowDiffer(Modes, '-|-|0|Yes / a|a|1|No'), 'the Modes that differ');
    end;

    // FlowFields of table 97597 that count, sum, take the minimum, maximum or average, check existence, look up a value, sum
    // negated or count with a const condition.
    [Test]
    procedure BgQueryCache_Run_FlowFieldMethods_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('calccount', 'calcsum', 'calcmin', 'calcmax', 'calcaverage', 'calcexist', 'calclookup');
        Modes.AddRange('calcnegsum', 'calcconst');
        Assert.AreEqual('', FlowDiffer(Modes, '-|-|0|Yes / -|-|1|No'), 'the Modes that differ');
    end;

    // A commit of the line after the first reads of a FlowField that takes the minimum: all four see it.
    [Test]
    procedure BgQueryCache_Run_FlowFieldMin_CommitOfTheLine_SeesTheChange()
    begin
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', FlowReads('calcmin', 'commit'), 'after the commit');
    end;

    // The test session's reads first: the worker's FindFirst and FindLast answer the test's, Count and IsEmpty read the rows
    // as they are, for a filter outside the key, the FlowField filter in filter group 5 and a FlowField that takes the minimum.
    [Test]
    procedure BgQueryCache_Run_TestReadsFirst_FlowFieldShapes_FindsAnswerThem()
    begin
        Assert.AreEqual('a|a|1|No / a|a|0|Yes', FlowTestReadsFirst('keydesc'), 'Description a');
        Assert.AreEqual('a|a|1|No / a|a|1|No', FlowTestReadsFirst('flowgroup5'), 'filter group 5');
        Assert.AreEqual('a|a|1|No / a|a|1|No', FlowTestReadsFirst('calcmin'), 'Amount Min');
    end;
}

// BQ Tests 8b (al-runner issue 8637, the FlowField shapes past BQ Tests 8; measured on a local BC 29.1 OnPrem w1 container):
// FindFirst, FindLast, Count and IsEmpty in one worker run with a filter on a FlowField next to one value on each key field,
// read again after the test session changed the rows without committing. FindFirst and FindLast answer the first answer and
// Count and IsEmpty see the change with a two-field key split across filter groups, with the FlowField filter >=, &, < or <=,
// in filter group 1, 7, 100 or 255, next to a filter outside the key in the OR group, filter group 1 or 255, for a lookup
// FlowField filtered on its own FlowFilter (table 97590) and for a FlowField a table extension declares. A wildcard on the
// FlowField reads the rows as they are, and so does a FlowField whose condition is a FlowField of its source table (tables
// 97591, 97592, 97595), after an insert committed or not. An empty FILTER() of a view on the FlowField, and a FlowField filter
// in the OR group, keep the row: FindFirst and FindLast answer the first answer after a change of the row.
// Fixtures (the header's Fixtures used lines have no room): tables 97590, 97591, 97592, 97595, table extension 97590 of table
// 97593, worker codeunit 97599, test codeunit 97547.
table 97590 "BQ Calc 2"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; "Kind Filter"; Code[10])
        {
            FieldClass = FlowFilter;
        }
        field(4; "Filtered Kind"; Code[10])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("BQ Child".Kind where(Parent = field("No."), Kind = field("Kind Filter")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97591 "BQ Busy"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; "Busy Lines"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Noted Line" where(Parent = field("No."), "Has Notes" = const(true)));
        }
        field(4; "Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Note" where(Parent = field("No.")));
        }
        field(5; "Amount At Note Count"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Noted Line".Amount where(Parent = field("No."), "Line No." = field("Note Count")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97592 "BQ Noted Line"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Amount; Decimal) { }
        field(4; "Has Notes"; Boolean)
        {
            FieldClass = FlowField;
            CalcFormula = exist("BQ Note" where(Parent = field(Parent), "Line No." = field("Line No.")));
        }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

table 97595 "BQ Note"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

tableextension 97590 "BQ Wide Ext" extends "BQ Wide"
{
    fields
    {
        field(97590; "Ext Child Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Child" where(Parent = field("No.")));
        }
    }
}

// The worker of BQ Tests 8b: the four reads of the filters a Mode names, on table 97593, 97594, 97597, 97590 or 97591.
codeunit 97599 "BQ Reader 8b"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        Results.Add('Seen', Forms(Mode, Group));
        Page.SetBackgroundTaskResult(Results);
    end;

    procedure Forms(Mode: Text; Group: Text): Text
    var
        Wide: Record "BQ Wide";
        Child: Record "BQ Child";
        Calc: Record "BQ Calc";
        Calc2: Record "BQ Calc 2";
        Busy: Record "BQ Busy";
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        case true of
            Mode.StartsWith('child'):
                begin
                    SetChild(Child, Mode, Group);
                    if Child.FindFirst() then
                        First := Format(Child."Line No.");
                    if Child.FindLast() then
                        Last := Format(Child."Line No.");
                    exit(First + '|' + Last + '|' + Format(Child.Count()) + '|' + Format(Child.IsEmpty()));
                end;
            Mode.StartsWith('calc2'):
                begin
                    Calc2.SetRange("No.", Group);
                    Calc2.SetRange("Kind Filter", 'K');
                    Calc2.SetRange("Filtered Kind", 'K');
                    if Calc2.FindFirst() then
                        First := Calc2.Description;
                    if Calc2.FindLast() then
                        Last := Calc2.Description;
                    exit(First + '|' + Last + '|' + Format(Calc2.Count()) + '|' + Format(Calc2.IsEmpty()));
                end;
            Mode.StartsWith('calc'):
                begin
                    SetCalc(Calc, Mode, Group);
                    if Calc.FindFirst() then
                        First := Calc.Description;
                    if Calc.FindLast() then
                        Last := Calc.Description;
                    exit(First + '|' + Last + '|' + Format(Calc.Count()) + '|' + Format(Calc.IsEmpty()));
                end;
            Mode.StartsWith('busy'), Mode.StartsWith('amount'):
                begin
                    Busy.SetRange("No.", Group);
                    if Mode.StartsWith('busy') then
                        Busy.SetRange("Busy Lines", 1)
                    else
                        Busy.SetRange("Amount At Note Count", 5);
                    if Busy.FindFirst() then
                        First := Busy.Description;
                    if Busy.FindLast() then
                        Last := Busy.Description;
                    exit(First + '|' + Last + '|' + Format(Busy.Count()) + '|' + Format(Busy.IsEmpty()));
                end;
            else begin
                SetWide(Wide, Mode, Group);
                if Wide.FindFirst() then
                    First := Wide.Description;
                if Wide.FindLast() then
                    Last := Wide.Description;
                exit(First + '|' + Last + '|' + Format(Wide.Count()) + '|' + Format(Wide.IsEmpty()));
            end;
        end;
    end;

    local procedure SetChild(var Child: Record "BQ Child"; Mode: Text; Group: Text)
    begin
        case Mode of
            'childsplit05':
                begin
                    Child.SetRange(Parent, Group);
                    Child.FilterGroup(5);
                    Child.SetRange("Line No.", 1);
                    Child.FilterGroup(0);
                end;
            'childsplit22':
                begin
                    Child.FilterGroup(2);
                    Child.SetRange(Parent, Group);
                    Child.SetRange("Line No.", 1);
                    Child.FilterGroup(0);
                end;
            'childsplit50':
                begin
                    Child.FilterGroup(5);
                    Child.SetRange(Parent, Group);
                    Child.FilterGroup(0);
                    Child.SetRange("Line No.", 1);
                end;
            'childboth0':
                begin
                    Child.SetRange(Parent, Group);
                    Child.SetRange("Line No.", 1);
                end;
        end;
        Child.SetRange("Sibling Count", 2);
    end;

    local procedure SetCalc(var Calc: Record "BQ Calc"; Mode: Text; Group: Text)
    begin
        case Mode of
            'calcwild':
                begin
                    Calc.SetRange("No.", Group);
                    Calc.SetFilter("Line Kind", '@k*');
                end;
            'calcat':
                begin
                    Calc.SetRange("No.", Group);
                    Calc.SetFilter("Line Kind", '@');
                end;
            'calcempty':
                Calc.SetView(StrSubstNo('WHERE("No."=FILTER(%1),"Line Kind"=FILTER())', Group));
        end;
    end;

    local procedure SetWide(var Wide: Record "BQ Wide"; Mode: Text; Group: Text)
    var
        G: Integer;
    begin
        Wide.SetRange("No.", Group);
        case Mode of
            'kindlt':
                Wide.SetFilter("Child Count", '<1');
            'kindle':
                Wide.SetFilter("Child Count", '<=0');
            'kindge':
                Wide.SetFilter("Child Count", '>=1');
            'kindand':
                Wide.SetFilter("Child Count", '>0&<5');
            'flowor', 'flowg1', 'flowg7', 'flowg100', 'flowg255':
                begin
                    if Mode = 'flowor' then
                        Wide.FilterGroup(-1)
                    else begin
                        Evaluate(G, CopyStr(Mode, 6));
                        Wide.FilterGroup(G);
                    end;
                    Wide.SetRange("Child Count", 1);
                    Wide.FilterGroup(0);
                end;
            'descor', 'descg1', 'descg255':
                begin
                    if Mode = 'descor' then
                        Wide.FilterGroup(-1)
                    else begin
                        Evaluate(G, CopyStr(Mode, 6));
                        Wide.FilterGroup(G);
                    end;
                    Wide.SetRange(Description, 'a');
                    Wide.FilterGroup(0);
                    Wide.SetRange("Child Count", 1);
                end;
            'extcount':
                Wide.SetRange("Ext Child Count", 1);
        end;
    end;
}

codeunit 97547 "BQ Tests 8b"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader 8b", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure AddChild(Parent: Code[20]; LineNo: Integer)
    var
        Child: Record "BQ Child";
    begin
        Child.Parent := Parent;
        Child."Line No." := LineNo;
        Child.Kind := 'K';
        Child.Amount := 5;
        Child.Insert();
        Child.SetRange(Parent, Parent);
        if Child.FindFirst() then;
    end;

    local procedure AddNote(Parent: Code[20])
    var
        Note: Record "BQ Note";
    begin
        Note.Parent := Parent;
        Note."Line No." := 1;
        Note.Insert();
        Note.SetRange(Parent, Parent);
        if Note.FindFirst() then;
    end;

    local procedure SetDescription(Mode: Text; G: Code[20])
    var
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
    begin
        if Mode.StartsWith('calc') then begin
            Calc.Get(G);
            Calc.Description := 'b';
            Calc.Modify();
        end else begin
            Wide.Get(G);
            Wide.Description := 'b';
            Wide.Modify();
        end;
    end;

    local procedure SetUp(Mode: Text; G: Code[20])
    var
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        Calc2: Record "BQ Calc 2";
        Busy: Record "BQ Busy";
        Line: Record "BQ Noted Line";
    begin
        case true of
            Mode.StartsWith('calc2'):
                begin
                    Calc2."No." := G;
                    Calc2.Description := 'a';
                    Calc2.Insert();
                end;
            Mode.StartsWith('calc'):
                begin
                    Calc."No." := G;
                    Calc.Description := 'a';
                    Calc.Insert();
                end;
            Mode.StartsWith('busy'), Mode.StartsWith('amount'):
                begin
                    Busy."No." := G;
                    Busy.Description := 'a';
                    Busy.Insert();
                    Line.Parent := G;
                    Line."Line No." := 1;
                    Line.Amount := 5;
                    Line.Insert();
                end;
            else begin
                Wide."No." := G;
                Wide.Description := 'a';
                Wide.Insert();
                if Mode.StartsWith('child') then
                    AddChild(G, 1);
            end;
        end;
        Commit();
    end;

    // The rows a Mode reads committed (the row, Description a; for 'child' Modes also its line 1; for 'busy' its line 1 of
    // table 97592); the four reads; the test's Change: 'line' inserts a line of table 97594 (line 2 for 'child' Modes), 'desc'
    // sets the row's Description to b, 'note' inserts a note of table 97595 for line 1, 'notecommit' does so and commits; the
    // four reads again: both answers, as 'first / again'.
    local procedure Reads(Mode: Text; Change: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        SetUp(Mode, G);
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        case Change of
            'line':
                if Mode.StartsWith('child') then
                    AddChild(G, 2)
                else
                    AddChild(G, 1);
            'desc':
                SetDescription(Mode, G);
            'note':
                AddNote(G);
            'notecommit':
                begin
                    AddNote(G);
                    Commit();
                end;
        end;
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // The Modes whose Reads answer after Change is not Expected, with their answers.
    local procedure Differ(Modes: List of [Text]; Change: Text; Expected: Text): Text
    var
        Mode: Text;
        Seen: Text;
        Found: Text;
    begin
        foreach Mode in Modes do begin
            Seen := Reads(Mode, Change);
            if Seen <> Expected then
                Found += ' ' + Mode + ': ' + Seen;
        end;
        exit(Found);
    end;

    // A two-field key with Parent in filter group 0 and "Line No." in 5, Parent in 5 and "Line No." in 0, both in 2; next to
    // "Sibling Count" 2: FindFirst and FindLast answer the first answer, Count and IsEmpty see the second line.
    [Test]
    procedure BgQueryCache_Run_FlowFieldKeyAcrossGroups_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('childsplit05', 'childsplit50', 'childsplit22');
        Assert.AreEqual('', Differ(Modes, 'line', '-|-|0|Yes / -|-|1|No'), 'the Modes that differ');
    end;

    // "Child Count" >=1 and >0&<5, then <1 and <=0, next to the key: FindFirst and FindLast answer the first answer.
    [Test]
    procedure BgQueryCache_Run_FlowFieldFilterKinds_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('kindge', 'kindand');
        Assert.AreEqual('', Differ(Modes, 'line', '-|-|0|Yes / -|-|1|No'), '>=1 and >0&<5');
        Clear(Modes);
        Modes.AddRange('kindlt', 'kindle');
        Assert.AreEqual('', Differ(Modes, 'line', 'a|a|1|No / a|a|0|Yes'), '<1 and <=0');
    end;

    // "Line Kind" (a lookup) @k* next to the key: all four see the line.
    [Test]
    procedure BgQueryCache_Run_FlowFieldWildcard_SeesTheChange()
    begin
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', Reads('calcwild', 'line'), '@k*');
    end;

    // "Child Count" 1 in the OR group next to the key; the test sets the Description to b.
    [Test]
    procedure BgQueryCache_Run_FlowFieldInOrGroup_DescriptionChanged()
    begin
        Assert.AreEqual('a|a|1|No / a|a|1|No', Reads('flowor', 'desc'), 'the OR group');
    end;

    // "Child Count" 1 in filter group 1, 7, 100 or 255 next to the key; Description a in the OR group, filter group 1 or 255
    // next to the key and "Child Count" 1.
    [Test]
    procedure BgQueryCache_Run_FlowFieldAndOtherFilterGroups_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('flowg1', 'flowg7', 'flowg100', 'flowg255', 'descor', 'descg1', 'descg255');
        Assert.AreEqual('', Differ(Modes, 'line', '-|-|0|Yes / -|-|1|No'), 'the Modes that differ');
    end;

    // A lookup FlowField filtered on its own FlowFilter (table 97590), and a FlowField a table extension declares.
    [Test]
    procedure BgQueryCache_Run_FlowFieldFlowFilterAndExtension_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('calc2lookup', 'extcount');
        Assert.AreEqual('', Differ(Modes, 'line', '-|-|0|Yes / -|-|1|No'), 'the Modes that differ');
    end;

    // "Busy Lines" counts the lines whose "Has Notes" (an exist FlowField over table 97595) is true: after a note, committed
    // or not, all four see it.
    [Test]
    procedure BgQueryCache_Run_FlowFieldOverAFlowField_SeesTheChange()
    begin
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', Reads('busy', 'note'), 'the note');
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', Reads('busy', 'notecommit'), 'the note committed');
    end;

    // "Line Kind" FILTER() of a view next to the key; the test sets the Description to b.
    [Test]
    procedure BgQueryCache_Run_FlowFieldEmptyFilter_DescriptionChanged()
    begin
        Assert.AreEqual('a|a|1|No / a|a|1|No', Reads('calcempty', 'desc'), 'FILTER()');
    end;
}

// BQ Tests 9 (al-runner issue 8637, the shapes past BQ Tests 8b; measured on a local BC 29.1 OnPrem w1 container): FindFirst,
// FindLast, Count and IsEmpty in one worker run, read again after the test session changed the rows without committing. The
// cache answers all four for 26215, 60000 and 100000 filters on table 97596 (502 fields, then again in filter groups 2 and
// on): no limit found. With a filter on a FlowField next to the key, FindFirst and FindLast answer the first answer
// and Count and IsEmpty see the change when the key is set twice to its value (in filter groups 0 and 2, or by SetRange then
// SetFilter in group 0), and for the FlowField filters 1.., ..0, >0&<5|9, the case-sensitive wildcards K*, *K, ? and k* (a
// Code filter is upper-cased), and @ alone; all four answer the first answers for the key set twice without a FlowField. @
// alone on a Code field keeps the rows whose value is empty, @k keeps K, and GetFilter returns @.
// Fixtures (the header's Fixtures used lines have no room): table 97596, worker codeunit 97528, test codeunit 97539.

table 97596 "BQ Widest"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; F3; Integer) { }
        field(4; F4; Integer) { }
        field(5; F5; Integer) { }
        field(6; F6; Integer) { }
        field(7; F7; Integer) { }
        field(8; F8; Integer) { }
        field(9; F9; Integer) { }
        field(10; F10; Integer) { }
        field(11; F11; Integer) { }
        field(12; F12; Integer) { }
        field(13; F13; Integer) { }
        field(14; F14; Integer) { }
        field(15; F15; Integer) { }
        field(16; F16; Integer) { }
        field(17; F17; Integer) { }
        field(18; F18; Integer) { }
        field(19; F19; Integer) { }
        field(20; F20; Integer) { }
        field(21; F21; Integer) { }
        field(22; F22; Integer) { }
        field(23; F23; Integer) { }
        field(24; F24; Integer) { }
        field(25; F25; Integer) { }
        field(26; F26; Integer) { }
        field(27; F27; Integer) { }
        field(28; F28; Integer) { }
        field(29; F29; Integer) { }
        field(30; F30; Integer) { }
        field(31; F31; Integer) { }
        field(32; F32; Integer) { }
        field(33; F33; Integer) { }
        field(34; F34; Integer) { }
        field(35; F35; Integer) { }
        field(36; F36; Integer) { }
        field(37; F37; Integer) { }
        field(38; F38; Integer) { }
        field(39; F39; Integer) { }
        field(40; F40; Integer) { }
        field(41; F41; Integer) { }
        field(42; F42; Integer) { }
        field(43; F43; Integer) { }
        field(44; F44; Integer) { }
        field(45; F45; Integer) { }
        field(46; F46; Integer) { }
        field(47; F47; Integer) { }
        field(48; F48; Integer) { }
        field(49; F49; Integer) { }
        field(50; F50; Integer) { }
        field(51; F51; Integer) { }
        field(52; F52; Integer) { }
        field(53; F53; Integer) { }
        field(54; F54; Integer) { }
        field(55; F55; Integer) { }
        field(56; F56; Integer) { }
        field(57; F57; Integer) { }
        field(58; F58; Integer) { }
        field(59; F59; Integer) { }
        field(60; F60; Integer) { }
        field(61; F61; Integer) { }
        field(62; F62; Integer) { }
        field(63; F63; Integer) { }
        field(64; F64; Integer) { }
        field(65; F65; Integer) { }
        field(66; F66; Integer) { }
        field(67; F67; Integer) { }
        field(68; F68; Integer) { }
        field(69; F69; Integer) { }
        field(70; F70; Integer) { }
        field(71; F71; Integer) { }
        field(72; F72; Integer) { }
        field(73; F73; Integer) { }
        field(74; F74; Integer) { }
        field(75; F75; Integer) { }
        field(76; F76; Integer) { }
        field(77; F77; Integer) { }
        field(78; F78; Integer) { }
        field(79; F79; Integer) { }
        field(80; F80; Integer) { }
        field(81; F81; Integer) { }
        field(82; F82; Integer) { }
        field(83; F83; Integer) { }
        field(84; F84; Integer) { }
        field(85; F85; Integer) { }
        field(86; F86; Integer) { }
        field(87; F87; Integer) { }
        field(88; F88; Integer) { }
        field(89; F89; Integer) { }
        field(90; F90; Integer) { }
        field(91; F91; Integer) { }
        field(92; F92; Integer) { }
        field(93; F93; Integer) { }
        field(94; F94; Integer) { }
        field(95; F95; Integer) { }
        field(96; F96; Integer) { }
        field(97; F97; Integer) { }
        field(98; F98; Integer) { }
        field(99; F99; Integer) { }
        field(100; F100; Integer) { }
        field(101; F101; Integer) { }
        field(102; F102; Integer) { }
        field(103; F103; Integer) { }
        field(104; F104; Integer) { }
        field(105; F105; Integer) { }
        field(106; F106; Integer) { }
        field(107; F107; Integer) { }
        field(108; F108; Integer) { }
        field(109; F109; Integer) { }
        field(110; F110; Integer) { }
        field(111; F111; Integer) { }
        field(112; F112; Integer) { }
        field(113; F113; Integer) { }
        field(114; F114; Integer) { }
        field(115; F115; Integer) { }
        field(116; F116; Integer) { }
        field(117; F117; Integer) { }
        field(118; F118; Integer) { }
        field(119; F119; Integer) { }
        field(120; F120; Integer) { }
        field(121; F121; Integer) { }
        field(122; F122; Integer) { }
        field(123; F123; Integer) { }
        field(124; F124; Integer) { }
        field(125; F125; Integer) { }
        field(126; F126; Integer) { }
        field(127; F127; Integer) { }
        field(128; F128; Integer) { }
        field(129; F129; Integer) { }
        field(130; F130; Integer) { }
        field(131; F131; Integer) { }
        field(132; F132; Integer) { }
        field(133; F133; Integer) { }
        field(134; F134; Integer) { }
        field(135; F135; Integer) { }
        field(136; F136; Integer) { }
        field(137; F137; Integer) { }
        field(138; F138; Integer) { }
        field(139; F139; Integer) { }
        field(140; F140; Integer) { }
        field(141; F141; Integer) { }
        field(142; F142; Integer) { }
        field(143; F143; Integer) { }
        field(144; F144; Integer) { }
        field(145; F145; Integer) { }
        field(146; F146; Integer) { }
        field(147; F147; Integer) { }
        field(148; F148; Integer) { }
        field(149; F149; Integer) { }
        field(150; F150; Integer) { }
        field(151; F151; Integer) { }
        field(152; F152; Integer) { }
        field(153; F153; Integer) { }
        field(154; F154; Integer) { }
        field(155; F155; Integer) { }
        field(156; F156; Integer) { }
        field(157; F157; Integer) { }
        field(158; F158; Integer) { }
        field(159; F159; Integer) { }
        field(160; F160; Integer) { }
        field(161; F161; Integer) { }
        field(162; F162; Integer) { }
        field(163; F163; Integer) { }
        field(164; F164; Integer) { }
        field(165; F165; Integer) { }
        field(166; F166; Integer) { }
        field(167; F167; Integer) { }
        field(168; F168; Integer) { }
        field(169; F169; Integer) { }
        field(170; F170; Integer) { }
        field(171; F171; Integer) { }
        field(172; F172; Integer) { }
        field(173; F173; Integer) { }
        field(174; F174; Integer) { }
        field(175; F175; Integer) { }
        field(176; F176; Integer) { }
        field(177; F177; Integer) { }
        field(178; F178; Integer) { }
        field(179; F179; Integer) { }
        field(180; F180; Integer) { }
        field(181; F181; Integer) { }
        field(182; F182; Integer) { }
        field(183; F183; Integer) { }
        field(184; F184; Integer) { }
        field(185; F185; Integer) { }
        field(186; F186; Integer) { }
        field(187; F187; Integer) { }
        field(188; F188; Integer) { }
        field(189; F189; Integer) { }
        field(190; F190; Integer) { }
        field(191; F191; Integer) { }
        field(192; F192; Integer) { }
        field(193; F193; Integer) { }
        field(194; F194; Integer) { }
        field(195; F195; Integer) { }
        field(196; F196; Integer) { }
        field(197; F197; Integer) { }
        field(198; F198; Integer) { }
        field(199; F199; Integer) { }
        field(200; F200; Integer) { }
        field(201; F201; Integer) { }
        field(202; F202; Integer) { }
        field(203; F203; Integer) { }
        field(204; F204; Integer) { }
        field(205; F205; Integer) { }
        field(206; F206; Integer) { }
        field(207; F207; Integer) { }
        field(208; F208; Integer) { }
        field(209; F209; Integer) { }
        field(210; F210; Integer) { }
        field(211; F211; Integer) { }
        field(212; F212; Integer) { }
        field(213; F213; Integer) { }
        field(214; F214; Integer) { }
        field(215; F215; Integer) { }
        field(216; F216; Integer) { }
        field(217; F217; Integer) { }
        field(218; F218; Integer) { }
        field(219; F219; Integer) { }
        field(220; F220; Integer) { }
        field(221; F221; Integer) { }
        field(222; F222; Integer) { }
        field(223; F223; Integer) { }
        field(224; F224; Integer) { }
        field(225; F225; Integer) { }
        field(226; F226; Integer) { }
        field(227; F227; Integer) { }
        field(228; F228; Integer) { }
        field(229; F229; Integer) { }
        field(230; F230; Integer) { }
        field(231; F231; Integer) { }
        field(232; F232; Integer) { }
        field(233; F233; Integer) { }
        field(234; F234; Integer) { }
        field(235; F235; Integer) { }
        field(236; F236; Integer) { }
        field(237; F237; Integer) { }
        field(238; F238; Integer) { }
        field(239; F239; Integer) { }
        field(240; F240; Integer) { }
        field(241; F241; Integer) { }
        field(242; F242; Integer) { }
        field(243; F243; Integer) { }
        field(244; F244; Integer) { }
        field(245; F245; Integer) { }
        field(246; F246; Integer) { }
        field(247; F247; Integer) { }
        field(248; F248; Integer) { }
        field(249; F249; Integer) { }
        field(250; F250; Integer) { }
        field(251; F251; Integer) { }
        field(252; F252; Integer) { }
        field(253; F253; Integer) { }
        field(254; F254; Integer) { }
        field(255; F255; Integer) { }
        field(256; F256; Integer) { }
        field(257; F257; Integer) { }
        field(258; F258; Integer) { }
        field(259; F259; Integer) { }
        field(260; F260; Integer) { }
        field(261; F261; Integer) { }
        field(262; F262; Integer) { }
        field(263; F263; Integer) { }
        field(264; F264; Integer) { }
        field(265; F265; Integer) { }
        field(266; F266; Integer) { }
        field(267; F267; Integer) { }
        field(268; F268; Integer) { }
        field(269; F269; Integer) { }
        field(270; F270; Integer) { }
        field(271; F271; Integer) { }
        field(272; F272; Integer) { }
        field(273; F273; Integer) { }
        field(274; F274; Integer) { }
        field(275; F275; Integer) { }
        field(276; F276; Integer) { }
        field(277; F277; Integer) { }
        field(278; F278; Integer) { }
        field(279; F279; Integer) { }
        field(280; F280; Integer) { }
        field(281; F281; Integer) { }
        field(282; F282; Integer) { }
        field(283; F283; Integer) { }
        field(284; F284; Integer) { }
        field(285; F285; Integer) { }
        field(286; F286; Integer) { }
        field(287; F287; Integer) { }
        field(288; F288; Integer) { }
        field(289; F289; Integer) { }
        field(290; F290; Integer) { }
        field(291; F291; Integer) { }
        field(292; F292; Integer) { }
        field(293; F293; Integer) { }
        field(294; F294; Integer) { }
        field(295; F295; Integer) { }
        field(296; F296; Integer) { }
        field(297; F297; Integer) { }
        field(298; F298; Integer) { }
        field(299; F299; Integer) { }
        field(300; F300; Integer) { }
        field(301; F301; Integer) { }
        field(302; F302; Integer) { }
        field(303; F303; Integer) { }
        field(304; F304; Integer) { }
        field(305; F305; Integer) { }
        field(306; F306; Integer) { }
        field(307; F307; Integer) { }
        field(308; F308; Integer) { }
        field(309; F309; Integer) { }
        field(310; F310; Integer) { }
        field(311; F311; Integer) { }
        field(312; F312; Integer) { }
        field(313; F313; Integer) { }
        field(314; F314; Integer) { }
        field(315; F315; Integer) { }
        field(316; F316; Integer) { }
        field(317; F317; Integer) { }
        field(318; F318; Integer) { }
        field(319; F319; Integer) { }
        field(320; F320; Integer) { }
        field(321; F321; Integer) { }
        field(322; F322; Integer) { }
        field(323; F323; Integer) { }
        field(324; F324; Integer) { }
        field(325; F325; Integer) { }
        field(326; F326; Integer) { }
        field(327; F327; Integer) { }
        field(328; F328; Integer) { }
        field(329; F329; Integer) { }
        field(330; F330; Integer) { }
        field(331; F331; Integer) { }
        field(332; F332; Integer) { }
        field(333; F333; Integer) { }
        field(334; F334; Integer) { }
        field(335; F335; Integer) { }
        field(336; F336; Integer) { }
        field(337; F337; Integer) { }
        field(338; F338; Integer) { }
        field(339; F339; Integer) { }
        field(340; F340; Integer) { }
        field(341; F341; Integer) { }
        field(342; F342; Integer) { }
        field(343; F343; Integer) { }
        field(344; F344; Integer) { }
        field(345; F345; Integer) { }
        field(346; F346; Integer) { }
        field(347; F347; Integer) { }
        field(348; F348; Integer) { }
        field(349; F349; Integer) { }
        field(350; F350; Integer) { }
        field(351; F351; Integer) { }
        field(352; F352; Integer) { }
        field(353; F353; Integer) { }
        field(354; F354; Integer) { }
        field(355; F355; Integer) { }
        field(356; F356; Integer) { }
        field(357; F357; Integer) { }
        field(358; F358; Integer) { }
        field(359; F359; Integer) { }
        field(360; F360; Integer) { }
        field(361; F361; Integer) { }
        field(362; F362; Integer) { }
        field(363; F363; Integer) { }
        field(364; F364; Integer) { }
        field(365; F365; Integer) { }
        field(366; F366; Integer) { }
        field(367; F367; Integer) { }
        field(368; F368; Integer) { }
        field(369; F369; Integer) { }
        field(370; F370; Integer) { }
        field(371; F371; Integer) { }
        field(372; F372; Integer) { }
        field(373; F373; Integer) { }
        field(374; F374; Integer) { }
        field(375; F375; Integer) { }
        field(376; F376; Integer) { }
        field(377; F377; Integer) { }
        field(378; F378; Integer) { }
        field(379; F379; Integer) { }
        field(380; F380; Integer) { }
        field(381; F381; Integer) { }
        field(382; F382; Integer) { }
        field(383; F383; Integer) { }
        field(384; F384; Integer) { }
        field(385; F385; Integer) { }
        field(386; F386; Integer) { }
        field(387; F387; Integer) { }
        field(388; F388; Integer) { }
        field(389; F389; Integer) { }
        field(390; F390; Integer) { }
        field(391; F391; Integer) { }
        field(392; F392; Integer) { }
        field(393; F393; Integer) { }
        field(394; F394; Integer) { }
        field(395; F395; Integer) { }
        field(396; F396; Integer) { }
        field(397; F397; Integer) { }
        field(398; F398; Integer) { }
        field(399; F399; Integer) { }
        field(400; F400; Integer) { }
        field(401; F401; Integer) { }
        field(402; F402; Integer) { }
        field(403; F403; Integer) { }
        field(404; F404; Integer) { }
        field(405; F405; Integer) { }
        field(406; F406; Integer) { }
        field(407; F407; Integer) { }
        field(408; F408; Integer) { }
        field(409; F409; Integer) { }
        field(410; F410; Integer) { }
        field(411; F411; Integer) { }
        field(412; F412; Integer) { }
        field(413; F413; Integer) { }
        field(414; F414; Integer) { }
        field(415; F415; Integer) { }
        field(416; F416; Integer) { }
        field(417; F417; Integer) { }
        field(418; F418; Integer) { }
        field(419; F419; Integer) { }
        field(420; F420; Integer) { }
        field(421; F421; Integer) { }
        field(422; F422; Integer) { }
        field(423; F423; Integer) { }
        field(424; F424; Integer) { }
        field(425; F425; Integer) { }
        field(426; F426; Integer) { }
        field(427; F427; Integer) { }
        field(428; F428; Integer) { }
        field(429; F429; Integer) { }
        field(430; F430; Integer) { }
        field(431; F431; Integer) { }
        field(432; F432; Integer) { }
        field(433; F433; Integer) { }
        field(434; F434; Integer) { }
        field(435; F435; Integer) { }
        field(436; F436; Integer) { }
        field(437; F437; Integer) { }
        field(438; F438; Integer) { }
        field(439; F439; Integer) { }
        field(440; F440; Integer) { }
        field(441; F441; Integer) { }
        field(442; F442; Integer) { }
        field(443; F443; Integer) { }
        field(444; F444; Integer) { }
        field(445; F445; Integer) { }
        field(446; F446; Integer) { }
        field(447; F447; Integer) { }
        field(448; F448; Integer) { }
        field(449; F449; Integer) { }
        field(450; F450; Integer) { }
        field(451; F451; Integer) { }
        field(452; F452; Integer) { }
        field(453; F453; Integer) { }
        field(454; F454; Integer) { }
        field(455; F455; Integer) { }
        field(456; F456; Integer) { }
        field(457; F457; Integer) { }
        field(458; F458; Integer) { }
        field(459; F459; Integer) { }
        field(460; F460; Integer) { }
        field(461; F461; Integer) { }
        field(462; F462; Integer) { }
        field(463; F463; Integer) { }
        field(464; F464; Integer) { }
        field(465; F465; Integer) { }
        field(466; F466; Integer) { }
        field(467; F467; Integer) { }
        field(468; F468; Integer) { }
        field(469; F469; Integer) { }
        field(470; F470; Integer) { }
        field(471; F471; Integer) { }
        field(472; F472; Integer) { }
        field(473; F473; Integer) { }
        field(474; F474; Integer) { }
        field(475; F475; Integer) { }
        field(476; F476; Integer) { }
        field(477; F477; Integer) { }
        field(478; F478; Integer) { }
        field(479; F479; Integer) { }
        field(480; F480; Integer) { }
        field(481; F481; Integer) { }
        field(482; F482; Integer) { }
        field(483; F483; Integer) { }
        field(484; F484; Integer) { }
        field(485; F485; Integer) { }
        field(486; F486; Integer) { }
        field(487; F487; Integer) { }
        field(488; F488; Integer) { }
        field(489; F489; Integer) { }
        field(490; F490; Integer) { }
        field(491; F491; Integer) { }
        field(492; F492; Integer) { }
        field(493; F493; Integer) { }
        field(494; F494; Integer) { }
        field(495; F495; Integer) { }
        field(496; F496; Integer) { }
        field(497; F497; Integer) { }
        field(498; F498; Integer) { }
        field(499; F499; Integer) { }
        field(500; F500; Integer) { }
        field(501; F501; Integer) { }
        field(502; F502; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The worker of BQ Tests 9: the four reads of the filters a Mode names, on table 97593, 97596 or 97597.
codeunit 97528 "BQ Reader 9"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        Results.Add('Seen', Forms(Mode, Group));
        Page.SetBackgroundTaskResult(Results);
    end;

    procedure Forms(Mode: Text; Group: Text): Text
    var
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        Ref: RecordRef;
        N: Integer;
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Mode.StartsWith('widest-') then begin
            Evaluate(N, CopyStr(Mode, 8));
            SetWidest(Ref, N, Group);
            if Ref.FindFirst() then
                First := Format(Ref.Field(2).Value());
            if Ref.FindLast() then
                Last := Format(Ref.Field(2).Value());
            exit(First + '|' + Last + '|' + Format(Ref.Count()) + '|' + Format(Ref.IsEmpty()));
        end;
        if Mode.StartsWith('calc') then begin
            Calc.SetRange("No.", Group);
            case Mode of
                'calcK*':
                    Calc.SetFilter("Line Kind", 'K*');
                'calc*K':
                    Calc.SetFilter("Line Kind", '*K');
                'calc?':
                    Calc.SetFilter("Line Kind", '?');
                'calck*':
                    Calc.SetFilter("Line Kind", 'k*');
                'calcat':
                    Calc.SetFilter("Line Kind", '@');
            end;
            if Calc.FindFirst() then
                First := Calc.Description;
            if Calc.FindLast() then
                Last := Calc.Description;
            exit(First + '|' + Last + '|' + Format(Calc.Count()) + '|' + Format(Calc.IsEmpty()));
        end;
        case Mode of
            'keytwice', 'keytwiceplain':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(2);
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(0);
                end;
            'keysame':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetFilter("No.", Group);
                end;
            'openfrom':
                Wide.SetFilter("Child Count", '1..');
            'opento':
                Wide.SetFilter("Child Count", '..0');
            'andinor':
                Wide.SetFilter("Child Count", '>0&<5|9');
        end;
        if Mode <> 'keytwice' then
            if Mode <> 'keytwiceplain' then
                if Mode <> 'keysame' then
                    Wide.SetRange("No.", Group);
        if Mode in ['keytwice', 'keysame'] then
            Wide.SetRange("Child Count", 1);
        if Mode = 'keytwiceplain' then
            Wide.SetRange(Description, 'a');
        if Wide.FindFirst() then
            First := Wide.Description;
        if Wide.FindLast() then
            Last := Wide.Description;
        exit(First + '|' + Last + '|' + Format(Wide.Count()) + '|' + Format(Wide.IsEmpty()));
    end;

    procedure SetWidest(var Ref: RecordRef; N: Integer; Group: Text)
    var
        Groups: List of [Integer];
        K: Integer;
        G: Integer;
        FieldNo: Integer;
    begin
        Ref.Open(Database::"BQ Widest");
        Groups.Add(0);
        for G := 2 to 255 do
            Groups.Add(G);
        Groups.Add(-1);
        Groups.Add(1);
        for K := 0 to N - 1 do begin
            FieldNo := K mod 502 + 1;
            Ref.FilterGroup(Groups.Get(K div 502 + 1));
            case FieldNo of
                1:
                    Ref.Field(1).SetRange(Group);
                2:
                    Ref.Field(2).SetFilter('<>zz');
                else
                    Ref.Field(FieldNo).SetRange(0);
            end;
        end;
        Ref.FilterGroup(0);
    end;
}

codeunit 97539 "BQ Tests 9"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader 9", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // Inserts a line of table 97594 (Amount 5), then reads the parent's lines.
    local procedure AddChild(Parent: Code[20]; LineNo: Integer; Kind: Code[10])
    var
        Child: Record "BQ Child";
    begin
        Child.Parent := Parent;
        Child."Line No." := LineNo;
        Child.Kind := Kind;
        Child.Amount := 5;
        Child.Insert();
        Child.SetRange(Parent, Parent);
        if Child.FindFirst() then;
    end;

    // The row a Mode reads committed (table 97593 or 97597, Description a; none for 'widest'); the four reads; the test's
    // change without a commit: a row of table 97596 for 'widest', Description b for 'keytwiceplain', else a line of table 97594
    // (Kind K); the four reads again: both answers, as 'first / again'.
    local procedure Reads(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        Widest: Record "BQ Widest";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        if Mode.StartsWith('calc') then begin
            Calc."No." := G;
            Calc.Description := 'a';
            Calc.Insert();
        end else
            if not Mode.StartsWith('widest') then begin
                Wide."No." := G;
                Wide.Description := 'a';
                Wide.Insert();
            end;
        Commit();
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        if Mode.StartsWith('widest') then begin
            Widest."No." := G;
            Widest.Description := 'a';
            Widest.Insert();
            Widest.SetRange("No.", G);
            if Widest.FindFirst() then;
        end else
            if Mode = 'keytwiceplain' then begin
                Wide.Get(G);
                Wide.Description := 'b';
                Wide.Modify();
            end else
                AddChild(G, 1, 'K');
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // The Modes whose Reads answer is not Expected, with their answers.
    local procedure Differ(Modes: List of [Text]; Expected: Text): Text
    var
        Mode: Text;
        Seen: Text;
        Found: Text;
    begin
        foreach Mode in Modes do begin
            Seen := Reads(Mode);
            if Seen <> Expected then
                Found += ' ' + Mode + ': ' + Seen;
        end;
        exit(Found);
    end;

    // 26215 and 60000 filters on table 97596 (502 fields in filter groups 0, 2 and on).
    [Test]
    procedure BgQueryCache_Run_AllForms_26215And60000Filters_FirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('widest-26215', 'widest-60000');
        Assert.AreEqual('', Differ(Modes, '-|-|0|Yes / -|-|0|Yes'), 'the Modes that differ');
    end;

    // 100000 filters on table 97596.
    [Test]
    procedure BgQueryCache_Run_AllForms_100000Filters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', Reads('widest-100000'), '100000 filters');
    end;

    // The key set twice to its value next to "Child Count" 1: in filter groups 0 and 2, and by SetRange then SetFilter.
    [Test]
    procedure BgQueryCache_Run_FlowFieldKeyTwice_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('keytwice', 'keysame');
        Assert.AreEqual('', Differ(Modes, '-|-|0|Yes / -|-|1|No'), 'the Modes that differ');
    end;

    // The key in filter groups 0 and 2 next to Description a, no FlowField; the test sets the Description to b.
    [Test]
    procedure BgQueryCache_Run_AllForms_KeyTwice_FirstAnswer()
    begin
        Assert.AreEqual('a|a|1|No / a|a|1|No', Reads('keytwiceplain'), 'after the change');
    end;

    // "Child Count" 1.. and >0&<5|9, then ..0, next to the key.
    [Test]
    procedure BgQueryCache_Run_FlowFieldOpenRangesAndAndInsideOr_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('openfrom', 'andinor');
        Assert.AreEqual('', Differ(Modes, '-|-|0|Yes / -|-|1|No'), '1.. and >0&<5|9');
        Assert.AreEqual('a|a|1|No / a|a|0|Yes', Reads('opento'), '..0');
    end;

    // "Line Kind" (a lookup) K*, *K, ? and k* next to the key.
    [Test]
    procedure BgQueryCache_Run_FlowFieldCaseSensitiveWildcards_FindsFirstAnswer()
    var
        Modes: List of [Text];
    begin
        Modes.AddRange('calcK*', 'calc*K', 'calc?', 'calck*');
        Assert.AreEqual('', Differ(Modes, '-|-|0|Yes / -|-|1|No'), 'the Modes that differ');
    end;

    // "Line Kind" @ alone next to the key: it keeps the row while the lookup is empty.
    [Test]
    procedure BgQueryCache_Run_FlowFieldAtSignAlone_FindsFirstAnswer()
    begin
        Assert.AreEqual('a|a|1|No / a|a|0|Yes', Reads('calcat'), '@');
    end;

    // @ alone on a Code field of the test session's own lines (Kind K on line 1, empty on line 2), @k, and GetFilter of @.
    [Test]
    procedure AtSignAlone_CodeField_KeepsTheEmptyValues()
    var
        Child: Record "BQ Child";
        G: Code[20];
    begin
        G := NewGroup();
        AddChild(G, 1, 'K');
        AddChild(G, 2, '');
        Child.SetRange(Parent, G);
        Child.SetFilter(Kind, '@');
        Assert.AreEqual(1, Child.Count(), 'the lines @ keeps');
        Child.FindFirst();
        Assert.AreEqual(2, Child."Line No.", 'the line @ keeps');
        Assert.AreEqual('@', Child.GetFilter(Kind), 'GetFilter');
        Child.SetFilter(Kind, '@k');
        Assert.AreEqual(1, Child.Count(), 'the lines @k keeps');
        Child.FindFirst();
        Assert.AreEqual(1, Child."Line No.", 'the line @k keeps');
    end;
}


// BQ Tests 10 (al-runner issue 8677, a page background task's CalcFields and CalcSums; measured on a local BC 29.1 OnPrem w1
// container). Each Mode commits a parent of tables 97597 and 97557 with line 1 of table 97594 (Kind A, Amount 5; none for
// 'exist') and opens a page; the worker reads; the test session inserts line 0 (Kind K, Amount 2) and line 2 (Kind B, Amount
// 11), or a note of table 97595 for the FlowFields over the notes, without a commit; the worker reads again: 'first / again'.
// A worker's CalcFields answers its first answer for a FlowField of every method (count, sum, min, max, average, exist,
// lookup), for several FlowFields at once, for a FlowFilter, and for a sum whose field() is a FlowField of the row; a commit
// of the lines, or of the notes the inner FlowField counts, ends it. The entry is the call: the same FlowFields in another
// order, one then several, several then one, another FlowFilter value, SetAutoCalcFields with FindFirst, and a count whose
// condition is a FlowField of its source read the table. The test session's CalcFields before its insert answers the
// worker's first one; without it the worker reads the insert. CalcSums answers its first answer, of one field or two, with a
// filter outside the key too.
// Fixtures (the header's Fixtures used lines have no room): table 97557, worker codeunit 97552, test codeunit 97553.

table 97557 "BQ Calc Filtered"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Kind Filter"; Code[10]) { FieldClass = FlowFilter; }
        field(3; "Kind Sum"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Child".Amount where(Parent = field("No."), Kind = field("Kind Filter")));
        }
        field(4; "Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Note" where(Parent = field("No.")));
        }
        field(5; "Amount At Note Count"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("BQ Child".Amount where(Parent = field("No."), "Line No." = field("Note Count")));
        }
        field(6; "Lone Lines"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("BQ Child" where(Parent = field("No."), "Sibling Count" = const(1)));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 97552 "BQ Reader 10"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
        Round: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        Parameters.Get('Round', Round);
        Results.Add('Seen', Calc(Mode, Group, Round = '2'));
        Page.SetBackgroundTaskResult(Results);
    end;

    local procedure D(Value: Decimal): Text
    begin
        exit(Format(Value, 0, 9));
    end;

    procedure Calc(Mode: Text; Group: Text; Again: Boolean): Text
    var
        Parent: Record "BQ Calc";
        Filtered: Record "BQ Calc Filtered";
        Child: Record "BQ Child";
    begin
        case Mode of
            'count', 'commit', 'testfirst', 'testinsertfirst':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Line Count");
                    exit(Format(Parent."Line Count"));
                end;
            'sum':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Amount Sum");
                    exit(D(Parent."Amount Sum"));
                end;
            'min':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Amount Min");
                    exit(D(Parent."Amount Min"));
                end;
            'max':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Amount Max");
                    exit(D(Parent."Amount Max"));
                end;
            'average':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Amount Average");
                    exit(D(Parent."Amount Average"));
                end;
            'exist':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Has Lines");
                    exit(Format(Parent."Has Lines"));
                end;
            'lookup':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Line Kind");
                    exit(Parent."Line Kind");
                end;
            'several':
                begin
                    Parent.Get(Group);
                    Parent.CalcFields("Line Count", "Amount Sum", "Amount Min", "Amount Max", "Line Kind");
                    exit(Format(Parent."Line Count") + ',' + D(Parent."Amount Sum") + ',' + D(Parent."Amount Min") + ',' +
                        D(Parent."Amount Max") + ',' + Parent."Line Kind");
                end;
            'onethenseveral':
                begin
                    Parent.Get(Group);
                    if Again then begin
                        Parent.CalcFields("Amount Sum", "Line Count");
                        exit(D(Parent."Amount Sum") + ',' + Format(Parent."Line Count"));
                    end;
                    Parent.CalcFields("Amount Sum");
                    exit(D(Parent."Amount Sum"));
                end;
            'severalthenone':
                begin
                    Parent.Get(Group);
                    if Again then begin
                        Parent.CalcFields("Amount Sum");
                        exit(D(Parent."Amount Sum"));
                    end;
                    Parent.CalcFields("Amount Sum", "Line Count");
                    exit(D(Parent."Amount Sum") + ',' + Format(Parent."Line Count"));
                end;
            'severalswapped':
                begin
                    Parent.Get(Group);
                    if Again then
                        Parent.CalcFields("Line Count", "Amount Sum")
                    else
                        Parent.CalcFields("Amount Sum", "Line Count");
                    exit(D(Parent."Amount Sum") + ',' + Format(Parent."Line Count"));
                end;
            'auto':
                begin
                    Parent.SetAutoCalcFields("Line Count", "Amount Sum");
                    Parent.SetRange("No.", Group);
                    Parent.FindFirst();
                    exit(Format(Parent."Line Count") + ',' + D(Parent."Amount Sum"));
                end;
            'flowfilter':
                begin
                    Filtered.Get(Group);
                    Filtered.SetFilter("Kind Filter", 'A|K');
                    Filtered.CalcFields("Kind Sum");
                    exit(D(Filtered."Kind Sum"));
                end;
            'flowfilterother':
                begin
                    Filtered.Get(Group);
                    if Again then
                        Filtered.SetFilter("Kind Filter", 'K')
                    else
                        Filtered.SetFilter("Kind Filter", 'A|K');
                    Filtered.CalcFields("Kind Sum");
                    exit(D(Filtered."Kind Sum"));
                end;
            'notecount':
                begin
                    Filtered.Get(Group);
                    Filtered.CalcFields("Note Count");
                    exit(Format(Filtered."Note Count"));
                end;
            'atnotecount', 'atnotecountcommit':
                begin
                    Filtered.Get(Group);
                    Filtered.CalcFields("Amount At Note Count");
                    exit(D(Filtered."Amount At Note Count"));
                end;
            'lonelines':
                begin
                    Filtered.Get(Group);
                    Filtered.CalcFields("Lone Lines");
                    exit(Format(Filtered."Lone Lines"));
                end;
            'calcsums':
                begin
                    Child.SetRange(Parent, Group);
                    Child.CalcSums(Amount);
                    exit(D(Child.Amount));
                end;
            'calcsumstwo':
                begin
                    Child.SetRange(Parent, Group);
                    Child.CalcSums(Amount, "Line No.");
                    exit(D(Child.Amount) + ',' + Format(Child."Line No."));
                end;
            'calcsumsfilter':
                begin
                    Child.SetRange(Parent, Group);
                    Child.SetFilter(Kind, 'A|K');
                    Child.CalcSums(Amount);
                    exit(D(Child.Amount));
                end;
        end;
        Error('Mode %1', Mode);
    end;
}

codeunit 97553 "BQ Tests 10"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]; Round: Text): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Parameters.Add('Round', Round);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader 10", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // Inserts a line of table 97594, then reads the parent's lines.
    local procedure AddChild(Parent: Code[20]; LineNo: Integer; Kind: Code[10]; Amount: Decimal)
    var
        Child: Record "BQ Child";
    begin
        Child.Parent := Parent;
        Child."Line No." := LineNo;
        Child.Kind := Kind;
        Child.Amount := Amount;
        Child.Insert();
        Child.SetRange(Parent, Parent);
        if Child.FindFirst() then;
    end;

    // The worker's read before and after the test session's change, as 'first / again'.
    local procedure Reads(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Parent: Record "BQ Calc";
        Filtered: Record "BQ Calc Filtered";
        Note: Record "BQ Note";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Parent."No." := G;
        Parent.Insert();
        Filtered."No." := G;
        Filtered.Insert();
        if Mode <> 'exist' then
            AddChild(G, 1, 'A', 5);
        Commit();
        Rows.OpenView();
        if Mode in ['testfirst', 'testinsertfirst'] then begin
            if Mode = 'testfirst' then
                Parent.CalcFields("Line Count");
            AddChild(G, 0, 'K', 2);
            AddChild(G, 2, 'B', 11);
        end;
        Seen := Run(Rows, Mode, G, '1');
        case Mode of
            'testfirst', 'testinsertfirst':
                ;
            'notecount', 'atnotecount', 'atnotecountcommit':
                begin
                    Note.Parent := G;
                    Note."Line No." := 1;
                    Note.Insert();
                    Note.SetRange(Parent, G);
                    if Note.FindFirst() then;
                end;
            else begin
                AddChild(G, 0, 'K', 2);
                AddChild(G, 2, 'B', 11);
            end;
        end;
        if Mode in ['commit', 'atnotecountcommit'] then
            Commit();
        Seen += ' / ' + Run(Rows, Mode, G, '2');
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure BgCalcFields_Count_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('count'), 'count');
    end;

    [Test]
    procedure BgCalcFields_Sum_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('sum'), 'sum');
    end;

    [Test]
    procedure BgCalcFields_Min_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('min'), 'min');
    end;

    [Test]
    procedure BgCalcFields_Max_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('max'), 'max');
    end;

    [Test]
    procedure BgCalcFields_Average_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('average'), 'average');
    end;

    [Test]
    procedure BgCalcFields_Exist_FirstAnswer()
    begin
        Assert.AreEqual('No / No', Reads('exist'), 'exist');
    end;

    [Test]
    procedure BgCalcFields_Lookup_FirstAnswer()
    begin
        Assert.AreEqual('A / A', Reads('lookup'), 'lookup');
    end;

    [Test]
    procedure BgCalcFields_Several_FirstAnswer()
    begin
        Assert.AreEqual('1,5,5,5,A / 1,5,5,5,A', Reads('several'), 'several');
    end;

    [Test]
    procedure BgCalcFields_OneThenSeveral_SeesTheChange()
    begin
        Assert.AreEqual('5 / 18,3', Reads('onethenseveral'), 'one, then several');
    end;

    [Test]
    procedure BgCalcFields_SeveralThenOne_SeesTheChange()
    begin
        Assert.AreEqual('5,1 / 18', Reads('severalthenone'), 'several, then one');
    end;

    [Test]
    procedure BgCalcFields_SeveralSwapped_SeesTheChange()
    begin
        Assert.AreEqual('5,1 / 18,3', Reads('severalswapped'), 'the same fields in another order');
    end;

    [Test]
    procedure BgCalcFields_SetAutoCalcFields_SeesTheChange()
    begin
        Assert.AreEqual('1,5 / 3,18', Reads('auto'), 'SetAutoCalcFields');
    end;

    [Test]
    procedure BgCalcFields_FlowFilter_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('flowfilter'), 'a FlowFilter');
    end;

    [Test]
    procedure BgCalcFields_OtherFlowFilter_SeesTheChange()
    begin
        Assert.AreEqual('5 / 2', Reads('flowfilterother'), 'another FlowFilter');
    end;

    [Test]
    procedure BgCalcFields_CommitOfTheLines_SeesTheChange()
    begin
        Assert.AreEqual('1 / 3', Reads('commit'), 'after the commit');
    end;

    [Test]
    procedure BgCalcFields_TestCalcFieldsFirst_FirstAnswer()
    begin
        Assert.AreEqual('1 / 1', Reads('testfirst'), 'the test session first');
    end;

    [Test]
    procedure BgCalcFields_TestInsertFirst_SeesTheChange()
    begin
        Assert.AreEqual('3 / 3', Reads('testinsertfirst'), 'the test session inserts first');
    end;

    [Test]
    procedure BgCalcFields_CountOfNotes_FirstAnswer()
    begin
        Assert.AreEqual('0 / 0', Reads('notecount'), 'the notes');
    end;

    [Test]
    procedure BgCalcFields_SumAtAFlowFieldOfTheRow_FirstAnswer()
    begin
        Assert.AreEqual('0 / 0', Reads('atnotecount'), 'field() of a FlowField of the row');
    end;

    [Test]
    procedure BgCalcFields_SumAtAFlowFieldOfTheRow_CommitOfTheNote_SeesTheChange()
    begin
        Assert.AreEqual('0 / 5', Reads('atnotecountcommit'), 'after the commit of the note');
    end;

    [Test]
    procedure BgCalcFields_CountOverAFlowFieldOfTheSource_SeesTheChange()
    begin
        Assert.AreEqual('1 / 0', Reads('lonelines'), 'a condition on a FlowField of the source');
    end;

    [Test]
    procedure BgCalcSums_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('calcsums'), 'CalcSums');
    end;

    [Test]
    procedure BgCalcSums_TwoFields_FirstAnswer()
    begin
        Assert.AreEqual('5,1 / 5,1', Reads('calcsumstwo'), 'CalcSums of two fields');
    end;

    [Test]
    procedure BgCalcSums_FilterOutsideTheKey_FirstAnswer()
    begin
        Assert.AreEqual('5 / 5', Reads('calcsumsfilter'), 'CalcSums with a filter outside the key');
    end;
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
