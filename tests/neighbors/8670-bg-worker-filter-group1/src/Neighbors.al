// The shapes next to the cells of the corpus probes of al-runner issue 8670 (pages/background-tasks/BgTaskWorkerFilterGroup1.al,
// in Probe.al, and BgTaskWorkerFilterGroup1Shapes.al, in Shapes.al) that no cell measures: a page background task's worker and
// the test session's filters in filter group 1. Each shape is a test codeunit of its own, so a refusal in one does not reach
// the others. Each test expects what Business Central 29.1 answered on a local W1 container (container-test gate, 3 runs, the
// answer is in the comment above the test). The runner refuses each of them on main and on the branch, except StartSession.

table 97565 "G1N Result"
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

// The worker of the neighbors: reads and filters past the cells.
codeunit 97566 "G1N Reader"
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
            'findlast':
                if Row.Find('+') then
                    Seen := Row.Label
                else
                    Seen := '-';
            'key':
                begin
                    Row.SetCurrentKey(Tag);
                    if Row.FindFirst() then
                        Seen := Row.Label
                    else
                        Seen := '-';
                end;
            'tempfirst':
                if TempRow.FindFirst() then
                    Seen := TempRow."No."
                else
                    Seen := '-';
            'setfilter':
                begin
                    Row.FilterGroup(1);
                    Row.SetFilter(Tag, '0');
                    Row.FilterGroup(0);
                    Seen := Format(Row.Count());
                end;
            'otherfield':
                begin
                    Row.FilterGroup(1);
                    Row.SetRange(Label, 'A');
                    Row.FilterGroup(0);
                    Seen := Format(Row.Count());
                end;
            'nextblank':
                Seen := Format(Row.Next()) + Row.Label;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

// Run by StartSession: what a fresh record of G1W Row reads in the new session.
codeunit 97567 "G1N Session Reader"
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

// The probe's setup and task run, for the neighbors.
codeunit 97568 "G1N Setup"
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

    // Takes back every filter of the record in filter group 1.
    procedure ClearGroup1(var Row: Record "G1W Row")
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag);
        Row.SetRange(Label);
        Row.FilterGroup(0);
    end;
}

// StartSession while the test session holds SetRange(Tag, 7) in filter group 1: the new session's fresh record reads every
// row and holds no filter there. BC: 3| (the session starts under RequiredTestIsolation = Disabled). main and head fail the
// same way: the runner starts a session only under a TestRunner with TestIsolation Disabled.
codeunit 97569 "G1N StartSession"
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

// The test session's filter in filter group 1 is SetFilter(Tag, '7|8|9'), three values. BC: 3|.
codeunit 97570 "G1N Three Values"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure ThreeValuesGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Row.FilterGroup(1);
        Row.SetFilter(Tag, '7|8|9');
        Row.FilterGroup(0);
        Seen := Setup.Run(Codeunit::"G1W Reader", 'repro');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('3|', Seen, 'the worker');
    end;
}

// The test session's one filter in filter group 1 is on a Text field, SetRange(Label, 'B'). BC: 3|.
codeunit 97571 "G1N Text Filter"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure TextFieldGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Row.FilterGroup(1);
        Row.SetRange(Label, 'B');
        Row.FilterGroup(0);
        Seen := Setup.Run(Codeunit::"G1W Reader", 'repro');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('3|', Seen, 'the worker');
    end;
}

// The test session's filter in filter group 1 is a range, SetRange(Tag, 0, 7). BC: 3|.
codeunit 97572 "G1N Range Filter"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure RangeGroup1_WorkerFreshRecord_EveryRowNoFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Row.FilterGroup(1);
        Row.SetRange(Tag, 0, 7);
        Row.FilterGroup(0);
        Seen := Setup.Run(Codeunit::"G1W Reader", 'repro');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('3|', Seen, 'the worker');
    end;
}

// The worker reads with Find('+'). BC: C.
codeunit 97573 "G1N Worker Find Last"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerFreshRecord_FindPlus_LastRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'findlast');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('C', Seen, 'the worker''s Find(''+'')');
    end;
}

// The worker reads with SetCurrentKey(Tag), then FindFirst. BC: A.
codeunit 97574 "G1N Worker Key"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerRecordOtherKey_FindFirst_FirstRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'key');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('A', Seen, 'the worker''s FindFirst on the key Tag');
    end;
}

// The worker reads an empty temporary record with FindFirst. BC: -.
codeunit 97575 "G1N Worker Temp FindFirst"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerTemporaryRecord_FindFirst_NoRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'tempfirst');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('-', Seen, 'the worker''s temporary record');
    end;
}

// The worker sets SetFilter(Tag, '0') in filter group 1 while the test session holds SetRange(Tag, 7) there. BC: 2.
codeunit 97576 "G1N Worker SetFilter"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerSetFilterGroup1_Count_ItsOwnFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'setfilter');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('2', Seen, 'the worker''s Count');
    end;
}

// The worker sets SetRange(Label, 'A') in filter group 1 while the test session holds SetRange(Tag, 7) there. BC: 1.
codeunit 97577 "G1N Worker Other Field"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerSetsAnotherFieldGroup1_Count_ItsOwnFilter()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'otherfield');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('1', Seen, 'the worker''s Count');
    end;
}

// The test session's filter in filter group 1 is on a temporary record; the worker reads with FindFirst. BC: A.
codeunit 97578 "G1N Temp Setter FindFirst"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure TemporaryRecordGroup1_WorkerFindFirst_FirstRow()
    var
        TempRow: Record "G1W Row" temporary;
        Seen: Text;
    begin
        Setup.Initialize();
        TempRow.FilterGroup(1);
        TempRow.SetRange(Tag, 7);
        TempRow.FilterGroup(0);
        Seen := Setup.Run(Codeunit::"G1W Reader", 'first');
        TempRow.FilterGroup(1);
        TempRow.SetRange(Tag);
        TempRow.FilterGroup(0);
        Assert.AreEqual('A', Seen, 'the worker''s FindFirst');
    end;
}

// The worker calls Next on a record that read no row. BC: 1A.
codeunit 97579 "G1N Worker Next Blank"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Setup: Codeunit "G1N Setup";

    [Test]
    procedure WorkerFreshRecord_NextWithoutFind_FirstRow()
    var
        Row: Record "G1W Row";
        Seen: Text;
    begin
        Setup.Initialize();
        Setup.SetGroup1(Row, 7);
        Seen := Setup.Run(Codeunit::"G1N Reader", 'nextblank');
        Setup.ClearGroup1(Row);
        Assert.AreEqual('1A', Seen, 'the worker''s Next');
    end;
}
