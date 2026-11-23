// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-singleinstance-property
// Scope: in-scope
// Fixtures: the state of the SingleInstance probe for al-runner issue 8696. "SIR State" holds one global per
// cell (a value kind and a way to read and a way to overwrite it); the Setter test codeunits put every global into its
// start value, the reader codeunits read a global into a local, overwrite it, then observe the local and the global.
// BC result (27.5 and 28.5 the same; 29 measured by corpus CI): a local copy of a scalar, Text, Code, a Record row or a List or
// Dictionary assigned from a getter keeps the value read; an in-place change (List.Add/Remove, Dictionary.Set/Remove, a codeunit
// Put) is seen through the local too; a guard refill leaves the refilled value. A value kept across tests or codeunits stays.
//
// singleinstance-shared-on-purpose: the probe measures what one test codeunit leaves for the next one in the same run.

table 73464 "SIR Row"
{
    DataClassification = SystemMetadata;
    TableType = Normal;

    fields
    {
        field(1; "Key"; Code[10]) { }
        field(2; Txt; Text[50]) { }
    }

    keys
    {
        key(PK; "Key") { Clustered = true; }
    }
}

interface "SIR IVal"
{
    procedure Get(): Text;
    procedure Put(NewValue: Text);
}

codeunit 73462 "SIR Impl" implements "SIR IVal"
{
    procedure Get(): Text
    begin
        exit(Held);
    end;

    procedure Put(NewValue: Text)
    begin
        Held := NewValue;
    end;

    var
        Held: Text;
}

codeunit 73463 "SIR Holder"
{
    procedure Put(NewValue: Integer)
    begin
        Held := NewValue;
    end;

    procedure Value(): Integer
    begin
        exit(Held);
    end;

    var
        Held: Integer;
}

