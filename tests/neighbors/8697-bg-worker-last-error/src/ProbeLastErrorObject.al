// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-getlasterrorobject-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
// Scope: in-scope, but ONLY for a Target = OnPrem app (GetLastErrorObject has scope OnPrem; must-not-compile case 0110), AL compiler 18.0 or newer
// local object: what a page background task's worker reads of GetLastErrorObject is what is under test
// Fixtures used: Assert (60021); table 97390 "LEW Host" and page 97392 "LEW Hosts" of the AL Language Coverage Tests app
//   (pages/background-tasks/BgTaskWorkerLastError.al); codeunits 99408 (worker) and 99409 (tests)
// Probe (al-runner issue 8697, next to BgTaskWorkerLastError.al): the worker runs in a session of its own and starts with no
//   last error. GetLastErrorObject in a session where no error has happened is not pinned in the test session
//   (dotnet-interop/GetLastErrorObjectKinds.al:2), but a worker is a new session.
// Cells:
//   - the worker's GetLastErrorObject before an error of its own, while the test session holds a last error (:91) and while it
//     holds none (:103): IsNull and Format;
//   - after the worker caught an error of its own: the exception's type (:114);
//   - the test session's GetLastErrorObject after a worker that caught its own error: the test session's exception (:126).
// BC versions: 29.1 (local W1 container).
dotnet
{
    assembly("mscorlib")
    {
        type("System.Exception"; LwoExc) { }
        type("System.Type"; LwoType) { }
    }
}

// The worker.
codeunit 99408 "LWO Reader"
{
    trigger OnRun()
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters := Page.GetBackgroundParameters();
        if Parameters.Get('Mode') = 'catch' then
            if not TryFail() then;
        Describe(Results);
        Page.SetBackgroundTaskResult(Results);
    end;

    local procedure Describe(var Results: Dictionary of [Text, Text])
    var
        Exc: DotNet LwoExc;
        TypeOfExc: DotNet LwoType;
    begin
        Exc := GetLastErrorObject();
        Results.Add('IsNull', Format(IsNull(Exc)));
        if IsNull(Exc) then begin
            Results.Add('Format', Format(Exc));
            exit;
        end;
        TypeOfExc := Exc.GetType();
        Results.Add('Type', TypeOfExc.FullName);
    end;

    [TryFunction]
    local procedure TryFail()
    begin
        Error('The worker''s error');
    end;
}

codeunit 99409 "LWO Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    local procedure Run(Mode: Text; var Results: Dictionary of [Text, Text])
    var
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
    begin
        Parameters.Add('Mode', Mode);
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"LWO Reader", Parameters, false);
        Hosts.Close();
    end;

    local procedure Value(Results: Dictionary of [Text, Text]; Name: Text): Text
    var
        Seen: Text;
    begin
        Results.Get(Name, Seen);
        exit(Seen);
    end;

    [Test]
    procedure TestSessionHoldsLastError_WorkerGetLastErrorObject_Null()
    var
        Host: Record "LEW Host";
        Results: Dictionary of [Text, Text];
    begin
        asserterror Host.Get('MISSING');
        Run('read', Results);
        Assert.AreEqual(Format(true), Value(Results, 'IsNull'), 'IsNull of the worker''s GetLastErrorObject');
        Assert.AreEqual('null', Value(Results, 'Format'), 'Format of the worker''s GetLastErrorObject');
    end;

    [Test]
    procedure TestSessionHoldsNone_WorkerGetLastErrorObject_Null()
    var
        Results: Dictionary of [Text, Text];
    begin
        ClearLastError();
        Run('read', Results);
        Assert.AreEqual(Format(true), Value(Results, 'IsNull'), 'IsNull of the worker''s GetLastErrorObject');
        Assert.AreEqual('null', Value(Results, 'Format'), 'Format of the worker''s GetLastErrorObject');
    end;

    [Test]
    procedure WorkerCaughtItsError_WorkerGetLastErrorObject_DialogException()
    var
        Host: Record "LEW Host";
        Results: Dictionary of [Text, Text];
    begin
        asserterror Host.Get('MISSING');
        Run('catch', Results);
        Assert.AreEqual(Format(false), Value(Results, 'IsNull'), 'IsNull of the worker''s GetLastErrorObject');
        Assert.AreEqual('Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLDialogException', Value(Results, 'Type'), 'the type of the worker''s GetLastErrorObject');
    end;

    [Test]
    procedure WorkerCaughtItsError_TestSessionGetLastErrorObject_ItsOwn()
    var
        Host: Record "LEW Host";
        Results: Dictionary of [Text, Text];
        Exc: DotNet LwoExc;
        TypeOfExc: DotNet LwoType;
    begin
        asserterror Host.Get('MISSING');
        Run('catch', Results);
        Exc := GetLastErrorObject();
        TypeOfExc := Exc.GetType();
        Assert.AreEqual('Microsoft.Dynamics.Nav.Types.Exceptions.NavCSideRecordNotFoundException', TypeOfExc.FullName, 'the type of the test session''s GetLastErrorObject');
    end;
}
