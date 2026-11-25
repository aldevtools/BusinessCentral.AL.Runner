// Neighbors of al-runner issue 8912, review round 3 of the ClearAll change: the price store and the register shared by the
// holder slots corpus PR 1564 measures in ClearSharedCodeunitHolderKinds.al and ClearSharedCodeunitHolderSlots.al (a local
// interface of the clearing routine, a by-value parameter, a Variant, a List of interface, a var interface parameter).
// Stand-in objects, invented code.
enum 50630 "R3P Kind"
{
    value(0; None) { }
    value(1; Alpha) { }
    value(2; Beta) { }
}

interface "R3P Face"
{
    procedure SetInt(V: Integer);
    procedure State(): Text;
}

// The price store shape: a temporary record, an Enum, an Integer and a Label with a Comment, in this order.
codeunit 50631 "R3P Store" implements "R3P Face"
{
    var
        T1: Record "R3 Row" temporary;
        Kind: Enum "R3P Kind";
        Int: Integer;
        LblC: Label 'store label', Comment = 'a comment';

    procedure Mark()
    begin
        Kind := Kind::Beta;
        Int := 7;
    end;

    procedure WriteT1(V: Integer)
    begin
        T1.No := V;
        T1.Insert();
    end;

    procedure SetInt(V: Integer)
    begin
        Int := V;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('t1=%1 kind=%2 int=%3 lc=%4', T1.Count(), Kind, Int, LblC));
    end;
}

codeunit 50632 "R3P Owner"
{
    var
        S: Codeunit "R3P Store";
        Cnt: Integer;
        LineNo: Integer;

    procedure SetLine()
    begin
        ClearAll();
        LineNo := 1;
    end;

    procedure Prepare()
    begin
        S.Mark();
        S.WriteT1(1);
        S.WriteT1(2);
        Cnt := 5;
    end;

    procedure State(): Text
    begin
        exit(S.State() + ' cnt=' + Format(Cnt) + ' line=' + Format(LineNo));
    end;

    procedure GetStore(var X: Codeunit "R3P Store")
    begin
        X := S;
    end;
}

// The register shape with an interface, owned by a plain codeunit that takes it through a setter.
interface "R3 Reg Face"
{
    procedure WriteRow(V: Integer);
    procedure State(): Text;
}

codeunit 50634 "R3 Reg FStore" implements "R3 Reg Face"
{
    TableNo = "R3 Row";

    var
        Rows: Record "R3 Row" temporary;
        Rec: Record "R3 Row";
        Dec: Decimal;
        Flag: Boolean;
        Cd: Code[20];
        Nested: Codeunit "R3 Nested";

    trigger OnRun()
    begin
    end;

    procedure Mark()
    begin
        Dec := 1.5;
        Flag := true;
        Cd := 'MARK';
        Nested.Set(4);
    end;

    procedure WriteRow(V: Integer)
    begin
        Rows.Init();
        Rows.No := V;
        Rows.Insert();
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1 dec=%2 flag=%3 code=%4 nested=%5',
            Rows.Count(), Format(Dec, 0, 9), Flag, Cd, Nested.Get()));
    end;
}

codeunit 50635 "R3 Reg Plain Owner"
{
    var
        Reg: Codeunit "R3 Reg FStore";

    procedure SetRegister(var NewReg: Codeunit "R3 Reg FStore")
    begin
        Reg := NewReg;
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;

    procedure StateReg(): Text
    begin
        exit(Reg.State());
    end;
}

codeunit 50633 "R3P Holder"
{
    procedure LoadIface(var O: Codeunit "R3P Owner"; var I: Interface "R3P Face")
    var
        Y: Codeunit "R3P Store";
    begin
        O.GetStore(Y);
        O.Prepare();
        I := Y;
    end;

    procedure LoadVariant(var O: Codeunit "R3P Owner"; var V: Variant)
    var
        Y: Codeunit "R3P Store";
    begin
        O.GetStore(Y);
        O.Prepare();
        V := Y;
    end;

    procedure LoadList(var O: Codeunit "R3P Owner"; var L: List of [Interface "R3P Face"])
    var
        Y: Codeunit "R3P Store";
        I: Interface "R3P Face";
    begin
        O.GetStore(Y);
        O.Prepare();
        I := Y;
        L.Add(I);
    end;

    // ClearAll runs while a local interface of this routine is the store's other holder.
    procedure ClearWithLocalIface(var O: Codeunit "R3P Owner"): Text
    var
        I: Interface "R3P Face";
    begin
        LoadIface(O, I);
        O.SetLine();
        I.SetInt(10);
        exit(I.State() + ' | ' + O.State());
    end;

    // ClearAll runs while this routine's by-value parameter shares the store.
    procedure ClearWithByValue(var O: Codeunit "R3P Owner"; Y: Codeunit "R3P Store"): Text
    begin
        O.SetLine();
        Y.SetInt(10);
        exit(Y.State() + ' | ' + O.State());
    end;

    // ClearAll runs while this routine's var interface parameter shares the store.
    procedure ClearWithIfaceVar(var O: Codeunit "R3P Owner"; var I: Interface "R3P Face"): Text
    begin
        O.SetLine();
        I.SetInt(10);
        exit(I.State() + ' | ' + O.State());
    end;

    // ClearAll runs while a local interface of this routine is the register's other holder.
    procedure ClearRegWithLocalIface(var O: Codeunit "R3 Reg Plain Owner"): Text
    var
        I: Interface "R3 Reg Face";
    begin
        LoadReg(O, I);
        O.ClearAllGlobals();
        I.WriteRow(11);
        exit(I.State() + ' | ' + O.StateReg());
    end;

    local procedure LoadReg(var O: Codeunit "R3 Reg Plain Owner"; var I: Interface "R3 Reg Face")
    var
        Y: Codeunit "R3 Reg FStore";
    begin
        Y.Mark();
        Y.WriteRow(1);
        Y.WriteRow(2);
        O.SetRegister(Y);
        I := Y;
    end;
}

