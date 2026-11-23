// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-page-triggers
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-testing-application#handler-methods
// Scope: in-scope
// local object: the handlers that catch an error before they handle the page are what is under test
// Fixtures used: Probe Observer (codeunit 69160); the CEM fixtures of CemFixtures.al (objects 99500..99512, codeunit 99512 sets the
//   scene); test codeunit 97373
// Probe (al-runner issue 8729, next to CemHandlerCaughtErrors.al): every test of CemDocumentTests.al again, with the same scene, run and
//   expectations, but the handler first catches an error with asserterror (`asserterror Error('CEM caught error')`) and
//   then does what the pinned handler does. A test that passes here gives the pinned answer of a handler that caught
//   nothing; the name of each test is the pinned one with `Handler` read as `HandlerCaught` (CardStaticRun_HandlerOk_NoError
//   becomes CardStaticRun_HandlerCaughtOk_NoError).
// Readings: those of CemDocumentTests.al.
// BC versions: 27, 28, 29 (corpus CI); 29.1 (local W1 container).
codeunit 97373 "CEM Caught Document Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";
        Runner: Codeunit "CEM Runner";

    // CLAIM: DocumentStaticRunModal: handler returns without closing, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 0);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'LookupCancel | Q-doc LookupCancel; C-doc; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-doc LookupCancel; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler OK.Invoke, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 1);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'LookupOK | Q-doc LookupOK; C-doc; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-doc LookupOK; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler Cancel.Invoke, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtCancel_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 2);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'LookupCancel | Q-doc LookupCancel; C-doc; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-doc LookupCancel; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler edits Name then returns, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtEditReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 4);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'LookupCancel | M-doc EDIT; T-mod R1 EDIT; Q-doc LookupCancel; C-doc; | R1=EDIT/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-doc EDIT; T-mod R1 EDIT; Q-doc LookupCancel; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler edits Name then OK, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtEditOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 5);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'LookupOK | M-doc EDIT; T-mod R1 EDIT; Q-doc LookupOK; C-doc; | R1=EDIT/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'M-doc EDIT; T-mod R1 EDIT; Q-doc LookupOK; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler raises Error, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtError_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 7);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM handler error');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'Q-doc LookupCancel; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets the Name the table refuses then returns, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtTableRefusesReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 8);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM table modify refused (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc FAILMOD; T-mod R1 FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets the Name the table refuses then OK, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtTableRefusesOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 9);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM table modify refused (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc FAILMOD; T-mod R1 FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets a line Qty the line table refuses then returns, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtLineRefusesReturns_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 11);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM line modify refused');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'L-mod R1 1 -1; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets a line Qty the line table refuses then OK, no error anywhere.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtLineRefusesOk_NoError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', 12);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'None | L-mod R1 1 -1; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'L-mod R1 1 -1; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler returns without closing, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 0);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'MissingUIHandler');
        O.Expect('log', 'Q-doc LookupCancel; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler OK.Invoke, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 1);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'TestWrapped:MissingUIHandler');
        O.Expect('log', 'Q-doc LookupOK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler Cancel.Invoke, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtCancel_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 2);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'TestWrapped:MissingUIHandler');
        O.Expect('log', 'Q-doc LookupCancel; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler edits Name then returns, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtEditReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 4);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'MissingUIHandler');
        O.Expect('log', 'M-doc EDIT; T-mod R1 EDIT; Q-doc LookupCancel; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler edits Name then OK, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtEditOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 5);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'TestWrapped:MissingUIHandler');
        O.Expect('log', 'M-doc EDIT; T-mod R1 EDIT; Q-doc LookupOK; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler raises Error, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtError_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 7);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Unhandled UI: Message CEM query close error');
        O.Expect('end|code', 'MissingUIHandler');
        O.Expect('log', 'Q-doc LookupCancel; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets the Name the table refuses then returns, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtTableRefusesReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 8);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM table modify refused (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc FAILMOD; T-mod R1 FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets the Name the table refuses then OK, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtTableRefusesOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 9);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM table modify refused (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc FAILMOD; T-mod R1 FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets a line Qty the line table refuses then returns, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtLineRefusesReturns_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 11);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM line modify refused');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'L-mod R1 1 -1; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets a line Qty the line table refuses then OK, OnQueryClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtLineRefusesOk_QueryCloseError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 12);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'None | L-mod R1 1 -1; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'L-mod R1 1 -1; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler returns without closing, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtReturns_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 0);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-doc LookupCancel; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler OK.Invoke, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 1);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-doc LookupOK; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler Cancel.Invoke, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtCancel_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 2);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-doc LookupCancel; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler edits Name then returns, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtEditReturns_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 4);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'M-doc EDIT; T-mod R1 EDIT; Q-doc LookupCancel; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler edits Name then OK, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtEditOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 5);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'M-doc EDIT; T-mod R1 EDIT; Q-doc LookupOK; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler raises Error, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtError_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 7);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'One or more errors occurred while closing the page.\\CEM close page error');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'Q-doc LookupCancel; C-doc; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets the Name the table refuses then returns, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtTableRefusesReturns_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 8);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM table modify refused (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc FAILMOD; T-mod R1 FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets the Name the table refuses then OK, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtTableRefusesOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 9);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM table modify refused (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc FAILMOD; T-mod R1 FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets a line Qty the line table refuses then returns, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtLineRefusesReturns_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 11);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'CEM line modify refused');
        O.Expect('end|code', 'TestWrapped');
        O.Expect('log', 'L-mod R1 1 -1; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets a line Qty the line table refuses then OK, OnClosePage raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtLineRefusesOk_ClosePageError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('close', 'doc', 12);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('returned', 'None | L-mod R1 1 -1; | R1=start/ (1 rows) L1=1 L2=2');
        O.Expect('end|text', 'CEM_NO_ERROR');
        O.Expect('end|code', 'Dialog');
        O.Expect('log', 'L-mod R1 1 -1; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler edits Name then returns, OnModifyRecord of the page raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtEditReturns_PageModifyRecordError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('modrec', 'doc', 4);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM page modify error (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc EDIT; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler edits Name then OK, OnModifyRecord of the page raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtEditOk_PageModifyRecordError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('modrec', 'doc', 5);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM page modify error (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc EDIT; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    // CLAIM: DocumentStaticRunModal: handler sets the Name the table refuses then returns, OnModifyRecord of the page raises an error.
    [Test]
    [HandlerFunctions('CemDocModal')]
    procedure DocumentStaticRunModal_HandlerCaughtTableRefusesReturns_PageModifyRecordError()
    var
        O: Codeunit "Probe Observer";
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('modrec', 'doc', 8);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            O.Observe('returned', Format(Res) + ' | ' + Cfg.Peek() + '| ' + Help.Names());
            Error('CEM_NO_ERROR');
        end;
        O.ObserveError('end');
        O.Observe('log', Cfg.Take());
        O.Observe('rows', Help.Names());
        Cfg.Quiet();
        O.Expect('end|text', 'Validation error for Field: Name,  Message = ''CEM page modify error (Select Refresh to discard errors)''');
        O.Expect('end|code', 'TestValidation');
        O.Expect('log', 'M-doc FAILMOD; ');
        O.Expect('rows', 'R1=start/ (1 rows) L1=1 L2=2');
        O.Verify();
    end;

    [ModalPageHandler]
    procedure CemDocModal(var P: TestPage "CEM Doc")
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
            11:
                begin
                    P.Lines.First();
                    P.Lines.Qty.SetValue(-1);
                end;
            12:
                begin
                    P.Lines.First();
                    P.Lines.Qty.SetValue(-1);
                    P.OK().Invoke();
                end;
        end;
    end;
}
