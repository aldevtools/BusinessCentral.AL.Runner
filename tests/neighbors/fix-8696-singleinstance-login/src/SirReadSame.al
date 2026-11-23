// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-singleinstance-property
// Scope: in-scope
// Each test reads one SingleInstance global (left by "SIR Setter A", the codeunit before this one) into a local,
// overwrites the global from this same test codeunit, then observes the local and the global.

codeunit 73471 "SIR Read Same"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Cell_Int()
    var
        SeenInt: Integer;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenInt := S.GetInt();
        S.WInt();
        Observer.Observe('seen', Format(SeenInt));
        Observer.Observe('global', Format(S.GetInt()));
        Observer.Expect('seen', '1');
        Observer.Expect('global', '2');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_Txt()
    var
        SeenTxt: Text;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenTxt := S.GetTxt();
        S.WTxt();
        Observer.Observe('seen', Format(SeenTxt));
        Observer.Observe('global', Format(S.GetTxt()));
        Observer.Expect('seen', 'set');
        Observer.Expect('global', 'second');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_Bool()
    var
        SeenBool: Boolean;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenBool := S.GetBool();
        S.WBool();
        Observer.Observe('seen', Format(SeenBool));
        Observer.Observe('global', Format(S.GetBool()));
        Observer.Expect('seen', 'Yes');
        Observer.Expect('global', 'No');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_Dec()
    var
        SeenDec: Decimal;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenDec := S.GetDec();
        S.WDec();
        Observer.Observe('seen', Format(SeenDec));
        Observer.Observe('global', Format(S.GetDec()));
        Observer.Expect('seen', '1.5');
        Observer.Expect('global', '2.5');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_Code()
    var
        SeenCode: Code[10];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenCode := S.GetCode();
        S.WCode();
        Observer.Observe('seen', Format(SeenCode));
        Observer.Observe('global', Format(S.GetCode()));
        Observer.Expect('seen', 'SET');
        Observer.Expect('global', 'SECOND');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_Txt10()
    var
        SeenTxt10: Text[10];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenTxt10 := S.GetTxt10();
        S.WTxt10();
        Observer.Observe('seen', Format(SeenTxt10));
        Observer.Observe('global', Format(S.GetTxt10()));
        Observer.Expect('seen', 'set');
        Observer.Expect('global', 'second');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_TxtSame()
    var
        SeenTxtSame: Text;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenTxtSame := S.GetTxtSame();
        S.WTxtSame();
        Observer.Observe('seen', SeenTxtSame);
        Observer.Observe('global', S.GetTxtSame());
        Observer.Expect('seen', 'set');
        Observer.Expect('global', 'set');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_TxtOut()
    var
        SeenTxtOut: Text;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        S.GetTxtOut(SeenTxtOut);
        S.WTxtOut();
        Observer.Observe('seen', SeenTxtOut);
        Observer.Observe('global', S.GetTxtOutNow());
        Observer.Expect('seen', 'set');
        Observer.Expect('global', 'second');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_IntEval()
    var
        SeenIntEval: Integer;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenIntEval := S.GetIntEval();
        S.WIntEval();
        Observer.Observe('seen', Format(SeenIntEval));
        Observer.Observe('global', Format(S.GetIntEval()));
        Observer.Expect('seen', '1');
        Observer.Expect('global', '7');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_TxtClear()
    var
        SeenTxtClear: Text;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenTxtClear := S.GetTxtClear();
        S.WTxtClear();
        Observer.Observe('seen', SeenTxtClear);
        Observer.Observe('global', S.GetTxtClear());
        Observer.Expect('seen', 'set');
        Observer.Expect('global', '');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_Guard()
    var
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        if S.GetGuard() <> 'refilled' then
            S.WGuard();
        Observer.Observe('global', S.GetGuard());
        Observer.Expect('global', 'refilled');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_GuardSeen()
    var
        SeenGuardSeen: Text;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenGuardSeen := S.GetGuardSeen();
        if SeenGuardSeen <> 'refilled' then
            S.WGuardSeen();
        Observer.Observe('seen', SeenGuardSeen);
        Observer.Observe('global', S.GetGuardSeen());
        Observer.Expect('seen', 'set');
        Observer.Expect('global', 'refilled');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_RecField()
    var
        R: Record "SIR Row" temporary;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        S.GetRecField(R);
        S.WRecField();
        Observer.Observe('local', R.Txt);
        Observer.Observe('global', S.TxtRecField());
        Observer.Expect('local', 'set');
        Observer.Expect('global', 'second');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_RecAssign()
    var
        R: Record "SIR Row" temporary;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        S.GetRecAssign(R);
        S.WRecAssign();
        Observer.Observe('local', R.Txt);
        Observer.Observe('global', S.TxtRecAssign());
        Observer.Expect('local', 'set');
        Observer.Expect('global', 'second');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_RecShared()
    var
        R: Record "SIR Row" temporary;
        Dummy: Boolean;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        S.GetRecShared(R);
        S.WRecShared();
        Observer.Observe('local buffer', R.Txt);
        Observer.Observe('local after find', Format(R.Get('K')) + '/' + R.Txt);
        Observer.Observe('global', S.TxtRecShared());
        Observer.Expect('local buffer', 'set');
        Observer.Expect('local after find', 'Yes/second');
        Observer.Expect('global', 'second');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_RecDelete()
    var
        R: Record "SIR Row" temporary;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        S.GetRecDelete(R);
        S.WRecDelete();
        Observer.Observe('local count', Format(R.Count()));
        Observer.Observe('global count', Format(S.CountRecDelete()));
        Observer.Expect('local count', '0');
        Observer.Expect('global count', '0');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_RecClear()
    var
        R: Record "SIR Row" temporary;
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        S.GetRecClear(R);
        S.WRecClear();
        Observer.Observe('local', R.Txt);
        Observer.Observe('global', S.TxtRecClear());
        Observer.Expect('local', 'set');
        Observer.Expect('global', '');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_ListNew()
    var
        SeenListNew: List of [Text];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenListNew := S.GetListNew();
        S.WListNew();
        Observer.Observe('seen', Join(SeenListNew));
        Observer.Observe('global', Join(S.GetListNew()));
        Observer.Expect('seen', '2:a,b,');
        Observer.Expect('global', '1:z,');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_ListAdd()
    var
        SeenListAdd: List of [Text];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenListAdd := S.GetListAdd();
        S.WListAdd();
        Observer.Observe('seen', Join(SeenListAdd));
        Observer.Observe('global', Join(S.GetListAdd()));
        Observer.Expect('seen', '3:a,b,c,');
        Observer.Expect('global', '3:a,b,c,');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_ListRemove()
    var
        SeenListRemove: List of [Text];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenListRemove := S.GetListRemove();
        S.WListRemove();
        Observer.Observe('seen', Join(SeenListRemove));
        Observer.Observe('global', Join(S.GetListRemove()));
        Observer.Expect('seen', '0:');
        Observer.Expect('global', '0:');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_ListClear()
    var
        SeenListClear: List of [Text];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenListClear := S.GetListClear();
        S.WListClear();
        Observer.Observe('seen', Join(SeenListClear));
        Observer.Observe('global', Join(S.GetListClear()));
        Observer.Expect('seen', '2:a,b,');
        Observer.Expect('global', '0:');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_DictNew()
    var
        SeenDictNew: Dictionary of [Text, Text];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenDictNew := S.GetDictNew();
        S.WDictNew();
        Observer.Observe('seen', DictText(SeenDictNew));
        Observer.Observe('global', DictText(S.GetDictNew()));
        Observer.Expect('seen', '1:k=v1,');
        Observer.Expect('global', '1:k=v2,');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_DictSet()
    var
        SeenDictSet: Dictionary of [Text, Text];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenDictSet := S.GetDictSet();
        S.WDictSet();
        Observer.Observe('seen', DictText(SeenDictSet));
        Observer.Observe('global', DictText(S.GetDictSet()));
        Observer.Expect('seen', '2:k=v2,n=x,');
        Observer.Expect('global', '2:k=v2,n=x,');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_DictRemove()
    var
        SeenDictRemove: Dictionary of [Text, Text];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenDictRemove := S.GetDictRemove();
        S.WDictRemove();
        Observer.Observe('seen', DictText(SeenDictRemove));
        Observer.Observe('global', DictText(S.GetDictRemove()));
        Observer.Expect('seen', '0:');
        Observer.Expect('global', '0:');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_DictClear()
    var
        SeenDictClear: Dictionary of [Text, Text];
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        SeenDictClear := S.GetDictClear();
        S.WDictClear();
        Observer.Observe('seen', DictText(SeenDictClear));
        Observer.Observe('global', DictText(S.GetDictClear()));
        Observer.Expect('seen', '1:k=v1,');
        Observer.Expect('global', '0:');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_CuPut()
    var
        HCuPut: Codeunit "SIR Holder";
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        HCuPut := S.GetCuPut();
        S.WCuPut();
        Observer.Observe('seen', Format(HCuPut.Value()));
        Observer.Observe('global', Format(S.NowCuPut()));
        Observer.Expect('seen', '2');
        Observer.Expect('global', '2');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_CuClear()
    var
        HCuClear: Codeunit "SIR Holder";
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        HCuClear := S.GetCuClear();
        S.WCuClear();
        Observer.Observe('seen', Format(HCuClear.Value()));
        Observer.Observe('global', Format(S.NowCuClear()));
        Observer.Expect('seen', '1');
        Observer.Expect('global', '3');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_IfNew()
    var
        HIfNew: Interface "SIR IVal";
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        HIfNew := S.GetIfNew();
        S.WIfNew();
        Observer.Observe('seen', HIfNew.Get());
        Observer.Observe('global', S.NowIfNew());
        Observer.Expect('seen', 'set');
        Observer.Expect('global', 'second');
        Observer.Verify();
    end;

    [Test]
    procedure Cell_IfPut()
    var
        HIfPut: Interface "SIR IVal";
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        HIfPut := S.GetIfPut();
        S.WIfPut();
        Observer.Observe('seen', HIfPut.Get());
        Observer.Observe('global', S.NowIfPut());
        Observer.Expect('seen', 'second');
        Observer.Expect('global', 'second');
        Observer.Verify();
    end;

    local procedure Join(Items: List of [Text]) Result: Text
    var
        Item: Text;
    begin
        foreach Item in Items do
            Result += Item + ',';
        Result := Format(Items.Count()) + ':' + Result;
    end;

    local procedure DictText(Items: Dictionary of [Text, Text]) Result: Text
    var
        Item: Text;
    begin
        foreach Item in Items.Keys() do
            Result += Item + '=' + Items.Get(Item) + ',';
        Result := Format(Items.Count()) + ':' + Result;
    end;
}
