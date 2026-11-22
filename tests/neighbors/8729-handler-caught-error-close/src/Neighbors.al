// Shapes next to the cells of corpus PR CemHandlerCaughtErrors.al (al-runner issue 8729), each measured on a Business Central
// 29.1 W1 container (3 runs): a handler that caught an error, in a close no cell measures with a caught error.
codeunit 97345 "CHN Run Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";
        Runner: Codeunit "CEM Runner";

    local procedure Check(Text: Text; Code: Text; Log: Text; EndText: Text; EndCode: Text)
    begin
        Assert.AreEqual(Text, EndText, 'how the call ended, text');
        Assert.AreEqual(Code, EndCode, 'how the call ended, code');
        Assert.AreEqual(Log, Cfg.Take(), 'the triggers that ran');
        Assert.AreEqual('R1=start/ (1 rows) L1=1 L2=2', Help.Names(), 'the rows afterwards');
        Cfg.Quiet();
    end;

    [PageHandler]
    procedure CardCatchesThenOk(var P: TestPage "CEM Card")
    begin
        asserterror Error('CEM caught error');
        P.OK().Invoke();
    end;

    [PageHandler]
    procedure ListCatchesThenOk(var P: TestPage "CEM List")
    begin
        asserterror Error('CEM caught error');
        P.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('CardCatchesThenOk')]
    procedure CardVariableRun_HandlerCaughtWithAssertError_HandlerOk()
    var
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'card', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunCardVariable(Row);
            Error('CEM_NO_ERROR');
        end;
        Check('CEM_NO_ERROR', 'Dialog', 'Q-card OK; C-card; ', GetLastErrorText(), GetLastErrorCode());
    end;

    [Test]
    [HandlerFunctions('ListCatchesThenOk')]
    procedure ListStaticRun_HandlerCaughtWithAssertError_HandlerOk()
    var
        Row: Record "CEM Row";
    begin
        Help.Setup('none', 'list', 1);
        Row.Get('R1');
        Row.SetRange("Code", 'R1');
        asserterror begin
            Runner.RunListStatic(Row);
            Error('CEM_NO_ERROR');
        end;
        Check('CEM_NO_ERROR', 'Dialog', 'Q-list OK; C-list; ', GetLastErrorText(), GetLastErrorCode());
    end;
}

codeunit 97346 "CHN RunModal Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Cfg: Codeunit "CEM Cfg";
        Help: Codeunit "CEM Help";

    [ModalPageHandler]
    procedure DocCatchesThenRaises(var P: TestPage "CEM Doc")
    begin
        asserterror Error('CEM caught error');
        Error('CEM handler error');
    end;

    [Test]
    [HandlerFunctions('DocCatchesThenRaises')]
    procedure DocumentStaticRunModal_HandlerCaughtWithAssertError_HandlerError_QueryCloseError()
    var
        Row: Record "CEM Row";
        Res: Action;
    begin
        Help.Setup('query', 'doc', 7);
        Row.Get('R1');
        asserterror begin
            Res := Page.RunModal(Page::"CEM Doc", Row);
            Error('CEM_NO_ERROR');
        end;
        Assert.AreEqual('CEM query close error|Dialog|Q-doc LookupCancel; ', GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Cfg.Take(), 'how the call ended, and the triggers that ran');
        Assert.AreEqual('R1=start/ (1 rows) L1=1 L2=2', Help.Names(), 'the rows afterwards');
        Cfg.Quiet();
    end;
}
