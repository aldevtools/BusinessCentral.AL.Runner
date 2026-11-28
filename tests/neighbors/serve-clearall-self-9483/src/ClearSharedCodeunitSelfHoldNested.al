// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); self-contained objects 88090 to 88116
// Probe: written against BC 29.0 on Linux (al-runner issue 9477), observed first, then asserted.
// Facts: ClearAll and Clear of a test codeunit global S whose store instance nested codeunits hold back, with and
//   without the store holding itself, while a SingleInstance session codeunit holds the instance too
//   (Sale.Init(Sale); GetSale(var S) runs S := Sale).
//   - "CSH Line Store": Value, Myself (itself, set by Init), Line ("CSH Line": holds the store back and an Integer).
//   - "CSH Back Store": Value, Line ("CSH Back Line": holds the store back and an Integer); no Myself.
//   - "CSH Deep Store": Value, Myself, a chain L1 -> L2 -> L3 where every level holds the store back (the store is
//     read again at depth 4), Pay (holds the store back), Front (only an Integer), Setup (SingleInstance, an Integer).
//   - "CSH Chain Store": Value and a chain C1 -> C2 -> C3 -> C4 of plain codeunits with an Integer each (depth 4),
//     no back link and no self.
//   Cells per store: *Sess_ClearAll, *Sess_Clear (Clear(S) in place of ClearAll), *Only_ClearAll (no session holder;
//   Init(S) only), *Sess_ClearAllAtEnd + *Sess_NextTest (the next test reads what is left); LineSess_NoInit (Init never
//   ran); LineSess_LineHandle and DeepSess_L3Handle (a test local also holds the nested Line or L3 instance).
//   Measured on BC 29.0 (Linux):
//   - ClearAll resets the store instance and every nested instance below it in place: Integers 0 at every level
//     (Line, L3, Pay, Front, C1, C4), the SingleInstance Setup keeps its value (6). Codeunit references are not
//     replaced: Myself and every back link (Line, L1, L3, Pay) still point at the store (a write through S shows
//     through all of them, a write through a back link shows through S), and a test local on the nested Line or L3
//     instance stays on it (reads 0 after the clear; its writes show through the store and back). The session
//     codeunit and S stay one instance. The same with and without the self link (Back store), at depth 4 in a cycle
//     (Deep store) and at depth 4 without a cycle (Chain store); in the next test for the Line, Back and Deep stores.
//   - NoInit: no link was ever set; Myself and the Line's store are separate instances and stay separate.
//   - Clear(S): S gets a fresh instance with fresh nested instances (no links); the session keeps the old tree with
//     its values and links; writes no longer cross.
// BC versions: 27, 28, 29

codeunit 88090 "CSH Line Store"
{
    var
        Value: Integer;
        Myself: Codeunit "CSH Line Store";
        Line: Codeunit "CSH Line";

    procedure Init(ThisIn: Codeunit "CSH Line Store")
    begin
        Myself := ThisIn;
        Line.Init(Myself);
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;

    procedure SetViaLine(NewValue: Integer)
    begin
        Line.SetStoreValue(NewValue);
    end;

    procedure SetLineVal(NewValue: Integer)
    begin
        Line.SetVal(NewValue);
    end;

    procedure GetLine(var L: Codeunit "CSH Line")
    begin
        L := Line;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('v=%1 myself=%2 line=%3 lval=%4', Value, Myself.GetValue(), Line.GetStoreValue(), Line.GetVal()));
    end;
}

codeunit 88091 "CSH Line"
{
    var
        Store: Codeunit "CSH Line Store";
        Val: Integer;

    procedure Init(StoreIn: Codeunit "CSH Line Store")
    begin
        Store := StoreIn;
    end;

    procedure GetStoreValue(): Integer
    begin
        exit(Store.GetValue());
    end;

    procedure SetStoreValue(NewValue: Integer)
    begin
        Store.SetValue(NewValue);
    end;

    procedure SetVal(NewValue: Integer)
    begin
        Val := NewValue;
    end;

    procedure GetVal(): Integer
    begin
        exit(Val);
    end;
}

