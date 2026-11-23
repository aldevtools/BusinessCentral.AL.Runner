// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-page-triggers
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-testing-application#handler-methods
// Scope: in-scope
// local object: the handlers that catch an error before they handle the page are what is under test
// Fixtures used: Assert (60021); the CEM fixtures of CemFixtures.al (objects 99500..99512, codeunit 99512 sets the scene);
//   codeunit 97347 "CEM Caught Failing" of CemHandlerCaughtErrors.al (fails)
// Probe (al-runner issue 8729): the six other ways of catching of CemHandlerCaughtErrors.al (a TryFunction, two errors,
//   asserterror and then ClearLastError, Codeunit.Run, JsonObject.ReadFrom, a procedure of the handler) before the handler
//   handles the wizard, the card run from a Page variable or the list, as the pinned cells of CemPageHandlerTests.al do.
//   The page closes as in the pinned cell of a handler that catches nothing. CemPageHandlerCaughtTests.al has these pages
//   with asserterror.
// Readings: how the call ended (GetLastErrorText and GetLastErrorCode after an asserterror around it, CEM_NO_ERROR when it
//   came back), the triggers that ran (Cfg.Take()) and the rows afterwards (Help.Names()).
// Cells, each after the handler caught an error in each of the six ways:
//   - Page.Run of the wizard from a Page variable, the handler raises an error (as CemPageHandlerTests.al
//     WizardRun_HandlerError_NoError) or returns while OnQueryClosePage raises (as WizardRun_HandlerReturns_QueryCloseError);
//   - Page.Run of the card from a Page variable, the handler returns while OnQueryClosePage raises (as
//     CardVariableRun_HandlerReturns_QueryCloseError) or invokes OK while OnClosePage raises (as
//     CardVariableRun_HandlerOk_ClosePageError);
//   - Page.Run of the list by object id, the handler raises an error (as ListStaticRun_HandlerError_NoError) or returns
//     while OnQueryClosePage raises (as ListStaticRun_HandlerReturns_QueryCloseError).
// BC versions: 29.1 (local W1 container).

