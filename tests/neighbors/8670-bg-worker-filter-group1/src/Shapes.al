// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-filtergroup-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
// Scope: in-scope
// local object: the shapes next to the cells of BgTaskWorkerFilterGroup1.al (who sets the test session's filter in filter group
//   1 and in which form, what the worker reads, writes and sets) are what is under test
// Fixtures used: Assert (60021); the tables 97550 to 97552, the page 97553 and the worker codeunit 97555 of
//   BgTaskWorkerFilterGroup1.al; table 97558; pages 97558, 97559; codeunits 97560 to 97562; test codeunits 97563, 97564
// Probe (al-runner issue 8670, the review of al-runner pull request 8676): the shapes next to BgTaskWorkerFilterGroup1.al. Each
//   test deletes the rows of its tables, inserts three rows A (Tag 0), B (Tag 7) and C (Tag 0) of G1W Row in that key order,
//   commits them, sets a filter in filter group 1 (SetRange(Tag, 7) on a record of G1W Row unless the cell says otherwise), runs
//   a page background task with TestPage.RunPageBackgroundTask, takes its filter back unless the cell says otherwise, and then
//   asserts.
// Readings: what the worker read (the result of RunPageBackgroundTask, or the row its completion trigger stores); the error a
//   failing worker raised; what the test session's records read after the task.
// Cells:
//   - who set the test session's filter: an event subscriber on the test's record (:319), a temporary record (:337), a
//     RecordRef (:355), the OnOpenPage trigger of the page the task runs on (:375);
//   - the form of the test session's filter: SetFilter(Tag, '7|8') (:399), SetRange on the primary key field (:415), filters on
//     two fields (:436);
//   - what the worker reads: a record with a filter of its own in filter group 0 (:456), Find('-') (:469), Find('>') (:482),
//     FindFirst then Next (:495), CalcSums while the test session's filter is SetRange(Tag, 0) (:508), a temporary record it
//     fills (:521), GetFilters after Reset, Clear and Init, then Count (:534);
//   - what the worker writes: Insert (:547) and Modify (:559), which raise the read-only session's error;
//   - the completion trigger, which runs in the test session (:573);
//   - a worker that sets a filter in filter group 1 and then fails (:595);
//   - a worker that takes back the filter in filter group 1 (SetRange(Tag)) while the test session holds one: the test session
//     keeps its filter (:657), also in the next test (:683).
// BC versions: 29.1 (local W1 container).

table 97558 "G1S Result"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "Key"; Code[50]) { }
        field(2; Seen; Text[100]) { }
    }

    keys
    {
        key(PK; "Key") { Clustered = true; }
    }
}

// A list of G1W Row whose OnOpenPage sets a filter in filter group 1 on its Rec.
page 97558 "G1S Rows"
{
    PageType = List;
    SourceTable = "G1W Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field(Tag; Rec.Tag) { }
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.FilterGroup(1);
        Rec.SetRange(Tag, 7);
        Rec.FilterGroup(0);
    end;
}

// A page whose completion trigger, which runs in the test session, stores the worker's answer with what a fresh record of
// G1W Row reads there, as the row COMPLETION of G1S Result.
page 97559 "G1S Completion Hosts"
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

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    var
        Fresh: Record "G1W Row";
        Result: Record "G1S Result";
    begin
        Result."Key" := 'COMPLETION';
        Result.Seen := CopyStr(Results.Get('Seen') + '/' + Format(Fresh.Count()), 1, MaxStrLen(Result.Seen));
        Fresh.FilterGroup(1);
        Result.Seen := CopyStr(Result.Seen + '|' + Fresh.GetFilters(), 1, MaxStrLen(Result.Seen));
        Result.Insert();
    end;
}

