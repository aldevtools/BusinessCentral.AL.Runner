// Neighbors past the cells of BgTaskWorkerSessionShapes.al and BgTaskWorkerLastErrorObject.al, each measured on a Business
// Central 29.1 W1 container (3 runs).

// A worker that collects its own errors.
codeunit 97384 "LEN More Worker"
{
    trigger OnRun()
    var
        Results: Dictionary of [Text, Text];
    begin
        Results.Add('Count', Format(CollectTestField()));
        Page.SetBackgroundTaskResult(Results);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectTestField(): Integer
    var
        Host: Record "LEW Host";
        FieldError: ErrorInfo;
        Collected: Integer;
    begin
        Host.FindFirst();
        FieldError.Collectible(true);
        Host.TestField("No.", 'OTHER', FieldError);
        Collected := GetCollectedErrors().Count();
        ClearCollectedErrors();
        exit(Collected);
    end;
}

codeunit 97385 "LEN More Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";
        Runner: Codeunit "CEM Runner";
        Seen: Text;

    local procedure Initialize()
    var
        Host: Record "LEW Host";
    begin
        Host.DeleteAll();
        Host."No." := 'H';
        Host.Insert();
        Commit();
    end;

    local procedure Run(Worker: Integer; Mode: Text; var Results: Dictionary of [Text, Text])
    var
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
    begin
        Parameters.Add('Mode', Mode);
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Worker, Parameters, false);
        Hosts.Close();
    end;

    local procedure WorkerCatches()
    var
        Results: Dictionary of [Text, Text];
    begin
        Run(Codeunit::"LWS Reader", 'catch', Results);
    end;

    [ModalPageHandler]
    procedure DocWithWorker(var P: TestPage "CEM Doc")
    begin
        WorkerCatches();
        Error('CEM handler error');
    end;

    [PageHandler]
    procedure CardWithWorker(var P: TestPage "CEM Card")
    begin
        WorkerCatches();
        P.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('DocWithWorker')]
    procedure DocumentStaticRunModal_WorkerCaughtItsError_HandlerError_NoError()
    var
        Row: Record "CEM Row";
        Res: Action;
    begin
        Initialize();
        Help.Setup('none', 'doc', 7);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            Error('CEM_NO_ERROR');
        end;
        Seen := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Cfg.Take();
        Cfg.Quiet();
        Assert.AreEqual('CEM handler error|Dialog|Q-doc LookupCancel; C-doc; ', Seen, 'the end of the run and the triggers that ran');
    end;

    [Test]
    [HandlerFunctions('CardWithWorker')]
    procedure CardVariableRun_WorkerCaughtItsError_HandlerOk_NoError()
    var
        Row: Record "CEM Row";
    begin
        Initialize();
        Help.Setup('none', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            Error('CEM_NO_ERROR');
        end;
        Seen := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Cfg.Take();
        Cfg.Quiet();
        Assert.AreEqual('CEM_NO_ERROR|Dialog|Q-card OK; C-card; ', Seen, 'the end of the run and the triggers that ran');
    end;

    [Test]
    procedure TestSessionCollects_WorkerCollectsItsOwnTestField_TheTaskSucceeds()
    begin
        Initialize();
        Seen := '';
        RunWhileCollecting();
        Assert.AreEqual('1', Seen, 'the number of errors the worker collected');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure RunWhileCollecting()
    var
        Results: Dictionary of [Text, Text];
    begin
        Run(Codeunit::"LEN More Worker", '', Results);
        Results.Get('Count', Seen);
        Assert.AreEqual(false, HasCollectedErrors() and (GetCollectedErrors().Count() > 0), 'the test session collected an error');
    end;

    [Test]
    procedure TwoTasks_SecondWorkerAfterAFirstCaughtItsError_GetLastErrorObjectNull()
    var
        Results: Dictionary of [Text, Text];
        IsNullSeen: Text;
    begin
        Initialize();
        Run(Codeunit::"LWO Reader", 'catch', Results);
        Run(Codeunit::"LWO Reader", 'read', Results);
        Results.Get('IsNull', IsNullSeen);
        Assert.AreEqual(Format(true), IsNullSeen, 'IsNull of the second worker''s GetLastErrorObject');
    end;
}
