// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-getlasterrortext-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpagetestpage-runpagebackgroundtask-method
// Scope: in-scope
// local object: what a page background task's worker reads of the test session's last error and of its error collection, and
//   what the test session reads after the task, are what is under test
// Fixtures used: Assert (60021); tables 97390, 97391, 97395; page 97392; codeunit 97393; test codeunit 97394
// Probe (al-runner issue 8697): the test session sets its last error with asserterror (a Get of a row that does not exist, so its
//   code is DB:RecordNotFound) and runs a page background task. The worker runs in a session of its own: it starts with no last
//   error, what it catches stays in it, and the test session reads its own last error after the task.
// Readings: what the worker read (the result of RunPageBackgroundTask, or the row a completion or error trigger stores); the test
//   session's GetLastErrorText, GetLastErrorCode, GetLastErrorCallStack and collected errors after the task.
// Cells:
//   - the test session holds a last error: the worker's GetLastErrorText (:266), GetLastErrorCode (:280),
//     GetLastErrorCallStack (:292) and GetLastErrorText(true) (:426);
//   - the worker catches its own error with a TryFunction: what it reads before and after, and the test session's last error
//     after the task, with one of its own (:303) and with none (:320); the test session's GetLastErrorCallStack (:437);
//   - the worker's ClearLastError while the test session holds a last error (:333);
//   - a worker error no TryFunction catches, under an asserterror around a procedure of the test that runs the task: the test
//     session reads the worker's error, for Error() (:347), a Modify (:551) and an Insert (:563) the read-only session
//     refuses, a TableRelation check (:575), a Get without a row (:587), a TestField (:599) and an ErrorInfo (:611);
//   - the same under an asserterror around the RunPageBackgroundTask statement itself: the test session keeps its own last
//     error, text, code and call stack (:640), an empty one when it held none (:660), also for a refused Modify (:674);
//   - the completion trigger, which runs in the test session, after a worker that caught its own error (:359);
//   - the error trigger, after a worker error no TryFunction catches (:376);
//   - two tasks in a row: the second worker after a first that caught its own error (:394);
//   - a test session that collects errors ([ErrorBehavior(ErrorBehavior::Collect)]): the worker's IsCollectingErrors (:409);
//     with an error collected, the worker's HasCollectedErrors (:451, true as outside any scope, TestErrorInfoType.al finding
//     D), GetCollectedErrors (:472) and ClearCollectedErrors (:481); a collectible error the worker raises fails the task
//     and reaches the test session as an error (:510).
// BC versions: 29.1 (local W1 container).

table 97390 "LEW Host"
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

// The rows the completion and error triggers store what they read in.
table 97391 "LEW Result"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "Key"; Code[20]) { }
        field(2; LastText; Text[250]) { }
        field(3; LastCode; Text[100]) { }
        field(4; ErrorText; Text[250]) { }
    }

    keys
    {
        key(PK; "Key") { Clustered = true; }
    }
}

// A table whose field relates to LEW Host: the worker validates a value no row of LEW Host has.
table 97395 "LEW Line"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Host No."; Code[20]) { TableRelation = "LEW Host"; }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The page the test runs its tasks on. Its completion and error triggers run in the test session and store its last error.
page 97392 "LEW Hosts"
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

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    var
        Result: Record "LEW Result";
    begin
        Result."Key" := 'COMPLETED';
        Result.LastText := CopyStr(GetLastErrorText(), 1, MaxStrLen(Result.LastText));
        Result.LastCode := CopyStr(GetLastErrorCode(), 1, MaxStrLen(Result.LastCode));
        Result.Insert();
    end;

    trigger OnPageBackgroundTaskError(TaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    var
        Result: Record "LEW Result";
    begin
        Result."Key" := 'ERROR';
        Result.LastText := CopyStr(GetLastErrorText(), 1, MaxStrLen(Result.LastText));
        Result.LastCode := CopyStr(GetLastErrorCode(), 1, MaxStrLen(Result.LastCode));
        Result.ErrorText := CopyStr(ErrorText, 1, MaxStrLen(Result.ErrorText));
        Result.Insert();
        IsHandled := true;
    end;
}

