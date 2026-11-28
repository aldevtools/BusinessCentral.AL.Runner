// Neighbors of the ClearAll cell of a self-holding store (assumption clear-all-self-holding-tree, corpus PR 1672):
// one test per fact the cell reads, each changed once. Expected values are what the in-place reset would give.

interface "CSN Face"
{
    procedure GetValue(): Integer;
}

// The probe's store with one more global kind (Text) in the cycle.
codeunit 88120 "CSN Text Store"
{
    var
        Value: Integer;
        Name: Text;
        Myself: Codeunit "CSN Text Store";

    procedure Init(ThisIn: Codeunit "CSN Text Store")
    begin
        Myself := ThisIn;
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
        Name := 'x';
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;
}

// The probe's store that implements an interface.
codeunit 88121 "CSN Face Store" implements "CSN Face"
{
    var
        Value: Integer;
        Myself: Codeunit "CSN Face Store";

    procedure Init(ThisIn: Codeunit "CSN Face Store")
    begin
        Myself := ThisIn;
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;
}

// A store whose nested codeunit has a TableNo and holds the store back.
codeunit 88122 "CSN TNo Store"
{
    var
        Value: Integer;
        Line: Codeunit "CSN TNo Line";

    procedure Init(ThisIn: Codeunit "CSN TNo Store")
    begin
        Line.Init(ThisIn);
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;
}

codeunit 88123 "CSN TNo Line"
{
    TableNo = "Probe Observer Entry";

    var
        Store: Codeunit "CSN TNo Store";

    procedure Init(StoreIn: Codeunit "CSN TNo Store")
    begin
        Store := StoreIn;
    end;
}

// A chain five levels deep below a self-holding store.
codeunit 88124 "CSN Deep5 Store"
{
    var
        Value: Integer;
        Myself: Codeunit "CSN Deep5 Store";
        C1: Codeunit "CSN D5 C1";

    procedure Init(ThisIn: Codeunit "CSN Deep5 Store")
    begin
        Myself := ThisIn;
        C1.Touch();
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;
}

codeunit 88125 "CSN D5 C1"
{
    var
        Next: Codeunit "CSN D5 C2";

    procedure Touch()
    begin
        Next.Touch();
    end;
}

codeunit 88126 "CSN D5 C2"
{
    var
        Next: Codeunit "CSN D5 C3";

    procedure Touch()
    begin
        Next.Touch();
    end;
}

codeunit 88127 "CSN D5 C3"
{
    var
        Next: Codeunit "CSN D5 C4";

    procedure Touch()
    begin
        Next.Touch();
    end;
}

codeunit 88128 "CSN D5 C4"
{
    var
        Next: Codeunit "CSN D5 C5";

    procedure Touch()
    begin
        Next.Touch();
    end;
}

codeunit 88129 "CSN D5 C5"
{
    var
        Val: Integer;

    procedure Touch()
    begin
        Val := 1;
    end;
}

// A store whose L2 holds L1 back: a cycle to a node other than the store.
codeunit 88130 "CSN Mid Store"
{
    var
        Value: Integer;
        Myself: Codeunit "CSN Mid Store";
        L1: Codeunit "CSN Mid L1";

    procedure Init(ThisIn: Codeunit "CSN Mid Store")
    begin
        Myself := ThisIn;
        L1.Init(L1);
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;
}

codeunit 88131 "CSN Mid L1"
{
    var
        L2: Codeunit "CSN Mid L2";

    procedure Init(ThisIn: Codeunit "CSN Mid L1")
    begin
        L2.Init(ThisIn);
    end;
}

codeunit 88132 "CSN Mid L2"
{
    var
        Back: Codeunit "CSN Mid L1";

    procedure Init(L1In: Codeunit "CSN Mid L1")
    begin
        Back := L1In;
    end;
}

// A store with a SingleInstance codeunit two levels down.
codeunit 88133 "CSN SI2 Store"
{
    var
        Value: Integer;
        Myself: Codeunit "CSN SI2 Store";
        Mid: Codeunit "CSN SI2 Mid";

    procedure Init(ThisIn: Codeunit "CSN SI2 Store")
    begin
        Myself := ThisIn;
        Mid.Touch();
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;
}