codeunit 73465 "SIR State"
{
    SingleInstance = true;

    var
        VInt: Integer;
        VTxt: Text;
        VBool: Boolean;
        VDec: Decimal;
        VCode: Code[10];
        VTxt10: Text[10];
        VTxtSame: Text;
        VTxtOut: Text;
        VIntEval: Integer;
        VTxtClear: Text;
        VGuard: Text;
        VGuardSeen: Text;
        VRecField: Record "SIR Row" temporary;
        VRecAssign: Record "SIR Row" temporary;
        VRecShared: Record "SIR Row" temporary;
        VRecDelete: Record "SIR Row" temporary;
        VRecClear: Record "SIR Row" temporary;
        VListNew: List of [Text];
        VListAdd: List of [Text];
        VListRemove: List of [Text];
        VListClear: List of [Text];
        VDictNew: Dictionary of [Text, Text];
        VDictSet: Dictionary of [Text, Text];
        VDictRemove: Dictionary of [Text, Text];
        VDictClear: Dictionary of [Text, Text];
        VCuPut: Codeunit "SIR Holder";
        VCuClear: Codeunit "SIR Holder";
        VIfNew: Interface "SIR IVal";
        VIfPut: Interface "SIR IVal";

    procedure InitAll()
    var
        A: Codeunit "SIR Impl";
    begin
        VInt := 1;
        VTxt := 'set';
        VBool := true;
        VDec := 1.5;
        VCode := 'SET';
        VTxt10 := 'set';
        VTxtSame := 'set';
        VTxtOut := 'set';
        VIntEval := 1;
        VTxtClear := 'set';
        VGuard := 'set';
        VGuardSeen := 'set';
        VRecField.DeleteAll();
        VRecField.Init();
        VRecField."Key" := 'K';
        VRecField.Txt := 'set';
        VRecAssign.DeleteAll();
        VRecAssign.Init();
        VRecAssign."Key" := 'K';
        VRecAssign.Txt := 'set';
        VRecShared.DeleteAll();
        VRecShared.Init();
        VRecShared."Key" := 'K';
        VRecShared.Txt := 'set';
        VRecShared.Insert();
        VRecDelete.DeleteAll();
        VRecDelete.Init();
        VRecDelete."Key" := 'K';
        VRecDelete.Txt := 'set';
        VRecDelete.Insert();
        VRecClear.DeleteAll();
        VRecClear.Init();
        VRecClear."Key" := 'K';
        VRecClear.Txt := 'set';
        Clear(VListNew);
        VListNew.Add('a');
        VListNew.Add('b');
        Clear(VListAdd);
        VListAdd.Add('a');
        VListAdd.Add('b');
        Clear(VListRemove);
        VListRemove.Add('a');
        VListRemove.Add('b');
        Clear(VListClear);
        VListClear.Add('a');
        VListClear.Add('b');
        Clear(VDictNew);
        VDictNew.Add('k', 'v1');
        Clear(VDictSet);
        VDictSet.Add('k', 'v1');
        Clear(VDictRemove);
        VDictRemove.Add('k', 'v1');
        Clear(VDictClear);
        VDictClear.Add('k', 'v1');
        VCuPut.Put(1);
        VCuClear.Put(1);
        A.Put('set');
        VIfNew := A;
        A.Put('set');
        VIfPut := A;
    end;

    procedure GetInt(): Integer
    begin
        exit(VInt);
    end;
    procedure WInt()
    begin
        VInt := 2;
    end;
    procedure GetTxt(): Text
    begin
        exit(VTxt);
    end;
    procedure WTxt()
    begin
        VTxt := 'second';
    end;
    procedure GetBool(): Boolean
    begin
        exit(VBool);
    end;
    procedure WBool()
    begin
        VBool := false;
    end;
    procedure GetDec(): Decimal
    begin
        exit(VDec);
    end;
    procedure WDec()
    begin
        VDec := 2.5;
    end;
    procedure GetCode(): Code[10]
    begin
        exit(VCode);
    end;
    procedure WCode()
    begin
        VCode := 'SECOND';
    end;
    procedure GetTxt10(): Text[10]
    begin
        exit(VTxt10);
    end;
    procedure WTxt10()
    begin
        VTxt10 := 'second';
    end;
    procedure GetTxtSame(): Text
    begin
        exit(VTxtSame);
    end;
    procedure WTxtSame()
    begin
        VTxtSame := 'set';
    end;
    procedure GetTxtOut(var T: Text)
    begin
        T := VTxtOut;
    end;
    procedure GetTxtOutNow(): Text
    begin
        exit(VTxtOut);
    end;
    procedure WTxtOut()
    begin
        VTxtOut := 'second';
    end;
    procedure GetIntEval(): Integer
    begin
        exit(VIntEval);
    end;
    procedure WIntEval()
    begin
        Evaluate(VIntEval, '7');
    end;
    procedure GetTxtClear(): Text
    begin
        exit(VTxtClear);
    end;
    procedure WTxtClear()
    begin
        Clear(VTxtClear);
    end;
    procedure GetGuard(): Text
    begin
        exit(VGuard);
    end;
    procedure WGuard()
    begin
        VGuard := 'refilled';
    end;
    procedure GetGuardSeen(): Text
    begin
        exit(VGuardSeen);
    end;
    procedure WGuardSeen()
    begin
        VGuardSeen := 'refilled';
    end;
    procedure GetRecField(var R: Record "SIR Row" temporary)
    begin
        R := VRecField;
    end;
    procedure TxtRecField(): Text
    begin
        exit(VRecField.Txt);
    end;
    procedure CountRecField(): Integer
    begin
        exit(VRecField.Count());
    end;
    procedure WRecField()
    begin
        VRecField.Txt := 'second';
    end;
    procedure GetRecAssign(var R: Record "SIR Row" temporary)
    begin
        R := VRecAssign;
    end;
    procedure TxtRecAssign(): Text
    begin
        exit(VRecAssign.Txt);
    end;
    procedure CountRecAssign(): Integer
    begin
        exit(VRecAssign.Count());
    end;
    procedure WRecAssign()
    var
        Tmp: Record "SIR Row" temporary;
    begin
        Tmp.Init();
        Tmp."Key" := 'K2';
        Tmp.Txt := 'second';
        VRecAssign := Tmp;
    end;
    procedure GetRecShared(var R: Record "SIR Row" temporary)
    begin
        R.Copy(VRecShared, true);
    end;
    procedure TxtRecShared(): Text
    begin
        exit(VRecShared.Txt);
    end;
    procedure CountRecShared(): Integer
    begin
        exit(VRecShared.Count());
    end;
    procedure WRecShared()
    begin
        VRecShared.Txt := 'second';
        VRecShared.Modify();
    end;
    procedure GetRecDelete(var R: Record "SIR Row" temporary)
    begin
        R.Copy(VRecDelete, true);
    end;
    procedure TxtRecDelete(): Text
    begin
        exit(VRecDelete.Txt);
    end;
    procedure CountRecDelete(): Integer
    begin
        exit(VRecDelete.Count());
    end;
    procedure WRecDelete()
    begin
        VRecDelete.DeleteAll();
    end;
    procedure GetRecClear(var R: Record "SIR Row" temporary)
    begin
        R := VRecClear;
    end;
    procedure TxtRecClear(): Text
    begin
        exit(VRecClear.Txt);
    end;
    procedure CountRecClear(): Integer
    begin
        exit(VRecClear.Count());
    end;
    procedure WRecClear()
    begin
        Clear(VRecClear);
    end;
    procedure GetListNew(): List of [Text]
    begin
        exit(VListNew);
    end;
    procedure WListNew()
    var
        Fresh: List of [Text];
    begin
        Fresh.Add('z');
        VListNew := Fresh;
    end;
    procedure GetListAdd(): List of [Text]
    begin
        exit(VListAdd);
    end;
    procedure WListAdd()
    begin
        VListAdd.Add('c');
    end;
    procedure GetListRemove(): List of [Text]
    begin
        exit(VListRemove);
    end;
    procedure WListRemove()
    begin
        VListRemove.RemoveRange(1, VListRemove.Count());
    end;
    procedure GetListClear(): List of [Text]
    begin
        exit(VListClear);
    end;
    procedure WListClear()
    begin
        Clear(VListClear);
    end;
    procedure GetDictNew(): Dictionary of [Text, Text]
    begin
        exit(VDictNew);
    end;
    procedure WDictNew()
    var
        Fresh: Dictionary of [Text, Text];
    begin
        Fresh.Add('k', 'v2');
        VDictNew := Fresh;
    end;
    procedure GetDictSet(): Dictionary of [Text, Text]
    begin
        exit(VDictSet);
    end;
    procedure WDictSet()
    begin
        VDictSet.Set('k', 'v2');
        VDictSet.Add('n', 'x');
    end;
    procedure GetDictRemove(): Dictionary of [Text, Text]
    begin
        exit(VDictRemove);
    end;
    procedure WDictRemove()
    begin
        VDictRemove.Remove('k');
    end;
    procedure GetDictClear(): Dictionary of [Text, Text]
    begin
        exit(VDictClear);
    end;
    procedure WDictClear()
    begin
        Clear(VDictClear);
    end;
    procedure GetCuPut(): Codeunit "SIR Holder"
    begin
        exit(VCuPut);
    end;
    procedure NowCuPut(): Integer
    begin
        exit(VCuPut.Value());
    end;
    procedure WCuPut()
    begin
        VCuPut.Put(2);
    end;
    procedure GetCuClear(): Codeunit "SIR Holder"
    begin
        exit(VCuClear);
    end;
    procedure NowCuClear(): Integer
    begin
        exit(VCuClear.Value());
    end;
    procedure WCuClear()
    begin
        Clear(VCuClear);
        VCuClear.Put(3);
    end;
    procedure GetIfNew(): Interface "SIR IVal"
    begin
        exit(VIfNew);
    end;
    procedure NowIfNew(): Text
    begin
        exit(VIfNew.Get());
    end;
    procedure WIfNew()
    var
        B: Codeunit "SIR Impl";
    begin
        B.Put('second');
        VIfNew := B;
    end;
    procedure GetIfPut(): Interface "SIR IVal"
    begin
        exit(VIfPut);
    end;
    procedure NowIfPut(): Text
    begin
        exit(VIfPut.Get());
    end;
    procedure WIfPut()
    begin
        VIfPut.Put('second');
    end;
}

