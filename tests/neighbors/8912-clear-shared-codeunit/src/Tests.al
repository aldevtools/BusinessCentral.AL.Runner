// Neighbors of al-runner issue 8912: Clear and ClearAll of a codeunit variable whose instance another holder shares.
// Each test reads X and the holder as 'x|alias' and fails with what it read when that is not the value named.
interface "NB CS IStore"
{
    procedure Set(V: Integer);
    procedure Get(): Integer;
}

codeunit 50100 "NB CS Store" implements "NB CS IStore"
{
    var
        N: Integer;

    procedure Set(V: Integer)
    begin
        N := V;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

// The measured shape plus a Text global.
codeunit 50101 "NB CS Store2" implements "NB CS IStore"
{
    var
        N: Integer;
        T: Text;

    procedure Set(V: Integer)
    begin
        N := V;
        T := Format(V);
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

// One Text global instead of the Integer.
codeunit 50102 "NB CS TextStore" implements "NB CS IStore"
{
    var
        T: Text;

    procedure Set(V: Integer)
    begin
        T := Format(V);
    end;

    procedure Get(): Integer
    var
        N: Integer;
    begin
        if T = '' then
            exit(0);
        Evaluate(N, T);
        exit(N);
    end;
}

codeunit 50103 "NB CS SI Store" implements "NB CS IStore"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure Set(V: Integer)
    begin
        N := V;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

// A store whose method calls back into its holder's ClearAll.
codeunit 50104 "NB CS Calling Store"
{
    var
        N: Integer;

    procedure Set(V: Integer)
    begin
        N := V;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;

    procedure SetThenClearHolder(var H: Codeunit "NB CS Holder"; V: Integer): Integer
    begin
        N := V;
        H.ClearAllGlobals();
        exit(N);
    end;
}

codeunit 50105 "NB CS Holder"
{
    var
        X: Codeunit "NB CS Store";
        X2: Codeunit "NB CS Store2";
        XT: Codeunit "NB CS TextStore";
        XC: Codeunit "NB CS Calling Store";
        Arr: array[2] of Codeunit "NB CS Store";

    procedure ShareX(var Y: Codeunit "NB CS Store") begin Y := X; end;
    procedure ShareX2(var I: Interface "NB CS IStore") begin I := X2; end;
    procedure ShareXT(var I: Interface "NB CS IStore") begin I := XT; end;
    procedure ShareXC(var Y: Codeunit "NB CS Calling Store") begin Y := XC; end;
    procedure ShareArr(var Y: Codeunit "NB CS Store") begin Y := Arr[1]; end;
    procedure ClearArr() begin Clear(Arr); end;
    procedure ClearArrElement() begin Clear(Arr[1]); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure GetX(): Integer begin exit(X.Get()); end;
    procedure GetX2(): Integer begin exit(X2.Get()); end;
    procedure GetXT(): Integer begin exit(XT.Get()); end;
    procedure GetXC(): Integer begin exit(XC.Get()); end;
    procedure GetArr(): Integer begin exit(Arr[1].Get()); end;
    procedure CallBack(var Self: Codeunit "NB CS Holder"): Integer begin exit(XC.SetThenClearHolder(Self, 3)); end;
}

codeunit 50106 "NB CS SI Holder"
{
    SingleInstance = true;

    var
        X: Codeunit "NB CS Store";

    procedure ShareX(var Y: Codeunit "NB CS Store") begin Y := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure GetX(): Integer begin exit(X.Get()); end;
}

codeunit 50107 "NB CS Table Holder"
{
    TableNo = "NB CS Row";

    var
        X: Codeunit "NB CS Store";

    trigger OnRun()
    begin
    end;

    procedure ShareX(var Y: Codeunit "NB CS Store") begin Y := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure GetX(): Integer begin exit(X.Get()); end;
}

table 50108 "NB CS Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

report 50109 "NB CS Report"
{
    ProcessingOnly = true;

    var
        X: Codeunit "NB CS Store";

    procedure ShareX(var Y: Codeunit "NB CS Store") begin Y := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure GetX(): Integer begin exit(X.Get()); end;
}

codeunit 50110 "NB CS Subscriber"
{
    EventSubscriberInstance = Manual;

    var
        Hits: Integer;

    procedure GetHits(): Integer begin exit(Hits); end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"NB CS Publisher", 'OnPing', '', false, false)]
    local procedure OnPing()
    begin
        Hits += 1;
    end;
}

