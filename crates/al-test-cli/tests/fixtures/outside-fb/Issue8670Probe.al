// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-filtergroup-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
// Scope: in-scope
// local object: what a page background task's worker reads of a table on which the test session holds a filter in filter
//   group 1, and what the test session holds after the task, are what is under test
// Fixtures used: Assert (60021); tables 97550, 97551, 97552; pages 97553, 97554; codeunit 97555; test codeunit 97556
// Probe (al-runner issue 8670): the test session sets a filter in filter group 1 (the global group) on a record of G1W Row,
//   then runs a page background task. The worker runs in a session of its own: a fresh record of the table reads every row and
//   holds no filter in filter group 1. Each test deletes the rows of the three tables, inserts three rows A (Tag 0), B (Tag 7)
//   and C (Tag 0) in that key order (and the same in G1W Other), commits them, and takes back its filter group 1 filters
//   before it asserts.
// Readings: what the worker read (the result of RunPageBackgroundTask, or the page field its completion trigger sets); what the
//   test session's records read after the task.
// Cells:
//   - the worker's fresh record, the test session holding SetRange(Tag, 7) in filter group 1: Count and GetFilters in filter
//     group 1 (:322), Count (:353), FindFirst (:366), FindLast (:379), IsEmpty under SetRange(Tag, 9) (:392), a FindSet loop
//     (:405), Get of a row the filter excludes (:418), GetFilters (:431) and GetFilter (:444) in filter group 1, a RecordRef's
//     Count and GetFilters in filter group 1 (:457);
//   - the same, the task enqueued by the page's OnAfterGetCurrRecord (CurrPage.EnqueueBackgroundTask) (:471);
//   - the test session's filter on another table, G1W Other: the worker reads G1W Row (:336);
//   - the worker sets its own filter in filter group 1 while the test session holds one (:489): the worker's record and a fresh
//     record of the worker read the worker's filter; after the task the test session holds its own filter, unchanged;
//   - the worker sets a filter in filter group 1 while the test session holds none (:506): after the task a fresh record of the
//     test session reads every row and holds no filter in filter group 1;
//   - after a task whose worker only reads, the test session's records still read its filter (:521);
//   - two tasks in a row (:539), and a task after one whose worker set a filter in filter group 1 (:556).
// BC versions: 29.1 (local W1 container).

table 97550 "G1W Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Tag; Integer) { }
        field(3; Label; Text[10]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97551 "G1W Other"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Tag; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97552 "G1W Host"
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

// The page the test runs its tasks on.
page 97553 "G1W Hosts"
{
    PageType = List;
    SourceTable = "G1W Host";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
            }
        }
    }
}

// A page that enqueues the worker's first read for its current row and shows the answer.
page 97554 "G1W Enqueue Hosts"
{
    PageType = List;
    SourceTable = "G1W Host";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field(Seen; Seen)
                {
                    Editable = false;
                }
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Parameters: Dictionary of [Text, Text];
        TaskId: Integer;
    begin
        Seen := '';
        if Rec."No." = '' then
            exit;
        Parameters.Add('Mode', 'repro');
        Parameters.Add('RowKey', '');
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"G1W Reader", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        Seen := CopyStr(Results.Get('Seen'), 1, MaxStrLen(Seen));
    end;

    var
        Seen: Text[50];
}

