// The shapes next to the cells of the corpus probe of al-runner issue 8670 (pages/background-tasks/BgTaskWorkerFilterGroup1.al,
// in Probe.al): a page background task's worker and the test session's filters in filter group 1. Each shape is a test codeunit
// of its own, so a refusal in one does not reach the others. Each test expects what Business Central 29.1 answered on a local
// W1 container (container-test gate, 3 runs, the answer is in the comment above the test).

table 97560 "G1N Result"
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
page 97561 "G1N Rows"
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
// G1W Row reads there, as the row COMPLETION of G1N Result.
page 97562 "G1N Completion Hosts"
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
        Result: Record "G1N Result";
    begin
        Result."Key" := 'COMPLETION';
        Result.Seen := CopyStr(Results.Get('Seen') + '/' + Format(Fresh.Count()), 1, MaxStrLen(Result.Seen));
        Fresh.FilterGroup(1);
        Result.Seen := CopyStr(Result.Seen + '|' + Fresh.GetFilters(), 1, MaxStrLen(Result.Seen));
        Result.Insert();
    end;
}

// The worker of the neighbors: reads past the probe's fresh records, or fails after it set a filter in filter group 1.
codeunit 97563 "G1N Reader"
{
    trigger OnRun()
    var
        Row: Record "G1W Row";
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

// Run by StartSession: what a fresh record of G1W Row reads in the new session.
codeunit 97564 "G1N Session Reader"
{
    TableNo = "G1N Result";

    trigger OnRun()
    var
        Row: Record "G1W Row";
    begin
        Rec.Seen := Format(Row.Count());
        Row.FilterGroup(1);
        Rec.Seen += '|' + Row.GetFilters();
        Rec.Insert();
    end;
}

codeunit 97565 "G1N Events"
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

codeunit 97566 "G1N Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"G1N Events", 'OnSetGroup1', '', false, false)]
    local procedure SetTag(var Row: Record "G1W Row")
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag, 7);
        Row.FilterGroup(0);
    end;
}

// The probe's setup and task run, for the neighbors.
codeunit 97576 "G1N Setup"
{
    // Rows A (Tag 0), B (Tag 7) and C (Tag 0) of G1W Row and one row of G1W Host, committed.
    procedure Initialize()
    var
        Row: Record "G1W Row";
        Other: Record "G1W Other";
        Host: Record "G1W Host";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Host.DeleteAll();
        AddRow('A', 0);
        AddRow('B', 7);
        AddRow('C', 0);
        Host."No." := NewKey('H');
        Host.Insert();
        Commit();
    end;

    procedure NewKey(Prefix: Text): Code[20]
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

    procedure Run(Worker: Integer; Mode: Text): Text
    var
        Hosts: TestPage "G1W Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Mode', Mode);
        Parameters.Add('RowKey', '');
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Worker, Parameters, false);
        Hosts.Close();
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    procedure SetGroup1(var Row: Record "G1W Row"; Tag: Integer)
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag, Tag);
        Row.FilterGroup(0);
    end;

    procedure ClearGroup1(var Row: Record "G1W Row")
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag);
        Row.FilterGroup(0);
    end;

    // A fresh record of G1W Row in the test session: its Count and its filters in filter group 1.
    procedure FreshSide(): Text
    var
        Fresh: Record "G1W Row";
        Seen: Text;
    begin
        Seen := Format(Fresh.Count());
        Fresh.FilterGroup(1);
        exit(Seen + '|' + Fresh.GetFilters());
    end;
}

// StartSession while the test session holds SetRange(Tag, 7) in filter group 1: the new session's fresh record reads every
// row and holds no filter there. BC: 3| (the session starts under RequiredTestIsolation = Disabled). main and head fail the
// same way: the runner starts a session only under a TestRunner with TestIsolation Disabled.
codeunit 97567 "G1N StartSession"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure StartSession_SessionFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Result: Record "G1N Result";
        SessionId: Integer;
        Tries: Integer;
        ResultKey: Code[50];
    begin
        Setup.Initialize();
        ResultKey := Setup.NewKey('S');
        Result."Key" := ResultKey;
        Setup.SetGroup1(Row, 7);
        Session.StartSession(SessionId, Codeunit::"G1N Session Reader", CompanyName(), Result);
        while Session.IsSessionActive(SessionId) and (Tries < 300) do begin
            Sleep(100);
            Tries += 1;
        end;
        Setup.ClearGroup1(Row);
        Result.Get(ResultKey);
        Assert.AreEqual('3|', Result.Seen, 'the session''s Count and its filters in filter group 1');
    end;
}

// The test session's filter in filter group 1 set by an event subscriber on the test's record. BC: 3| in the worker; 1|Tag: 7
// in the test session before the task.
codeunit 97568 "G1N Subscriber Filter"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure SubscriberSetsGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Events: Codeunit "G1N Events";
        Before: Text;
        Seen: Text;
    begin
        Setup.Initialize();
        Events.SetGroup1(Row);
        Before := Setup.FreshSide();
        Seen := Setup.Run(Codeunit::"G1W Reader", 'repro');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('3|', Seen, 'the worker');
        Assert.AreEqual('1|Tag: 7', Before, 'the test session before the task');
    end;
}