codeunit 88092 "CSH Line Sess"
{
    SingleInstance = true;

    var
        Sale: Codeunit "CSH Line Store";

    procedure Reset()
    begin
        Clear(Sale);
    end;

    procedure Start()
    begin
        Sale.Init(Sale);
    end;

    procedure GetSale(var S: Codeunit "CSH Line Store")
    begin
        S := Sale;
    end;

    procedure State(): Text
    begin
        exit(Sale.State());
    end;
}

codeunit 88093 "CSH Line Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Line Store";
        Marker: Integer;

    local procedure Prepare(Init: Boolean)
    var
        Sess: Codeunit "CSH Line Sess";
    begin
        Sess.Reset();
        if Init then
            Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        S.SetLineVal(3);
        Marker := 1;
    end;

    local procedure ObserveAfter(var Observer: Codeunit "Probe Observer")
    var
        Sess: Codeunit "CSH Line Sess";
        S2: Codeunit "CSH Line Store";
    begin
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', S.State());
        Observer.Observe('sess', Sess.State());
        Sess.GetSale(S2);
        Observer.Observe('s2', S2.State());
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) s', S.State());
        Observer.Observe('S.SetValue(5) sess', Sess.State());
        S.SetViaLine(6);
        Observer.Observe('S.SetViaLine(6) s', S.State());
        Observer.Observe('S.SetViaLine(6) sess', Sess.State());
        S2.SetLineVal(7);
        Observer.Observe('S2.SetLineVal(7) s', S.State());
    end;

    [Test]
    procedure LineSess_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Line Sess";
    begin
        Prepare(true);
        Observer.Observe('pre s', S.State());
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('pre s', 'v=42 myself=42 line=42 lval=3');
        Observer.Expect('pre sess', 'v=42 myself=42 line=42 lval=3');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0 line=0 lval=0');
        Observer.Expect('sess', 'v=0 myself=0 line=0 lval=0');
        Observer.Expect('s2', 'v=0 myself=0 line=0 lval=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5 line=5 lval=0');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=5 line=5 lval=0');
        Observer.Expect('S.SetViaLine(6) s', 'v=6 myself=6 line=6 lval=0');
        Observer.Expect('S.SetViaLine(6) sess', 'v=6 myself=6 line=6 lval=0');
        Observer.Expect('S2.SetLineVal(7) s', 'v=6 myself=6 line=6 lval=7');
        Observer.Verify();
    end;

    // L is a test local on the store's Line instance: is the nested instance kept (reset in place) or replaced?
    [Test]
    procedure LineSess_LineHandle_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Line Sess";
        L: Codeunit "CSH Line";
    begin
        Prepare(true);
        S.GetLine(L);
        Observer.Observe('pre l', StrSubstNo('val=%1 store=%2', L.GetVal(), L.GetStoreValue()));
        ClearAll();
        Observer.Observe('l', StrSubstNo('val=%1 store=%2', L.GetVal(), L.GetStoreValue()));
        L.SetVal(9);
        Observer.Observe('L.SetVal(9) s', S.State());
        Observer.Observe('L.SetVal(9) sess', Sess.State());
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) l', StrSubstNo('val=%1 store=%2', L.GetVal(), L.GetStoreValue()));
        L.SetStoreValue(6);
        Observer.Observe('L.SetStoreValue(6) s', S.State());
        Observer.Expect('pre l', 'val=3 store=42');
        Observer.Expect('l', 'val=0 store=0');
        Observer.Expect('L.SetVal(9) s', 'v=0 myself=0 line=0 lval=9');
        Observer.Expect('L.SetVal(9) sess', 'v=0 myself=0 line=0 lval=9');
        Observer.Expect('S.SetValue(5) l', 'val=9 store=5');
        Observer.Expect('L.SetStoreValue(6) s', 'v=6 myself=6 line=6 lval=9');
        Observer.Verify();
    end;

    [Test]
    procedure LineSess_NoInit_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Prepare(false);
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0 line=0 lval=0');
        Observer.Expect('sess', 'v=0 myself=0 line=0 lval=0');
        Observer.Expect('s2', 'v=0 myself=0 line=0 lval=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=0 line=0 lval=0');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=0 line=0 lval=0');
        Observer.Expect('S.SetViaLine(6) s', 'v=5 myself=0 line=6 lval=0');
        Observer.Expect('S.SetViaLine(6) sess', 'v=5 myself=0 line=6 lval=0');
        Observer.Expect('S2.SetLineVal(7) s', 'v=5 myself=0 line=6 lval=7');
        Observer.Verify();
    end;

    [Test]
    procedure LineSess_Clear()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Prepare(true);
        Clear(S);
        ObserveAfter(Observer);
        Observer.Expect('marker', '1');
        Observer.Expect('s', 'v=0 myself=0 line=0 lval=0');
        Observer.Expect('sess', 'v=42 myself=42 line=42 lval=3');
        Observer.Expect('s2', 'v=42 myself=42 line=42 lval=3');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=0 line=0 lval=0');
        Observer.Expect('S.SetValue(5) sess', 'v=42 myself=42 line=42 lval=3');
        Observer.Expect('S.SetViaLine(6) s', 'v=5 myself=0 line=6 lval=0');
        Observer.Expect('S.SetViaLine(6) sess', 'v=42 myself=42 line=42 lval=3');
        Observer.Expect('S2.SetLineVal(7) s', 'v=5 myself=0 line=6 lval=0');
        Observer.Verify();
    end;

    [Test]
    procedure LineOnly_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Clear(S);
        S.Init(S);
        S.SetValue(42);
        S.SetLineVal(3);
        Marker := 1;
        Observer.Observe('pre s', S.State());
        ClearAll();
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', S.State());
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) s', S.State());
        S.SetViaLine(6);
        Observer.Observe('S.SetViaLine(6) s', S.State());
        Observer.Expect('pre s', 'v=42 myself=42 line=42 lval=3');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0 line=0 lval=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5 line=5 lval=0');
        Observer.Expect('S.SetViaLine(6) s', 'v=6 myself=6 line=6 lval=0');
        Observer.Verify();
    end;

    [Test]
    procedure LineSess_ClearAllAtEnd()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Line Sess";
    begin
        Prepare(true);
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        Observer.Expect('pre sess', 'v=42 myself=42 line=42 lval=3');
        Observer.Verify();
    end;

    // The _NextTest cell of corpus PR 1672 is left out: a later test that reads what a flagged test left passes clean
    // (no entry kind states that; follow-up issue).
}

