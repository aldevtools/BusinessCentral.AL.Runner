// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row", "CKK Color" of ClearSharedCodeunitGlobalKinds.al, "CKW UOM" of ClearSharedCodeunitWhseRegister.al; self-contained objects 86288 to 86292
// Probe: written against BC 27.5, 28.5 and 29.0 (al-runner issue 8964), observed first, then asserted.
// Facts: ClearAll in the Reservation Management shape resets the shared Late Binding store and its nested store
// (Text, RecordRef, array, Option, Decimal, Integer); temporary records keep their rows. Caller and owner keep using
// one instance after the clear.
// BC versions: 27, 28, 29

// "Reservation Management" shape as a nested store: records, a temporary record, a RecordRef, an Enum, an Option,
// a Text[1], an Integer, an array[30] of Integer, a Decimal, a Boolean and a nested codeunit.
codeunit 86288 "CKL Reserv"
{
    var
        CalcReservEntry: Record "CKK Row";
        Item: Record "CKK Row";
        TempTrackingSpec: Record "CKK Row" temporary;
        SourceRecRef: RecordRef;
        RefOrderType: Enum "CKK Color";
        Handling: Option "None","Allow deletion",Match;
        InvSearch: Text[1];
        InvNextStep: Integer;
        ValueArray: array[30] of Integer;
        TotalAvailQty: Decimal;
        Positive: Boolean;
        UOM: Codeunit "CKW UOM";

    procedure Prime()
    begin
        CalcReservEntry.SetRange("No.", 3);
        Item."No." := 4;
        TempTrackingSpec.Init();
        TempTrackingSpec."No." := 1;
        TempTrackingSpec.Insert();
        SourceRecRef.Open(Database::"CKK Row");
        RefOrderType := RefOrderType::B;
        Handling := Handling::Match;
        InvSearch := '-';
        InvNextStep := 1;
        ValueArray[30] := 7;
        TotalAvailQty := 2.5;
        Positive := true;
        UOM.Prime();
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('calc=%1 item=%2 temp=%3 src=%4 ref=%5 hdl=%6 inv=%7|%8 arr=%9',
            CalcReservEntry.GetFilters(), Item."No.", TempTrackingSpec.Count(), SourceRecRef.Number(),
            RefOrderType.AsInteger(), Format(Handling, 0, 9), InvSearch, InvNextStep, ValueArray[30]) +
            StrSubstNo(' qty=%1 pos=%2 %3', Format(TotalAvailQty, 0, 9), Format(Positive, 0, 9), UOM.State()));
    end;
}

// "Late Binding Management" shape: seven temporary records, a nested codeunit, an Integer.
codeunit 86289 "CKL Late Binding"
{
    var
        TempCurrSupply: Record "CKK Row" temporary;
        TempCurrDemand: Record "CKK Row" temporary;
        TempSupply: Record "CKK Row" temporary;
        TempDelete: Record "CKK Row" temporary;
        TempModify: Record "CKK Row" temporary;
        TempInsert: Record "CKK Row" temporary;
        TempSurplus: Record "CKK Row" temporary;
        ReservMgt: Codeunit "CKL Reserv";
        LastEntryNo: Integer;

    procedure Prime()
    begin
        AddRow(TempCurrSupply, 1);
        AddRow(TempCurrDemand, 1);
        AddRow(TempSupply, 1);
        AddRow(TempSupply, 2);
        AddRow(TempDelete, 1);
        AddRow(TempModify, 1);
        AddRow(TempInsert, 1);
        AddRow(TempSurplus, 1);
        TempSurplus.SetRange("No.", 1);
        ReservMgt.Prime();
        LastEntryNo := 9;
    end;

    procedure AddSurplus(V: Integer)
    begin
        AddRow(TempSurplus, V);
        LastEntryNo := V;
    end;

    local procedure AddRow(var R: Record "CKK Row" temporary; V: Integer)
    begin
        R.Init();
        R."No." := V;
        R.Insert();
    end;

    procedure ShareReserv(var R: Codeunit "CKL Reserv")
    begin
        R := ReservMgt;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1/%2/%3/%4/%5/%6/%7 surplusf=%8 last=%9',
            TempCurrSupply.Count(), TempCurrDemand.Count(), TempSupply.Count(), TempDelete.Count(), TempModify.Count(),
            TempInsert.Count(), TempSurplus.Count(), TempSurplus.GetFilters(), LastEntryNo));
    end;

    procedure ReservState(): Text
    begin
        exit(ReservMgt.State());
    end;
}

