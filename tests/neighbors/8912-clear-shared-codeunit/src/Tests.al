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

    // Clear of an unused X nothing else holds: a fresh instance, no holder to see it.
    [Test]
    procedure Clear_LocalUnused_NoHolder()
    var
        X: Codeunit "NB CS Store";
    begin
        Clear(X);
        X.Set(5);
        Check(R(X.Get(), 0), '5|0');
    end;

    // Assumed beyond the cells: Clear of a value parameter itself; the caller's X keeps the instance.
    [Test]
    procedure Clear_ValueParamItself()
    var
        X: Codeunit "NB CS Store";
    begin
        ClearByValue(X);
        Check(R(X.Get(), 0), '0|0');
    end;

    // Assumed beyond the cells: an unused X bound by BindSubscription; Clear ends the binding as for a used one.
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

    // Main kept: an element of an array of codeunits nothing used yet; Clear of the array leaves it shared.
    [Test]
    procedure Clear_ArrayElementUnused_KeepsMain()
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

    // Main kept: ClearAll of a store with a second (Text) global gives X a fresh instance.
    [Test]
    procedure ClearAll_StoreWithTwoGlobals_KeepsMain()
    var
        H: Codeunit "NB CS Holder";
        I: Interface "NB CS IStore";
    begin
        H.ShareX2(I);
        I.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX2(), I.Get()), '0|3');
    end;

    // Main kept: ClearAll of a store whose one global is a Text.
    [Test]
    procedure ClearAll_StoreWithTextGlobal_KeepsMain()
    var
        H: Codeunit "NB CS Holder";
        I: Interface "NB CS IStore";
    begin
        H.ShareXT(I);
        I.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetXT(), I.Get()), '0|3');
    end;

    // Main kept: ClearAll while a method of X's codeunit runs.
    [Test]
    procedure ClearAll_WhileStoreRuns_KeepsMain()
    var
        H: Codeunit "NB CS Holder";
        Y: Codeunit "NB CS Calling Store";
        Inner: Integer;
    begin
        H.ShareXC(Y);
        Inner := H.CallBack(H);
        Check(R(H.GetXC(), Inner), '0|3');
    end;

    // Main kept: ClearAll in a SingleInstance holder.
    [Test]
    procedure ClearAll_SingleInstanceHolder_KeepsMain()
    var
        H: Codeunit "NB CS SI Holder";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareX(Y);
        Y.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX(), Y.Get()), '0|3');
    end;

    // Main kept: ClearAll in a holder with a TableNo.
    [Test]
    procedure ClearAll_TableNoHolder_KeepsMain()
    var
        H: Codeunit "NB CS Table Holder";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareX(Y);
        Y.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX(), Y.Get()), '0|3');
    end;

    // Main kept: ClearAll in a report.
    [Test]
    procedure ClearAll_ReportHolder_KeepsMain()
    var
        H: Report "NB CS Report";
        Y: Codeunit "NB CS Store";
    begin
        H.ShareX(Y);
        Y.Set(3);
        H.ClearAllGlobals();
        Check(R(H.GetX(), Y.Get()), '0|3');
    end;

    // Main kept: ClearAll in the test codeunit itself.
    [Test]
    procedure ClearAll_TestCodeunit_KeepsMain()
    var
        Y: Codeunit "NB CS Store";
    begin
        Y := TX;
        Y.Set(3);
        ClearAll();
        Check(R(TX.Get(), Y.Get()), '0|3');
    end;
}