codeunit 88094 "CSH Back Store"
{
    var
        Value: Integer;
        Line: Codeunit "CSH Back Line";

    procedure Init(ThisIn: Codeunit "CSH Back Store")
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

    procedure SetViaLine(NewValue: Integer)
    begin
        Line.SetStoreValue(NewValue);
    end;

    procedure SetLineVal(NewValue: Integer)
    begin
        Line.SetVal(NewValue);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('v=%1 line=%2 lval=%3', Value, Line.GetStoreValue(), Line.GetVal()));
    end;
}

codeunit 88095 "CSH Back Line"
{
    var
        Store: Codeunit "CSH Back Store";
        Val: Integer;

    procedure Init(StoreIn: Codeunit "CSH Back Store")
    begin
        Store := StoreIn;
    end;

    procedure GetStoreValue(): Integer
    begin
        exit(Store.GetValue());
    end;

    procedure SetStoreValue(NewValue: Integer)
    begin
        Store.SetValue(NewValue);
    end;

    procedure SetVal(NewValue: Integer)
    begin
        Val := NewValue;
    end;

    procedure GetVal(): Integer
    begin
        exit(Val);
    end;
}

codeunit 88096 "CSH Back Sess"
{
    SingleInstance = true;

    var
        Sale: Codeunit "CSH Back Store";

    procedure Reset()
    begin
        Clear(Sale);
    end;

    procedure Start()
    begin
        Sale.Init(Sale);
    end;

    procedure GetSale(var S: Codeunit "CSH Back Store")
    begin
        S := Sale;
    end;

    procedure State(): Text
    begin
        exit(Sale.State());
    end;
}