codeunit 50636 "R3 Holder Tests"
{
    Subtype = Test;

    // Measured: ClearSharedCodeunitHolderSlots.al IfaceCallerLocal (Price_IfaceCallerLocal_Rows :4447). Runs flagged.
    [Test]
    procedure PriceStoreSharedByLocalInterface()
    var
        O: Codeunit "R3P Owner";
        K: Codeunit "R3P Holder";
    begin
        Check(K.ClearWithLocalIface(O),
            't1=2 kind=None int=10 lc=store label | t1=2 kind=None int=10 lc=store label cnt=0 line=1');
    end;

    // Measured: ClearSharedCodeunitHolderSlots.al ByValPassed (Price_ByValPassed_Rows :4235). Runs flagged.
    [Test]
    procedure PriceStoreSharedByValueParam()
    var
        O: Codeunit "R3P Owner";
        Y: Codeunit "R3P Store";
        K: Codeunit "R3P Holder";
    begin
        O.GetStore(Y);
        O.Prepare();
        Check(K.ClearWithByValue(O, Y),
            't1=2 kind=None int=10 lc=store label | t1=2 kind=None int=10 lc=store label cnt=0 line=1');
        Check(Y.State(), 't1=2 kind=None int=10 lc=store label');
    end;

    // Measured: ClearSharedCodeunitHolderSlots.al VariantLocal (Price_VariantLocal_Rows :4338). Runs flagged.
    [Test]
    procedure PriceStoreSharedByVariant()
    var
        O: Codeunit "R3P Owner";
        K: Codeunit "R3P Holder";
        V: Variant;
        Y: Codeunit "R3P Store";
    begin
        K.LoadVariant(O, V);
        O.SetLine();
        Y := V;
        Check(Y.State(), 't1=2 kind=None int=0 lc=store label');
        Y.SetInt(10);
        Check(O.State(), 't1=2 kind=None int=10 lc=store label cnt=0 line=1');
    end;

    // Measured: ClearSharedCodeunitHolderSlots.al ListLocal (Price_ListLocal_Rows :4492). Runs flagged.
    [Test]
    procedure PriceStoreSharedByListOfInterface()
    var
        O: Codeunit "R3P Owner";
        K: Codeunit "R3P Holder";
        L: List of [Interface "R3P Face"];
        I: Interface "R3P Face";
    begin
        K.LoadList(O, L);
        O.SetLine();
        L.Get(1, I);
        Check(I.State(), 't1=2 kind=None int=0 lc=store label');
        I.SetInt(10);
        Check(O.State(), 't1=2 kind=None int=10 lc=store label cnt=0 line=1');
    end;

    // Measured: ClearSharedCodeunitHolderSlots.al IfaceVarParam (Price_IfaceVarParam_Rows :4618). Runs flagged.
    [Test]
    procedure PriceStoreSharedByInterfaceVarParam()
    var
        O: Codeunit "R3P Owner";
        K: Codeunit "R3P Holder";
        I: Interface "R3P Face";
    begin
        K.LoadIface(O, I);
        Check(K.ClearWithIfaceVar(O, I),
            't1=2 kind=None int=10 lc=store label | t1=2 kind=None int=10 lc=store label cnt=0 line=1');
    end;

    // Measured: ClearSharedCodeunitHolderSlots.al Price_IfaceLocal_Never :4905 (no row when ClearAll runs). Runs flagged.
    [Test]
    procedure PriceStoreSharedByInterfaceNoRow()
    var
        O: Codeunit "R3P Owner";
        K: Codeunit "R3P Holder";
        I: Interface "R3P Face";
        Y: Codeunit "R3P Store";
    begin
        O.GetStore(Y);
        Y.Mark();
        I := Y;
        Clear(Y);
        O.SetLine();
        Check(I.State(), 't1=0 kind=None int=0 lc=store label');
        I.SetInt(10);
        Check(O.State(), 't1=0 kind=None int=10 lc=store label cnt=0 line=1');
    end;

    // Measured: ClearSharedCodeunitHolderSlots.al RegPlain_IfaceCallerLocal_Rows :6084. Runs flagged.
    [Test]
    procedure RegisterSharedByLocalInterface()
    var
        O: Codeunit "R3 Reg Plain Owner";
        K: Codeunit "R3P Holder";
    begin
        Check(K.ClearRegWithLocalIface(O),
            'rows=3 dec=0 flag=No code= nested=0 | rows=3 dec=0 flag=No code= nested=0');
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