// The worker of the shapes: one read, write or filter with records of G1W Row, chosen by the Mode parameter.
codeunit 97560 "G1S Reader"
{
    trigger OnRun()
    var
        Row: Record "G1W Row";
        TempRow: Record "G1W Row" temporary;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        case Parameters.Get('Mode') of
            'filtered':
                begin
                    Row.SetRange(Label, 'A', 'C');
                    Seen := Format(Row.Count());
                end;
            'find':
                if Row.Find('-') then
                    Seen := Row.Label
                else
                    Seen := '-';
            'findnext':
                if Row.Find('>') then
                    Seen := Row.Label
                else
                    Seen := '-';
            'next':
                if Row.FindFirst() then begin
                    Seen := Row.Label;
                    if Row.Next() <> 0 then
                        Seen += Row.Label;
                end;
            'calcsums':
                begin
                    Row.CalcSums(Tag);
                    Seen := Format(Row.Tag);
                end;
            'temporary':
                begin
                    TempRow."No." := 'A';
                    TempRow.Tag := 0;
                    TempRow.Insert();
                    TempRow."No." := 'B';
                    TempRow.Tag := 7;
                    TempRow.Insert();
                    Seen := Format(TempRow.Count());
                    TempRow.FilterGroup(1);
                    Seen += '|' + TempRow.GetFilters();
                end;
            'reset':
                begin
                    Row.Reset();
                    Clear(Row);
                    Row.Init();
                    Seen := Row.GetFilters() + '|' + Format(Row.Count());
                end;
            'insert':
                begin
                    Row."No." := 'NEW';
                    Row.Tag := 7;
                    Row.Insert();
                end;
            'modify':
                begin
                    Row.Get(Parameters.Get('RowKey'));
                    Row.Label := 'Z';
                    Row.Modify();
                end;
            'fail':
                begin
                    Row.FilterGroup(1);
                    Row.SetRange(Tag, 0);
                    Row.FilterGroup(0);
                    Error('The worker failed after it counted %1', Row.Count());
                end;
            'clear':
                begin
                    Row.FilterGroup(1);
                    Row.SetRange(Tag);
                    Row.FilterGroup(0);
                    Seen := Format(Row.Count());
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97561 "G1S Events"
{
    procedure SetGroup1(var Row: Record "G1W Row")
    begin
        OnSetGroup1(Row);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnSetGroup1(var Row: Record "G1W Row")
    begin
    end;
}

codeunit 97562 "G1S Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"G1S Events", 'OnSetGroup1', '', false, false)]
    local procedure SetTag(var Row: Record "G1W Row")
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag, 7);
        Row.FilterGroup(0);
    end;
}

