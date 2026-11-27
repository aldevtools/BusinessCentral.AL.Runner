// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/errorinfo/errorinfo-addaction-method
// Scope: in-scope
// Probe: al-runner issue 8401. The reporter measured on BC 28.4 W1 that Error(ErrorInfo) whose action names the method Run
// fails with the text and code asserted below; the other cells are what the corpus CI matrix measures.
//
// Error(ErrorInfo) with AddAction: the method name Run (any case, with a description, after another action, on codeunit 0
// or a missing codeunit, collectible in and out of a collecting scope, Internal), the names OnRun and NoSuchMethod, methods
// of other signatures. BC refuses the AddAction call itself for a Run method, so no Error, TestField or FieldError call with such an ErrorInfo is reachable.
// Fixtures used: Probe Observer (69160); table 73026 and codeunit 73027 (EnTErrorInfoProps.al, EnTErrorInfoRaise.al);
// self-contained codeunits 75840, 75841, 75842
// BC versions: 27.5+

codeunit 75842 "LT EnT ErrorInfo Run Action"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Error(ErrorInfo) whose action names the method Run: the area matrix of al-runner issue 8401. Each cell raises one
    // ErrorInfo and observes the text and the code of the error that comes out (or that no error came out).

    [Test]
    procedure RunAction_AddActionAlone()
    var
        Observer: Codeunit "Probe Observer";
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'run action, not raised';
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run');
        Observer.Observe('text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Expect('text', 'The type Microsoft.Dynamics.Nav.Types.DataError is unknown.');
        Observer.Expect('code', 'NotSupportedType');
        Observer.Verify();
    end;

    [Test]
    procedure RunAction_NoTableNo_AssertError()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'plain with run action';
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run');
        ExpectLastError('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure RunAction_TableNo_AssertError()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'plain with table run action';
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Run Target", 'Run');
        ExpectLastError('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure RunAction_CodeunitRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Observer.Observe('run', Format(Codeunit.Run(Codeunit::"LT EnT ErrorInfo Run Raiser")));
        Observer.Observe('text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Expect('run', 'No');
        Observer.Expect('text', 'The type Microsoft.Dynamics.Nav.Types.DataError is unknown.');
        Observer.Expect('code', 'NotSupportedType');
        Observer.Verify();
    end;

    [Test]
    procedure RunAction_LowerCase()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'lower case run';
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'run');
        ExpectLastError('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure RunAction_WithDescription()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'run with description';
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run', 'A description');
        ExpectLastError('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure RunAction_AfterProperAction()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'proper then run';
        ErrInfo.AddAction('Do it', Codeunit::"LT EnT ErrorInfo Actions", 'DoAction');
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run');
        ExpectLastError('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure RunAction_CodeunitZero()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'run on codeunit zero';
        ErrInfo.AddAction('Fix it', 0, 'Run');
        ExpectRaise(ErrInfo, 'run on codeunit zero', 'Dialog');
    end;

    [Test]
    procedure RunAction_CodeunitMissing()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'run on missing codeunit';
        ErrInfo.AddAction('Fix it', 99999999, 'Run');
        ExpectRaise(ErrInfo, 'run on missing codeunit', 'Dialog');
    end;

    [Test]
    procedure RunAction_EmptyMessage()
    var
        ErrInfo: ErrorInfo;
    begin
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run');
        ExpectLastError('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure RunAction_Internal()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'internal with run action';
        ErrInfo.ErrorType := ErrorType::Internal;
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run');
        ExpectLastError('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure RunAction_Collectible_InCollectScope()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ClearCollectedErrors();
        asserterror CollectRunAction(Observer);
        Observer.Observe('text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Observe('collected', Format(GetCollectedErrors().Count()));
        ClearCollectedErrors();
        Observer.Expect('text', 'The type Microsoft.Dynamics.Nav.Types.DataError is unknown.');
        Observer.Expect('code', 'NotSupportedType');
        Observer.Expect('collected', '0');
        Observer.Verify();
    end;

    [Test]
    procedure RunAction_Collectible_OutsideScope()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo := ErrorInfo.Create('collectible run outside scope', true);
        asserterror ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run');
        ExpectLastError('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure RunAction_OnRunName()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'onrun as action';
        ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'OnRun');
        ExpectRaise(ErrInfo, 'onrun as action', 'Dialog');
    end;

    [Test]
    procedure RunAction_NoSuchMethod()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'plain with bad action';
        ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'NoSuchMethod');
        ExpectRaise(ErrInfo, 'plain with bad action', 'Dialog');
    end;

    [Test]
    procedure RunAction_RecordParameterMethod()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'record parameter action';
        ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Run Target", 'TakesRecord');
        ExpectRaise(ErrInfo, 'record parameter action', 'Dialog');
    end;

    [Test]
    procedure RunAction_NoParameterMethod()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'no parameter action';
        ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Run Target", 'NoParams');
        ExpectRaise(ErrInfo, 'no parameter action', 'Dialog');
    end;

    local procedure ExpectLastError(ExpectedText: Text; ExpectedCode: Text)
    var
        Observer: Codeunit "Probe Observer";
    begin
        Observer.Observe('text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Expect('text', ExpectedText);
        Observer.Expect('code', ExpectedCode);
        Observer.Verify();
    end;

    local procedure ExpectRaise(ErrInfo: ErrorInfo; ExpectedText: Text; ExpectedCode: Text)
    var
        Observer: Codeunit "Probe Observer";
    begin
        asserterror Error(ErrInfo);
        Observer.Observe('text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Expect('text', ExpectedText);
        Observer.Expect('code', ExpectedCode);
        Observer.Verify();
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectRunAction(var Observer: Codeunit "Probe Observer")
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo := ErrorInfo.Create('collectible with run action', true);
        ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run');
        Error(ErrInfo);
        Observer.Observe('after raise', 'went on');
    end;
}

codeunit 75840 "LT EnT ErrorInfo Run Target"
{
    TableNo = "LT EnT Info Rec";

    trigger OnRun()
    begin
    end;

    procedure TakesRecord(var Rec: Record "LT EnT Info Rec")
    begin
    end;

    procedure NoParams()
    begin
    end;
}

codeunit 75841 "LT EnT ErrorInfo Run Raiser"
{
    trigger OnRun()
    var
        ErrInfo: ErrorInfo;
    begin
        ErrInfo.Message := 'plain with run action';
        ErrInfo.AddAction('Fix it', Codeunit::"LT EnT ErrorInfo Actions", 'Run');
        Error(ErrInfo);
    end;
}
