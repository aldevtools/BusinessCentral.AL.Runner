// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-page-triggers
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-testing-application#handler-methods
// Scope: in-scope
// local object: the handlers that catch an error before they handle the page are what is under test
// Fixtures used: Probe Observer (codeunit 69160); the CEM fixtures of CemFixtures.al (objects 99500..99512, codeunit 99512 sets the
//   scene); test codeunit 97372
// Probe (al-runner issue 8729, next to CemHandlerCaughtErrors.al): every test of CemPageHandlerTests.al again, with the same scene, run and
//   expectations, but the handler first catches an error with asserterror (`asserterror Error('CEM caught error')`) and
//   then does what the pinned handler does. A test that passes here gives the pinned answer of a handler that caught
//   nothing; the name of each test is the pinned one with `Handler` read as `HandlerCaught` (CardStaticRun_HandlerOk_NoError
//   becomes CardStaticRun_HandlerCaughtOk_NoError).
// Readings: those of CemPageHandlerTests.al.
// BC versions: 27, 28, 29 (corpus CI); 29.1 (local W1 container).
codeunit 97372 "CEM Caught Page Handler Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";
        Runner: Codeunit "CEM Runner";

    // CLAIM: CardStaticRun: handler returns without closing, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler OK.Invoke, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler Cancel.Invoke, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtCancel_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 2);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'The built-in action = Cancel is not found on the page.');
        O.Expect('end|code', 'TestActionNotFound');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler Close(), no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtClose_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler raises Error, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtError_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 7);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM handler error');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler edits Name then returns, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtEditReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 4);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; C-card; | R1=EDIT/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler edits Name then OK, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtEditOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 5);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; C-card; | R1=EDIT/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler sets the Name the table refuses then returns, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtTableRefusesReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 8);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card FAILMOD; T-mod R1 FAILMOD; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card FAILMOD; T-mod R1 FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler returns without closing, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler OK.Invoke, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'TestWrapped:MissingUIHandler');
        O.Expect('log', 'Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler Cancel.Invoke, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtCancel_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 2);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'The built-in action = Cancel is not found on the page.');
        O.Expect('end|code', 'TestActionNotFound');
        O.Expect('log', 'Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler Close(), OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtClose_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'MissingUIHandler');
        O.Expect('log', 'Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler raises Error, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtError_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 7);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM handler error');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler edits Name then returns, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtEditReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 4);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; | R1=EDIT/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler edits Name then OK, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtEditOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 5);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'TestWrapped:MissingUIHandler');
        O.Expect('log', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler sets the Name the table refuses then returns, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtTableRefusesReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 8);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card FAILMOD; T-mod R1 FAILMOD; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card FAILMOD; T-mod R1 FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler returns without closing, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtReturns_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'card', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler OK.Invoke, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler Close(), OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtClose_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'card', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler edits Name then OK, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtEditOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'card', 5);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler returns without closing, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtReturns_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'card', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler OK.Invoke, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtOk_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\Unhandled UI: Confirm CEM close confirm?');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler Close(), OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtClose_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'card', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\Unhandled UI: Confirm CEM close confirm?');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler edits Name then OK, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtEditOk_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'card', 5);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\Unhandled UI: Confirm CEM close confirm?');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler edits Name then returns, OnModifyRecord of the page raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtEditReturns_PageModifyRecordError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('modrec', 'card', 4);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card EDIT; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card EDIT; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler edits Name then OK, OnModifyRecord of the page raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtEditOk_PageModifyRecordError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('modrec', 'card', 5);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card EDIT; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card EDIT; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardStaticRun: handler sets the Name the table refuses then returns, OnModifyRecord of the page raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardStaticRun_HandlerCaughtTableRefusesReturns_PageModifyRecordError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('modrec', 'card', 8);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card FAILMOD; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler returns without closing, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler OK.Invoke, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler Close(), no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtClose_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler edits Name then returns, no error anywhere.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtEditReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 4);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; C-card; | R1=EDIT/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler returns without closing, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler OK.Invoke, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'TestWrapped:MissingUIHandler');
        O.Expect('log', 'Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler Close(), OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtClose_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'MissingUIHandler');
        O.Expect('log', 'Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler edits Name then returns, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtEditReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'card', 4);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; | R1=EDIT/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-card EDIT; T-mod R1 EDIT; Q-card OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler returns without closing, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtReturns_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'card', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler OK.Invoke, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler returns without closing, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtReturns_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'card', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-card OK; C-card; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: CardVariableRun: handler OK.Invoke, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemCardRun')]
    procedure CardVariableRun_HandlerCaughtOk_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\Unhandled UI: Confirm CEM close confirm?');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-card OK; C-card; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler returns without closing, no error anywhere.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'list', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-list OK; C-list; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-list OK; C-list; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler OK.Invoke, no error anywhere.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'list', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-list OK; C-list; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-list OK; C-list; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler Close(), no error anywhere.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtClose_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'list', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-list OK; C-list; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-list OK; C-list; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler raises Error, no error anywhere.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtError_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'list', 7);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM handler error');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-list OK; C-list; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler returns without closing, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'list', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-list OK; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-list OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler OK.Invoke, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'list', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'TestWrapped:MissingUIHandler');
        O.Expect('log', 'Q-list OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler Close(), OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtClose_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'list', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'MissingUIHandler');
        O.Expect('log', 'Q-list OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler raises Error, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtError_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'list', 7);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM handler error');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-list OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler returns without closing, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtReturns_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'list', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-list OK; C-list; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-list OK; C-list; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler OK.Invoke, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'list', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-list OK; C-list; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler returns without closing, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtReturns_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'list', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-list OK; C-list; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-list OK; C-list; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: ListStaticRun: handler OK.Invoke, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemListRun')]
    procedure ListStaticRun_HandlerCaughtOk_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'list', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\Unhandled UI: Confirm CEM close confirm?');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-list OK; C-list; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler returns without closing, no error anywhere.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'wizard', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-wizard OK; C-wizard; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler OK.Invoke, no error anywhere.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'wizard', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-wizard OK; C-wizard; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler Cancel.Invoke, no error anywhere.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtCancel_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'wizard', 2);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'The built-in action = Cancel is not found on the page.');
        O.Expect('end|code', 'TestActionNotFound');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler Close(), no error anywhere.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtClose_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'wizard', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-wizard OK; C-wizard; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler raises Error, no error anywhere.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtError_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'wizard', 7);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM handler error');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler returns without closing, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'wizard', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-wizard OK; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-wizard OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler OK.Invoke, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('query', 'wizard', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'TestWrapped:MissingUIHandler');
        O.Expect('log', 'Q-wizard OK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler returns without closing, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtReturns_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'wizard', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-wizard OK; C-wizard; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler OK.Invoke, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('close', 'wizard', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler returns without closing, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtReturns_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'wizard', 0);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'Q-wizard OK; C-wizard; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler OK.Invoke, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtOk_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'wizard', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\Unhandled UI: Confirm CEM close confirm?');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler Cancel.Invoke, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtCancel_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'wizard', 2);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'The built-in action = Cancel is not found on the page.');
        O.Expect('end|code', 'TestActionNotFound');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler Close(), OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtClose_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'wizard', 3);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\Unhandled UI: Confirm CEM close confirm?');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: WizardRun: handler raises Error, OnClosePage asks a Confirm nobody handles.
    [Test]
    [HandlerFunctions('CemWizardRun')]
    procedure WizardRun_HandlerCaughtError_ClosePageConfirmUnhandled()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
    begin
        Help.Setup('confirm', 'wizard', 7);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunWizard(Row);
            O.Observe('returned', Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM handler error');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-wizard OK; C-wizard; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    [PageHandler]
    procedure CemCardRun(var P: TestPage "CEM Card")
    begin
        asserterror Error('CEM caught error');
        case Cfg.Mode() of
            0:
                ;
            1:
                begin
                    P.OK().Invoke();
                end;
            2:
                begin
                    P.Cancel().Invoke();
                end;
            3:
                begin
                    P.Close();
                end;
            4:
                begin
                    P.Name.SetValue('EDIT');
                end;
            5:
                begin
                    P.Name.SetValue('EDIT');
                    P.OK().Invoke();
                end;
            6:
                begin
                    P.Name.SetValue('EDIT');
                    P.Cancel().Invoke();
                end;
            7:
                begin
                    Error('CEM handler error');
                end;
            8:
                begin
                    P.Name.SetValue('FAILMOD');
                end;
            9:
                begin
                    P.Name.SetValue('FAILMOD');
                    P.OK().Invoke();
                end;
            10:
                begin
                    P.Name.SetValue('EDIT');
                    P.Close();
                end;
        end;
    end;
    [PageHandler]
    procedure CemListRun(var P: TestPage "CEM List")
    begin
        asserterror Error('CEM caught error');
        case Cfg.Mode() of
            0:
                ;
            1:
                begin
                    P.OK().Invoke();
                end;
            2:
                begin
                    P.Cancel().Invoke();
                end;
            3:
                begin
                    P.Close();
                end;
            4:
                begin
                    P.Name.SetValue('EDIT');
                end;
            5:
                begin
                    P.Name.SetValue('EDIT');
                    P.OK().Invoke();
                end;
            6:
                begin
                    P.Name.SetValue('EDIT');
                    P.Cancel().Invoke();
                end;
            7:
                begin
                    Error('CEM handler error');
                end;
            8:
                begin
                    P.Name.SetValue('FAILMOD');
                end;
            9:
                begin
                    P.Name.SetValue('FAILMOD');
                    P.OK().Invoke();
                end;
            10:
                begin
                    P.Name.SetValue('EDIT');
                    P.Close();
                end;
        end;
    end;
    [PageHandler]
    procedure CemWizardRun(var P: TestPage "CEM Wizard")
    begin
        asserterror Error('CEM caught error');
        case Cfg.Mode() of
            0:
                ;
            1:
                begin
                    P.OK().Invoke();
                end;
            2:
                begin
                    P.Cancel().Invoke();
                end;
            3:
                begin
                    P.Close();
                end;
            7:
                begin
                    Error('CEM handler error');
                end;
        end;
    end;
}