codeunit 97563 "G1S Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        KeyA: Code[20];

    local procedure NewKey(Prefix: Text): Code[20]
    begin
        exit(CopyStr(Prefix + DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    // Rows A (Tag 0), B (Tag 7) and C (Tag 0) of G1W Row and one row of G1W Host, committed: a worker's read of a table sees a
    // committed change of it.
    local procedure Initialize()
    var
        Row: Record "G1W Row";
        Other: Record "G1W Other";
        Host: Record "G1W Host";
        Result: Record "G1S Result";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Host.DeleteAll();
        Result.DeleteAll();
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
    begin
        Row."No." := RowKey;
        Row.Tag := Tag;
        Row.Label := Label;
        Row.Insert();
    end;

    // The worker Worker run by the test on an open page, without its completion trigger.
    local procedure Run(Worker: Integer; Mode: Text; RowKey: Code[20]): Text
    var
        Hosts: TestPage "G1W Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Mode', Mode);
        Parameters.Add('RowKey', RowKey);
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Worker, Parameters, false);
        Hosts.Close();
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure RunFailing(Mode: Text; RowKey: Code[20])
    var
        Seen: Text;
    begin
        Seen := Run(Codeunit::"G1S Reader", Mode, RowKey);
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

    // A fresh record of G1W Row in the test session: its Count and its filters in filter group 1.
    local procedure FreshSide(): Text
    var
        Fresh: Record "G1W Row";
        Seen: Text;
    begin
        Seen := Format(Fresh.Count());
        Fresh.FilterGroup(1);
        exit(Seen + '|' + Fresh.GetFilters());
    end;

    // An event subscriber sets SetRange(Tag, 7) in filter group 1 on the test's record.
    [Test]
    procedure SubscriberSetsGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Events: Codeunit "G1S Events";
        Before: Text;
        Seen: Text;
    begin
        Initialize();
        Events.SetGroup1(Row);
        Before := FreshSide();
        Seen := Run(Codeunit::"G1W Reader", 'repro', '');
        ClearGroup1(Row);
        Assert.AreEqual('1|Tag: 7', Before, 'the test session before the task');
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    // The test session's filter in filter group 1 is on a temporary record of G1W Row.
    [Test]
    procedure TemporaryRecordGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        TempRow: Record "G1W Row" temporary;
        Seen: Text;
    begin
        Initialize();
        TempRow.FilterGroup(1);
        TempRow.SetRange(Tag, 7);
        TempRow.FilterGroup(0);
        Seen := Run(Codeunit::"G1W Reader", 'repro', '');
        TempRow.FilterGroup(1);
        TempRow.SetRange(Tag);
        TempRow.FilterGroup(0);
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    // The test session's filter in filter group 1 is set through a RecordRef.
    [Test]
    procedure RecordRefGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        RowRef: RecordRef;
        Seen: Text;
    begin
        Initialize();
        RowRef.Open(Database::"G1W Row");
        RowRef.FilterGroup(1);
        RowRef.Field(2).SetRange(7);
        RowRef.FilterGroup(0);
        Seen := Run(Codeunit::"G1W Reader", 'repro', '');
        RowRef.FilterGroup(1);
        RowRef.Field(2).SetRange();
        RowRef.FilterGroup(0);
        RowRef.Close();
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    // The OnOpenPage trigger of the page sets the filter in filter group 1 on its Rec; the task runs on that page.
    [Test]
    procedure PageRecGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Fresh: Record "G1W Row";
        Rows: TestPage "G1S Rows";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Before: Text;
        Seen: Text;
    begin
        Initialize();
        Parameters.Add('Mode', 'repro');
        Parameters.Add('RowKey', '');
        Rows.OpenView();
        Before := FreshSide();
        Results := Rows.RunPageBackgroundTask(Codeunit::"G1W Reader", Parameters, false);
        Rows.Close();
        ClearGroup1(Fresh);
        Results.Get('Seen', Seen);
        Assert.AreEqual('1|Tag: 7', Before, 'the test session while the page is open');
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    // The test session's filter in filter group 1 is SetFilter(Tag, '7|8').
    [Test]
    procedure SetFilterTwoValuesGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        Row.FilterGroup(1);
        Row.SetFilter(Tag, '7|8');
        Row.FilterGroup(0);
        Seen := Run(Codeunit::"G1W Reader", 'repro', '');
        ClearGroup1(Row);
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    // The test session's filter in filter group 1 is SetRange on the primary key field, to row B.
    [Test]
    procedure PrimaryKeyFieldGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        Row.SetRange(Tag, 7);
        Row.FindFirst();
        Row.SetRange(Tag);
        Row.FilterGroup(1);
        Row.SetRange("No.", Row."No.");
        Row.FilterGroup(0);
        Seen := Run(Codeunit::"G1W Reader", 'repro', '');
        Row.FilterGroup(1);
        Row.SetRange("No.");
        Row.FilterGroup(0);
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    // The test session's filters in filter group 1 are on two fields: SetRange(Tag, 7) and SetRange(Label, 'B').
    [Test]
    procedure TwoFieldsGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        Row.FilterGroup(1);
        Row.SetRange(Tag, 7);
        Row.SetRange(Label, 'B');
        Row.FilterGroup(0);
        Seen := Run(Codeunit::"G1W Reader", 'repro', '');
        Row.FilterGroup(1);
        Row.SetRange(Tag);
        Row.SetRange(Label);
        Row.FilterGroup(0);
        Assert.AreEqual('3|', Seen, 'the worker''s Count and its filters in filter group 1');
    end;

    // The worker's record holds a filter of its own in filter group 0, SetRange(Label, 'A', 'C').
    [Test]
    procedure WorkerRecordWithOwnFilter_Count_EveryRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run(Codeunit::"G1S Reader", 'filtered', '');
        ClearGroup1(Row);
        Assert.AreEqual('3', Seen, 'the worker''s Count');
    end;

    [Test]
    procedure WorkerFreshRecord_FindMinus_FirstRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run(Codeunit::"G1S Reader", 'find', '');
        ClearGroup1(Row);
        Assert.AreEqual('A', Seen, 'the worker''s Find(''-'')');
    end;

    [Test]
    procedure WorkerFreshRecord_FindGreater_FirstRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run(Codeunit::"G1S Reader", 'findnext', '');
        ClearGroup1(Row);
        Assert.AreEqual('A', Seen, 'the worker''s Find(''>'') from a blank key');
    end;

    [Test]
    procedure WorkerFreshRecord_FindFirstThenNext_FirstTwoRows()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run(Codeunit::"G1S Reader", 'next', '');
        ClearGroup1(Row);
        Assert.AreEqual('AB', Seen, 'the worker''s FindFirst, then Next');
    end;

    [Test]
    procedure WorkerFreshRecord_CalcSums_EveryRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 0);
        Seen := Run(Codeunit::"G1S Reader", 'calcsums', '');
        ClearGroup1(Row);
        Assert.AreEqual('7', Seen, 'the worker''s CalcSums(Tag)');
    end;

    [Test]
    procedure WorkerTemporaryRecord_Count_EveryRowItInserted()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run(Codeunit::"G1S Reader", 'temporary', '');
        ClearGroup1(Row);
        Assert.AreEqual('2|', Seen, 'the worker''s temporary record: Count and its filters in filter group 1');
    end;

    [Test]
    procedure WorkerFreshRecord_ResetClearInit_NoFilterEveryRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        SetGroup1(Row, 7);
        Seen := Run(Codeunit::"G1S Reader", 'reset', '');
        ClearGroup1(Row);
        Assert.AreEqual('|3', Seen, 'the worker''s GetFilters and Count after Reset, Clear and Init');
    end;

    [Test]
    procedure WorkerInsert_RefusedByReadOnlySession()
    var
        Row: Record "G1W Row";
    begin
        Initialize();
        SetGroup1(Row, 7);
        asserterror RunFailing('insert', '');
        ClearGroup1(Row);
        Assert.ExpectedError('Sorry, the current permissions prevented the action');
    end;

    [Test]
    procedure WorkerModify_RefusedByReadOnlySession()
    var
        Row: Record "G1W Row";
    begin
        Initialize();
        SetGroup1(Row, 7);
        asserterror RunFailing('modify', KeyA);
        ClearGroup1(Row);
        Assert.ExpectedError('Sorry, the current permissions prevented the action');
    end;

    // RunPageBackgroundTask with the completion triggers: the worker reads every row, the completion trigger runs in the test
    // session and reads its filter.
    [Test]
    procedure CompletionTrigger_ReadsTheTestSessionsFilter_WorkerDoesNot()
    var
        Row: Record "G1W Row";
        Result: Record "G1S Result";
        Hosts: TestPage "G1S Completion Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Initialize();
        SetGroup1(Row, 7);
        Parameters.Add('Mode', 'count');
        Parameters.Add('RowKey', '');
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"G1W Reader", Parameters, true);
        Hosts.Close();
        ClearGroup1(Row);
        Result.Get('COMPLETION');
        Assert.AreEqual('3/1|Tag: 7', Result.Seen, 'the worker''s Count / the test session''s Count | its filters in filter group 1');
    end;

    // The worker sets SetRange(Tag, 0) in filter group 1 and then fails; the test session holds no filter there.
    [Test]
    procedure FailingWorkerSetsGroup1_EndsWithTheWorker()
    begin
        Initialize();
        asserterror RunFailing('fail', '');
        Assert.ExpectedError('The worker failed after it counted 2');
        Assert.AreEqual('3|', FreshSide(), 'the test session after the task');
    end;
}