codeunit 88134 "CSN SI2 Mid"
{
    var
        Setup: Codeunit "CSN SI2 Setup";
        Val: Integer;

    procedure Touch()
    begin
        Val := 1;
        Setup.Touch();
    end;
}

codeunit 88135 "CSN SI2 Setup"
{
    SingleInstance = true;

    var
        Val: Integer;

    procedure Touch()
    begin
        Val := 6;
    end;
}

// A plain owner (no Subtype) that is not the store.
codeunit 88136 "CSN Plain Owner"
{
    var
        S: Codeunit "CSH Self Store";

    procedure Take(var From: Codeunit "CSH Self Store")
    begin
        S := From;
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

// A SingleInstance codeunit that holds the probe's store in a Variant.
codeunit 88137 "CSN Variant Sess"
{
    SingleInstance = true;

    var
        Held: Variant;

    procedure Hold(S: Codeunit "CSH Self Store")
    begin
        Held := S;
    end;

    procedure Drop()
    begin
        Clear(Held);
    end;
}

codeunit 88140 "CSN Refused_TextGlobalInCycle"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        TS: Codeunit "CSN Text Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_TextGlobalInCycle()
    begin
        Clear(TS);
        TS.Init(TS);
        TS.SetValue(42);
        ClearAll();
        Check(TS.GetValue(), 0);
    end;
}

codeunit 88141 "CSN Refused_StoreImplementsIn"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        FS: Codeunit "CSN Face Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_StoreImplementsInterface()
    begin
        Clear(FS);
        FS.Init(FS);
        FS.SetValue(42);
        ClearAll();
        Check(FS.GetValue(), 0);
    end;
}

codeunit 88142 "CSN Refused_TableNoCodeunitIn"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        NS: Codeunit "CSN TNo Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_TableNoCodeunitInCycle()
    var
        Other: Codeunit "CSN TNo Store";
    begin
        Clear(NS);
        NS.Init(NS);
        Other := NS;
        NS.SetValue(42);
        ClearAll();
        Check(Other.GetValue(), 0);
    end;
}

codeunit 88143 "CSN Refused_Depth5"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        DS: Codeunit "CSN Deep5 Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_Depth5()
    begin
        Clear(DS);
        DS.Init(DS);
        DS.SetValue(42);
        ClearAll();
        Check(DS.GetValue(), 0);
    end;
}

codeunit 88144 "CSN Refused_CycleToMiddleNode"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        MS: Codeunit "CSN Mid Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_CycleToMiddleNode()
    begin
        Clear(MS);
        MS.Init(MS);
        MS.SetValue(42);
        ClearAll();
        Check(MS.GetValue(), 0);
    end;
}

codeunit 88145 "CSN Refused_SingleInstanceTwo"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        IS: Codeunit "CSN SI2 Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_SingleInstanceTwoLevelsDown()
    begin
        Clear(IS);
        IS.Init(IS);
        IS.SetValue(42);
        ClearAll();
        Check(IS.GetValue(), 0);
    end;
}

codeunit 88147 "CSN Refused_TwoOtherHoldersSe"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Self Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_TwoOtherHoldersSessAndLocal()
    var
        Sess: Codeunit "CSH Self Sess A";
        Other: Codeunit "CSH Self Store";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        Sess.GetSale(Other);
        S.SetValue(42);
        ClearAll();
        Check(Other.GetValue(), 0);
    end;
}

codeunit 88148 "CSN Refused_NeverCalled"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Self Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_NeverCalled()
    var
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Clear(S);
        Sess.GetSale(S);
        ClearAll();
        Check(S.GetValue(), 0);
    end;
}

codeunit 88149 "CSN Refused_PlainOwnerNotTheS"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_PlainOwnerNotTheStore()
    var
        Sess: Codeunit "CSH Self Sess A";
        Owner: Codeunit "CSN Plain Owner";
        Mine: Codeunit "CSH Self Store";
    begin
        Sess.Reset();
        Sess.Start();
        Sess.GetSale(Mine);
        Owner.Take(Mine);
        Mine.SetValue(42);
        Clear(Mine);
        Owner.ClearAllGlobals();
        Sess.GetSale(Mine);
        Check(Mine.GetValue(), 0);
    end;
}