// The worker: what it reads of the last error, chosen by the Mode parameter.
codeunit 97393 "LEW Reader"
{
    trigger OnRun()
    var
        Host: Record "LEW Host";
        Line: Record "LEW Line";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        WorkerError: ErrorInfo;
    begin
        Parameters := Page.GetBackgroundParameters();
        case Parameters.Get('Mode') of
            'read':
                begin
                    Results.Add('Text', GetLastErrorText());
                    Results.Add('Code', GetLastErrorCode());
                    Results.Add('Stack', GetLastErrorCallStack());
                    Results.Add('TextExcl', GetLastErrorText(true));
                end;
            'catch':
                begin
                    Results.Add('Before', GetLastErrorText());
                    if not TryFail() then;
                    Results.Add('Text', GetLastErrorText());
                    Results.Add('Code', GetLastErrorCode());
                end;
            'clear':
                begin
                    ClearLastError();
                    Results.Add('Text', GetLastErrorText());
                end;
            'fail':
                Error('The worker''s unhandled error');
            'modify':
                begin
                    Host.FindFirst();
                    Host.Modify();
                end;
            'insert':
                begin
                    Host."No." := 'NEW';
                    Host.Insert();
                end;
            'relation':
                Line.Validate("Host No.", 'MISSING');
            'getmissing':
                Host.Get('ABSENT');
            'testfield':
                begin
                    Host.FindFirst();
                    Host.TestField("No.", 'OTHER');
                end;
            'errorinfo':
                begin
                    WorkerError.Message('The worker''s ErrorInfo');
                    Error(WorkerError);
                end;
            'collecting':
                Results.Add('Collecting', Format(IsCollectingErrors()));
            'hascollected':
                Results.Add('HasCollected', Format(HasCollectedErrors()));
            'getcollected':
                Results.Add('Count', Format(GetCollectedErrors().Count()));
            'clearcollected':
                ClearCollectedErrors();
            'collectible':
                begin
                    WorkerError.Message('The worker''s collectible error');
                    WorkerError.Collectible(true);
                    Error(WorkerError);
                end;
        end;
        Page.SetBackgroundTaskResult(Results);
    end;

    [TryFunction]
    local procedure TryFail()
    begin
        Error('The worker''s error');
    end;
}

