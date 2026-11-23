codeunit 50120 "NB XAU Tests"
{
    Subtype = Test;

    local procedure Source(Xml: Text; var Src: Record "NB XAU Src"; var InS: InStream)
    var
        OutS: OutStream;
    begin
        Src.DeleteAll();
        Src.Id := 1;
        Src.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Xml);
        Src.Insert();
        Src.CalcFields(Data);
        Src.Data.CreateInStream(InS, TextEncoding::UTF8);
    end;

    local procedure RunPort(XmlportId: Integer; Xml: Text)
    var
        Src: Record "NB XAU Src";
        InS: InStream;
    begin
        Source(Xml, Src, InS);
        Xmlport.Import(XmlportId, InS);
    end;

    local procedure RunSuite(App: Text[30]; HashValue: Text[30]; Xml: Text)
    var
        Imp: XmlPort "NB XAU Suite";
        Src: Record "NB XAU Src";
        InS: InStream;
    begin
        Source(Xml, Src, InS);
        Imp.SetCaller(App, HashValue);
        Imp.SetSource(InS);
        Imp.Import();
    end;

    local procedure SeedPar()
    var
        Par: Record "NB XAU Par";
        Chi: Record "NB XAU Chi";
        ChiW: Record "NB XAU ChiW";
    begin
        Par.DeleteAll();
        Chi.DeleteAll();
        ChiW.DeleteAll();
        Par.Id := 1;
        Par.Name := 'old';
        Par.Stamp := 's0';
        Par.Extra := 'e0';
        Par.Insert();
    end;

    local procedure ParRow(Id: Integer): Text
    var
        Par: Record "NB XAU Par";
    begin
        Par.Get(Id);
        exit(Format(Par.Id) + '/' + Par.Name + '/' + Par.Stamp + '/' + Par.Extra);
    end;

    local procedure SuiteRows(): Text
    var
        Suite: Record "NB XAU Suite";
        Lang: Record "NB XAU Lang";
        Line: Record "NB XAU Line";
        Eval: Record "NB XAU Eval";
        Txt: Text;
    begin
        if Suite.FindSet() then
            repeat
                Txt += 'S' + Suite.Code + '/' + Suite.Description + '/' + Suite.AppId + '/' + Suite.Hash + '/' + Format(Suite.Validation) + '/' + Format(Suite.Runner) + ';';
            until Suite.Next() = 0;
        if Lang.FindSet() then
            repeat
                Txt += 'L' + Lang.SuiteCode + '/' + Lang.Tag + ';';
            until Lang.Next() = 0;
        if Line.FindSet() then
            repeat
                Txt += 'N' + Line.SuiteCode + '/' + Format(Line.LineNo) + '/' + Format(Line.Cu) + ';';
            until Line.Next() = 0;
        if Eval.FindSet() then
            repeat
                Txt += 'E' + Eval.SuiteCode + '/' + Format(Eval.LineNo) + '/' + Eval.Name + ';';
            until Eval.Next() = 0;
        exit(Txt);
    end;

    local procedure WipeSuites()
    var
        Suite: Record "NB XAU Suite";
        Lang: Record "NB XAU Lang";
        Line: Record "NB XAU Line";
        Eval: Record "NB XAU Eval";
    begin
        Suite.DeleteAll();
        Lang.DeleteAll();
        Line.DeleteAll();
        Eval.DeleteAll();
    end;

    local procedure AreEqual(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected <%1>, got <%2>', Expected, Actual);
    end;

    // The reporter's shape: new suites, child and grandchild rows, the parent stored again after them.
    [Test]
    procedure Suite_Fresh()
    begin
        WipeSuites();
        RunSuite('app1', 'h1', '<Root><Suite Code="A" Description="da" Validation="true"><Language Tag="en"/><Line Cu="5"><Evaluator Name="e1"/></Line><Line Cu="6"/></Suite><Suite Code="B"><Line Cu="7"/></Suite></Root>');
        AreEqual('SA/da/app1/h1/Yes/7;SB//app1/h1/No/7;LA/en;NA/10000/5;NA/20000/6;NB/10000/7;EA/10000/e1;', SuiteRows());
    end;

    // A suite without child rows: stored once, no second save (main's path).
    [Test]
    procedure Suite_Fresh_NoChildren()
    begin
        WipeSuites();
        RunSuite('app1', 'h1', '<Root><Suite Code="A" Description="da"/></Root>');
        AreEqual('SA/da/app1/h1/No/7;', SuiteRows());
    end;

    // The same suite again: the key attribute's trigger skips it and its child rows.
    [Test]
    procedure Suite_SameAppSameHash_Skips()
    begin
        WipeSuites();
        RunSuite('app1', 'h1', '<Root><Suite Code="A" Description="da"><Line Cu="5"/></Suite></Root>');
        RunSuite('app1', 'h1', '<Root><Suite Code="A" Description="db"><Line Cu="6"/></Suite><Suite Code="B"><Line Cu="7"/></Suite></Root>');
        AreEqual('SA/da/app1/h1/No/7;SB//app1/h1/No/7;NA/10000/5;NB/10000/7;', SuiteRows());
    end;

    // Another hash: the trigger deletes the stored suite and the import inserts it again.
    [Test]
    procedure Suite_SameAppOtherHash_Replaces()
    begin
        WipeSuites();
        RunSuite('app1', 'h1', '<Root><Suite Code="A" Description="da" Validation="true"><Line Cu="5"/></Suite></Root>');
        RunSuite('app1', 'h2', '<Root><Suite Code="A" Description="db"><Line Cu="6"/></Suite></Root>');
        AreEqual('SA/db/app1/h2/No/7;NA/10000/5;NA/20000/6;', SuiteRows());
    end;

    // Another app: the trigger raises.
    [Test]
    procedure Suite_OtherApp_Errors()
    begin
        WipeSuites();
        RunSuite('app1', 'h1', '<Root><Suite Code="A" Description="da"><Line Cu="5"/></Suite></Root>');
        asserterror RunSuite('app2', 'h1', '<Root><Suite Code="A" Description="db"><Line Cu="6"/></Suite></Root>');
        AreEqual('Suite A belongs to another app.', GetLastErrorText());
    end;

    // A linked child with a SourceTableView: refused on main and head (not this change).
    [Test]
    procedure Suite_SuiteEvaluator()
    begin
        WipeSuites();
        RunSuite('app1', 'h1', '<Root><Suite Code="A"><Evaluator Name="e0"/><Line Cu="5"/></Suite></Root>');
        AreEqual('SA//app1/h1/No/7;NA/10000/5;EA/0/e0;', SuiteRows());
    end;

    // A field element trigger assigns a field, two child rows: the second save runs.
    [Test]
    procedure Again_FieldElementTrigger()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU Again", '<Root><P><Id>2</Id><Name>two</Name><C><CId>10</CId></C><C><CId>11</CId></C></P></Root>');
        AreEqual('2/two/trig/', ParRow(2));
    end;

    // A child trigger changes the parent's record variable: the second save differs by semantics, refused.
    [Test]
    procedure Stale_ChildTriggerSetsParentVariable()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU Stale", '<Root><P><Id>2</Id><Name>two</Name><C><CId>10</CId></C></P></Root>');
        AreEqual('2/two/trig/child', ParRow(2));
    end;

    // The child table's OnInsert changes the stored parent row: refused.
    [Test]
    procedure StaleDb_ChildOnInsertChangesStoredParent()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU StaleDb", '<Root><P><Id>2</Id><Name>two</Name><C><CId>10</CId></C></P></Root>');
        AreEqual('2/two/db/trig', ParRow(2));
    end;

    // OnBeforeModifyRecord of the second save changes a field the document does not carry: refused.
    [Test]
    procedure BeforeModify_ChangesUncarriedField()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU BeforeModify", '<Root><P><Id>2</Id><Name>two</Name><C><CId>10</CId></C></P></Root>');
        AreEqual('2/two/trig/bm', ParRow(2));
    end;

    // The same xmlport without child rows: stored once (main's path).
    [Test]
    procedure BeforeModify_NoChild()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU BeforeModify", '<Root><P><Id>2</Id><Name>two</Name></P></Root>');
        AreEqual('2/two/trig/', ParRow(2));
    end;

    // AutoUpdate with an assign trigger on an existing row: the stored row with the carried fields, the trigger's Stamp
    // dropped (corpus PR 1550, KeyTrig_Existing_NameCarried).
    [Test]
    procedure KeyTrig_AutoUpdate_Existing()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU KeyTrig", '<Root><P><Id>1</Id><Name>new</Name></P></Root>');
        AreEqual('1/new/s0/e0', ParRow(1));
    end;

    // The same with an OnBeforeModifyRecord that changes a field: refused on head.
    [Test]
    procedure UpdBm_ChangesRow()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU UpdBm", '<Root><P><Id>1</Id><Name>new</Name></P></Root>');
        AreEqual('1/new/s0/bm', ParRow(1));
    end;

    // AutoUpdate with only an OnAfterInitRecord that assigns an uncarried field (corpus PR 1550, InitTrig_Existing_NameCarried).
    [Test]
    procedure UpdInit_Existing()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU UpdInit", '<Root><P><Id>1</Id><Name>new</Name></P></Root>');
        AreEqual('1/new/s0/e0', ParRow(1));
    end;

    // AutoUpdate on a temporary table element whose second element meets the row of the first: still refused.
    [Test]
    procedure UpdTemp_SameKeyTwice()
    var
        Imp: XmlPort "NB XAU UpdTemp";
        Src: Record "NB XAU Src";
        InS: InStream;
    begin
        SeedPar();
        Source('<Root><P><Id>1</Id><Name>a</Name></P><P><Id>1</Id><Name>b</Name></P></Root>', Src, InS);
        Imp.SetSource(InS);
        Imp.Import();
        AreEqual('1/b/trig;', Imp.Rows());
    end;

    // AutoUpdate with an assign trigger, a new row: inserted (main's path).
    [Test]
    procedure KeyTrig_AutoUpdate_NewRow()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU KeyTrig", '<Root><P><Id>2</Id><Name>new</Name></P></Root>');
        AreEqual('2/new/trig/', ParRow(2));
    end;

    // AutoUpdate on a parent with a linked child: still refused.
    [Test]
    procedure NestUpd_AutoUpdate_NewRowWithChild()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU NestUpd", '<Root><P><Id>2</Id><Name>new</Name><C><CId>10</CId><Note>a</Note></C></P></Root>');
        AreEqual('2/new/trig/', ParRow(2));
    end;

    // A temporary parent element with an assign trigger and a linked child.
    [Test]
    procedure TempPar_WithChild()
    var
        Imp: XmlPort "NB XAU TempPar";
        Src: Record "NB XAU Src";
        InS: InStream;
    begin
        SeedPar();
        Source('<Root><P><Id>2</Id><Name>two</Name><C><CId>10</CId></C></P></Root>', Src, InS);
        Imp.SetSource(InS);
        Imp.Import();
        AreEqual('2/two/trig;', Imp.Rows());
    end;

    // The KeyTrig shape in a VariableText port, an existing row: still refused.
    [Test]
    procedure UpdVarText_Existing()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU UpdVarText", '1,new');
        AreEqual('1/new/s0/e0', ParRow(1));
    end;

    // The KeyTrig shape with field attributes, an existing row: still refused.
    [Test]
    procedure UpdAttr_Existing()
    begin
        SeedPar();
        RunPort(Xmlport::"NB XAU UpdAttr", '<Root><P Id="1" Name="new"/></Root>');
        AreEqual('1/new/s0/e0', ParRow(1));
    end;
}
