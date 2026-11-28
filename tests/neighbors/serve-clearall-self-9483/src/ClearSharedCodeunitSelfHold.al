// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); self-contained objects 88080 to 88087
// Probe: written against BC 29.0 on Linux (al-runner issue 9477), observed first, then asserted.
// Facts: ClearAll and Clear of a codeunit variable S whose instance holds ITSELF: "CSH Self Store" has a global
//   Myself: Codeunit "CSH Self Store", and Init(ThisIn) runs Myself := ThisIn with the caller's own variable, so the
//   instance holds itself. A SingleInstance session codeunit ("CSH Self Sess A", "CSH Self Sess B") also holds the
//   instance (Sale.Init(Sale); GetSale(var S) runs S := Sale).
//   State text: v = Value of the instance, myself = Value read through its Myself global.
//   Cells: S is a test codeunit global (owner = the test codeunit) unless named; SelfOnly = nothing else holds the
//   instance; SelfSess = the session codeunit holds it; Never = no write before the clear; NoInit = Init never ran
//   (Myself never assigned, same globals); InitVar = Init takes a var parameter; SetSaleFromTest = the test runs
//   S.Init(S) and hands S to the session; ClearAllTwice = a write between two ClearAll; HolderClearAll = ClearAll
//   inside the session codeunit; StoreClearAllSelf = ClearAll inside the store; SelfHolderLocal / SelfHolderGlobal =
//   a plain (not SingleInstance) holder, a test local / a test codeunit global; ClearAllAtEnd + NextTest = the test
//   ends with ClearAll() and the next test reads what is left; "CSH Self Owner Tests" = the owner of S is a plain
//   codeunit (Subtype = Normal) or a SingleInstance codeunit that runs ClearAll.
//   Measured on BC 29.0 (Linux):
//   - ClearAll resets the one instance in place in every cell: S, the session codeunit, a variable taken from it
//     after the clear and Myself all read 0, and stay one instance: a write through S, through the variable taken
//     from the session or through Myself shows through all of them. The owner's Integer goes to 0. The same for every
//     owner (test codeunit, plain codeunit, SingleInstance codeunit), for ClearAll in the holder or in the store
//     itself, for a plain holder (local or a global the same ClearAll resets), and for two ClearAll in a row.
//   - Myself still holds the instance after the ClearAll (a write through Myself shows through S and back).
//   - NoInit: Myself is a separate instance (made by the first read through it); the ClearAll does not touch it and
//     it stays separate.
//   - Clear(S): S gets a fresh instance (Myself not assigned in it); the session keeps the old one with its values,
//     still holding itself; writes no longer cross.
//   - Next test after ClearAll at the end: the session's instance reads 0 and is still the test global's instance.
// BC versions: 27, 28, 29

codeunit 88080 "CSH Self Store"
{
    var
        Value: Integer;
        Myself: Codeunit "CSH Self Store";

    procedure Init(ThisIn: Codeunit "CSH Self Store")
    begin
        Myself := ThisIn;
    end;

    procedure InitVar(var ThisIn: Codeunit "CSH Self Store")
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

    procedure SetViaMyself(NewValue: Integer)
    begin
        Myself.SetValue(NewValue);
    end;

    procedure GetViaMyself(): Integer
    begin
        exit(Myself.GetValue());
    end;

    procedure ClearAllSelf()
    begin
        ClearAll();
    end;
}

codeunit 88081 "CSH Self Sess A"
{
    SingleInstance = true;

    var
        Sale: Codeunit "CSH Self Store";

    procedure Reset()
    begin
        Clear(Sale);
    end;

    procedure Start()
    begin
        Sale.Init(Sale);
    end;

    procedure StartVar()
    begin
        Sale.InitVar(Sale);
    end;

    procedure SetSale(NewSale: Codeunit "CSH Self Store")
    begin
        Sale := NewSale;
    end;

    procedure GetSale(var S: Codeunit "CSH Self Store")
    begin
        S := Sale;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('v=%1 myself=%2', Sale.GetValue(), Sale.GetViaMyself()));
    end;

    procedure ClearAllSess()
    begin
        ClearAll();
    end;
}