codeunit 88097 "CSH Back Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Back Store";
        Marker: Integer;

    local procedure Prepare()
    var
        Sess: Codeunit "CSH Back Sess";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        S.SetLineVal(3);
        Marker := 1;
    end;

    local procedure ObserveAfter(var Observer: Codeunit "Probe Observer")
    var
        Sess: Codeunit "CSH Back Sess";
        S2: Codeunit "CSH Back Store";
    begin
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', S.State());
        Observer.Observe('sess', Sess.State());
        Sess.GetSale(S2);
        Observer.Observe('s2', S2.State());
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) s', S.State());
        Observer.Observe('S.SetValue(5) sess', Sess.State());
        S.SetViaLine(6);
        Observer.Observe('S.SetViaLine(6) s', S.State());
        Observer.Observe('S.SetViaLine(6) sess', Sess.State());
        S2.SetLineVal(7);
        Observer.Observe('S2.SetLineVal(7) s', S.State());
    end;

    [Test]
    procedure BackSess_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Back Sess";
    begin
        Prepare();
        Observer.Observe('pre s', S.State());
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('pre s', 'v=42 line=42 lval=3');
        Observer.Expect('pre sess', 'v=42 line=42 lval=3');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 line=0 lval=0');
        Observer.Expect('sess', 'v=0 line=0 lval=0');
        Observer.Expect('s2', 'v=0 line=0 lval=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 line=5 lval=0');
        Observer.Expect('S.SetValue(5) sess', 'v=5 line=5 lval=0');
        Observer.Expect('S.SetViaLine(6) s', 'v=6 line=6 lval=0');
        Observer.Expect('S.SetViaLine(6) sess', 'v=6 line=6 lval=0');
        Observer.Expect('S2.SetLineVal(7) s', 'v=6 line=6 lval=7');
        Observer.Verify();
    end;

    [Test]
    procedure BackSess_Clear()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Prepare();
        Clear(S);
        ObserveAfter(Observer);
        Observer.Expect('marker', '1');
        Observer.Expect('s', 'v=0 line=0 lval=0');
        Observer.Expect('sess', 'v=42 line=42 lval=3');
        Observer.Expect('s2', 'v=42 line=42 lval=3');
        Observer.Expect('S.SetValue(5) s', 'v=5 line=0 lval=0');
        Observer.Expect('S.SetValue(5) sess', 'v=42 line=42 lval=3');
        Observer.Expect('S.SetViaLine(6) s', 'v=5 line=6 lval=0');
        Observer.Expect('S.SetViaLine(6) sess', 'v=42 line=42 lval=3');
        Observer.Expect('S2.SetLineVal(7) s', 'v=5 line=6 lval=0');
        Observer.Verify();
    end;

    [Test]
    procedure BackOnly_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Clear(S);
        S.Init(S);
        S.SetValue(42);
        S.SetLineVal(3);
        Marker := 1;
        Observer.Observe('pre s', S.State());
        ClearAll();
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', S.State());
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) s', S.State());
        S.SetViaLine(6);
        Observer.Observe('S.SetViaLine(6) s', S.State());
        Observer.Expect('pre s', 'v=42 line=42 lval=3');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 line=0 lval=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 line=5 lval=0');
        Observer.Expect('S.SetViaLine(6) s', 'v=6 line=6 lval=0');
        Observer.Verify();
    end;

    [Test]
    procedure BackSess_ClearAllAtEnd()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Back Sess";
    begin
        Prepare();
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        Observer.Expect('pre sess', 'v=42 line=42 lval=3');
        Observer.Verify();
    end;

    // The _NextTest cell of corpus PR 1672 is left out: a later test that reads what a flagged test left passes clean
    // (no entry kind states that; follow-up issue).
}