// A worker takes back the filter in filter group 1 with SetRange(Tag) while the test session holds SetRange(Tag, 7) there. The
// first test leaves its filter set on purpose; the second, the next test of the codeunit, reads it and takes it back.
codeunit 97564 "G1S Worker Clears Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    // Rows A (Tag 0), B (Tag 7) and C (Tag 0) of G1W Row and one row of G1W Host, committed. Under the first test's filter its
    // DeleteAll deletes only the rows with Tag 7.
    local procedure Initialize()
    var
        Row: Record "G1W Row";
        Host: Record "G1W Host";
    begin
        Row.DeleteAll();
        Host.DeleteAll();
        AddRow('A', 0);
        AddRow('B', 7);
        AddRow('C', 0);
        Host."No." := NewKey('H');
        Host.Insert();
        Commit();
    end;

    local procedure NewKey(Prefix: Text): Code[20]
    begin
        exit(CopyStr(Prefix + DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    local procedure AddRow(Label: Text[10]; Tag: Integer)
    var
        Row: Record "G1W Row";
    begin
        Row."No." := NewKey(Label);
        Row.Tag := Tag;
        Row.Label := Label;
        Row.Insert();
    end;

    local procedure FreshSide(): Text
    var
        Fresh: Record "G1W Row";
        Seen: Text;
    begin
        Seen := Format(Fresh.Count());
        Fresh.FilterGroup(1);
        exit(Seen + '|' + Fresh.GetFilters());
    end;

    // The worker counts every row after its SetRange(Tag) in filter group 1; the test session keeps SetRange(Tag, 7).
    [Test]
    procedure WorkerClearsGroup1_TestKeepsItsFilter()
    var
        Row: Record "G1W Row";
        Hosts: TestPage "G1W Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
        After: Text;
    begin
        Initialize();
        Row.FilterGroup(1);
        Row.SetRange(Tag, 7);
        Row.FilterGroup(0);
        Parameters.Add('Mode', 'clear');
        Parameters.Add('RowKey', '');
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"G1S Reader", Parameters, false);
        Hosts.Close();
        Results.Get('Seen', Seen);
        After := FreshSide();
        Assert.AreEqual('3', Seen, 'the worker''s Count after its SetRange(Tag) in filter group 1');
        Assert.AreEqual('1|Tag: 7', After, 'the test session after the task');
    end;

    // The next test: a fresh record still reads the first test's filter, which this test takes back.
    [Test]
    procedure WorkerClearsGroup1_NextTestFreshRecord_KeepsTheFilter()
    var
        Fresh: Record "G1W Row";
        Seen: Text;
    begin
        Initialize();
        Seen := FreshSide();
        Fresh.FilterGroup(1);
        Fresh.SetRange(Tag);
        Fresh.FilterGroup(0);
        Assert.AreEqual('1|Tag: 7', Seen, 'a fresh record of the next test');
    end;
}