// "Reservation Management" as the owner: the late binding store is a global; SetReservSource runs ClearAll, then sets
// the source RecordRef.
codeunit 86290 "CKL Reserv Owner"
{
    var
        LateBindingMgt: Codeunit "CKL Late Binding";
        SourceRecRef: RecordRef;
        InvSearch: Text[1];
        ValueArray: array[30] of Integer;

    procedure SetLateBinding(var LB: Codeunit "CKL Late Binding")
    begin
        LateBindingMgt := LB;
    end;

    procedure GetLateBinding(var LB: Codeunit "CKL Late Binding")
    begin
        LB := LateBindingMgt;
    end;

    procedure PrimeLateBinding()
    begin
        LateBindingMgt.Prime();
        InvSearch := '+';
        ValueArray[1] := 4;
    end;

    procedure SetReservSource(NewSourceRecRef: RecordRef)
    begin
        ClearAll();
        SourceRecRef := NewSourceRecRef;
    end;

    procedure AddSurplus(V: Integer)
    begin
        LateBindingMgt.AddSurplus(V);
    end;

    procedure LateState(): Text
    begin
        exit(LateBindingMgt.State());
    end;

    procedure ObserveOwner(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.lb', LateBindingMgt.State());
        Observer.Observe('owner.reserv', LateBindingMgt.ReservState());
        Observer.Observe('owner.own', StrSubstNo('src=%1 inv=%2 arr=%3', SourceRecRef.Number(), InvSearch, ValueArray[1]));
    end;
}

codeunit 86291 "CKL Caller"
{
    var
        LB: Codeunit "CKL Late Binding";
        Owner: Codeunit "CKL Reserv Owner";
        ReservHandle: Codeunit "CKL Reserv";

    // Mode 1: caller made and primed the store, setter by var. 2: owner made and primed it, caller takes it by a var
    // getter. 3: as 1, the caller also holds the store's nested codeunit.
    procedure Run(Mode: Integer; var Observer: Codeunit "Probe Observer")
    var
        Row: Record "CKK Row";
        RR: RecordRef;
    begin
        Row.DeleteAll();
        if Mode = 2 then begin
            Owner.PrimeLateBinding();
            Owner.GetLateBinding(LB);
        end else begin
            LB.Prime();
            Owner.SetLateBinding(LB);
        end;
        if Mode = 3 then
            LB.ShareReserv(ReservHandle);
        RR.Open(Database::"CKK Row");
        Owner.SetReservSource(RR);
        Observer.Observe('caller.lb', LB.State());
        Observer.Observe('caller.reserv', LB.ReservState());
        if Mode = 3 then
            Observer.Observe('handle.reserv', ReservHandle.State());
        Owner.ObserveOwner(Observer);
        LB.AddSurplus(21);
        Observer.Observe('wcaller', Owner.LateState());
        Owner.AddSurplus(22);
        Observer.Observe('wowner', LB.State());
    end;
}

codeunit 86292 "CKL Late Binding Test"
{
    Subtype = Test;

    [Test]
    procedure LateBinding_CallerMade_SetterVar()
    var
        Caller: Codeunit "CKL Caller";
        Observer: Codeunit "Probe Observer";
    begin
        Caller.Run(1, Observer);
        Observer.Expect('caller.lb', 'rows=1/1/2/1/1/1/1 surplusf= last=0');
        Observer.Expect('caller.reserv', 'calc= item=0 temp=1 src=0 ref=0 hdl=0 inv=|0 arr=0 qty=0 pos=false uom=0|');
        Observer.Expect('owner.lb', 'rows=1/1/2/1/1/1/1 surplusf= last=0');
        Observer.Expect('owner.reserv', 'calc= item=0 temp=1 src=0 ref=0 hdl=0 inv=|0 arr=0 qty=0 pos=false uom=0|');
        Observer.Expect('owner.own', 'src=86270 inv= arr=0');
        Observer.Expect('wcaller', 'rows=1/1/2/1/1/1/2 surplusf= last=21');
        Observer.Expect('wowner', 'rows=1/1/2/1/1/1/3 surplusf= last=22');
        Observer.Verify();
    end;

    [Test]
    procedure LateBinding_OwnerMade_GetterVar()
    var
        Caller: Codeunit "CKL Caller";
        Observer: Codeunit "Probe Observer";
    begin
        Caller.Run(2, Observer);
        Observer.Expect('caller.lb', 'rows=1/1/2/1/1/1/1 surplusf= last=0');
        Observer.Expect('caller.reserv', 'calc= item=0 temp=1 src=0 ref=0 hdl=0 inv=|0 arr=0 qty=0 pos=false uom=0|');
        Observer.Expect('owner.lb', 'rows=1/1/2/1/1/1/1 surplusf= last=0');
        Observer.Expect('owner.reserv', 'calc= item=0 temp=1 src=0 ref=0 hdl=0 inv=|0 arr=0 qty=0 pos=false uom=0|');
        Observer.Expect('owner.own', 'src=86270 inv= arr=0');
        Observer.Expect('wcaller', 'rows=1/1/2/1/1/1/2 surplusf= last=21');
        Observer.Expect('wowner', 'rows=1/1/2/1/1/1/3 surplusf= last=22');
        Observer.Verify();
    end;

    [Test]
    procedure LateBinding_CallerMade_NestedHandle()
    var
        Caller: Codeunit "CKL Caller";
        Observer: Codeunit "Probe Observer";
    begin
        Caller.Run(3, Observer);
        Observer.Expect('caller.lb', 'rows=1/1/2/1/1/1/1 surplusf= last=0');
        Observer.Expect('caller.reserv', 'calc= item=0 temp=1 src=0 ref=0 hdl=0 inv=|0 arr=0 qty=0 pos=false uom=0|');
        Observer.Expect('handle.reserv', 'calc= item=0 temp=1 src=0 ref=0 hdl=0 inv=|0 arr=0 qty=0 pos=false uom=0|');
        Observer.Expect('owner.lb', 'rows=1/1/2/1/1/1/1 surplusf= last=0');
        Observer.Expect('owner.reserv', 'calc= item=0 temp=1 src=0 ref=0 hdl=0 inv=|0 arr=0 qty=0 pos=false uom=0|');
        Observer.Expect('owner.own', 'src=86270 inv= arr=0');
        Observer.Expect('wcaller', 'rows=1/1/2/1/1/1/2 surplusf= last=21');
        Observer.Expect('wowner', 'rows=1/1/2/1/1/1/3 surplusf= last=22');
        Observer.Verify();
    end;
}
