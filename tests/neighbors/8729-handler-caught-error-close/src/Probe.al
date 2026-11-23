// BC Documentation:
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-page-triggers
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-testing-application#handler-methods
// Scope: in-scope
// local object: the handlers that catch an error before they handle the page are what is under test
// Fixtures used: Assert (60021); the CEM fixtures of CemFixtures.al (objects 99500..99512, codeunit 99512 sets the scene);
//   test codeunits 97343 (Page.Run, PageHandlers) and 97344 (Page.RunModal, ModalPageHandlers), codeunit 97347 (fails)
// Probe (al-runner issue 8729): a handler that catches an error (asserterror, a TryFunction, two errors) and then handles
//   the page as the pinned cells of CemPageHandlerTests.al, CemModalHandlerTests.al and CemDocumentTests.al do. The page
//   closes as in the pinned cell of a handler that catches nothing: the caught error changes neither the triggers that run
//   at the close nor how the call ends.
// Readings: how the call ended (GetLastErrorText and GetLastErrorCode after an asserterror around it, CEM_NO_ERROR when it
//   came back), the triggers that ran (Cfg.Take()) and the rows afterwards (Help.Names()).
// Cells:
//   - Page.Run of the card by object id, the handler raises an error after it caught one with asserterror (:150), with a
//     TryFunction (:157), after two caught errors (:164), with asserterror and then ClearLastError (:199), the error
//     of Codeunit.Run (:206) and of JsonObject.ReadFrom (:213) with their result used, in a procedure of the handler (:220),
//     and after none (:171, as CemPageHandlerTests.al CardStaticRun_HandlerError_NoError);
//   - the same card, the handler caught an error with asserterror and returns (:178), invokes OK (:185), returns while
//     OnQueryClosePage raises (:192);
//   - Page.Run of the list by object id, the handler caught an error with asserterror and raises (:227) or returns (:234);
//   - Page.RunModal of the card by object id, the handler caught an error with asserterror and raises (:370) or returns
//     (:377), or caught nothing and raises (:384);
//   - Page.RunModal of the list by object id, the handler caught an error with asserterror and raises (:391);
//   - Page.RunModal of the document, the handler raises after it caught an error with asserterror (:398), a TryFunction
//     (:405), asserterror and then ClearLastError (:412), Codeunit.Run (:419), JsonObject.ReadFrom (:426), or in a
//     procedure of the handler (:433).
// BC versions: 29.1 (local W1 container).