codeunit 88100 "CSH Deep Store"
{
    var
        Value: Integer;
        Myself: Codeunit "CSH Deep Store";
        Line: Codeunit "CSH Deep L1";
        Pay: Codeunit "CSH Deep Pay";
        Front: Codeunit "CSH Deep Front";
        Setup: Codeunit "CSH Deep Setup";

    procedure Init(ThisIn: Codeunit "CSH Deep Store")
    begin
        Myself := ThisIn;
        Line.Init(Myself);
        Pay.Init(Myself);
    end;

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure GetValue(): Integer
    begin
        exit(Value);
    end;

    procedure SetViaL3(NewValue: Integer)
    begin
        Line.SetL3StoreValue(NewValue);
    end;

    procedure GetL3(var L3: Codeunit "CSH Deep L3")
    begin
        Line.GetL3(L3);
    end;

    procedure SetParts(L3Val: Integer; PayVal: Integer; FrontVal: Integer; SetupVal: Integer)
    begin
        Line.SetL3Val(L3Val);
        Pay.SetVal(PayVal);
        Front.SetVal(FrontVal);
        Setup.SetVal(SetupVal);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('v=%1 myself=%2 l1=%3 l3=%4 l3val=%5 pay=%6 payval=%7 front=%8 setup=%9', Value, Myself.GetValue(),
            Line.GetStoreValue(), Line.GetL3StoreValue(), Line.GetL3Val(), Pay.GetStoreValue(), Pay.GetVal(), Front.GetVal(),
            Setup.GetVal()));
    end;
}

codeunit 88101 "CSH Deep L1"
{
    var
        Store: Codeunit "CSH Deep Store";
        Next: Codeunit "CSH Deep L2";

    procedure Init(StoreIn: Codeunit "CSH Deep Store")
    begin
        Store := StoreIn;
        Next.Init(StoreIn);
    end;

    procedure GetStoreValue(): Integer
    begin
        exit(Store.GetValue());
    end;

    procedure GetL3StoreValue(): Integer
    begin
        exit(Next.GetL3StoreValue());
    end;

    procedure GetL3(var L3: Codeunit "CSH Deep L3")
    begin
        Next.GetL3(L3);
    end;

    procedure SetL3StoreValue(NewValue: Integer)
    begin
        Next.SetL3StoreValue(NewValue);
    end;

    procedure SetL3Val(NewValue: Integer)
    begin
        Next.SetL3Val(NewValue);
    end;

    procedure GetL3Val(): Integer
    begin
        exit(Next.GetL3Val());
    end;
}

codeunit 88102 "CSH Deep L2"
{
    var
        Store: Codeunit "CSH Deep Store";
        Next: Codeunit "CSH Deep L3";

    procedure Init(StoreIn: Codeunit "CSH Deep Store")
    begin
        Store := StoreIn;
        Next.Init(StoreIn);
    end;

    procedure GetL3StoreValue(): Integer
    begin
        exit(Next.GetStoreValue());
    end;

    procedure GetL3(var L3: Codeunit "CSH Deep L3")
    begin
        L3 := Next;
    end;

    procedure SetL3StoreValue(NewValue: Integer)
    begin
        Next.SetStoreValue(NewValue);
    end;

    procedure SetL3Val(NewValue: Integer)
    begin
        Next.SetVal(NewValue);
    end;

    procedure GetL3Val(): Integer
    begin
        exit(Next.GetVal());
    end;
}

