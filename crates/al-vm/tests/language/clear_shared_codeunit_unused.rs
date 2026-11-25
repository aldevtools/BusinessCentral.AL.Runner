//! `ClearAll()` in a plain owner of a store global that another variable shares while no call has made the instance yet
//! (al-runner issue 8912). Open corpus pull request 1564 measures that shape for the one-Integer store
//! (ClearSharedCodeunit.al ClearAll_GlobalUnusedIface), "CLB Store" (ClearSharedCodeunitTempRef.al
//! ClearAll_GlobalUnusedIface :382), the array store (ClearSharedCodeunitOwnerKinds.al ArrStore_ClearAll_UnusedAlias :418)
//! and the store with one more Integer (StorePlus_UnusedAlias :684; ClearSharedCodeunitPlusUncalled.al, "CLQ Store Plus",
//! which implements no interface): one instance, run flagged. A store with one more Integer that implements an interface
//! refuses with no call: no cell shares it through an Interface, List or Dictionary slot before a call. For the wrapper,
//! the 2-D store, the register and the price store every cell that runs made the instance with a call that wrote a scalar
//! first (ClearSharedCodeunitHolderKinds.al Price_VarParam_Never :1323 and the other `_Never` cells), so the shapes with
//! no call (measured since corpus PR 1564 commit b77f20dc, not opened yet) and with calls that wrote nothing refuse.

use crate::clear_shared_codeunit::{assert_refused, run_cell};