codeunit 97394 "LEW Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Collecting: Text;
        TestHasCollected: Text;
        TestCount: Text;
        Reached: Boolean;

    // One row of LEW Host, committed, and no rows of LEW Result and LEW Line.
    local procedure Initialize()
    var
        Host: Record "LEW Host";
        Result: Record "LEW Result";
        Line: Record "LEW Line";
    begin
        Host.DeleteAll();
        Result.DeleteAll();
        Line.DeleteAll();
        Host."No." := 'H';
        Host.Insert();
        Commit();
    end;

    // The test session's last error: a Get of a row that does not exist.
    local procedure SetTestSessionError()
    var
        Host: Record "LEW Host";
    begin
        asserterror Host.Get('MISSING');
    end;

    local procedure Run(Mode: Text; CompletionTriggers: Boolean; var Results: Dictionary of [Text, Text])
    var
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
    begin
        Parameters.Add('Mode', Mode);
        Hosts.OpenView();
        Results := Hosts.RunPageBackgroundTask(Codeunit::"LEW Reader", Parameters, CompletionTriggers);
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
    procedure TestSessionHoldsLastError_WorkerGetLastErrorText_Empty()
    var
        Results: Dictionary of [Text, Text];
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        Run('read', false, Results);
        Assert.AreEqual('', Value(Results, 'Text'), 'the worker''s GetLastErrorText');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
    end;

    [Test]
    procedure TestSessionHoldsLastError_WorkerGetLastErrorCode_Empty()
    var
        Results: Dictionary of [Text, Text];
    begin
        Initialize();
        SetTestSessionError();
        Run('read', false, Results);
        Assert.AreEqual('', Value(Results, 'Code'), 'the worker''s GetLastErrorCode');
        Assert.AreEqual('DB:RecordNotFound', GetLastErrorCode(), 'the test session''s GetLastErrorCode after the task');
    end;

    [Test]
    procedure TestSessionHoldsLastError_WorkerGetLastErrorCallStack_Empty()
    var
        Results: Dictionary of [Text, Text];
    begin
        Initialize();
        SetTestSessionError();
        Run('read', false, Results);
        Assert.AreEqual('', Value(Results, 'Stack'), 'the worker''s GetLastErrorCallStack');
    end;

    [Test]
    procedure WorkerCatchesItsOwnError_TestSessionKeepsItsLastError()
    var
        Results: Dictionary of [Text, Text];
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        Run('catch', false, Results);
        Assert.AreEqual('', Value(Results, 'Before'), 'the worker''s GetLastErrorText before its error');
        Assert.AreEqual('The worker''s error', Value(Results, 'Text'), 'the worker''s GetLastErrorText after its error');
        Assert.AreEqual('Dialog', Value(Results, 'Code'), 'the worker''s GetLastErrorCode after its error');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
        Assert.AreEqual('DB:RecordNotFound', GetLastErrorCode(), 'the test session''s GetLastErrorCode after the task');
    end;

    [Test]
    procedure WorkerCatchesItsOwnError_TestSessionHoldsNone_StaysEmpty()
    var
        Results: Dictionary of [Text, Text];
    begin
        Initialize();
        ClearLastError();
        Run('catch', false, Results);
        Assert.AreEqual('The worker''s error', Value(Results, 'Text'), 'the worker''s GetLastErrorText after its error');
        Assert.AreEqual('', GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
        Assert.AreEqual('', GetLastErrorCode(), 'the test session''s GetLastErrorCode after the task');
    end;

    [Test]
    procedure WorkerClearLastError_TestSessionKeepsItsLastError()
    var
        Results: Dictionary of [Text, Text];
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        Run('clear', false, Results);
        Assert.AreEqual('', Value(Results, 'Text'), 'the worker''s GetLastErrorText after its ClearLastError');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
    end;

    [Test]
    procedure WorkerUnhandledError_TestSessionCatchesIt()
    var
        Results: Dictionary of [Text, Text];
    begin
        Initialize();
        SetTestSessionError();
        asserterror Run('fail', false, Results);
        Assert.AreEqual('The worker''s unhandled error', GetLastErrorText(), 'the test session''s GetLastErrorText');
        Assert.AreEqual('Dialog', GetLastErrorCode(), 'the test session''s GetLastErrorCode');
    end;

    [Test]
    procedure CompletionTrigger_AfterAWorkerCaughtItsError_ReadsTheTestSessionsLastError()
    var
        Result: Record "LEW Result";
        Results: Dictionary of [Text, Text];
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        Run('catch', true, Results);
        Result.Get('COMPLETED');
        Assert.AreEqual(Before, Result.LastText, 'the completion trigger''s GetLastErrorText');
        Assert.AreEqual('DB:RecordNotFound', Result.LastCode, 'the completion trigger''s GetLastErrorCode');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
    end;

    [Test]
    procedure ErrorTrigger_AfterAnUnhandledWorkerError_ReadsTheTestSessionsLastError()
    var
        Result: Record "LEW Result";
        Results: Dictionary of [Text, Text];
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        Run('fail', true, Results);
        Result.Get('ERROR');
        Assert.AreEqual('The worker''s unhandled error', Result.ErrorText, 'the error trigger''s ErrorText');
        Assert.AreEqual(Before, Result.LastText, 'the error trigger''s GetLastErrorText');
        Assert.AreEqual('DB:RecordNotFound', Result.LastCode, 'the error trigger''s GetLastErrorCode');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the task');
    end;

    [Test]
    procedure TwoTasks_SecondWorkerAfterAFirstCaughtItsError_Empty()
    var
        First: Dictionary of [Text, Text];
        Second: Dictionary of [Text, Text];
    begin
        Initialize();
        ClearLastError();
        Run('catch', false, First);
        Run('read', false, Second);
        Assert.AreEqual('The worker''s error', Value(First, 'Text'), 'the first worker''s GetLastErrorText');
        Assert.AreEqual('', Value(Second, 'Text'), 'the second worker''s GetLastErrorText');
        Assert.AreEqual('', GetLastErrorText(), 'the test session''s GetLastErrorText after the tasks');
    end;

    [Test]
    procedure TestSessionCollectsErrors_WorkerIsCollectingErrors_No()
    begin
        Initialize();
        RunWhileCollecting();
        Assert.AreEqual(Format(false), Collecting, 'the worker''s IsCollectingErrors');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure RunWhileCollecting()
    var
        Results: Dictionary of [Text, Text];
    begin
        Run('collecting', false, Results);
        Collecting := Value(Results, 'Collecting');
    end;

    [Test]
    procedure TestSessionHoldsLastError_WorkerGetLastErrorTextWithoutCustomerContent_Empty()
    var
        Results: Dictionary of [Text, Text];
    begin
        Initialize();
        SetTestSessionError();
        Run('read', false, Results);
        Assert.AreEqual('', Value(Results, 'TextExcl'), 'the worker''s GetLastErrorText(true)');
    end;

    [Test]
    procedure WorkerCatchesItsOwnError_TestSessionKeepsItsCallStack()
    var
        Results: Dictionary of [Text, Text];
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorCallStack();
        Run('catch', false, Results);
        Assert.AreEqual(Before, GetLastErrorCallStack(), 'the test session''s GetLastErrorCallStack after the task');
    end;

    // Outside a collecting scope HasCollectedErrors answers true (errors/errorinfo/TestErrorInfoType.al, finding D).
    [Test]
    procedure TestSessionCollectedAnError_WorkerHasCollectedErrors_Yes()
    begin
        Initialize();
        CollectThenRun();
        Assert.AreEqual(Format(true), Collecting, 'the worker''s HasCollectedErrors');
        Assert.AreEqual(Format(true), TestHasCollected, 'the test session''s HasCollectedErrors after the task');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectThenRun()
    var
        Results: Dictionary of [Text, Text];
    begin
        RaiseCollectible();
        Run('hascollected', false, Results);
        Collecting := Value(Results, 'HasCollected');
        TestHasCollected := Format(HasCollectedErrors());
        ClearCollectedErrors();
    end;

    [Test]
    procedure TestSessionCollectedAnError_WorkerGetCollectedErrors_None()
    begin
        Initialize();
        CollectThenRunMode('getcollected');
        Assert.AreEqual('0', Collecting, 'the number of the worker''s GetCollectedErrors');
        Assert.AreEqual('1', TestCount, 'the number of the test session''s GetCollectedErrors after the task');
    end;

    [Test]
    procedure TestSessionCollectedAnError_WorkerClearCollectedErrors_TestKeepsItsError()
    begin
        Initialize();
        CollectThenRunMode('clearcollected');
        Assert.AreEqual('1', TestCount, 'the number of the test session''s GetCollectedErrors after the task');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectThenRunMode(Mode: Text)
    var
        Results: Dictionary of [Text, Text];
    begin
        RaiseCollectible();
        Run(Mode, false, Results);
        if Results.Get('Count', Collecting) then;
        TestCount := Format(GetCollectedErrors().Count());
        ClearCollectedErrors();
    end;

    local procedure RaiseCollectible()
    var
        TestError: ErrorInfo;
    begin
        TestError.Message('The test session''s collectible error');
        TestError.Collectible(true);
        Error(TestError);
    end;

    [Test]
    procedure TestSessionCollects_WorkerCollectibleError_FailsTheTask()
    begin
        Initialize();
        Reached := false;
        TestHasCollected := '';
        ClearLastError();
        if not TryRunCollectibleWhileCollecting() then;
        Assert.AreEqual('The worker''s collectible error', GetLastErrorText(), 'the test session''s GetLastErrorText');
        Assert.AreEqual(false, Reached, 'the test session went on after the task');
    end;

    [TryFunction]
    local procedure TryRunCollectibleWhileCollecting()
    begin
        RunCollectibleWhileCollecting();
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure RunCollectibleWhileCollecting()
    var
        Results: Dictionary of [Text, Text];
    begin
        Run('collectible', false, Results);
        Reached := true;
        TestHasCollected := Format(HasCollectedErrors());
        ClearCollectedErrors();
    end;

    // A worker error no TryFunction catches, under an asserterror around a procedure of the test that runs the task: the test
    // session's GetLastErrorText and GetLastErrorCode read the worker's error, by kind.
    local procedure FailingWorker(Mode: Text; var LastText: Text; var LastCode: Text)
    var
        Results: Dictionary of [Text, Text];
    begin
        SetTestSessionError();
        asserterror Run(Mode, false, Results);
        LastText := GetLastErrorText();
        LastCode := GetLastErrorCode();
    end;

    [Test]
    procedure WorkerModifyRefused_AssertErrorOnAProcedure_TheWorkersError()
    var
        LastText: Text;
        LastCode: Text;
    begin
        Initialize();
        FailingWorker('modify', LastText, LastCode);
        Assert.IsTrue(StrPos(LastText, 'Sorry, the current permissions prevented the action. (TableData 97390 LEW Host Modify') = 1, LastText);
        Assert.AreEqual('DB:ReadOnlySession', LastCode, 'the test session''s GetLastErrorCode');
    end;

    [Test]
    procedure WorkerInsertRefused_AssertErrorOnAProcedure_TheWorkersError()
    var
        LastText: Text;
        LastCode: Text;
    begin
        Initialize();
        FailingWorker('insert', LastText, LastCode);
        Assert.IsTrue(StrPos(LastText, 'Sorry, the current permissions prevented the action. (TableData 97390 LEW Host Insert') = 1, LastText);
        Assert.AreEqual('DB:ReadOnlySession', LastCode, 'the test session''s GetLastErrorCode');
    end;

    [Test]
    procedure WorkerTableRelationValidate_AssertErrorOnAProcedure_TheWorkersError()
    var
        LastText: Text;
        LastCode: Text;
    begin
        Initialize();
        FailingWorker('relation', LastText, LastCode);
        Assert.AreEqual('The field Host No. of table LEW Line contains a value (MISSING) that cannot be found in the related table (LEW Host).', LastText, 'the test session''s GetLastErrorText');
        Assert.AreEqual('DB:PrimRecordNotFound', LastCode, 'the test session''s GetLastErrorCode');
    end;

    [Test]
    procedure WorkerGetMissingRow_AssertErrorOnAProcedure_TheWorkersError()
    var
        LastText: Text;
        LastCode: Text;
    begin
        Initialize();
        FailingWorker('getmissing', LastText, LastCode);
        Assert.AreEqual('The LEW Host does not exist. Identification fields and values: No.=''ABSENT''', LastText, 'the test session''s GetLastErrorText');
        Assert.AreEqual('DB:RecordNotFound', LastCode, 'the test session''s GetLastErrorCode');
    end;

    [Test]
    procedure WorkerTestField_AssertErrorOnAProcedure_TheWorkersError()
    var
        LastText: Text;
        LastCode: Text;
    begin
        Initialize();
        FailingWorker('testfield', LastText, LastCode);
        Assert.AreEqual('No. must be equal to ''OTHER''  in LEW Host: No.=H. Current value is ''H''.', LastText, 'the test session''s GetLastErrorText');
        Assert.AreEqual('TestField', LastCode, 'the test session''s GetLastErrorCode');
    end;

    [Test]
    procedure WorkerErrorInfo_AssertErrorOnAProcedure_TheWorkersError()
    var
        LastText: Text;
        LastCode: Text;
    begin
        Initialize();
        FailingWorker('errorinfo', LastText, LastCode);
        Assert.AreEqual('The worker''s ErrorInfo', LastText, 'the test session''s GetLastErrorText');
        Assert.AreEqual('Dialog', LastCode, 'the test session''s GetLastErrorCode');
    end;

    // The asserterror around the RunPageBackgroundTask statement itself: what the test session reads right after it, before
    // the page closes.
    local procedure FailingWorkerOnTheCall(Mode: Text; var LastText: Text; var LastCode: Text; var LastStack: Text)
    var
        Hosts: TestPage "LEW Hosts";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters.Add('Mode', Mode);
        Hosts.OpenView();
        asserterror Results := Hosts.RunPageBackgroundTask(Codeunit::"LEW Reader", Parameters, false);
        LastText := GetLastErrorText();
        LastCode := GetLastErrorCode();
        LastStack := GetLastErrorCallStack();
        Hosts.Close();
    end;

    [Test]
    procedure WorkerUnhandledError_AssertErrorOnTheCall_TestSessionKeepsItsLastError()
    var
        LastText: Text;
        LastCode: Text;
        LastStack: Text;
        Before: Text;
        BeforeStack: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        BeforeStack := GetLastErrorCallStack();
        FailingWorkerOnTheCall('fail', LastText, LastCode, LastStack);
        Assert.AreEqual(Before, LastText, 'the test session''s GetLastErrorText');
        Assert.AreEqual('DB:RecordNotFound', LastCode, 'the test session''s GetLastErrorCode');
        Assert.AreEqual(BeforeStack, LastStack, 'the test session''s GetLastErrorCallStack');
        Assert.AreEqual(Before, GetLastErrorText(), 'the test session''s GetLastErrorText after the page closed');
    end;

    [Test]
    procedure WorkerUnhandledError_AssertErrorOnTheCall_TestSessionHoldsNone_Empty()
    var
        LastText: Text;
        LastCode: Text;
        LastStack: Text;
    begin
        Initialize();
        ClearLastError();
        FailingWorkerOnTheCall('fail', LastText, LastCode, LastStack);
        Assert.AreEqual('', LastText, 'the test session''s GetLastErrorText');
        Assert.AreEqual('', LastCode, 'the test session''s GetLastErrorCode');
    end;

    [Test]
    procedure WorkerModifyRefused_AssertErrorOnTheCall_TestSessionKeepsItsLastError()
    var
        LastText: Text;
        LastCode: Text;
        LastStack: Text;
        Before: Text;
    begin
        Initialize();
        SetTestSessionError();
        Before := GetLastErrorText();
        FailingWorkerOnTheCall('modify', LastText, LastCode, LastStack);
        Assert.AreEqual(Before, LastText, 'the test session''s GetLastErrorText');
        Assert.AreEqual('DB:RecordNotFound', LastCode, 'the test session''s GetLastErrorCode');
    end;
}