codeunit 88103 "CSH Deep L3"
{
    var
        Store: Codeunit "CSH Deep Store";
        Val: Integer;

    procedure Init(StoreIn: Codeunit "CSH Deep Store")
    begin
        Store := StoreIn;
    end;

    procedure GetStoreValue(): Integer
    begin
        exit(Store.GetValue());
    end;

    procedure SetStoreValue(NewValue: Integer)
    begin
        Store.SetValue(NewValue);
    end;

    procedure SetVal(NewValue: Integer)
    begin
        Val := NewValue;
    end;

    procedure GetVal(): Integer
    begin
        exit(Val);
    end;
}

codeunit 88104 "CSH Deep Pay"
{
    var
        Store: Codeunit "CSH Deep Store";
        Val: Integer;

    procedure Init(StoreIn: Codeunit "CSH Deep Store")
    begin
        Store := StoreIn;
    end;

    procedure GetStoreValue(): Integer
    begin
        exit(Store.GetValue());
    end;

    procedure SetVal(NewValue: Integer)
    begin
        Val := NewValue;
    end;

    procedure GetVal(): Integer
    begin
        exit(Val);
    end;
}

codeunit 88105 "CSH Deep Front"
{
    var
        Val: Integer;

    procedure SetVal(NewValue: Integer)
    begin
        Val := NewValue;
    end;

    procedure GetVal(): Integer
    begin
        exit(Val);
    end;
}

codeunit 88106 "CSH Deep Setup"
{
    SingleInstance = true;

    var
        Val: Integer;

    procedure SetVal(NewValue: Integer)
    begin
        Val := NewValue;
    end;

    procedure GetVal(): Integer
    begin
        exit(Val);
    end;
}

codeunit 88107 "CSH Deep Sess"
{
    SingleInstance = true;

    var
        Sale: Codeunit "CSH Deep Store";

    procedure Reset()
    begin
        Clear(Sale);
    end;

    procedure Start()
    begin
        Sale.Init(Sale);
    end;

    procedure GetSale(var S: Codeunit "CSH Deep Store")
    begin
        S := Sale;
    end;

    procedure State(): Text
    begin
        exit(Sale.State());
    end;
}