// The same as "CSH Self Sess A", not SingleInstance.
codeunit 88082 "CSH Self Holder"
{
    var
        Sale: Codeunit "CSH Self Store";

    procedure Start()
    begin
        Sale.Init(Sale);
    end;

    procedure GetSale(var S: Codeunit "CSH Self Store")
    begin
        S := Sale;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('v=%1 myself=%2', Sale.GetValue(), Sale.GetViaMyself()));
    end;
}

codeunit 88083 "CSH Self Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        S: Codeunit "CSH Self Store";
        HG: Codeunit "CSH Self Holder";
        Marker: Integer;

    local procedure State(var X: Codeunit "CSH Self Store"): Text
    begin
        exit(StrSubstNo('v=%1 myself=%2', X.GetValue(), X.GetViaMyself()));
    end;

    // After a clear: read S, the session holder and a fresh variable taken from the holder; then write through each.
    local procedure ObserveAfter(var Observer: Codeunit "Probe Observer")
    var
        Sess: Codeunit "CSH Self Sess A";
        S2: Codeunit "CSH Self Store";
    begin
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', State(S));
        Observer.Observe('sess', Sess.State());
        Sess.GetSale(S2);
        Observer.Observe('s2', State(S2));
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) s', State(S));
        Observer.Observe('S.SetValue(5) sess', Sess.State());
        S2.SetValue(7);
        Observer.Observe('S2.SetValue(7) s', State(S));
        Observer.Observe('S2.SetValue(7) sess', Sess.State());
        S.SetViaMyself(8);
        Observer.Observe('S.SetViaMyself(8) s', State(S));
        Observer.Observe('S.SetViaMyself(8) sess', Sess.State());
    end;

    [Test]
    procedure SelfOnly_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Clear(S);
        S.Init(S);
        S.SetValue(42);
        Marker := 1;
        Observer.Observe('pre', State(S));
        ClearAll();
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', State(S));
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) s', State(S));
        S.SetViaMyself(6);
        Observer.Observe('S.SetViaMyself(6) s', State(S));
        Observer.Expect('pre', 'v=42 myself=42');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5');
        Observer.Expect('S.SetViaMyself(6) s', 'v=6 myself=6');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        Observer.Observe('pre s', State(S));
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('pre s', 'v=42 myself=42');
        Observer.Expect('pre sess', 'v=42 myself=42');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('s2', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('S2.SetValue(7) s', 'v=7 myself=7');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=7');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_Never_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        Marker := 1;
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('s2', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('S2.SetValue(7) s', 'v=7 myself=7');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=7');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_NoInit_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        Observer.Observe('pre s', State(S));
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('pre s', 'v=42 myself=0');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('s2', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=0');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=0');
        Observer.Expect('S2.SetValue(7) s', 'v=7 myself=0');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=0');
        Observer.Expect('S.SetViaMyself(8) s', 'v=7 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=7 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_InitVar_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Sess.StartVar();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        Observer.Observe('pre s', State(S));
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('pre s', 'v=42 myself=42');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('s2', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('S2.SetValue(7) s', 'v=7 myself=7');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=7');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_SetSaleFromTest_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Clear(S);
        S.Init(S);
        Sess.SetSale(S);
        S.SetValue(42);
        Marker := 1;
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('pre sess', 'v=42 myself=42');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('s2', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('S2.SetValue(7) s', 'v=7 myself=7');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=7');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_ClearAllTwice()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        ClearAll();
        S.SetValue(43);
        Marker := 1;
        Observer.Observe('between s', State(S));
        Observer.Observe('between sess', Sess.State());
        ClearAll();
        ObserveAfter(Observer);
        Observer.Expect('between s', 'v=43 myself=43');
        Observer.Expect('between sess', 'v=43 myself=43');
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('s2', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('S2.SetValue(7) s', 'v=7 myself=7');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=7');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_Clear()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        Clear(S);
        ObserveAfter(Observer);
        Observer.Expect('marker', '1');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=42 myself=42');
        Observer.Expect('s2', 'v=42 myself=42');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=0');
        Observer.Expect('S.SetValue(5) sess', 'v=42 myself=42');
        Observer.Expect('S2.SetValue(7) s', 'v=5 myself=0');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=7');
        Observer.Expect('S.SetViaMyself(8) s', 'v=5 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=7 myself=7');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_HolderClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        Sess.ClearAllSess();
        ObserveAfter(Observer);
        Observer.Expect('marker', '1');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('s2', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('S2.SetValue(7) s', 'v=7 myself=7');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=7');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfSess_StoreClearAllSelf()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        S.ClearAllSelf();
        ObserveAfter(Observer);
        Observer.Expect('marker', '1');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('s2', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) s', 'v=5 myself=5');
        Observer.Expect('S.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('S2.SetValue(7) s', 'v=7 myself=7');
        Observer.Expect('S2.SetValue(7) sess', 'v=7 myself=7');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfHolderLocal_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        H: Codeunit "CSH Self Holder";
    begin
        H.Start();
        Clear(S);
        H.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        ClearAll();
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', State(S));
        Observer.Observe('h', H.State());
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) h', H.State());
        S.SetViaMyself(8);
        Observer.Observe('S.SetViaMyself(8) s', State(S));
        Observer.Observe('S.SetViaMyself(8) h', H.State());
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('h', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) h', 'v=5 myself=5');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) h', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SelfHolderGlobal_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Clear(HG);
        HG.Start();
        Clear(S);
        HG.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        ClearAll();
        Observer.Observe('marker', Format(Marker));
        Observer.Observe('s', State(S));
        Observer.Observe('hg', HG.State());
        S.SetValue(5);
        Observer.Observe('S.SetValue(5) hg', HG.State());
        S.SetViaMyself(8);
        Observer.Observe('S.SetViaMyself(8) s', State(S));
        Observer.Observe('S.SetViaMyself(8) hg', HG.State());
        Observer.Expect('marker', '0');
        Observer.Expect('s', 'v=0 myself=0');
        Observer.Expect('hg', 'v=0 myself=0');
        Observer.Expect('S.SetValue(5) hg', 'v=5 myself=5');
        Observer.Expect('S.SetViaMyself(8) s', 'v=8 myself=8');
        Observer.Expect('S.SetViaMyself(8) hg', 'v=8 myself=8');
        Observer.Verify();
    end;

    // The test ends with ClearAll(), the next test reads what it left (tests of a codeunit run in declaration order).
    [Test]
    procedure SelfSess_ClearAllAtEnd()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess A";
    begin
        Sess.Reset();
        Sess.Start();
        Clear(S);
        Sess.GetSale(S);
        S.SetValue(42);
        Marker := 1;
        Observer.Observe('pre s', State(S));
        Observer.Observe('pre sess', Sess.State());
        ClearAll();
        Observer.Expect('pre s', 'v=42 myself=42');
        Observer.Expect('pre sess', 'v=42 myself=42');
        Observer.Verify();
    end;

    // The _NextTest cell of corpus PR 1672 is left out: a later test that reads what a flagged test left passes clean
    // (no entry kind states that; follow-up issue).
}

codeunit 88084 "CSH Self Sess B"
{
    SingleInstance = true;

    var
        Sale: Codeunit "CSH Self Store";

    procedure Reset()
    begin
        Clear(Sale);
    end;

    procedure Start()
    begin
        Sale.Init(Sale);
    end;

    procedure GetSale(var S: Codeunit "CSH Self Store")
    begin
        S := Sale;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('v=%1 myself=%2', Sale.GetValue(), Sale.GetViaMyself()));
    end;
}

// The owner of S is a plain codeunit (Subtype = Normal) that runs ClearAll.
codeunit 88085 "CSH Self Owner"
{
    Subtype = Normal;

    var
        S: Codeunit "CSH Self Store";
        Marker: Integer;

    procedure TakeFromSess()
    var
        Sess: Codeunit "CSH Self Sess B";
    begin
        Sess.GetSale(S);
    end;

    procedure InitSelf()
    begin
        S.Init(S);
    end;

    procedure SetValue(NewValue: Integer)
    begin
        S.SetValue(NewValue);
        Marker := 1;
    end;

    procedure SetViaMyself(NewValue: Integer)
    begin
        S.SetViaMyself(NewValue);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('marker=%1 v=%2 myself=%3', Marker, S.GetValue(), S.GetViaMyself()));
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

// The same as "CSH Self Owner", SingleInstance.
codeunit 88086 "CSH Self SI Owner"
{
    SingleInstance = true;

    var
        S: Codeunit "CSH Self Store";
        Marker: Integer;

    procedure Reset()
    begin
        Clear(S);
    end;

    procedure TakeFromSess()
    var
        Sess: Codeunit "CSH Self Sess B";
    begin
        Sess.GetSale(S);
    end;

    procedure SetValue(NewValue: Integer)
    begin
        S.SetValue(NewValue);
        Marker := 1;
    end;

    procedure SetViaMyself(NewValue: Integer)
    begin
        S.SetViaMyself(NewValue);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('marker=%1 v=%2 myself=%3', Marker, S.GetValue(), S.GetViaMyself()));
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;
}

codeunit 88087 "CSH Self Owner Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure PlainOwner_Sess_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess B";
        Owner: Codeunit "CSH Self Owner";
    begin
        Sess.Reset();
        Sess.Start();
        Owner.TakeFromSess();
        Owner.SetValue(42);
        Observer.Observe('pre owner', Owner.State());
        Observer.Observe('pre sess', Sess.State());
        Owner.ClearAllGlobals();
        Observer.Observe('owner', Owner.State());
        Observer.Observe('sess', Sess.State());
        Owner.SetValue(5);
        Observer.Observe('Owner.SetValue(5) sess', Sess.State());
        Owner.SetViaMyself(8);
        Observer.Observe('Owner.SetViaMyself(8) owner', Owner.State());
        Observer.Observe('Owner.SetViaMyself(8) sess', Sess.State());
        Observer.Expect('pre owner', 'marker=1 v=42 myself=42');
        Observer.Expect('pre sess', 'v=42 myself=42');
        Observer.Expect('owner', 'marker=0 v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('Owner.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('Owner.SetViaMyself(8) owner', 'marker=1 v=8 myself=8');
        Observer.Expect('Owner.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure PlainOwner_NoHolder_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Owner: Codeunit "CSH Self Owner";
    begin
        Owner.InitSelf();
        Owner.SetValue(42);
        Observer.Observe('pre owner', Owner.State());
        Owner.ClearAllGlobals();
        Observer.Observe('owner', Owner.State());
        Owner.SetValue(5);
        Observer.Observe('Owner.SetValue(5) owner', Owner.State());
        Owner.SetViaMyself(8);
        Observer.Observe('Owner.SetViaMyself(8) owner', Owner.State());
        Observer.Expect('pre owner', 'marker=1 v=42 myself=42');
        Observer.Expect('owner', 'marker=0 v=0 myself=0');
        Observer.Expect('Owner.SetValue(5) owner', 'marker=1 v=5 myself=5');
        Observer.Expect('Owner.SetViaMyself(8) owner', 'marker=1 v=8 myself=8');
        Observer.Verify();
    end;

    [Test]
    procedure SIOwner_Sess_ClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Sess: Codeunit "CSH Self Sess B";
        Owner: Codeunit "CSH Self SI Owner";
    begin
        Sess.Reset();
        Sess.Start();
        Owner.Reset();
        Owner.TakeFromSess();
        Owner.SetValue(42);
        Observer.Observe('pre owner', Owner.State());
        Observer.Observe('pre sess', Sess.State());
        Owner.ClearAllGlobals();
        Observer.Observe('owner', Owner.State());
        Observer.Observe('sess', Sess.State());
        Owner.SetValue(5);
        Observer.Observe('Owner.SetValue(5) sess', Sess.State());
        Owner.SetViaMyself(8);
        Observer.Observe('Owner.SetViaMyself(8) owner', Owner.State());
        Observer.Observe('Owner.SetViaMyself(8) sess', Sess.State());
        Observer.Expect('pre owner', 'marker=1 v=42 myself=42');
        Observer.Expect('pre sess', 'v=42 myself=42');
        Observer.Expect('owner', 'marker=0 v=0 myself=0');
        Observer.Expect('sess', 'v=0 myself=0');
        Observer.Expect('Owner.SetValue(5) sess', 'v=5 myself=5');
        Observer.Expect('Owner.SetViaMyself(8) owner', 'marker=1 v=8 myself=8');
        Observer.Expect('Owner.SetViaMyself(8) sess', 'v=8 myself=8');
        Observer.Verify();
    end;
}