codeunit 50111 "NB CS Publisher"
{
    procedure Ping() begin OnPing(); end;

    [IntegrationEvent(false, false)]
    local procedure OnPing()
    begin
    end;
}

codeunit 50112 "NB CS Var Publisher"
{
    procedure Raise(var S: Codeunit "NB CS Store") begin OnClear(S); end;

    [IntegrationEvent(false, false)]
    local procedure OnClear(var S: Codeunit "NB CS Store")
    begin
    end;
}

codeunit 50113 "NB CS Var Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"NB CS Var Publisher", 'OnClear', '', false, false)]
    local procedure OnClear(var S: Codeunit "NB CS Store")
    begin
        Clear(S);
    end;
}

interface "NB CS IClearer"
{
    procedure ClearIt(var S: Codeunit "NB CS Store");
}

codeunit 50114 "NB CS Clearer" implements "NB CS IClearer"
{
    procedure ClearIt(var S: Codeunit "NB CS Store") begin Clear(S); end;
}

codeunit 50115 "NB CS Global Chain"
{
    var
        X: Codeunit "NB CS Store";

    local procedure ReassignThenClear(var P: Codeunit "NB CS Store")
    var
        Z: Codeunit "NB CS Store";
    begin
        X := Z;
        Clear(P);
    end;

    procedure Run(): Text
    var
        Y: Codeunit "NB CS Store";
    begin
        Y := X;
        ReassignThenClear(X);
        Y.Set(5);
        exit(Format(X.Get()) + '|' + Format(Y.Get()));
    end;
}