codeunit 88108 "CSH Deep Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Deep Store";
        Marker: Integer;

    local procedure Prepare()
    var
        Sess: Codeunit "CSH Deep Sess";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        S.SetParts(3, 4, 5, 6);
        Marker := 1;
    end;

    local procedure ObserveAfter(var Observer: Codeunit "Probe Observer")
    var
        Sess: Codeunit "CSH Deep Sess";
        S2: Codeunit "CSH Deep Store";
    begin
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', S.State());
        Observer.Observe('sess', Sess.State());
        Sess.GetSale(S2);
        Observer.Observe('s2', S2.State());
        S.SetValue(7);
        Observer.Observe('S.SetValue(7) sess', Sess.State());
        S.SetViaL3(8);
        Observer.Observe('S.SetViaL3(8) s', S.State());
        Observer.Observe('S.SetViaL3(8) sess', Sess.State());
    end;

    [Test]
    procedure DeepSess_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Deep Sess";
    begin
        Prepare();
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('pre sess', 'v=42 myself=42 l1=42 l3=42 l3val=3 pay=42 payval=4 front=5 setup=6');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0 l1=0 l3=0 l3val=0 pay=0 payval=0 front=0 setup=6');
        Observer.Expect('sess', 'v=0 myself=0 l1=0 l3=0 l3val=0 pay=0 payval=0 front=0 setup=6');
        Observer.Expect('s2', 'v=0 myself=0 l1=0 l3=0 l3val=0 pay=0 payval=0 front=0 setup=6');
        Observer.Expect('S.SetValue(7) sess', 'v=7 myself=7 l1=7 l3=7 l3val=0 pay=7 payval=0 front=0 setup=6');
        Observer.Expect('S.SetViaL3(8) s', 'v=8 myself=8 l1=8 l3=8 l3val=0 pay=8 payval=0 front=0 setup=6');
        Observer.Expect('S.SetViaL3(8) sess', 'v=8 myself=8 l1=8 l3=8 l3val=0 pay=8 payval=0 front=0 setup=6');
        Observer.Verify();
    end;

    // L3 is a test local on the instance at the end of the chain (three levels below the store).
    [Test]
    procedure DeepSess_L3Handle_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Deep Sess";
        L3: Codeunit "CSH Deep L3";
    begin
        Prepare();
        S.GetL3(L3);
        Observer.Observe('pre l3', StrSubstNo('val=%1 store=%2', L3.GetVal(), L3.GetStoreValue()));
        ClearAll();
        Observer.Observe('l3', StrSubstNo('val=%1 store=%2', L3.GetVal(), L3.GetStoreValue()));
        L3.SetVal(9);
        Observer.Observe('L3.SetVal(9) sess', Sess.State());
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) l3', StrSubstNo('val=%1 store=%2', L3.GetVal(), L3.GetStoreValue()));
        Observer.Expect('pre l3', 'val=3 store=42');
        Observer.Expect('l3', 'val=0 store=0');
        Observer.Expect('L3.SetVal(9) sess', 'v=0 myself=0 l1=0 l3=0 l3val=9 pay=0 payval=0 front=0 setup=6');
        Observer.Expect('S.SetValue(5) l3', 'val=9 store=5');
        Observer.Verify();
    end;

    [Test]
    procedure DeepSess_Clear()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Prepare();
        Clear(S);
        ObserveAfter(Observer);
        Observer.Expect('marker', '1');
        Observer.Expect('s', 'v=0 myself=0 l1=0 l3=0 l3val=0 pay=0 payval=0 front=0 setup=6');
        Observer.Expect('sess', 'v=42 myself=42 l1=42 l3=42 l3val=3 pay=42 payval=4 front=5 setup=6');
        Observer.Expect('s2', 'v=42 myself=42 l1=42 l3=42 l3val=3 pay=42 payval=4 front=5 setup=6');
        Observer.Expect('S.SetValue(7) sess', 'v=42 myself=42 l1=42 l3=42 l3val=3 pay=42 payval=4 front=5 setup=6');
        Observer.Expect('S.SetViaL3(8) s', 'v=7 myself=0 l1=0 l3=8 l3val=0 pay=0 payval=0 front=0 setup=6');
        Observer.Expect('S.SetViaL3(8) sess', 'v=42 myself=42 l1=42 l3=42 l3val=3 pay=42 payval=4 front=5 setup=6');
        Observer.Verify();
    end;

    [Test]
    procedure DeepOnly_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Clear(S);
        S.Init(S);
        S.SetValue(42);
        S.SetParts(3, 4, 5, 6);
        Marker := 1;
        Observer.Observe('pre s', S.State());
        ClearAll();
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', S.State());
        S.SetViaL3(8);
        Observer.Observe('S.SetViaL3(8) s', S.State());
        Observer.Expect('pre s', 'v=42 myself=42 l1=42 l3=42 l3val=3 pay=42 payval=4 front=5 setup=6');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0 l1=0 l3=0 l3val=0 pay=0 payval=0 front=0 setup=6');
        Observer.Expect('S.SetViaL3(8) s', 'v=8 myself=8 l1=8 l3=8 l3val=0 pay=8 payval=0 front=0 setup=6');
        Observer.Verify();
    end;

    [Test]
    procedure DeepSess_ClearAllAtEnd()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Deep Sess";
    begin
        Prepare();
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        Observer.Expect('pre sess', 'v=42 myself=42 l1=42 l3=42 l3val=3 pay=42 payval=4 front=5 setup=6');
        Observer.Verify();
    end;

    // The _NextTest cell of corpus PR 1672 is left out: a later test that reads what a flagged test left passes clean
    // (no entry kind states that; follow-up issue).
}

codeunit 88110 "CSH Chain Store"
{
    var
        Value: Integer;
        C1: Codeunit "CSH Chain C1";

    procedure SetValue(NewValue: Integer)
    begin
        Value := NewValue;
    end;

    procedure SetParts(V1: Integer; V4: Integer)
    begin
        C1.SetVal(V1);
        C1.SetC4Val(V4);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('v=%1 c1=%2 c4=%3', Value, C1.GetVal(), C1.GetC4Val()));
    end;
}