// Writes the globals of "SIR State" from a codeunit other than the reader (not SingleInstance).
codeunit 73466 "SIR Writer"
{
    procedure WInt()
    var
        S: Codeunit "SIR State";
    begin
        S.WInt();
    end;
    procedure WTxt()
    var
        S: Codeunit "SIR State";
    begin
        S.WTxt();
    end;
    procedure WBool()
    var
        S: Codeunit "SIR State";
    begin
        S.WBool();
    end;
    procedure WDec()
    var
        S: Codeunit "SIR State";
    begin
        S.WDec();
    end;
    procedure WCode()
    var
        S: Codeunit "SIR State";
    begin
        S.WCode();
    end;
    procedure WTxt10()
    var
        S: Codeunit "SIR State";
    begin
        S.WTxt10();
    end;
    procedure WTxtSame()
    var
        S: Codeunit "SIR State";
    begin
        S.WTxtSame();
    end;
    procedure WTxtOut()
    var
        S: Codeunit "SIR State";
    begin
        S.WTxtOut();
    end;
    procedure WIntEval()
    var
        S: Codeunit "SIR State";
    begin
        S.WIntEval();
    end;
    procedure WTxtClear()
    var
        S: Codeunit "SIR State";
    begin
        S.WTxtClear();
    end;
    procedure WGuard()
    var
        S: Codeunit "SIR State";
    begin
        S.WGuard();
    end;
    procedure WGuardSeen()
    var
        S: Codeunit "SIR State";
    begin
        S.WGuardSeen();
    end;
    procedure WRecField()
    var
        S: Codeunit "SIR State";
    begin
        S.WRecField();
    end;
    procedure WRecAssign()
    var
        S: Codeunit "SIR State";
    begin
        S.WRecAssign();
    end;
    procedure WRecShared()
    var
        S: Codeunit "SIR State";
    begin
        S.WRecShared();
    end;
    procedure WRecDelete()
    var
        S: Codeunit "SIR State";
    begin
        S.WRecDelete();
    end;
    procedure WRecClear()
    var
        S: Codeunit "SIR State";
    begin
        S.WRecClear();
    end;
    procedure WListNew()
    var
        S: Codeunit "SIR State";
    begin
        S.WListNew();
    end;
    procedure WListAdd()
    var
        S: Codeunit "SIR State";
    begin
        S.WListAdd();
    end;
    procedure WListRemove()
    var
        S: Codeunit "SIR State";
    begin
        S.WListRemove();
    end;
    procedure WListClear()
    var
        S: Codeunit "SIR State";
    begin
        S.WListClear();
    end;
    procedure WDictNew()
    var
        S: Codeunit "SIR State";
    begin
        S.WDictNew();
    end;
    procedure WDictSet()
    var
        S: Codeunit "SIR State";
    begin
        S.WDictSet();
    end;
    procedure WDictRemove()
    var
        S: Codeunit "SIR State";
    begin
        S.WDictRemove();
    end;
    procedure WDictClear()
    var
        S: Codeunit "SIR State";
    begin
        S.WDictClear();
    end;
    procedure WCuPut()
    var
        S: Codeunit "SIR State";
    begin
        S.WCuPut();
    end;
    procedure WCuClear()
    var
        S: Codeunit "SIR State";
    begin
        S.WCuClear();
    end;
    procedure WIfNew()
    var
        S: Codeunit "SIR State";
    begin
        S.WIfNew();
    end;
    procedure WIfPut()
    var
        S: Codeunit "SIR State";
    begin
        S.WIfPut();
    end;
}

// Keeps what a reader copied, across test codeunits.
codeunit 73467 "SIR Keep"
{
    SingleInstance = true;

    procedure KeepInt(NewValue: Integer)
    begin
        KeptInt := NewValue;
    end;

    procedure KeepText(NewValue: Text)
    begin
        KeptText := NewValue;
    end;

    procedure KeepList(NewValue: List of [Text])
    begin
        KeptList := NewValue;
    end;

    procedure KeptIntNow(): Integer
    begin
        exit(KeptInt);
    end;

    procedure KeptTextNow(): Text
    begin
        exit(KeptText);
    end;

    procedure KeptListCount(): Integer
    begin
        exit(KeptList.Count());
    end;

    var
        KeptInt: Integer;
        KeptText: Text;
        KeptList: List of [Text];
}
