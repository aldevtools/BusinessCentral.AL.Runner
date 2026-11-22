// The shapes next to the cells of the corpus probe of al-runner issue 8697 (pages/background-tasks/BgTaskWorkerLastError.al, in
// Probe.al): a page background task's worker and the test session's last error. Each test expects what Business Central 29.1
// answered on a local W1 container (container-test gate, 3 runs).

// A page whose OnAfterGetCurrRecord enqueues the worker's read; its completion trigger stores what the worker read.
page 97395 "LEN Enqueue Hosts"
{
    PageType = List;
    SourceTable = "LEW Host";
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

    trigger OnAfterGetCurrRecord()
    var
        Parameters: Dictionary of [Text, Text];
        TaskId: Integer;
    begin
        if Rec."No." = '' then
            exit;
        Parameters.Add('Mode', 'read');
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LEW Reader", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    var
        Result: Record "LEW Result";
        Seen: Text;
    begin
        Results.Get('Text', Seen);
        Result."Key" := 'ENQUEUED';
        Result.LastText := CopyStr(Seen, 1, MaxStrLen(Result.LastText));
        if Result.Insert() then;
    end;
}

// The worker of the neighbors: it catches its own error through a guarded Codeunit.Run.
codeunit 97396 "LEN Reader"
{
    trigger OnRun()
    var
        Results: Dictionary of [Text, Text];
    begin
        if not Codeunit.Run(Codeunit::"LEN Failing") then;
        Results.Add('Text', GetLastErrorText());
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97397 "LEN Failing"
{
    trigger OnRun()
    begin
        Error('The worker''s guarded error');
    end;
}

codeunit 97398 "LEN Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Initialize()
    var
        Host: Record "LEW Host";
        Result: Record "LEW Result";
    begin
        Host.DeleteAll();
        Result.DeleteAll();
        Host."No." := 'H';
        Host.Insert();
        Commit();
    end;

    local procedure SetTestSessionError()
    var
        Host: Record "LEW Host";
    begin
        asserterror Host.Get('MISSING');
    end;

    [TryFunction]
    local procedure TryGetMissing()
    var
        Host: Record "LEW Host";
    begin
        Host.Get('MISSING');
    end;

    // The task CurrPage.EnqueueBackgroundTask enqueues: the worker reads no last error. BC: ''.
    [Test]
    procedure EnqueuedTask_TestSessionHoldsLastError_WorkerReadsEmpty()
    var
        Result: Record "LEW Result";
        Hosts: TestPage "LEN Enqueue Hosts";
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        Hosts.OpenView();
        Hosts.First();
        Hosts.Close();
        Result.Get('ENQUEUED');
        Assert.AreEqual('', Result.LastText, 'the worker''s GetLastErrorText');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
    end;

    // The test session's last error comes from a TryFunction, not asserterror. BC: ''.
    [Test]
    procedure TryFunctionError_WorkerReadsEmpty()
    var
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
        Before: Text;
    begin
        Initialize();
        if not TryGetMissing() then;
        Before := GetLastErrorText();
        Parameters.Add('Mode', 'read');
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"LEW Reader", Parameters, false);
        Hosts.Close();
        Results.Get('Text', Seen);
        Assert.AreEqual('', Seen, 'the worker''s GetLastErrorText');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
    end;

    // The worker catches its own error through a guarded Codeunit.Run. BC: the worker reads its error; the test session keeps
    // its own.
    [Test]
    procedure WorkerGuardedCodeunitRun_TestSessionKeepsItsLastError()
    var
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"LEN Reader", Parameters, false);
        Hosts.Close();
        Results.Get('Text', Seen);
        Assert.AreEqual('The worker''s guarded error', Seen, 'the worker''s GetLastErrorText');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
    end;
}