// Every test binds only PageHandlers, as in CemPageHandlerTests.al.
codeunit 97374 "CEM Caught Ways Run Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";
        Runner: Codeunit "CEM Runner";
        Catch: Integer;

    // What the handler catches before it handles the page: 2 one error with a TryFunction, 3 two errors with asserterror,
    // 4 one error with asserterror and then ClearLastError, 5 the error of a codeunit run with Codeunit.Run, 6 the error of
    // JsonObject.ReadFrom, 7 an error a procedure of the handler catches.
    local procedure CatchErrors()
    var
        Json: JsonObject;
    begin
        case Catch of
            2:
                if TryRaise() then;
            3:
                begin
                    asserterror Error('CEM caught error');
                    asserterror Error('CEM second caught error');
                end;
            4:
                begin
                    asserterror Error('CEM caught error');
                    ClearLastError();
                end;
            5:
                if Codeunit.Run(Codeunit::"CEM Caught Failing") then;
            6:
                if Json.ReadFrom('x') then;
            7:
                CatchInAProcedure();
        end;
    end;

    local procedure CatchInAProcedure()
    begin
        asserterror Error('CEM caught error');
    end;

    [TryFunction]
    local procedure TryRaise()
    begin
        Error('CEM caught error');
    end;

    local procedure Check(Text: Text; Code: Text; Log: Text; EndText: Text; EndCode: Text)
    begin
        Assert.AreEqual(Text, EndText, 'how the call ended, text');
        Assert.AreEqual(Code, EndCode, 'how the call ended, code');
        Assert.AreEqual(Log, Cfg.Take(), 'the triggers that ran');
        Assert.AreEqual('R1=start/ (1 rows) L1=1 L2=2', Help.Names(), 'the rows afterwards');
        Cfg.Quiet();
    end;

    local procedure RunWizardCell(Source: Text; Mode: Integer; CatchMode: Integer; Text: Text; Code: Text; Log: Text)
    var
        Row: Record "CEM Row";
    begin
        Help.Setup(Source, 'wizard', Mode);
        Catch := CatchMode;
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            Error('CEM_NO_ERROR');
        end;
        Check(Text, Code, Log, GetLastErrorText(), GetLastErrorCode());
    end;

    local procedure RunCardVariableCell(Source: Text; Mode: Integer; CatchMode: Integer; Text: Text; Code: Text; Log: Text)
    var
        Row: Record "CEM Row";
    begin
        Help.Setup(Source, 'card', Mode);
        Catch := CatchMode;
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            Error('CEM_NO_ERROR');
        end;
        Check(Text, Code, Log, GetLastErrorText(), GetLastErrorCode());
    end;

    local procedure RunListCell(Source: Text; Mode: Integer; CatchMode: Integer; Text: Text; Code: Text; Log: Text)
    var
        Row: Record "CEM Row";
    begin
        Help.Setup(Source, 'list', Mode);
        Catch := CatchMode;
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            Error('CEM_NO_ERROR');
        end;
        Check(Text, Code, Log, GetLastErrorText(), GetLastErrorCode());
    end;

    [PageHandler]
    procedure WizardCatches(var P: TestPage "CEM Wizard")
    begin
        CatchErrors();
        if Cfg.Mode() = 7 then
            Error('CEM handler error');
    end;

    [PageHandler]
    procedure CardCatches(var P: TestPage "CEM Card")
    begin
        CatchErrors();
        if Cfg.Mode() = 1 then
            P.OK().Invoke();
    end;

    [PageHandler]
    procedure ListCatches(var P: TestPage "CEM List")
    begin
        CatchErrors();
        if Cfg.Mode() = 7 then
            Error('CEM handler error');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtWithTryFunction_HandlerError_NoError()
    begin
        RunWizardCell('none', 7, 2, 'CEM handler error', 'Dialog', 'Q-wizard OK; C-wizard; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtTwoErrors_HandlerError_NoError()
    begin
        RunWizardCell('none', 7, 3, 'CEM handler error', 'Dialog', 'Q-wizard OK; C-wizard; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtThenClearLastError_HandlerError_NoError()
    begin
        RunWizardCell('none', 7, 4, 'CEM handler error', 'Dialog', 'Q-wizard OK; C-wizard; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtByCodeunitRun_HandlerError_NoError()
    begin
        RunWizardCell('none', 7, 5, 'CEM handler error', 'Dialog', 'Q-wizard OK; C-wizard; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtByJsonReadFrom_HandlerError_NoError()
    begin
        RunWizardCell('none', 7, 6, 'CEM handler error', 'Dialog', 'Q-wizard OK; C-wizard; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtInAProcedure_HandlerError_NoError()
    begin
        RunWizardCell('none', 7, 7, 'CEM handler error', 'Dialog', 'Q-wizard OK; C-wizard; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtWithTryFunction_HandlerReturns_QueryCloseError()
    begin
        RunWizardCell('query', 0, 2, 'CEM_NO_ERROR', 'Dialog', 'Q-wizard OK; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtTwoErrors_HandlerReturns_QueryCloseError()
    begin
        RunWizardCell('query', 0, 3, 'CEM_NO_ERROR', 'Dialog', 'Q-wizard OK; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtThenClearLastError_HandlerReturns_QueryCloseError()
    begin
        RunWizardCell('query', 0, 4, 'CEM_NO_ERROR', 'Dialog', 'Q-wizard OK; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtByCodeunitRun_HandlerReturns_QueryCloseError()
    begin
        RunWizardCell('query', 0, 5, 'CEM_NO_ERROR', 'Dialog', 'Q-wizard OK; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtByJsonReadFrom_HandlerReturns_QueryCloseError()
    begin
        RunWizardCell('query', 0, 6, 'CEM_NO_ERROR', 'Dialog', 'Q-wizard OK; ');
    end;

    [Test]
    [HandlerFunctions('WizardCatches')]
    procedure WizardRun_CaughtInAProcedure_HandlerReturns_QueryCloseError()
    begin
        RunWizardCell('query', 0, 7, 'CEM_NO_ERROR', 'Dialog', 'Q-wizard OK; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtWithTryFunction_HandlerReturns_QueryCloseError()
    begin
        RunCardVariableCell('query', 0, 2, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtTwoErrors_HandlerReturns_QueryCloseError()
    begin
        RunCardVariableCell('query', 0, 3, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtThenClearLastError_HandlerReturns_QueryCloseError()
    begin
        RunCardVariableCell('query', 0, 4, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtByCodeunitRun_HandlerReturns_QueryCloseError()
    begin
        RunCardVariableCell('query', 0, 5, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtByJsonReadFrom_HandlerReturns_QueryCloseError()
    begin
        RunCardVariableCell('query', 0, 6, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtInAProcedure_HandlerReturns_QueryCloseError()
    begin
        RunCardVariableCell('query', 0, 7, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtWithTryFunction_HandlerOk_ClosePageError()
    begin
        RunCardVariableCell('close', 1, 2, 'One or more errors occurred while closing the page.\\CEM close page error', 'TestWrapped', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtTwoErrors_HandlerOk_ClosePageError()
    begin
        RunCardVariableCell('close', 1, 3, 'One or more errors occurred while closing the page.\\CEM close page error', 'TestWrapped', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtThenClearLastError_HandlerOk_ClosePageError()
    begin
        RunCardVariableCell('close', 1, 4, 'One or more errors occurred while closing the page.\\CEM close page error', 'TestWrapped', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtByCodeunitRun_HandlerOk_ClosePageError()
    begin
        RunCardVariableCell('close', 1, 5, 'One or more errors occurred while closing the page.\\CEM close page error', 'TestWrapped', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtByJsonReadFrom_HandlerOk_ClosePageError()
    begin
        RunCardVariableCell('close', 1, 6, 'One or more errors occurred while closing the page.\\CEM close page error', 'TestWrapped', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardVariableRun_CaughtInAProcedure_HandlerOk_ClosePageError()
    begin
        RunCardVariableCell('close', 1, 7, 'One or more errors occurred while closing the page.\\CEM close page error', 'TestWrapped', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtWithTryFunction_HandlerError_NoError()
    begin
        RunListCell('none', 7, 2, 'CEM handler error', 'Dialog', 'Q-list OK; C-list; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtTwoErrors_HandlerError_NoError()
    begin
        RunListCell('none', 7, 3, 'CEM handler error', 'Dialog', 'Q-list OK; C-list; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtThenClearLastError_HandlerError_NoError()
    begin
        RunListCell('none', 7, 4, 'CEM handler error', 'Dialog', 'Q-list OK; C-list; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtByCodeunitRun_HandlerError_NoError()
    begin
        RunListCell('none', 7, 5, 'CEM handler error', 'Dialog', 'Q-list OK; C-list; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtByJsonReadFrom_HandlerError_NoError()
    begin
        RunListCell('none', 7, 6, 'CEM handler error', 'Dialog', 'Q-list OK; C-list; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtInAProcedure_HandlerError_NoError()
    begin
        RunListCell('none', 7, 7, 'CEM handler error', 'Dialog', 'Q-list OK; C-list; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtWithTryFunction_HandlerReturns_QueryCloseError()
    begin
        RunListCell('query', 0, 2, 'CEM_NO_ERROR', 'Dialog', 'Q-list OK; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtTwoErrors_HandlerReturns_QueryCloseError()
    begin
        RunListCell('query', 0, 3, 'CEM_NO_ERROR', 'Dialog', 'Q-list OK; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtThenClearLastError_HandlerReturns_QueryCloseError()
    begin
        RunListCell('query', 0, 4, 'CEM_NO_ERROR', 'Dialog', 'Q-list OK; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtByCodeunitRun_HandlerReturns_QueryCloseError()
    begin
        RunListCell('query', 0, 5, 'CEM_NO_ERROR', 'Dialog', 'Q-list OK; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtByJsonReadFrom_HandlerReturns_QueryCloseError()
    begin
        RunListCell('query', 0, 6, 'CEM_NO_ERROR', 'Dialog', 'Q-list OK; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_CaughtInAProcedure_HandlerReturns_QueryCloseError()
    begin
        RunListCell('query', 0, 7, 'CEM_NO_ERROR', 'Dialog', 'Q-list OK; ');
    end;
}