const SOURCE: &str = "
table 50940 URow { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
enum 50941 UKind { value(0; None) { } value(1; Beta) { } }
codeunit 50942 UInt { var N: Integer; procedure Set(V: Integer) begin N := V; end; procedure Touch() begin end; }
codeunit 50943 UClb { var Rows: Record URow temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer;
    procedure Set(V: Integer) begin Ints[1] := V; end; procedure Touch() begin end; }
codeunit 50944 UPlus { var Rows: Record URow temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer; N: Integer;
    procedure Set(V: Integer) begin N := V; end; procedure Touch() begin end; }
codeunit 50945 UWrap { var Inner: Codeunit UClb; N: Integer; procedure Set(V: Integer) begin N := V; end; procedure Touch() begin end; }
codeunit 50946 UArr { var Rows: array[2] of Record URow temporary; Refs: array[2] of RecordRef; Ints: array[3] of Integer;
    procedure Set(V: Integer) begin Ints[1] := V; end; procedure Touch() begin end; }
codeunit 50947 UTwoDim { var Rows: array[2, 2] of Record URow temporary; Ints: array[2] of Integer;
    procedure Set(V: Integer) begin Ints[1] := V; end; procedure Touch() begin end; }
codeunit 50948 UReg { TableNo = URow;
    var Rows: Record URow temporary; Rec: Record URow; Dec: Decimal; Flag: Boolean; Cd: Code[20]; Nested: Codeunit UInt;
    procedure Set(V: Integer) begin Dec := V; end; procedure Touch() begin end;
    procedure ShareRows(var T: Record URow temporary) begin T.Copy(Rows, true); end; }
codeunit 50949 UPrice { var T1: Record URow temporary; Kind: Enum UKind; Int: Integer; LblC: Label 'store label', Comment = 'c';
    procedure Set(V: Integer) begin Int := V; end; procedure Touch() begin end; }
interface UFace { procedure Set(V: Integer); }
codeunit 50952 UPlusFace implements UFace { var Rows: Record URow temporary; Refs: array[2, 3] of RecordRef;
    Ints: array[3] of Integer; N: Integer; procedure Set(V: Integer) begin N := V; end; }
codeunit 50953 UFaceOwner { var P: Codeunit UPlusFace;
    procedure SharePlus(var Y: Codeunit UPlusFace) begin Y := P; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50950 UOwner {
    var I: Codeunit UInt; C: Codeunit UClb; P: Codeunit UPlus; W: Codeunit UWrap; A: Codeunit UArr; D: Codeunit UTwoDim;
        R: Codeunit UReg; Pr: Codeunit UPrice;
    procedure ShareInt(var Y: Codeunit UInt) begin Y := I; end;
    procedure ShareClb(var Y: Codeunit UClb) begin Y := C; end;
    procedure SharePlus(var Y: Codeunit UPlus) begin Y := P; end;
    procedure ShareWrap(var Y: Codeunit UWrap) begin Y := W; end;
    procedure ShareArr(var Y: Codeunit UArr) begin Y := A; end;
    procedure ShareTwoDim(var Y: Codeunit UTwoDim) begin Y := D; end;
    procedure SetReg(var Y: Codeunit UReg) begin R := Y; end;
    procedure SharePrice(var Y: Codeunit UPrice) begin Y := Pr; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50951 T {
    procedure Int(Mode: Integer): Text var O: Codeunit UOwner; Y: Codeunit UInt;
    begin O.ShareInt(Y); if Mode = 1 then Y.Set(3); if Mode = 2 then Y.Touch(); O.ClearAllGlobals(); exit('done'); end;
    procedure Clb(Mode: Integer): Text var O: Codeunit UOwner; Y: Codeunit UClb;
    begin O.ShareClb(Y); if Mode = 1 then Y.Set(3); if Mode = 2 then Y.Touch(); O.ClearAllGlobals(); exit('done'); end;
    procedure Plus(Mode: Integer): Text var O: Codeunit UOwner; Y: Codeunit UPlus;
    begin O.SharePlus(Y); if Mode = 1 then Y.Set(3); if Mode = 2 then Y.Touch(); O.ClearAllGlobals(); exit('done'); end;
    procedure Wrap(Mode: Integer): Text var O: Codeunit UOwner; Y: Codeunit UWrap;
    begin O.ShareWrap(Y); if Mode = 1 then Y.Set(3); if Mode = 2 then Y.Touch(); O.ClearAllGlobals(); exit('done'); end;
    procedure Arr(Mode: Integer): Text var O: Codeunit UOwner; Y: Codeunit UArr;
    begin O.ShareArr(Y); if Mode = 1 then Y.Set(3); if Mode = 2 then Y.Touch(); O.ClearAllGlobals(); exit('done'); end;
    procedure TwoDim(Mode: Integer): Text var O: Codeunit UOwner; Y: Codeunit UTwoDim;
    begin O.ShareTwoDim(Y); if Mode = 1 then Y.Set(3); if Mode = 2 then Y.Touch(); O.ClearAllGlobals(); exit('done'); end;
    procedure Reg(Mode: Integer): Text var O: Codeunit UOwner; Y: Codeunit UReg; T: Record URow temporary;
    begin O.SetReg(Y); if Mode = 4 then Y.ShareRows(T); if Mode = 1 then Y.Set(3); if Mode = 2 then Y.Touch(); O.ClearAllGlobals(); exit('done'); end;
    procedure Price(Mode: Integer): Text var O: Codeunit UOwner; Y: Codeunit UPrice;
    begin O.SharePrice(Y); if Mode = 1 then Y.Set(3); if Mode = 2 then Y.Touch(); O.ClearAllGlobals(); exit('done'); end;
    procedure PlusFace(Mode: Integer): Text var O: Codeunit UFaceOwner; Y: Codeunit UPlusFace; I: Interface UFace;
    begin O.SharePlus(Y); if Mode = 3 then I := Y; O.ClearAllGlobals(); exit('done'); end;
}
";

/// No call on the store before `ClearAll`.
const NONE: i32 = 0;
/// A call that writes a scalar of the store.
const WRITES: i32 = 1;
/// A call that writes nothing.
const WRITES_NOTHING: i32 = 2;
/// A call that only shares the register's table with a record of the test (`Copy(Rows, true)`): no row, no scalar.
const SHARES_ROWS: i32 = 4;
/// No call; an Interface variable shares the store too.
const INTERFACE: i32 = 3;

/// What `name` of [`SOURCE`] gives with argument `mode` and whether it ran a path of `clear-shared-codeunit`
/// ([`run_cell`]).
fn cell(name: &str, mode: i32) -> (String, bool) {
    run_cell(SOURCE, name, vec![al_builtins::value::Value::Integer(mode)])
}

/// The stores whose shared instance no call made is measured, and the stores where every shared cell calls first.
const MEASURED_UNCALLED: [&str; 4] = ["Int", "Clb", "Arr", "Plus"];
const UNMEASURED_UNCALLED: [&str; 4] = ["Wrap", "TwoDim", "Reg", "Price"];

// A call made the shared instance (a scalar written, no row): one instance, flagged, for every store.
#[test]
fn clear_all_of_a_shared_store_a_call_made_runs() {
    for name in MEASURED_UNCALLED.into_iter().chain(UNMEASURED_UNCALLED) {
        assert_eq!(cell(name, WRITES), ("done".to_string(), true), "{name}");
    }
}

// No call made the shared instance: measured for the one-Integer store, "CLB Store", the array store and the store with
// one more Integer only.
#[test]
fn clear_all_of_a_shared_store_no_call_made_runs_where_measured() {
    for name in MEASURED_UNCALLED {
        assert_eq!(cell(name, NONE), ("done".to_string(), true), "{name}");
    }
    for name in UNMEASURED_UNCALLED {
        let why = "ClearAll of a global codeunit variable whose instance another variable shares";
        assert_refused(name, &cell(name, NONE).0, why);
    }
}

// A call that wrote nothing made the shared instance: every cell of the wrapper, the 2-D store, the register and the price
// store writes a scalar first, so this refuses. The four stores measured with no call keep running.
#[test]
fn clear_all_of_a_shared_store_a_call_that_wrote_nothing_made() {
    for name in MEASURED_UNCALLED {
        assert_eq!(cell(name, WRITES_NOTHING), ("done".to_string(), true), "{name}");
    }
    for name in UNMEASURED_UNCALLED {
        let why = "ClearAll of a global codeunit variable whose instance another variable shares";
        assert_refused(name, &cell(name, WRITES_NOTHING).0, why);
    }
}

// The store with one more Integer, no call made: measured only for a codeunit that implements no interface, so no
// Interface, List or Dictionary slot shares it. One that implements an interface refuses, with or without an Interface
// variable sharing it.
#[test]
fn clear_all_of_an_uncalled_store_that_implements_an_interface_refuses() {
    let why = "ClearAll of a global codeunit variable whose instance another variable shares";
    for mode in [NONE, INTERFACE] {
        assert_refused("PlusFace", &cell("PlusFace", mode).0, why);
    }
}

// The register's table shared with a record of the test through `Copy(Rows, true)`, and no call wrote a row or a scalar:
// every register cell of corpus PR 1564 calls `MarkReg()` first (ClearSharedCodeunitSetterOwner.al
// TableNoRun_VarSetter_SharedNever :437), so this refuses.
#[test]
fn clear_all_of_a_shared_register_whose_table_is_shared_and_no_call_wrote_refuses() {
    let (answer, _) = cell("Reg", SHARES_ROWS);
    assert_refused(
        "Reg",
        &answer,
        "ClearAll of a global variable that reaches a temporary table that another record shares",
    );
}