codeunit 50120 "NB CS Tests"
{
    Subtype = Test;

    var
        TX: Codeunit "NB CS Store";

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;

    local procedure R(A: Integer; B: Integer): Text
    begin
        exit(Format(A) + '|' + Format(B));
    end;

    local procedure ClearByValue(P: Codeunit "NB CS Store")
    begin
        Clear(P);
        P.Set(5);
    end;

    // The measured cell: Clear of an unused local X an interface shares (ClearSharedCodeunit.al Clear_LocalUnusedIface).
    [Test]
    procedure Clear_LocalUnused_Interface()
    var
        X: Codeunit "NB CS Store";
        I: Interface "NB CS IStore";
    begin
        I := X;
        Clear(X);
        I.Set(5);
        Check(R(X.Get(), I.Get()), '0|5');
    end;

    // The measured cell: ClearAll in the holder resets the unused shared instance in place (ClearAll_GlobalUnusedAlias).
    [Test]
    procedure ClearAll_HolderUnused_Alias()
    var
        H: Codeunit "NB CS Holder";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareX(Y);
        H.ClearAllGlobals();
        Y.Set(5);
        Check(R(H.GetX(), Y.Get()), '5|5');
    end;

    // The measured cell with a write before: the alias sees the reset (ClearAll_GlobalUsedAlias_AliasWroteBefore).
    [Test]
    procedure ClearAll_HolderUsed_AliasWroteBefore()
    var
        H: Codeunit "NB CS Holder";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareX(Y);
        Y.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX(), Y.Get()), '0|0');
    end;

    // Clear of a used X: main's fresh instance, unflagged (Clear_LocalUsedAlias).
    [Test]
    procedure Clear_LocalUsed_Alias()
    var
        X: Codeunit "NB CS Store";
        Y: Codeunit "NB CS Store";
    begin
        X.Get();
        Y := X;
        Clear(X);
        Y.Set(5);
        Check(R(X.Get(), Y.Get()), '0|5');
    end;

    // Clear of an unused X nothing else holds: main's answer, unflagged (a fresh unused instance reads the same).
    [Test]
    procedure Clear_LocalUnused_NoHolder()
    var
        X: Codeunit "NB CS Store";
    begin
        Clear(X);
        X.Set(5);
        Check(R(X.Get(), 0), '5|0');
    end;

    // Refused: Clear of a by-value parameter nothing used yet that the caller's X shares (the probe never clears one).
    [Test]
    procedure Clear_ValueParamItself()
    var
        X: Codeunit "NB CS Store";
    begin
        ClearByValue(X);
        Check(R(X.Get(), 0), '0|0');
    end;

    // BindSubscription makes the instance, so a bound S is a used one: main's Clear of a used variable, unflagged.
    [Test]
    procedure Clear_UnusedBound()
    var
        S: Codeunit "NB CS Subscriber";
        P: Codeunit "NB CS Publisher";
    begin
        BindSubscription(S);
        Clear(S);
        P.Ping();
        Check(R(S.GetHits(), 0), '0|0');
    end;

    // Refused: Clear of an array whose unused codeunit element another variable shares (not measured).
    [Test]
    procedure Clear_ArrayElementUnusedShared()
    var
        H: Codeunit "NB CS Holder";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareArr(Y);
        H.ClearArr();
        Y.Set(5);
        Check(R(H.GetArr(), Y.Get()), '5|5');
    end;

    // Main kept: an unused SingleInstance codeunit variable; every variable reads the one instance (ClearSharedCodeunitSI.al).
    [Test]
    procedure Clear_SingleInstanceUnused()
    var
        R0: Codeunit "NB CS SI Store";
        X: Codeunit "NB CS SI Store";
        I: Interface "NB CS IStore";
    begin
        R0.Set(0);
        I := X;
        Clear(X);
        I.Set(5);
        Check(R(X.Get(), I.Get()), '5|5');
    end;

    // Refused: ClearAll of a shared store with a second (Text) global (the probe's store has one Integer global).
    [Test]
    procedure ClearAll_StoreWithTwoGlobals()
    var
        H: Codeunit "NB CS Holder";
        I: Interface "NB CS IStore";
    begin
        H.ShareX2(I);
        I.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX2(), I.Get()), '0|3');
    end;

    // Refused: ClearAll of a shared store whose one global is a Text.
    [Test]
    procedure ClearAll_StoreWithTextGlobal()
    var
        H: Codeunit "NB CS Holder";
        I: Interface "NB CS IStore";
    begin
        H.ShareXT(I);
        I.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetXT(), I.Get()), '0|3');
    end;

    // Refused: ClearAll while a method of X's codeunit runs.
    [Test]
    procedure ClearAll_WhileStoreRuns()
    var
        H: Codeunit "NB CS Holder";
        Y: Codeunit "NB CS Calling Store";
        Inner: Integer;
    begin
        H.ShareXC(Y);
        Inner := H.CallBack(H);
        Check(R(H.GetXC(), Inner), '0|3');
    end;

    // ClearAll of a shared X in a SingleInstance holder after the alias wrote: one instance, reset in place
    // (corpus PR 1564, ClearSharedCodeunitOwnerKinds.al:1492 SICLX_ClearAll_AliasWroteBefore).
    [Test]
    procedure ClearAll_SingleInstanceHolder()
    var
        H: Codeunit "NB CS SI Holder";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareX(Y);
        Y.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX(), Y.Get()), '0|0');
    end;

    // Refused: ClearAll of a shared X in a holder with a TableNo.
    [Test]
    procedure ClearAll_TableNoHolder()
    var
        H: Codeunit "NB CS Table Holder";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareX(Y);
        Y.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX(), Y.Get()), '0|3');
    end;

    // Refused: ClearAll of a shared X in a report.
    [Test]
    procedure ClearAll_ReportHolder()
    var
        H: Report "NB CS Report";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareX(Y);
        Y.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX(), Y.Get()), '0|3');
    end;

    // ClearAll of a shared X in the test codeunit itself after the alias wrote: one instance, reset in place
    // (corpus PR 1564, ClearSharedCodeunitOwnerKinds.al:1568 ClearAll_TestGlobalCLX_LocalAliasWroteBefore).
    [Test]
    procedure ClearAll_TestCodeunit()
    var
        Y: Codeunit "NB CS Store";
    begin
        Y := TX;
        Y.Set(3);
        ClearAll();
        Check(R(TX.Get(), Y.Get()), '0|0');
    end;

    // ClearAll in the holder of an X nothing else holds: main's fresh instance, unflagged.
    [Test]
    procedure ClearAll_Holder_NoHolder()
    var
        H: Codeunit "NB CS Holder";
    begin
        H.ClearAllGlobals();
        Check(R(H.GetX(), 0), '0|0');
    end;

    // Clear of an unused X nothing else holds, through a var parameter: main's answer, unflagged.
    [Test]
    procedure Clear_LocalUnused_ThroughVarParam_NoHolder()
    var
        X: Codeunit "NB CS Store";
    begin
        ClearByVar(X);
        X.Set(5);
        Check(R(X.Get(), 0), '5|0');
    end;

    // Clear of a used by-value parameter: main's fresh instance; the caller's X keeps the old one.
    [Test]
    procedure Clear_ValueParamItself_Used()
    var
        X: Codeunit "NB CS Store";
    begin
        X.Get();
        ClearByValue(X);
        Check(R(X.Get(), 0), '0|0');
    end;

    // Refused: Clear of an unused codeunit array element itself that another variable shares (not measured).
    [Test]
    procedure Clear_ArrayElementItselfUnusedShared()
    var
        H: Codeunit "NB CS Holder";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareArr(Y);
        H.ClearArrElement();
        Y.Set(5);
        Check(R(H.GetArr(), Y.Get()), '5|5');
    end;

    // Clear of an unused X nothing else holds, through a var parameter handed on as a var parameter (a facade handing it to
    // its implementation): the slots of the chain are not other holders, main's answer, unflagged.
    [Test]
    procedure Clear_LocalUnused_ThroughNestedVarParam_NoHolder()
    var
        X: Codeunit "NB CS Store";
    begin
        ClearByVarVar(X);
        X.Set(5);
        Check(R(X.Get(), 0), '5|0');
    end;

    // Clear of an Interface whose unused X another codeunit variable shares: the Interface place keeps main's path.
    [Test]
    procedure Clear_InterfaceOfSharedUnused()
    var
        X: Codeunit "NB CS Store";
        Y: Codeunit "NB CS Store";
        I: Interface "NB CS IStore";
    begin
        Y := X;
        I := X;
        Clear(I);
        Y.Set(5);
        Check(R(X.Get(), Y.Get()), '5|5');
    end;

    // The measured cell through a var parameter assigned another variable's instance (Clear_LocalUnusedAlias): the
    // parameter gets a fresh instance, Y keeps the old one.
    [Test]
    procedure Clear_VarParamAssigned_Shared()
    var
        X: Codeunit "NB CS Store";
    begin
        Check(R(AssignThenClear(X), 0), '0|0');
    end;

    // Clear of an unused X nothing else holds, in an event subscriber's var parameter: the platform's call keeps no record of
    // the chain, so the publisher's copy counts as a holder and it is flagged; the value is the same as main's.
    [Test]
    procedure Clear_SubscriberVarParam_NoHolder()
    var
        X: Codeunit "NB CS Store";
        P: Codeunit "NB CS Var Publisher";
    begin
        P.Raise(X);
        X.Set(5);
        Check(R(X.Get(), 0), '5|0');
    end;

    // Clear of an unused X nothing else holds, in an interface method's var parameter: main's answer, unflagged.
    [Test]
    procedure Clear_InterfaceVarParam_NoHolder()
    var
        X: Codeunit "NB CS Store";
        C: Codeunit "NB CS Clearer";
        I: Interface "NB CS IClearer";
    begin
        I := C;
        I.ClearIt(X);
        X.Set(5);
        Check(R(X.Get(), 0), '5|0');
    end;

    // Refused: Clear of a var parameter whose caller's global was assigned another instance while the routine runs.
    [Test]
    procedure Clear_VarParamOfReassignedGlobal_Shared()
    var
        G: Codeunit "NB CS Global Chain";
    begin
        Check(G.Run(), '0|5');
    end;

    local procedure AssignThenClear(var P: Codeunit "NB CS Store"): Integer
    var
        Y: Codeunit "NB CS Store";
    begin
        P := Y;
        Clear(P);
        P.Set(5);
        exit(Y.Get());
    end;

    local procedure ClearByVar(var P: Codeunit "NB CS Store")
    begin
        Clear(P);
    end;

    local procedure ClearByVarVar(var P: Codeunit "NB CS Store")
    begin
        ClearByVar(P);
    end;
}