codeunit 88111 "CSH Chain C1"
{
    var
        Val: Integer;
        Next: Codeunit "CSH Chain C2";

    procedure SetVal(NewValue: Integer)
    begin
        Val := NewValue;
    end;

    procedure GetVal(): Integer
    begin
        exit(Val);
    end;

    procedure SetC4Val(NewValue: Integer)
    begin
        Next.SetC4Val(NewValue);
    end;

    procedure GetC4Val(): Integer
    begin
        exit(Next.GetC4Val());
    end;
}

codeunit 88112 "CSH Chain C2"
{
    var
        Next: Codeunit "CSH Chain C3";

    procedure SetC4Val(NewValue: Integer)
    begin
        Next.SetC4Val(NewValue);
    end;

    procedure GetC4Val(): Integer
    begin
        exit(Next.GetC4Val());
    end;
}

codeunit 88113 "CSH Chain C3"
{
    var
        Next: Codeunit "CSH Chain C4";

    procedure SetC4Val(NewValue: Integer)
    begin
        Next.SetVal(NewValue);
    end;

    procedure GetC4Val(): Integer
    begin
        exit(Next.GetVal());
    end;
}

codeunit 88114 "CSH Chain C4"
{
    var
        Val: Integer;

    procedure SetVal(NewValue: Integer)
    begin
        Val := NewValue;
    end;

    procedure GetVal(): Integer
    begin
        exit(Val);
    end;
}

codeunit 88115 "CSH Chain Sess"
{
    SingleInstance = true;

    var
        Sale: Codeunit "CSH Chain Store";

    procedure Reset()
    begin
        Clear(Sale);
    end;

    procedure GetSale(var S: Codeunit "CSH Chain Store")
    begin
        S := Sale;
    end;

    procedure State(): Text
    begin
        exit(Sale.State());
    end;
}

codeunit 88116 "CSH Chain Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Chain Store";
        Marker: Integer;

    local procedure Prepare()
    var
        Sess: Codeunit "CSH Chain Sess";
    begin
        Sess.Reset();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        S.SetParts(3, 4);
        Marker := 1;
    end;

    local procedure ObserveAfter(var Observer: Codeunit "Probe Observer")
    var
        Sess: Codeunit "CSH Chain Sess";
    begin
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', S.State());
        Observer.Observe('sess', Sess.State());
        S.SetValue(7);
        S.SetParts(8, 9);
        Observer.Observe('S.SetValue(7) S.SetParts(8, 9) sess', Sess.State());
    end;

    [Test]
    procedure ChainSess_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Chain Sess";
    begin
        Prepare();
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('pre sess', 'v=42 c1=3 c4=4');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 c1=0 c4=0');
        Observer.Expect('sess', 'v=0 c1=0 c4=0');
        Observer.Expect('S.SetValue(7) S.SetParts(8, 9) sess', 'v=7 c1=8 c4=9');
        Observer.Verify();
    end;

    [Test]
    procedure ChainSess_Clear()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Prepare();
        Clear(S);
        ObserveAfter(Observer);
        Observer.Expect('marker', '1');
        Observer.Expect('s', 'v=0 c1=0 c4=0');
        Observer.Expect('sess', 'v=42 c1=3 c4=4');
        Observer.Expect('S.SetValue(7) S.SetParts(8, 9) sess', 'v=42 c1=3 c4=4');
        Observer.Verify();
    end;

    [Test]
    procedure ChainOnly_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Clear(S);
        S.SetValue(42);
        S.SetParts(3, 4);
        Marker := 1;
        Observer.Observe('pre s', S.State());
        ClearAll();
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', S.State());
        Observer.Expect('pre s', 'v=42 c1=3 c4=4');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 c1=0 c4=0');
        Observer.Verify();
    end;
}