// Every test binds only PageHandlers, as in CemPageHandlerTests.al.
codeunit 97343 "CEM Caught Run Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";
        Runner: Codeunit "CEM Runner";
        Catch: Integer;

    // What the handler catches before it handles the page: 0 nothing, 1 one error with asserterror, 2 one error with a
    // TryFunction, 3 two errors with asserterror, 4 one error with asserterror and then ClearLastError, 5 the error of a
    // codeunit run with Codeunit.Run, 6 the error of JsonObject.ReadFrom, 7 an error a procedure of the handler catches.
    local procedure CatchErrors()
    var
        Json: JsonObject;
    begin
        case Catch of
            1:
                asserterror Error('CEM caught error');
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

    local procedure RunCard(Source: Text; Mode: Integer; CatchMode: Integer; Text: Text; Code: Text; Log: Text)
    var
        Row: Record "CEM Row";
    begin
        Help.Setup(Source, 'card', Mode);
        Catch := CatchMode;
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardStatic(Row);
            Error('CEM_NO_ERROR');
        end;
        Check(Text, Code, Log, GetLastErrorText(), GetLastErrorCode());
    end;

    local procedure RunList(Source: Text; Mode: Integer; CatchMode: Integer; Text: Text; Code: Text; Log: Text)
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
    procedure CardCatches(var P: TestPage "CEM Card")
    begin
        CatchErrors();
        case Cfg.Mode() of
            1:
                P.OK().Invoke();
            7:
                Error('CEM handler error');
        end;
    end;

    [PageHandler]
    procedure ListCatches(var P: TestPage "CEM List")
    begin
        CatchErrors();
        case Cfg.Mode() of
            1:
                P.OK().Invoke();
            7:
                Error('CEM handler error');
        end;
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtWithAssertError_HandlerError()
    begin
        RunCard('none', 7, 1, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtWithTryFunction_HandlerError()
    begin
        RunCard('none', 7, 2, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtTwoErrors_HandlerError()
    begin
        RunCard('none', 7, 3, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtNothing_HandlerError()
    begin
        RunCard('none', 7, 0, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtWithAssertError_HandlerReturns()
    begin
        RunCard('none', 0, 1, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtWithAssertError_HandlerOk()
    begin
        RunCard('none', 1, 1, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtWithAssertError_HandlerReturns_QueryCloseError()
    begin
        RunCard('query', 0, 1, 'CEM_NO_ERROR', 'Dialog', 'Q-card OK; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtThenClearLastError_HandlerError()
    begin
        RunCard('none', 7, 4, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtByCodeunitRun_HandlerError()
    begin
        RunCard('none', 7, 5, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtByJsonReadFrom_HandlerError()
    begin
        RunCard('none', 7, 6, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardCatches')]
    procedure CardStaticRun_HandlerCaughtInAProcedure_HandlerError()
    begin
        RunCard('none', 7, 7, 'CEM handler error', 'Dialog', 'Q-card OK; C-card; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_HandlerCaughtWithAssertError_HandlerError()
    begin
        RunList('none', 7, 1, 'CEM handler error', 'Dialog', 'Q-list OK; C-list; ');
    end;

    [Test]
    [HandlerFunctions('ListCatches')]
    procedure ListStaticRun_HandlerCaughtWithAssertError_HandlerReturns()
    begin
        RunList('none', 0, 1, 'CEM_NO_ERROR', 'Dialog', 'Q-list OK; C-list; ');
    end;
}

// Every test binds only ModalPageHandlers, as in CemModalHandlerTests.al and CemDocumentTests.al.
codeunit 97344 "CEM Caught RunModal Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";
        Catch: Integer;

    // What the handler catches before it handles the page: 0 nothing, 1 one error with asserterror, 2 one error with a
    // TryFunction, 4 one error with asserterror and then ClearLastError, 5 the error of a codeunit run with Codeunit.Run,
    // 6 the error of JsonObject.ReadFrom, 7 an error a procedure of the handler catches.
    local procedure CatchErrors()
    var
        Json: JsonObject;
    begin
        case Catch of
            1:
                asserterror Error('CEM caught error');
            2:
                if TryRaise() then;
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

    local procedure RunModalCard(Mode: Integer; CatchMode: Integer; Text: Text; Code: Text; Log: Text)
    var
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'card', Mode);
        Catch := CatchMode;
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Card", Row);
            Error('CEM_NO_ERROR');
        end;
        Check(Text, Code, Log, GetLastErrorText(), GetLastErrorCode());
    end;

    local procedure RunModalList(Mode: Integer; CatchMode: Integer; Text: Text; Code: Text; Log: Text)
    var
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'list', Mode);
        Catch := CatchMode;
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM List", Row);
            Error('CEM_NO_ERROR');
        end;
        Check(Text, Code, Log, GetLastErrorText(), GetLastErrorCode());
    end;

    local procedure RunModalDoc(Mode: Integer; CatchMode: Integer; Text: Text; Code: Text; Log: Text)
    var
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('none', 'doc', Mode);
        Catch := CatchMode;
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            Error('CEM_NO_ERROR');
        end;
        Check(Text, Code, Log, GetLastErrorText(), GetLastErrorCode());
    end;

    [ModalPageHandler]
    procedure CardModalCatches(var P: TestPage "CEM Card")
    begin
        CatchErrors();
        if Cfg.Mode() = 7 then
            Error('CEM handler error');
    end;

    [ModalPageHandler]
    procedure ListModalCatches(var P: TestPage "CEM List")
    begin
        CatchErrors();
        if Cfg.Mode() = 7 then
            Error('CEM handler error');
    end;

    [ModalPageHandler]
    procedure DocModalCatches(var P: TestPage "CEM Doc")
    begin
        CatchErrors();
        if Cfg.Mode() = 7 then
            Error('CEM handler error');
    end;

    [Test]
    [HandlerFunctions('CardModalCatches')]
    procedure CardStaticRunModal_HandlerCaughtWithAssertError_HandlerError()
    begin
        RunModalCard(7, 1, 'CEM handler error', 'Dialog', 'Q-card LookupCancel; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardModalCatches')]
    procedure CardStaticRunModal_HandlerCaughtWithAssertError_HandlerReturns()
    begin
        RunModalCard(0, 1, 'CEM_NO_ERROR', 'Dialog', 'Q-card LookupCancel; C-card; ');
    end;

    [Test]
    [HandlerFunctions('CardModalCatches')]
    procedure CardStaticRunModal_HandlerCaughtNothing_HandlerError()
    begin
        RunModalCard(7, 0, 'CEM handler error', 'Dialog', 'Q-card LookupCancel; C-card; ');
    end;

    [Test]
    [HandlerFunctions('ListModalCatches')]
    procedure ListStaticRunModal_HandlerCaughtWithAssertError_HandlerError()
    begin
        RunModalList(7, 1, 'CEM handler error', 'Dialog', 'Q-list LookupCancel; C-list; ');
    end;

    [Test]
    [HandlerFunctions('DocModalCatches')]
    procedure DocumentStaticRunModal_HandlerCaughtWithAssertError_HandlerError()
    begin
        RunModalDoc(7, 1, 'CEM handler error', 'Dialog', 'Q-doc LookupCancel; C-doc; ');
    end;

    [Test]
    [HandlerFunctions('DocModalCatches')]
    procedure DocumentStaticRunModal_HandlerCaughtWithTryFunction_HandlerError()
    begin
        RunModalDoc(7, 2, 'CEM handler error', 'Dialog', 'Q-doc LookupCancel; C-doc; ');
    end;

    [Test]
    [HandlerFunctions('DocModalCatches')]
    procedure DocumentStaticRunModal_HandlerCaughtThenClearLastError_HandlerError()
    begin
        RunModalDoc(7, 4, 'CEM handler error', 'Dialog', 'Q-doc LookupCancel; C-doc; ');
    end;

    [Test]
    [HandlerFunctions('DocModalCatches')]
    procedure DocumentStaticRunModal_HandlerCaughtByCodeunitRun_HandlerError()
    begin
        RunModalDoc(7, 5, 'CEM handler error', 'Dialog', 'Q-doc LookupCancel; C-doc; ');
    end;

    [Test]
    [HandlerFunctions('DocModalCatches')]
    procedure DocumentStaticRunModal_HandlerCaughtByJsonReadFrom_HandlerError()
    begin
        RunModalDoc(7, 6, 'CEM handler error', 'Dialog', 'Q-doc LookupCancel; C-doc; ');
    end;

    [Test]
    [HandlerFunctions('DocModalCatches')]
    procedure DocumentStaticRunModal_HandlerCaughtInAProcedure_HandlerError()
    begin
        RunModalDoc(7, 7, 'CEM handler error', 'Dialog', 'Q-doc LookupCancel; C-doc; ');
    end;
}

// The codeunit a handler runs with Codeunit.Run: it fails.
codeunit 97347 "CEM Caught Failing"
{
    trigger OnRun()
    begin
        Error('CEM caught error');
    end;
}
