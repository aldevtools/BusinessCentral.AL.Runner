codeunit 58400 "NB Action Target"
{
    procedure FixIt(E: ErrorInfo)
    begin
    end;
}

codeunit 58401 "NB Table Target"
{
    TableNo = "NB Rec";

    trigger OnRun()
    begin
    end;
}

codeunit 58402 "NB Raise With Run Action"
{
    trigger OnRun()
    var
        E: ErrorInfo;
    begin
        E.Message := 'plain with run action';
        E.AddAction('Fix it', Codeunit::"NB Action Target", 'Run');
        Error(E);
    end;
}

codeunit 58403 "NB Test Subtype Target"
{
    Subtype = Test;
}

table 58400 "NB Rec"
{
    fields { field(1; PK; Integer) { } }
    keys { key(PK; PK) { Clustered = true; } }
}

codeunit 58404 "NB Run Action Tests"
{
    Subtype = Test;

    [Test]
    procedure IssueRepro_CodeunitRun()
    begin
        if Codeunit.Run(Codeunit::"NB Raise With Run Action") then
            Error('the codeunit should fail');
        Expect('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure Run_TableNoCodeunit()
    var
        E: ErrorInfo;
    begin
        asserterror E.AddAction('Fix it', Codeunit::"NB Table Target", 'Run');
        Expect('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure Run_LowerCase()
    var
        E: ErrorInfo;
    begin
        asserterror E.AddAction('Fix it', Codeunit::"NB Action Target", 'run');
        Expect('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure Run_WithDescription()
    var
        E: ErrorInfo;
    begin
        asserterror E.AddAction('Fix it', Codeunit::"NB Action Target", 'Run', 'A description');
        Expect('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
    end;

    [Test]
    procedure Run_CollectibleInCollectScope()
    begin
        ClearCollectedErrors();
        asserterror CollectRunAction();
        Expect('The type Microsoft.Dynamics.Nav.Types.DataError is unknown.', 'NotSupportedType');
        if GetCollectedErrors().Count() <> 0 then
            Error('nothing should be collected');
    end;

    [Test]
    procedure Run_CodeunitZero()
    var
        E: ErrorInfo;
    begin
        E.Message := 'zero';
        E.AddAction('Fix it', 0, 'Run');
        asserterror Error(E);
        Expect('zero', 'Dialog');
    end;

    [Test]
    procedure Run_MissingCodeunit()
    var
        E: ErrorInfo;
    begin
        E.Message := 'missing';
        E.AddAction('Fix it', 99999999, 'Run');
        asserterror Error(E);
        Expect('missing', 'Dialog');
    end;

    [Test]
    procedure OnRunName()
    var
        E: ErrorInfo;
    begin
        E.Message := 'onrun';
        E.AddAction('Fix it', Codeunit::"NB Action Target", 'OnRun');
        asserterror Error(E);
        Expect('onrun', 'Dialog');
    end;

    [Test]
    procedure ProperMethod()
    var
        E: ErrorInfo;
    begin
        E.Message := 'proper';
        E.AddAction('Fix it', Codeunit::"NB Action Target", 'FixIt');
        asserterror Error(E);
        Expect('proper', 'Dialog');
    end;

    [Test]
    procedure Run_UpperCase_Refused()
    var
        E: ErrorInfo;
    begin
        asserterror E.AddAction('Fix it', Codeunit::"NB Action Target", 'RUN');
    end;

    [Test]
    procedure Run_TestSubtypeCodeunit_Refused()
    var
        E: ErrorInfo;
    begin
        asserterror E.AddAction('Fix it', Codeunit::"NB Test Subtype Target", 'Run');
    end;

    [Test]
    procedure Run_NegativeId_Refused()
    var
        E: ErrorInfo;
    begin
        asserterror E.AddAction('Fix it', -1, 'Run');
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectRunAction()
    var
        E: ErrorInfo;
    begin
        E := ErrorInfo.Create('collectible run', true);
        E.AddAction('Fix it', Codeunit::"NB Action Target", 'Run');
        Error(E);
    end;

    local procedure Expect(T: Text; C: Text)
    begin
        if GetLastErrorText() <> T then
            Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> C then
            Error('code: %1', GetLastErrorCode());
    end;
}