codeunit 88150 "CSN Refused_StoreClearAllOnAn"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Self Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_StoreClearAllOnAnotherInstance()
    var
        Sess: Codeunit "CSH Self Sess A";
        X: Codeunit "CSH Self Store";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        X.Init(S);
        X.ClearAllSelf();
        Check(S.GetValue(), 0);
    end;
}

codeunit 88151 "CSN MainKept_HolderGlobalAlon"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Self Store";
        HG: Codeunit "CSH Self Holder";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure MainKept_HolderGlobalAlone()
    begin
        Clear(S);
        Clear(HG);
        HG.Start();
        ClearAll();
        Check(0, 0);
    end;
}

codeunit 88152 "CSN Refused_VariantSessHolder"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Self Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_VariantSessHolder()
    var
        VSess: Codeunit "CSN Variant Sess";
    begin
        VSess.Drop();
        Clear(S);
        S.Init(S);
        VSess.Hold(S);
        S.SetValue(42);
        ClearAll();
        Check(S.GetValue(), 0);
        VSess.Drop();
    end;
}

// The probe's store with a TableNo.
codeunit 88153 "CSN TNo Self Store"
{
    TableNo = "Probe Observer Entry";

    var
        Value: Integer;
        Myself: Codeunit "CSN TNo Self Store";

    procedure Init(ThisIn: Codeunit "CSN TNo Self Store")
    begin
        Myself := ThisIn;
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;
}

codeunit 88154 "CSN Refused_TableNoStore"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        TS: Codeunit "CSN TNo Self Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_TableNoStore()
    begin
        Clear(TS);
        TS.Init(TS);
        TS.SetValue(42);
        ClearAll();
        Check(TS.GetValue(), 0);
    end;
}

// The probe's store with a Label global.
codeunit 88155 "CSN Label Store"
{
    var
        Value: Integer;
        Myself: Codeunit "CSN Label Store";
        Caption: Label 'x';

    procedure Init(ThisIn: Codeunit "CSN Label Store")
    begin
        Myself := ThisIn;
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue + StrLen(Caption) - 1;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;
}

codeunit 88156 "CSN Refused_LabelGlobal"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        LS: Codeunit "CSN Label Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_LabelGlobal()
    begin
        Clear(LS);
        LS.Init(LS);
        LS.SetValue(42);
        ClearAll();
        Check(LS.GetValue(), 0);
    end;
}

// Two test locals on the store's nested Line instance: two outside holders of a nested instance.
codeunit 88157 "CSN Refused_TwoLineHandles"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Line Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_TwoLineHandles()
    var
        Sess: Codeunit "CSH Line Sess";
        L1: Codeunit "CSH Line";
        L2: Codeunit "CSH Line";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetLineVal(3);
        S.GetLine(L1);
        S.GetLine(L2);
        ClearAll();
        Check(L1.GetVal(), 0);
    end;
}

// A store that holds a second called instance of its own codeunit, which a test local also holds.
codeunit 88158 "CSN Two Store"
{
    var
        Value: Integer;
        Myself: Codeunit "CSN Two Store";
        Other: Codeunit "CSN Two Store";

    procedure Init(ThisIn: Codeunit "CSN Two Store")
    begin
        Myself := ThisIn;
        Other.SetValue(1);
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;

    procedure GetOther(var O: Codeunit "CSN Two Store")
    begin
        O := Other;
    end;
}

codeunit 88159 "CSN Refused_SecondStoreHeld"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSN Two Store";

    local procedure Check(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Refused_SecondStoreHeld()
    var
        T: Codeunit "CSN Two Store";
        O: Codeunit "CSN Two Store";
    begin
        Clear(S);
        S.Init(S);
        S.GetOther(O);
        T := S;
        S.SetValue(42);
        ClearAll();
        Check(O.GetValue(), 0);
    end;
}
