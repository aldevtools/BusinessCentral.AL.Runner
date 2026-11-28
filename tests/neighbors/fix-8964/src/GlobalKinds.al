// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); self-contained objects 86270 to 86278
// Probe: written against BC 27.5, 28.5 and 29.0 (al-runner issue 8964), observed first, then asserted.
// Facts: ClearAll in the owner resets every global of the store codeunit it shares with the caller, and the caller
// sees the reset values; all five ways of sharing the store give the same result. A temporary record keeps its rows
// and loses its filter. A cleared RecordRef reads as not open; a cleared FieldRef or KeyRef reads as not initialized.
// Handles the test took before the clear (a Variant, a list and a dictionary Interface value) keep their values.
// When the nested codeunits were never written, reading the cleared Interface raises "Interface not initialized".
// BC versions: 27, 28, 29

table 86270 "CKK Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Integer) { }
        field(2; Txt; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

enum 86271 "CKK Color"
{
    Extensible = false;

    value(0; A) { }
    value(1; B) { }
}

interface "CKK Face"
{
    procedure Get(): Integer;
    procedure Set(V: Integer);
}

codeunit 86272 "CKK Leaf" implements "CKK Face"
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

codeunit 86273 "CKK Mid"
{
    var
        N: Integer;
        Leaf: Codeunit "CKK Leaf";
        TmpRows: Record "CKK Row" temporary;

    procedure Set(V: Integer)
    begin
        N := V;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;

    procedure LeafSet(V: Integer)
    begin
        Leaf.Set(V);
    end;

    procedure LeafGet(): Integer
    begin
        exit(Leaf.Get());
    end;

    procedure ShareLeaf(var L: Codeunit "CKK Leaf")
    begin
        L := Leaf;
    end;

    procedure AddRow(V: Integer)
    begin
        TmpRows.Init();
        TmpRows."No." := V;
        TmpRows.Insert();
    end;

    procedure Rows(): Integer
    begin
        exit(TmpRows.Count());
    end;
}

// The shared instance: one global of each kind.
codeunit 86274 "CKK Store"
{
    var
        I: Integer;
        D: Decimal;
        B: Boolean;
        T: Text;
        T1: Text[1];
        T30: Text[30];
        C: Code[20];
        Dt: Date;
        Tm: Time;
        Dtm: DateTime;
        G: Guid;
        Opt: Option X,Y,Z;
        En: Enum "CKK Color";
        BI: BigInteger;
        Dur: Duration;
        Ch: Char;
        Rec: Record "CKK Row";
        TmpRec: Record "CKK Row" temporary;
        RR: RecordRef;
        RRTmp: RecordRef;
        RRClosed: RecordRef;
        FR: FieldRef;
        KR: KeyRef;
        Mid: Codeunit "CKK Mid";
        Leaf: Codeunit "CKK Leaf";
        Face: Interface "CKK Face";
        V: Variant;
        ArrCU: array[2] of Codeunit "CKK Leaf";
        LFace: List of [Interface "CKK Face"];
        DFace: Dictionary of [Integer, Interface "CKK Face"];
        LI: List of [Integer];
        LT: List of [Text];
        DI: Dictionary of [Integer, Text];
        JO: JsonObject;
        JA: JsonArray;
        ArrI: array[3] of Integer;
        ArrT1: array[2] of Text[1];
        TB: TextBuilder;
        Cnt: Integer;

    procedure Mark()
    var
        LeafF: Codeunit "CKK Leaf";
        LeafV: Codeunit "CKK Leaf";
        LeafL: Codeunit "CKK Leaf";
        LeafD: Codeunit "CKK Leaf";
    begin
        I := 5;
        D := 1.5;
        B := true;
        T := 'long text';
        T1 := 'Q';
        T30 := 'thirty';
        C := 'CODE';
        Dt := DMY2Date(1, 2, 2020);
        Tm := 120000T;
        Dtm := CreateDateTime(DMY2Date(1, 2, 2020), 120000T);
        G := '{0C2B6C5A-9F3E-4A51-8D7B-1E2F3A4B5C6D}';
        Opt := Opt::Z;
        En := En::B;
        BI := 7;
        Dur := 1000;
        Ch := 'A';
        Rec.SetRange("No.", 7);
        Rec."No." := 9;
        Rec.Txt := 'RX';
        TmpRec.Init();
        TmpRec."No." := 1;
        TmpRec.Insert();
        TmpRec.Init();
        TmpRec."No." := 2;
        TmpRec.Insert();
        TmpRec.SetRange("No.", 1);
        RR.Open(Database::"CKK Row");
        RR.Field(1).SetRange(7);
        RRTmp.Open(Database::"CKK Row", true);
        RRTmp.Init();
        RRTmp.Field(1).Value := 1;
        RRTmp.Insert();
        RRTmp.Init();
        RRTmp.Field(1).Value := 2;
        RRTmp.Insert();
        FR := RR.Field(2);
        FR.Value := 'FV';
        KR := RR.KeyIndex(1);
        Mid.Set(5);
        Mid.LeafSet(6);
        Mid.AddRow(1);
        Leaf.Set(4);
        LeafF.Set(7);
        Face := LeafF;
        LeafV.Set(8);
        V := LeafV;
        ArrCU[1].Set(9);
        ArrCU[2].Set(10);
        LeafL.Set(12);
        LFace.Add(LeafL);
        LeafD.Set(13);
        DFace.Add(1, LeafD);
        LI.Add(1);
        LT.Add('a');
        DI.Add(1, 'one');
        JO.Add('k', 1);
        JA.Add(1);
        ArrI[2] := 3;
        ArrT1[1] := 'Z';
        TB.Append('tb');
    end;

    procedure ShareMid(var M: Codeunit "CKK Mid")
    begin
        M := Mid;
    end;

    procedure SetMid(var M: Codeunit "CKK Mid")
    begin
        Mid := M;
    end;

    procedure ShareFace(var F: Interface "CKK Face")
    begin
        F := Face;
    end;

    procedure ShareVariant(var L: Codeunit "CKK Leaf")
    begin
        L := V;
    end;

    procedure ShareArr(var L: Codeunit "CKK Leaf")
    begin
        L := ArrCU[1];
    end;

    // List.Get(Index, var Value) into an Interface var parameter fails the server-side C# compile on BC 27.5
    // (CS1503, NavInterfaceHandle to ByRef), so the value form of Get is used.
    procedure ShareListFace(var F: Interface "CKK Face")
    begin
        F := LFace.Get(1);
    end;

    procedure ShareDictFace(var F: Interface "CKK Face")
    begin
        F := DFace.Get(1);
    end;

    procedure ShareRRTmp(var R: RecordRef)
    begin
        R := RRTmp;
    end;

    procedure SetCnt(NewCnt: Integer)
    begin
        Cnt := NewCnt;
    end;

    procedure GetCnt(): Integer
    begin
        exit(Cnt);
    end;

    procedure MidGet(): Integer
    begin
        exit(Mid.Get());
    end;

    procedure MidLeafGet(): Integer
    begin
        exit(Mid.LeafGet());
    end;

    procedure ArrGet(Idx: Integer): Integer
    begin
        exit(ArrCU[Idx].Get());
    end;

    // Group 1 scalars, 2 records, 3 RecordRef/FieldRef/KeyRef, 4 codeunits and interfaces, 5 collections, JSON, arrays.
    procedure Facts(Group: Integer; var Names: List of [Text]; var Values: List of [Text])
    var
        Tmp2: Record "CKK Row" temporary;
        L: Codeunit "CKK Leaf";
        ChI: Integer;
        DurI: BigInteger;
        Txt: Text;
    begin
        case Group of
            1:
                begin
                    ChI := Ch;
                    DurI := Dur;
                    Add(Names, Values, 'i', Format(I));
                    Add(Names, Values, 'd', Format(D, 0, 9));
                    Add(Names, Values, 'b', Format(B, 0, 9));
                    Add(Names, Values, 't', T);
                    Add(Names, Values, 't1', T1);
                    Add(Names, Values, 't30', T30);
                    Add(Names, Values, 'c', C);
                    Add(Names, Values, 'dt', Format(Dt, 0, 9));
                    Add(Names, Values, 'tm', Format(Tm, 0, 9));
                    Add(Names, Values, 'dtmzero', Format(Dtm = 0DT, 0, 9));
                    Add(Names, Values, 'g', Format(G));
                    Add(Names, Values, 'opt', Format(Opt, 0, 9));
                    Add(Names, Values, 'en', Format(En.AsInteger()));
                    Add(Names, Values, 'bi', Format(BI));
                    Add(Names, Values, 'dur', Format(DurI));
                    Add(Names, Values, 'ch', Format(ChI));
                end;
            2:
                begin
                    Add(Names, Values, 'recf', Rec.GetFilters());
                    Add(Names, Values, 'recno', Format(Rec."No."));
                    Add(Names, Values, 'rectxt', Rec.Txt);
                    Add(Names, Values, 'tmpf', TmpRec.GetFilters());
                    Add(Names, Values, 'tmpcount', Format(TmpRec.Count()));
                    Add(Names, Values, 'tmpcur', Format(TmpRec."No."));
                    Tmp2.Copy(TmpRec, true);
                    Tmp2.Reset();
                    Add(Names, Values, 'tmptotal', Format(Tmp2.Count()));
                end;
            3:
                begin
                    Add(Names, Values, 'rr', Try(1));
                    Add(Names, Values, 'rrf', Try(2));
                    Add(Names, Values, 'rrt', Try(3));
                    Add(Names, Values, 'rrtc', Try(4));
                    Add(Names, Values, 'rrtt', Try(5));
                    Add(Names, Values, 'rrc', Try(6));
                    Add(Names, Values, 'fr', Try(7));
                    Add(Names, Values, 'frn', Try(8));
                    Add(Names, Values, 'kr', Try(9));
                end;
            4:
                begin
                    Add(Names, Values, 'mid', Format(Mid.Get()));
                    Add(Names, Values, 'midleaf', Format(Mid.LeafGet()));
                    Add(Names, Values, 'midrows', Format(Mid.Rows()));
                    Add(Names, Values, 'leaf', Format(Leaf.Get()));
                    Add(Names, Values, 'face', Try(10));
                    Add(Names, Values, 'varcu', Format(V.IsCodeunit(), 0, 9));
                    if V.IsCodeunit() then begin
                        L := V;
                        Add(Names, Values, 'var', Format(L.Get()));
                    end;
                    Add(Names, Values, 'arr1', Format(ArrCU[1].Get()));
                    Add(Names, Values, 'arr2', Format(ArrCU[2].Get()));
                    Add(Names, Values, 'lface', Format(LFace.Count()));
                    Add(Names, Values, 'dface', Format(DFace.Count()));
                end;
            5:
                begin
                    Add(Names, Values, 'li', Format(LI.Count()));
                    Add(Names, Values, 'lt', Format(LT.Count()));
                    Add(Names, Values, 'di', Format(DI.Count()));
                    JO.WriteTo(Txt);
                    Add(Names, Values, 'jo', Txt);
                    JA.WriteTo(Txt);
                    Add(Names, Values, 'ja', Txt);
                    Add(Names, Values, 'arri', Format(ArrI[2]));
                    Add(Names, Values, 'arrt1', ArrT1[1]);
                    Add(Names, Values, 'tb', TB.ToText());
                end;
        end;
    end;

    local procedure Add(var Names: List of [Text]; var Values: List of [Text]; Name: Text; Value: Text)
    begin
        Names.Add(Name);
        Values.Add(Value);
    end;

    local procedure Try(Which: Integer): Text
    var
        R: Text;
    begin
        if TryRead(Which, R) then
            exit(R);
        exit('ERR: ' + GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryRead(Which: Integer; var R: Text)
    begin
        case Which of
            1:
                R := Format(RR.Number());
            2:
                R := RR.GetFilters();
            3:
                R := Format(RRTmp.Number());
            4:
                R := Format(RRTmp.Count());
            5:
                R := Format(RRTmp.IsTemporary(), 0, 9);
            6:
                R := Format(RRClosed.Number());
            7:
                R := Format(FR.Value());
            8:
                R := Format(FR.Number());
            9:
                R := Format(KR.FieldCount());
            10:
                R := Format(Face.Get());
        end;
    end;
}

// The holder: a plain codeunit whose global is the shared store; ClearAll runs in a procedure.
codeunit 86275 "CKK Owner"
{
    var
        Store: Codeunit "CKK Store";
        OwnCnt: Integer;

    procedure SetStore(var NewStore: Codeunit "CKK Store")
    begin
        Store := NewStore;
    end;

    procedure SetStoreByValue(NewStore: Codeunit "CKK Store")
    begin
        Store := NewStore;
    end;

    procedure GetStore(): Codeunit "CKK Store"
    begin
        exit(Store);
    end;

    procedure GetStoreVar(var S: Codeunit "CKK Store")
    begin
        S := Store;
    end;

    procedure MarkStore()
    begin
        Store.Mark();
    end;

    procedure ClearAllGlobals()
    begin
        ClearAll();
    end;

    procedure SetOwnCnt(V: Integer)
    begin
        OwnCnt := V;
    end;

    procedure GetOwnCnt(): Integer
    begin
        exit(OwnCnt);
    end;

    procedure SetStoreCnt(V: Integer)
    begin
        Store.SetCnt(V);
    end;

    procedure StoreCnt(): Integer
    begin
        exit(Store.GetCnt());
    end;

    procedure StoreFacts(Group: Integer; var Names: List of [Text]; var Values: List of [Text])
    begin
        Store.Facts(Group, Names, Values);
    end;
}

// The holder with TableNo: ClearAll in a local procedure that OnRun calls (the Sales-Post shape), reached by Owner.Run(Rec).
codeunit 86276 "CKK TNo Owner"
{
    TableNo = "CKK Row";

    var
        Store: Codeunit "CKK Store";
        OwnCnt: Integer;

    trigger OnRun()
    begin
        RunWithCheck(Rec);
    end;

    local procedure RunWithCheck(var Row: Record "CKK Row")
    begin
        ClearAllVariables();
    end;

    local procedure ClearAllVariables()
    begin
        ClearAll();
    end;

    procedure SetStore(var NewStore: Codeunit "CKK Store")
    begin
        Store := NewStore;
    end;

    procedure SetOwnCnt(V: Integer)
    begin
        OwnCnt := V;
    end;

    procedure GetOwnCnt(): Integer
    begin
        exit(OwnCnt);
    end;

    procedure SetStoreCnt(V: Integer)
    begin
        Store.SetCnt(V);
    end;

    procedure StoreCnt(): Integer
    begin
        exit(Store.GetCnt());
    end;

    procedure StoreFacts(Group: Integer; var Names: List of [Text]; var Values: List of [Text])
    begin
        Store.Facts(Group, Names, Values);
    end;
}

// The caller: holds the store as a global, hands it to an owner (or takes it from one), keeps handles on the store's
// nested codeunits, interfaces and temporary RecordRef, lets the owner run ClearAll, then reads.
codeunit 86277 "CKK Probe"
{
    var
        S: Codeunit "CKK Store";
        Owner: Codeunit "CKK Owner";
        TOwner: Codeunit "CKK TNo Owner";
        M: Codeunit "CKK Mid";
        L3: Codeunit "CKK Leaf";
        VL: Codeunit "CKK Leaf";
        AL: Codeunit "CKK Leaf";
        F: Interface "CKK Face";
        LF: Interface "CKK Face";
        DF: Interface "CKK Face";
        RRx: RecordRef;
        UseTNo: Boolean;
        Marked: Boolean;

    // Mode 1: caller made the store, setter by var. 2: setter by value. 3: owner made it, caller takes the return
    // value of a getter (plain assignment). 4: owner made it, getter by var. 5: setter by var, TableNo owner, ClearAll
    // in a local procedure of OnRun through Owner.Run(Rec). 6: as 1, no Mark: only the handles write the nested
    // codeunits. 7: as 1, the caller makes the nested codeunit and hands it to the store (SetMid by var) before Mark.
    procedure Run(Mode: Integer)
    var
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        Marked := Mode <> 6;
        UseTNo := Mode = 5;
        case Mode of
            1, 2, 5, 6, 7:
                begin
                    if Mode = 7 then begin
                        M.Set(1);
                        S.SetMid(M);
                    end;
                    if Marked then
                        S.Mark();
                    TakeHandles(Mode <> 7);
                    case Mode of
                        2:
                            Owner.SetStoreByValue(S);
                        5:
                            TOwner.SetStore(S);
                        else
                            Owner.SetStore(S);
                    end;
                end;
            3:
                begin
                    Owner.MarkStore();
                    S := Owner.GetStore();
                    TakeHandles(true);
                end;
            4:
                begin
                    Owner.MarkStore();
                    Owner.GetStoreVar(S);
                    TakeHandles(true);
                end;
        end;
        if Mode = 6 then begin
            M.Set(5);
            L3.Set(6);
            AL.Set(9);
        end;
        S.SetCnt(2);
        if UseTNo then begin
            TOwner.SetOwnCnt(3);
            Row."No." := 1;
            TOwner.Run(Row);
        end else begin
            Owner.SetOwnCnt(3);
            Owner.ClearAllGlobals();
        end;
    end;

    local procedure TakeHandles(ShareMid: Boolean)
    begin
        if ShareMid then
            S.ShareMid(M);
        M.ShareLeaf(L3);
        S.ShareArr(AL);
        if not Marked then
            exit;
        S.ShareFace(F);
        S.ShareVariant(VL);
        S.ShareListFace(LF);
        S.ShareDictFace(DF);
        S.ShareRRTmp(RRx);
    end;

    procedure ObserveGroup(Group: Integer; var Observer: Codeunit "Probe Observer")
    var
        Names: List of [Text];
        Values: List of [Text];
        OwnerNames: List of [Text];
        OwnerValues: List of [Text];
        Idx: Integer;
        Same: Boolean;
    begin
        S.Facts(Group, Names, Values);
        for Idx := 1 to Names.Count() do
            Observer.Observe(Names.Get(Idx), Values.Get(Idx));
        if UseTNo then
            TOwner.StoreFacts(Group, OwnerNames, OwnerValues)
        else
            Owner.StoreFacts(Group, OwnerNames, OwnerValues);
        Same := OwnerNames.Count() = Names.Count();
        if Same then
            for Idx := 1 to Names.Count() do
                if OwnerValues.Get(Idx) <> Values.Get(Idx) then
                    Same := false;
        Observer.Observe('ownersame', Format(Same, 0, 9));
        case Group of
            3:
                if Marked then
                    Observer.Observe('hrrx', TryHandleCount());
            4:
                begin
                    Observer.Observe('hm', Format(M.Get()));
                    Observer.Observe('hl3', Format(L3.Get()));
                    Observer.Observe('hmrows', Format(M.Rows()));
                    Observer.Observe('ha', Format(AL.Get()));
                    if Marked then begin
                        Observer.Observe('hf', Format(F.Get()));
                        Observer.Observe('hv', Format(VL.Get()));
                        Observer.Observe('hlf', Format(LF.Get()));
                        Observer.Observe('hdf', Format(DF.Get()));
                    end;
                    M.Set(21);
                    Observer.Observe('wmid', Format(S.MidGet()));
                    L3.Set(22);
                    Observer.Observe('wleaf3', Format(S.MidLeafGet()));
                    AL.Set(23);
                    Observer.Observe('warr1', Format(S.ArrGet(1)));
                end;
        end;
        // Counter: do the caller and the owner still share one instance, and is the owner's own Integer reset?
        Observer.Observe('cnt', Format(S.GetCnt()));
        S.SetCnt(11);
        if UseTNo then begin
            Observer.Observe('wcaller', Format(TOwner.StoreCnt()));
            TOwner.SetStoreCnt(12);
            Observer.Observe('own', Format(TOwner.GetOwnCnt()));
        end else begin
            Observer.Observe('wcaller', Format(Owner.StoreCnt()));
            Owner.SetStoreCnt(12);
            Observer.Observe('own', Format(Owner.GetOwnCnt()));
        end;
        Observer.Observe('wowner', Format(S.GetCnt()));
    end;

    local procedure TryHandleCount(): Text
    var
        R: Text;
    begin
        if TryCount(R) then
            exit(R);
        exit('ERR: ' + GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryCount(var R: Text)
    begin
        R := Format(RRx.Number()) + '/' + Format(RRx.Count());
    end;
}

codeunit 86278 "CKK Kinds Test"
{
    Subtype = Test;

    [Test]
    procedure ClearAll_SetterVar_Scalars()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(1);
        Probe.ObserveGroup(1, Observer);
        Observer.Expect('i', '0');
        Observer.Expect('d', '0');
        Observer.Expect('b', 'false');
        Observer.Expect('t', '');
        Observer.Expect('t1', '');
        Observer.Expect('t30', '');
        Observer.Expect('c', '');
        Observer.Expect('dt', '');
        Observer.Expect('tm', '');
        Observer.Expect('dtmzero', 'true');
        Observer.Expect('g', '{00000000-0000-0000-0000-000000000000}');
        Observer.Expect('opt', '0');
        Observer.Expect('en', '0');
        Observer.Expect('bi', '0');
        Observer.Expect('dur', '0');
        Observer.Expect('ch', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterVar_Records()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(1);
        Probe.ObserveGroup(2, Observer);
        Observer.Expect('recf', '');
        Observer.Expect('recno', '0');
        Observer.Expect('rectxt', '');
        Observer.Expect('tmpf', '');
        Observer.Expect('tmpcount', '2');
        Observer.Expect('tmpcur', '0');
        Observer.Expect('tmptotal', '2');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterVar_Refs()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(1);
        Probe.ObserveGroup(3, Observer);
        Observer.Expect('rr', '0');
        Observer.Expect('rrf', 'ERR: The record is not open.');
        Observer.Expect('rrt', '0');
        Observer.Expect('rrtc', 'ERR: The record is not open.');
        Observer.Expect('rrtt', 'ERR: The record is not open.');
        Observer.Expect('rrc', '0');
        Observer.Expect('fr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('frn', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('kr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hrrx', '86270/2');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterVar_Codeunits()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(1);
        Probe.ObserveGroup(4, Observer);
        Observer.Expect('mid', '0');
        Observer.Expect('midleaf', '0');
        Observer.Expect('midrows', '1');
        Observer.Expect('leaf', '0');
        Observer.Expect('face', '0');
        Observer.Expect('varcu', 'false');
        Observer.Expect('arr1', '0');
        Observer.Expect('arr2', '0');
        Observer.Expect('lface', '0');
        Observer.Expect('dface', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hm', '0');
        Observer.Expect('hl3', '0');
        Observer.Expect('hmrows', '1');
        Observer.Expect('ha', '0');
        Observer.Expect('hf', '0');
        Observer.Expect('hv', '8');
        Observer.Expect('hlf', '12');
        Observer.Expect('hdf', '13');
        Observer.Expect('wmid', '21');
        Observer.Expect('wleaf3', '22');
        Observer.Expect('warr1', '23');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterVar_Collections()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(1);
        Probe.ObserveGroup(5, Observer);
        Observer.Expect('li', '0');
        Observer.Expect('lt', '0');
        Observer.Expect('di', '0');
        Observer.Expect('jo', '{}');
        Observer.Expect('ja', '[]');
        Observer.Expect('arri', '0');
        Observer.Expect('arrt1', '');
        Observer.Expect('tb', '');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterByValue_Scalars()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(2);
        Probe.ObserveGroup(1, Observer);
        Observer.Expect('i', '0');
        Observer.Expect('d', '0');
        Observer.Expect('b', 'false');
        Observer.Expect('t', '');
        Observer.Expect('t1', '');
        Observer.Expect('t30', '');
        Observer.Expect('c', '');
        Observer.Expect('dt', '');
        Observer.Expect('tm', '');
        Observer.Expect('dtmzero', 'true');
        Observer.Expect('g', '{00000000-0000-0000-0000-000000000000}');
        Observer.Expect('opt', '0');
        Observer.Expect('en', '0');
        Observer.Expect('bi', '0');
        Observer.Expect('dur', '0');
        Observer.Expect('ch', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterByValue_Records()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(2);
        Probe.ObserveGroup(2, Observer);
        Observer.Expect('recf', '');
        Observer.Expect('recno', '0');
        Observer.Expect('rectxt', '');
        Observer.Expect('tmpf', '');
        Observer.Expect('tmpcount', '2');
        Observer.Expect('tmpcur', '0');
        Observer.Expect('tmptotal', '2');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterByValue_Refs()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(2);
        Probe.ObserveGroup(3, Observer);
        Observer.Expect('rr', '0');
        Observer.Expect('rrf', 'ERR: The record is not open.');
        Observer.Expect('rrt', '0');
        Observer.Expect('rrtc', 'ERR: The record is not open.');
        Observer.Expect('rrtt', 'ERR: The record is not open.');
        Observer.Expect('rrc', '0');
        Observer.Expect('fr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('frn', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('kr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hrrx', '86270/2');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterByValue_Codeunits()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(2);
        Probe.ObserveGroup(4, Observer);
        Observer.Expect('mid', '0');
        Observer.Expect('midleaf', '0');
        Observer.Expect('midrows', '1');
        Observer.Expect('leaf', '0');
        Observer.Expect('face', '0');
        Observer.Expect('varcu', 'false');
        Observer.Expect('arr1', '0');
        Observer.Expect('arr2', '0');
        Observer.Expect('lface', '0');
        Observer.Expect('dface', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hm', '0');
        Observer.Expect('hl3', '0');
        Observer.Expect('hmrows', '1');
        Observer.Expect('ha', '0');
        Observer.Expect('hf', '0');
        Observer.Expect('hv', '8');
        Observer.Expect('hlf', '12');
        Observer.Expect('hdf', '13');
        Observer.Expect('wmid', '21');
        Observer.Expect('wleaf3', '22');
        Observer.Expect('warr1', '23');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterByValue_Collections()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(2);
        Probe.ObserveGroup(5, Observer);
        Observer.Expect('li', '0');
        Observer.Expect('lt', '0');
        Observer.Expect('di', '0');
        Observer.Expect('jo', '{}');
        Observer.Expect('ja', '[]');
        Observer.Expect('arri', '0');
        Observer.Expect('arrt1', '');
        Observer.Expect('tb', '');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterReturn_Scalars()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(3);
        Probe.ObserveGroup(1, Observer);
        Observer.Expect('i', '0');
        Observer.Expect('d', '0');
        Observer.Expect('b', 'false');
        Observer.Expect('t', '');
        Observer.Expect('t1', '');
        Observer.Expect('t30', '');
        Observer.Expect('c', '');
        Observer.Expect('dt', '');
        Observer.Expect('tm', '');
        Observer.Expect('dtmzero', 'true');
        Observer.Expect('g', '{00000000-0000-0000-0000-000000000000}');
        Observer.Expect('opt', '0');
        Observer.Expect('en', '0');
        Observer.Expect('bi', '0');
        Observer.Expect('dur', '0');
        Observer.Expect('ch', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterReturn_Records()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(3);
        Probe.ObserveGroup(2, Observer);
        Observer.Expect('recf', '');
        Observer.Expect('recno', '0');
        Observer.Expect('rectxt', '');
        Observer.Expect('tmpf', '');
        Observer.Expect('tmpcount', '2');
        Observer.Expect('tmpcur', '0');
        Observer.Expect('tmptotal', '2');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterReturn_Refs()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(3);
        Probe.ObserveGroup(3, Observer);
        Observer.Expect('rr', '0');
        Observer.Expect('rrf', 'ERR: The record is not open.');
        Observer.Expect('rrt', '0');
        Observer.Expect('rrtc', 'ERR: The record is not open.');
        Observer.Expect('rrtt', 'ERR: The record is not open.');
        Observer.Expect('rrc', '0');
        Observer.Expect('fr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('frn', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('kr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hrrx', '86270/2');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterReturn_Codeunits()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(3);
        Probe.ObserveGroup(4, Observer);
        Observer.Expect('mid', '0');
        Observer.Expect('midleaf', '0');
        Observer.Expect('midrows', '1');
        Observer.Expect('leaf', '0');
        Observer.Expect('face', '0');
        Observer.Expect('varcu', 'false');
        Observer.Expect('arr1', '0');
        Observer.Expect('arr2', '0');
        Observer.Expect('lface', '0');
        Observer.Expect('dface', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hm', '0');
        Observer.Expect('hl3', '0');
        Observer.Expect('hmrows', '1');
        Observer.Expect('ha', '0');
        Observer.Expect('hf', '0');
        Observer.Expect('hv', '8');
        Observer.Expect('hlf', '12');
        Observer.Expect('hdf', '13');
        Observer.Expect('wmid', '21');
        Observer.Expect('wleaf3', '22');
        Observer.Expect('warr1', '23');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterReturn_Collections()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(3);
        Probe.ObserveGroup(5, Observer);
        Observer.Expect('li', '0');
        Observer.Expect('lt', '0');
        Observer.Expect('di', '0');
        Observer.Expect('jo', '{}');
        Observer.Expect('ja', '[]');
        Observer.Expect('arri', '0');
        Observer.Expect('arrt1', '');
        Observer.Expect('tb', '');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterVar_Scalars()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(4);
        Probe.ObserveGroup(1, Observer);
        Observer.Expect('i', '0');
        Observer.Expect('d', '0');
        Observer.Expect('b', 'false');
        Observer.Expect('t', '');
        Observer.Expect('t1', '');
        Observer.Expect('t30', '');
        Observer.Expect('c', '');
        Observer.Expect('dt', '');
        Observer.Expect('tm', '');
        Observer.Expect('dtmzero', 'true');
        Observer.Expect('g', '{00000000-0000-0000-0000-000000000000}');
        Observer.Expect('opt', '0');
        Observer.Expect('en', '0');
        Observer.Expect('bi', '0');
        Observer.Expect('dur', '0');
        Observer.Expect('ch', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterVar_Records()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(4);
        Probe.ObserveGroup(2, Observer);
        Observer.Expect('recf', '');
        Observer.Expect('recno', '0');
        Observer.Expect('rectxt', '');
        Observer.Expect('tmpf', '');
        Observer.Expect('tmpcount', '2');
        Observer.Expect('tmpcur', '0');
        Observer.Expect('tmptotal', '2');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterVar_Refs()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(4);
        Probe.ObserveGroup(3, Observer);
        Observer.Expect('rr', '0');
        Observer.Expect('rrf', 'ERR: The record is not open.');
        Observer.Expect('rrt', '0');
        Observer.Expect('rrtc', 'ERR: The record is not open.');
        Observer.Expect('rrtt', 'ERR: The record is not open.');
        Observer.Expect('rrc', '0');
        Observer.Expect('fr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('frn', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('kr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hrrx', '86270/2');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterVar_Codeunits()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(4);
        Probe.ObserveGroup(4, Observer);
        Observer.Expect('mid', '0');
        Observer.Expect('midleaf', '0');
        Observer.Expect('midrows', '1');
        Observer.Expect('leaf', '0');
        Observer.Expect('face', '0');
        Observer.Expect('varcu', 'false');
        Observer.Expect('arr1', '0');
        Observer.Expect('arr2', '0');
        Observer.Expect('lface', '0');
        Observer.Expect('dface', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hm', '0');
        Observer.Expect('hl3', '0');
        Observer.Expect('hmrows', '1');
        Observer.Expect('ha', '0');
        Observer.Expect('hf', '0');
        Observer.Expect('hv', '8');
        Observer.Expect('hlf', '12');
        Observer.Expect('hdf', '13');
        Observer.Expect('wmid', '21');
        Observer.Expect('wleaf3', '22');
        Observer.Expect('warr1', '23');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_OwnerMadeGetterVar_Collections()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(4);
        Probe.ObserveGroup(5, Observer);
        Observer.Expect('li', '0');
        Observer.Expect('lt', '0');
        Observer.Expect('di', '0');
        Observer.Expect('jo', '{}');
        Observer.Expect('ja', '[]');
        Observer.Expect('arri', '0');
        Observer.Expect('arrt1', '');
        Observer.Expect('tb', '');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_TableNoOwnerRun_Scalars()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(5);
        Probe.ObserveGroup(1, Observer);
        Observer.Expect('i', '0');
        Observer.Expect('d', '0');
        Observer.Expect('b', 'false');
        Observer.Expect('t', '');
        Observer.Expect('t1', '');
        Observer.Expect('t30', '');
        Observer.Expect('c', '');
        Observer.Expect('dt', '');
        Observer.Expect('tm', '');
        Observer.Expect('dtmzero', 'true');
        Observer.Expect('g', '{00000000-0000-0000-0000-000000000000}');
        Observer.Expect('opt', '0');
        Observer.Expect('en', '0');
        Observer.Expect('bi', '0');
        Observer.Expect('dur', '0');
        Observer.Expect('ch', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_TableNoOwnerRun_Records()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(5);
        Probe.ObserveGroup(2, Observer);
        Observer.Expect('recf', '');
        Observer.Expect('recno', '0');
        Observer.Expect('rectxt', '');
        Observer.Expect('tmpf', '');
        Observer.Expect('tmpcount', '2');
        Observer.Expect('tmpcur', '0');
        Observer.Expect('tmptotal', '2');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_TableNoOwnerRun_Refs()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(5);
        Probe.ObserveGroup(3, Observer);
        Observer.Expect('rr', '0');
        Observer.Expect('rrf', 'ERR: The record is not open.');
        Observer.Expect('rrt', '0');
        Observer.Expect('rrtc', 'ERR: The record is not open.');
        Observer.Expect('rrtt', 'ERR: The record is not open.');
        Observer.Expect('rrc', '0');
        Observer.Expect('fr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('frn', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.');
        Observer.Expect('kr', 'ERR: Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hrrx', '86270/2');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_TableNoOwnerRun_Codeunits()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(5);
        Probe.ObserveGroup(4, Observer);
        Observer.Expect('mid', '0');
        Observer.Expect('midleaf', '0');
        Observer.Expect('midrows', '1');
        Observer.Expect('leaf', '0');
        Observer.Expect('face', '0');
        Observer.Expect('varcu', 'false');
        Observer.Expect('arr1', '0');
        Observer.Expect('arr2', '0');
        Observer.Expect('lface', '0');
        Observer.Expect('dface', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hm', '0');
        Observer.Expect('hl3', '0');
        Observer.Expect('hmrows', '1');
        Observer.Expect('ha', '0');
        Observer.Expect('hf', '0');
        Observer.Expect('hv', '8');
        Observer.Expect('hlf', '12');
        Observer.Expect('hdf', '13');
        Observer.Expect('wmid', '21');
        Observer.Expect('wleaf3', '22');
        Observer.Expect('warr1', '23');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_TableNoOwnerRun_Collections()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(5);
        Probe.ObserveGroup(5, Observer);
        Observer.Expect('li', '0');
        Observer.Expect('lt', '0');
        Observer.Expect('di', '0');
        Observer.Expect('jo', '{}');
        Observer.Expect('ja', '[]');
        Observer.Expect('arri', '0');
        Observer.Expect('arrt1', '');
        Observer.Expect('tb', '');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_SetterVarHandlesOnly_Codeunits()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(6);
        Probe.ObserveGroup(4, Observer);
        Observer.Expect('mid', '0');
        Observer.Expect('midleaf', '0');
        Observer.Expect('midrows', '0');
        Observer.Expect('leaf', '0');
        Observer.Expect('face', 'ERR: Interface not initialized');
        Observer.Expect('varcu', 'false');
        Observer.Expect('arr1', '0');
        Observer.Expect('arr2', '0');
        Observer.Expect('lface', '0');
        Observer.Expect('dface', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hm', '0');
        Observer.Expect('hl3', '0');
        Observer.Expect('hmrows', '0');
        Observer.Expect('ha', '0');
        Observer.Expect('wmid', '21');
        Observer.Expect('wleaf3', '22');
        Observer.Expect('warr1', '23');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;

    [Test]
    procedure ClearAll_CallerHandsNested_Codeunits()
    var
        Probe: Codeunit "CKK Probe";
        Observer: Codeunit "Probe Observer";
    begin
        Probe.Run(7);
        Probe.ObserveGroup(4, Observer);
        Observer.Expect('mid', '0');
        Observer.Expect('midleaf', '0');
        Observer.Expect('midrows', '1');
        Observer.Expect('leaf', '0');
        Observer.Expect('face', '0');
        Observer.Expect('varcu', 'false');
        Observer.Expect('arr1', '0');
        Observer.Expect('arr2', '0');
        Observer.Expect('lface', '0');
        Observer.Expect('dface', '0');
        Observer.Expect('ownersame', 'true');
        Observer.Expect('hm', '0');
        Observer.Expect('hl3', '0');
        Observer.Expect('hmrows', '1');
        Observer.Expect('ha', '0');
        Observer.Expect('hf', '0');
        Observer.Expect('hv', '8');
        Observer.Expect('hlf', '12');
        Observer.Expect('hdf', '13');
        Observer.Expect('wmid', '21');
        Observer.Expect('wleaf3', '22');
        Observer.Expect('warr1', '23');
        Observer.Expect('cnt', '0');
        Observer.Expect('wcaller', '11');
        Observer.Expect('own', '0');
        Observer.Expect('wowner', '12');
        Observer.Verify();
    end;
}
