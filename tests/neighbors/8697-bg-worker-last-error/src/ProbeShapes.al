// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-testfield-joker-errorinfo-method
// Scope: in-scope
// local object: what a page background task's worker does with a collectible error, and what a worker's caught error does to
//   the close of a page a PageHandler handles, are what is under test
// Fixtures used: Assert (60021); table 97390 "LEW Host" and page 97392 "LEW Hosts" (BgTaskWorkerLastError.al); the CEM
//   fixtures of pages/close-errors/CemFixtures.al (objects 99500..99512); codeunits 97386 (worker), 97387 and 97388 (tests)
// Probe (al-runner issue 8697, next to BgTaskWorkerLastError.al): the worker runs in a session of its own.
// Cells:
//   - a test session that collects errors ([ErrorBehavior(ErrorBehavior::Collect)]) runs a task whose worker raises a
//     collectible TestField (Rec.TestField(Field, Value, ErrorInfo)): the task fails, and the test session reads the
//     TestField error (:98);
//   - a PageHandler of Page.Run (pages/close-errors/CemPageHandlerTests.al, card by object id) runs a page background task
//     whose worker catches an error of its own, then handles the card as the pinned cell does: the close ends as in that
//     cell, for OK with no error (:191, CardStaticRun_HandlerOk_NoError), a handler that returns with an OnQueryClosePage
//     error (:198, _HandlerReturns_QueryCloseError) and a handler error (:205, _HandlerError_NoError).
// BC versions: 29.1 (local W1 container).

// The worker.
codeunit 97386 "LWS Reader"
{
    trigger OnRun()
    var
        Host: Record "LEW Host";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        FieldError: ErrorInfo;
    begin
        Parameters := Page.GetBackgroundParameters();
        case Parameters.Get('Mode') of
            'collectibletestfield':
                begin
                    Host.FindFirst();
                    FieldError.Collectible(true);
                    Host.TestField("No.", 'OTHER', FieldError);
                end;
            'catch':
                if not TryFail() then;
        end;
        Page.SetBackgroundTaskResult(Results);
    end;

    [TryFunction]
    local procedure TryFail()
    begin
        Error('The worker''s error');
    end;
}

codeunit 97387 "LWS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Reached: Boolean;

    // One row of LEW Host, committed.
    local procedure Initialize()
    var
        Host: Record "LEW Host";
    begin
        Host.DeleteAll();
        Host."No." := 'H';
        Host.Insert();
        Commit();
    end;

    local procedure Run(Mode: Text)
    var
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters.Add('Mode', Mode);
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"LWS Reader", Parameters, false);
        Hosts.Close();
    end;

    [TryFunction]
    local procedure TryRunWhileCollecting(Mode: Text)
    begin
        RunWhileCollecting(Mode);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure RunWhileCollecting(Mode: Text)
    begin
        Run(Mode);
        Reached := true;
        ClearCollectedErrors();
    end;

    [Test]
    procedure TestSessionCollects_WorkerCollectibleTestField_FailsTheTask()
    begin
        Initialize();
        Reached := false;
        ClearLastError();
        if not TryRunWhileCollecting('collectibletestfield') then;
        Assert.AreEqual('No. must be equal to ''OTHER''  in LEW Host: No.=H. Current value is ''H''.', GetLastErrorText(), 'the test session''s GetLastErrorText');
        Assert.AreEqual('TestField', GetLastErrorCode(), 'the test session''s GetLastErrorCode');
        Assert.AreEqual(false, Reached, 'the test session went on after the task');
    end;
}

// Every test binds the PageHandler, as in CemPageHandlerTests.al.
codeunit 97388 "LWS Handler Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";
        Runner: Codeunit "CEM Runner";

    // One row of LEW Host, committed.
    local procedure Initialize()
    var
        Host: Record "LEW Host";
    begin
        Host.DeleteAll();
        Host."No." := 'H';
        Host.Insert();
        Commit();
    end;

    // A task whose worker catches an error of its own.
    local procedure RunCatchingWorker()
    var
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters.Add('Mode', 'catch');
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"LWS Reader", Parameters, false);
        Hosts.Close();
    end;

    // The CEM card by object id, with the PageHandler below; what the close ended with.
    local procedure RunCard(Source: Text; Mode: Integer; var EndText: Text; var EndCode: Text; var Log: Text)
    var
        Row: Record "CEM Row";
    begin
        Help.Setup(Source, 'card', Mode);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            Error('CEM_NO_ERROR');
        end;
        EndText := GetLastErrorText();
        EndCode := GetLastErrorCode();
        Log := Cfg.Take();
        Cfg.Quiet();
    end;

    [PageHandler]
    procedure CardWithWorker(var P: TestPage "CEM Card")
    begin
        RunCatchingWorker();
        case Cfg.Mode() of
            1:
                P.OK().Invoke();
            7:
                Error('CEM handler error');
        end;
    end;

    local procedure CheckCard(Source: Text; Mode: Integer; Text: Text; Code: Text; Log: Text)
    var
        EndText: Text;
        EndCode: Text;
        SeenLog: Text;
    begin
        Initialize();
        RunCard(Source, Mode, EndText, EndCode, SeenLog);
        Assert.AreEqual(Text, EndText, 'the end of the run, text');
        Assert.AreEqual(Code, EndCode, 'the end of the run, code');
        Assert.AreEqual(Log, SeenLog, 'the triggers that ran');
    end;

    [Test]
    [HandlerFunctions('CardWithWorker')]
    procedure CardStaticRun_WorkerCaughtItsError_HandlerOk_NoError()
    begin
        CheckCard('none', 1, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardWithWorker')]
    procedure CardStaticRun_WorkerCaughtItsError_HandlerReturns_QueryCloseError()
    begin
        CheckCard('query', 0, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; ');
    end;

    [Test]
    [HandlerFunctions('CardWithWorker')]
    procedure CardStaticRun_WorkerCaughtItsError_HandlerError_NoError()
    begin
        CheckCard('none', 7, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;
}