// The test session's filter in filter group 1 on a temporary record of G1W Row. BC: 3|.
codeunit 97569 "G1N Temporary"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure TemporaryRecordGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        TempRow: Record "G1W Row" temporary;
        Seen: Text;
    begin
        Setup.Initialize();
        TempRow.FilterGroup(1);
        TempRow.SetRange(Tag, 7);
        TempRow.FilterGroup(0);
        Seen := Setup.Run(Codeunit::"G1W Reader", 'repro');
        TempRow.FilterGroup(1);
        TempRow.SetRange(Tag);
        TempRow.FilterGroup(0);
        Assert.AreEqual('3|', Seen, 'the worker');
    end;
}

// The test session's filter in filter group 1 set through a RecordRef. BC: 3|.
codeunit 97570 "G1N RecordRef Setter"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure RecordRefGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        RowRef: RecordRef;
        Seen: Text;
    begin
        Setup.Initialize();
        RowRef.Open(Database::"G1W Row");
        RowRef.FilterGroup(1);
        RowRef.Field(2).SetRange(7);
        RowRef.FilterGroup(0);
        Seen := Setup.Run(Codeunit::"G1W Reader", 'repro');
        RowRef.FilterGroup(1);
        RowRef.Field(2).SetRange();
        RowRef.FilterGroup(0);
        RowRef.Close();
        Assert.AreEqual('3|', Seen, 'the worker');
    end;
}

// The worker's record holds a filter of its own in filter group 0. BC: 3.
codeunit 97571 "G1N Worker Own Filter"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerRecordWithOwnFilter_Count_EveryRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'filtered');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('3', Seen, 'the worker''s Count');
    end;
}

// The worker reads with Find('-'). BC: A.
codeunit 97572 "G1N Worker Find"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerFreshRecord_FindMinus_FirstRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'find');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('A', Seen, 'the worker''s Find(''-'')');
    end;
}

// RunPageBackgroundTask with the completion triggers: the worker reads every row, the completion trigger runs in the test
// session and reads its filter. BC: 3/1|Tag: 7.
codeunit 97573 "G1N Completion Trigger"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure CompletionTrigger_ReadsTheTestSessionsFilter_WorkerDoesNot()
    var
        Row: Record "G1W Row";
        Result: Record "G1N Result";
        Hosts: TestPage "G1N Completion Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Setup.Initialize();
        Result.DeleteAll();
        Setup.SetGroup1(Row, 7);
        Parameters.Add('Mode', 'count');
        Parameters.Add('RowKey', '');
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"G1W Reader", Parameters, true);
        Hosts.Close();
        Setup.ClearGroup1(Row);
        Result.Get('COMPLETION');
        Assert.AreEqual('3/1|Tag: 7', Result.Seen, 'the worker''s Count / the test session''s Count | its filters in filter group 1');
    end;
}

// A worker that sets a filter in filter group 1 and then fails: the filter ends with it. BC: the error, then 3| in the test
// session.
codeunit 97574 "G1N Failing Worker"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure FailingWorkerSetsGroup1_EndsWithTheWorker()
    var
        After: Text;
    begin
        Setup.Initialize();
        asserterror RunFailingWorker();
        Assert.AreEqual('The worker failed after it counted 2', GetLastErrorText(), 'the worker''s error');
        After := Setup.FreshSide();
        Assert.AreEqual('3|', After, 'the test session after the task');
    end;

    local procedure RunFailingWorker()
    var
        Seen: Text;
    begin
        Seen := Setup.Run(Codeunit::"G1N Reader", 'fail');
    end;
}

// The page's OnOpenPage sets the filter in filter group 1 on its Rec; the task runs on that page. BC: 3| in the worker; 1|Tag: 7
// in the test session while the page is open.
codeunit 97575 "G1N Page Rec Filter"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure PageRecGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Fresh: Record "G1W Row";
        Rows: TestPage "G1N Rows";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Before: Text;
        Seen: Text;
    begin
        Setup.Initialize();
        Parameters.Add('Mode', 'repro');
        Parameters.Add('RowKey', '');
        Rows.OpenView();
        Before := Setup.FreshSide();
        Results := Rows.RunPageBackgroundTask(Codeunit::"G1W Reader", Parameters, false);
        Rows.Close();
        Setup.ClearGroup1(Fresh);
        Results.Get('Seen', Seen);
        Assert.AreEqual('3|', Seen, 'the worker');
        Assert.AreEqual('1|Tag: 7', Before, 'the test session while the page is open');
    end;
}

// A worker takes back the filter in filter group 1 with SetRange(Tag) while the test session holds SetRange(Tag, 7) there. The
// first test leaves its filter set on purpose; the second, the next test of the codeunit, reads it and takes it back.
// BC: the worker counts 3; after the task the test session reads 1|Tag: 7, and so does the next test.
codeunit 97577 "G1N Worker Clears"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerClearsGroup1_TestKeepsItsFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
        After: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'clear');
        After := Setup.FreshSide();
        Assert.AreEqual('3', Seen, 'the worker''s Count after its SetRange(Tag) in filter group 1');
        Assert.AreEqual('1|Tag: 7', After, 'the test session after the task');
    end;

    [Test]
    procedure WorkerClearsGroup1_NextTestFreshCount()
    var
        Fresh: Record "G1W Row";
        Seen: Text;
    begin
        Seen := Setup.FreshSide();
        Setup.ClearGroup1(Fresh);
        Assert.AreEqual('1|Tag: 7', Seen, 'a fresh record of the next test');
    end;
}