// The worker: one read with fresh records, chosen by the Mode parameter.
codeunit 97555 "G1W Reader"
{
    trigger OnRun()
    var
        Row: Record "G1W Row";
        Fresh: Record "G1W Row";
        RowRef: RecordRef;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        case Parameters.Get('Mode') of
            'repro':
                begin
                    Seen := Format(Row.Count());
                    Row.FilterGroup(1);
                    Seen += '|' + Row.GetFilters();
                end;
            'count':
                Seen := Format(Row.Count());
            'first':
                if Row.FindFirst() then
                    Seen := Row.Label
                else
                    Seen := '-';
            'last':
                if Row.FindLast() then
                    Seen := Row.Label
                else
                    Seen := '-';
            'isempty':
                if Row.IsEmpty() then
                    Seen := 'empty'
                else
                    Seen := 'rows';
            'loop':
                if Row.FindSet() then
                    repeat
                        Seen += Row.Label;
                    until Row.Next() = 0;
            'get':
                if Row.Get(Parameters.Get('RowKey')) then
                    Seen := Row.Label
                else
                    Seen := '-';
            'getfilters':
                begin
                    Row.FilterGroup(1);
                    Seen := Row.GetFilters();
                end;
            'getfilter':
                begin
                    Row.FilterGroup(1);
                    Seen := Row.GetFilter(Tag);
                end;
            'ref':
                begin
                    RowRef.Open(Database::"G1W Row");
                    Seen := Format(RowRef.Count());
                    RowRef.FilterGroup(1);
                    Seen += '|' + RowRef.GetFilters();
                    RowRef.Close();
                end;
            'own':
                begin
                    Row.FilterGroup(1);
                    Row.SetRange(Tag, 0);
                    Row.FilterGroup(0);
                    Seen := Format(Row.Count()) + '|' + Format(Fresh.Count());
                    Fresh.FilterGroup(1);
                    Seen += '|' + Fresh.GetFilters();
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97556 "G1W Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        KeyA: Code[20];

    local procedure NewKey(Prefix: Text): Code[20]
    begin
        exit(CopyStr(Prefix + DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    // Rows A (Tag 0), B (Tag 7) and C (Tag 0) of G1W Row and G1W Other, one row of G1W Host, committed: a worker's read of a
    // table sees a committed change of it.
    local procedure Initialize()
    var
        Row: Record "G1W Row";
        Other: Record "G1W Other";
        Host: Record "G1W Host";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Host.DeleteAll();
        KeyA := NewKey('A');
        AddRow(KeyA, 0, 'A');
        AddRow(NewKey('B'), 7, 'B');
        AddRow(NewKey('C'), 0, 'C');
        Host."No." := NewKey('H');
        Host.Insert();
        Commit();
    end;

    local procedure AddRow(RowKey: Code[20]; Tag: Integer; Label: Text[10])
    var
        Row: Record "G1W Row";
        Other: Record "G1W Other";
    begin
        Row."No." := RowKey;
        Row.Tag := Tag;
        Row.Label := Label;
        Row.Insert();
        Other."No." := RowKey;
        Other.Tag := Tag;
        Other.Insert();
    end;

    // The worker run by the test on an open page, without its completion trigger.
    local procedure Run(Mode: Text; RowKey: Code[20]): Text
    var
        Hosts: TestPage "G1W Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Mode', Mode);
        Parameters.Add('RowKey', RowKey);
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"G1W Reader", Parameters, false);
        Hosts.Close();
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure SetGroup1(var Row: Record "G1W Row"; Tag: Integer)
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag, Tag);
        Row.FilterGroup(0);
    end;

    local procedure ClearGroup1(var Row: Record "G1W Row")
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag);
        Row.FilterGroup(0);
    end;

    // The test session's records of G1W Row after the task: a fresh record's Count, and the filter group 1 filters of a fresh
    // record and of the record that set the filter.
    local procedure TestSide(var Row: Record "G1W Row"): Text
    var
        Fresh: Record "G1W Row";
        Seen: Text;
    begin
        Seen := Format(Fresh.Count());
        Fresh.FilterGroup(1);
        Seen += '|' + Fresh.GetFilters();
        Row.FilterGroup(1);
        Seen += '|' + Row.GetFilters();
        Row.FilterGroup(0);
        exit(Seen);
    end;

    // The issue's shape: the worker's fresh record counts every row and holds no filter in filter group 1.
    [Test]
    procedure WorkerFreshRecord_CountAndGroup1Filters_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('repro', '');
        ClearGroup1(Row);
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    // The test session's filter in filter group 1 is on G1W Other; the worker reads G1W Row.
    [Test]
    procedure WorkerFreshRecord_Group1FilterOnAnotherTable_EveryRowNoFilter()
    var
        Other: Record "G1W Other";
        Seen: Text;
    begin
        Initialize();
        Other.FilterGroup(1);
        Other.SetRange(Tag, 7);
        Other.FilterGroup(0);
        Seen := Run('repro', '');
        Other.FilterGroup(1);
        Other.SetRange(Tag);
        Other.FilterGroup(0);
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    [Test]
    procedure WorkerFreshRecord_Count_EveryRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('count', '');
        ClearGroup1(Row);
        Assert.AreEqual('3', Seen, 'the worker''s Count');
    end;

    [Test]
    procedure WorkerFreshRecord_FindFirst_FirstRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('first', '');
        ClearGroup1(Row);
        Assert.AreEqual('A', Seen, 'the worker''s FindFirst');
    end;

    [Test]
    procedure WorkerFreshRecord_FindLast_LastRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('last', '');
        ClearGroup1(Row);
        Assert.AreEqual('C', Seen, 'the worker''s FindLast');
    end;

    [Test]
    procedure WorkerFreshRecord_IsEmpty_FilterThatKeepsNoRow_HasRows()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 9);
        Seen := Run('isempty', '');
        ClearGroup1(Row);
        Assert.AreEqual('rows', Seen, 'the worker''s IsEmpty');
    end;

    [Test]
    procedure WorkerFreshRecord_FindSetLoop_EveryRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('loop', '');
        ClearGroup1(Row);
        Assert.AreEqual('ABC', Seen, 'the worker''s FindSet loop');
    end;

    [Test]
    procedure WorkerFreshRecord_Get_RowTheFilterExcludes_Found()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('get', KeyA);
        ClearGroup1(Row);
        Assert.AreEqual('A', Seen, 'the worker''s Get of row A (Tag 0)');
    end;

    [Test]
    procedure WorkerFreshRecord_GetFiltersInGroup1_Empty()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('getfilters', '');
        ClearGroup1(Row);
        Assert.AreEqual('', Seen, 'the worker''s GetFilters in filter group 1');
    end;

    [Test]
    procedure WorkerFreshRecord_GetFilterInGroup1_Empty()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('getfilter', '');
        ClearGroup1(Row);
        Assert.AreEqual('', Seen, 'the worker''s GetFilter(Tag) in filter group 1');
    end;

    [Test]
    procedure WorkerRecordRef_CountAndGroup1Filters_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('ref', '');
        ClearGroup1(Row);
        Assert.AreEqual('3|', Seen, 'the worker''s RecordRef Count and its filters in filter group 1');
    end;

    // The task the page's OnAfterGetCurrRecord enqueues: its completion trigger shows the worker's answer.
    [Test]
    procedure EnqueuedTask_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Hosts: TestPage "G1W Enqueue Hosts";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Hosts.OpenView();
        Hosts.First();
        Seen := Hosts.Seen.Value();
        Hosts.Close();
        ClearGroup1(Row);
        Assert.AreEqual('3|', Seen, 'the page''s answer');
    end;

    // The worker sets SetRange(Tag, 0) in filter group 1 while the test session holds SetRange(Tag, 7) there.
    [Test]
    procedure WorkerSetsGroup1_WhileTestHoldsOne_WorkerReadsItsOwn_TestKeepsItsOwn()
    var
        Row: Record "G1W Row";
        Seen: Text;
        After: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run('own', '');
        After := TestSide(Row);
        ClearGroup1(Row);
        Assert.AreEqual('2|2|Tag: 0', Seen, 'the worker''s record Count, a fresh record''s Count, its filters in filter group 1');
        Assert.AreEqual('1|Tag: 7|Tag: 7', After, 'the test session after the task');
    end;

    // The worker sets SetRange(Tag, 0) in filter group 1 while the test session holds no filter there.
    [Test]
    procedure WorkerSetsGroup1_TestHoldsNone_EndsWithTheWorker()
    var
        Row: Record "G1W Row";
        Seen: Text;
        After: Text;
    begin
        Initialize();
        Seen := Run('own', '');
        After := TestSide(Row);
        Assert.AreEqual('2|2|Tag: 0', Seen, 'the worker''s record Count, a fresh record''s Count, its filters in filter group 1');
        Assert.AreEqual('3||', After, 'the test session after the task');
    end;

    // After a task whose worker only reads, the test session's records read its filter as before.
    [Test]
    procedure TestSession_AfterTheTask_KeepsItsGroup1Filter()
    var
        Row: Record "G1W Row";
        Before: Text;
        Seen: Text;
        After: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Before := TestSide(Row);
        Seen := Run('count', '');
        After := TestSide(Row);
        ClearGroup1(Row);
        Assert.AreEqual('1|Tag: 7|Tag: 7', Before, 'the test session before the task');
        Assert.AreEqual('1|Tag: 7|Tag: 7', After, 'the test session after the task');
    end;

    [Test]
    procedure TwoTasks_EachWorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        First: Text;
        Second: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        First := Run('repro', '');
        Second := Run('repro', '');
        ClearGroup1(Row);
        Assert.AreEqual('3|', First, 'the first worker');
        Assert.AreEqual('3|', Second, 'the second worker');
    end;

    // The first worker sets SetRange(Tag, 0) in filter group 1; the second worker's fresh record does not read it.
    [Test]
    procedure TwoTasks_AfterAWorkerSetGroup1_NextWorkerEveryRowNoFilter()
    var
        Row: Record "G1W Row";
        First: Text;
        Second: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        First := Run('own', '');
        Second := Run('repro', '');
        ClearGroup1(Row);
        Assert.AreEqual('2|2|Tag: 0', First, 'the first worker');
        Assert.AreEqual('3|', Second, 'the second worker');
    end;
}
