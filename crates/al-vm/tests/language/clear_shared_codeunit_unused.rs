//! `ClearAll()` in a plain owner of a store global that another variable shares while no call has made the instance yet
//! (al-runner issue 8912). Open corpus pull request 1564 measures that shape for the one-Integer store
//! (ClearSharedCodeunit.al ClearAll_GlobalUnusedIface), "CLB Store" (ClearSharedCodeunitTempRef.al
//! ClearAll_GlobalUnusedIface :382), the array store (ClearSharedCodeunitOwnerKinds.al ArrStore_ClearAll_UnusedAlias :418)
//! and the store with one more Integer (StorePlus_UnusedAlias :684): one instance, run flagged. For the wrapper, the 2-D
//! store, the register and the price store every shared cell made the instance with a call first
//! (ClearSharedCodeunitHolderKinds.al Price_VarParam_Never :1323 and the other `_Never` cells), so the shape with no call
//! refuses and the shape with a call that wrote no row runs flagged.

use crate::clear_shared_codeunit::{assert_refused, run_cell};

const SOURCE: &str = "
table 50940 URow { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
enum 50941 UKind { value(0; None) { } value(1; Beta) { } }
codeunit 50942 UInt { var N: Integer; procedure Set(V: Integer) begin N := V; end; }
codeunit 50943 UClb { var Rows: Record URow temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer;
    procedure Set(V: Integer) begin Ints[1] := V; end; }
codeunit 50944 UPlus { var Rows: Record URow temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer; N: Integer;
    procedure Set(V: Integer) begin N := V; end; }
codeunit 50945 UWrap { var Inner: Codeunit UClb; N: Integer; procedure Set(V: Integer) begin N := V; end; }
codeunit 50946 UArr { var Rows: array[2] of Record URow temporary; Refs: array[2] of RecordRef; Ints: array[3] of Integer;
    procedure Set(V: Integer) begin Ints[1] := V; end; }
codeunit 50947 UTwoDim { var Rows: array[2, 2] of Record URow temporary; Ints: array[2] of Integer;
    procedure Set(V: Integer) begin Ints[1] := V; end; }
codeunit 50948 UReg { TableNo = URow;
    var Rows: Record URow temporary; Rec: Record URow; Dec: Decimal; Flag: Boolean; Cd: Code[20]; Nested: Codeunit UInt;
    procedure Set(V: Integer) begin Dec := V; end; }
codeunit 50949 UPrice { var T1: Record URow temporary; Kind: Enum UKind; Int: Integer; LblC: Label 'store label', Comment = 'c';
    procedure Set(V: Integer) begin Int := V; end; }
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
    procedure Int(Call: Boolean): Text var O: Codeunit UOwner; Y: Codeunit UInt;
    begin O.ShareInt(Y); if Call then Y.Set(3); O.ClearAllGlobals(); exit('done'); end;
    procedure Clb(Call: Boolean): Text var O: Codeunit UOwner; Y: Codeunit UClb;
    begin O.ShareClb(Y); if Call then Y.Set(3); O.ClearAllGlobals(); exit('done'); end;
    procedure Plus(Call: Boolean): Text var O: Codeunit UOwner; Y: Codeunit UPlus;
    begin O.SharePlus(Y); if Call then Y.Set(3); O.ClearAllGlobals(); exit('done'); end;
    procedure Wrap(Call: Boolean): Text var O: Codeunit UOwner; Y: Codeunit UWrap;
    begin O.ShareWrap(Y); if Call then Y.Set(3); O.ClearAllGlobals(); exit('done'); end;
    procedure Arr(Call: Boolean): Text var O: Codeunit UOwner; Y: Codeunit UArr;
    begin O.ShareArr(Y); if Call then Y.Set(3); O.ClearAllGlobals(); exit('done'); end;
    procedure TwoDim(Call: Boolean): Text var O: Codeunit UOwner; Y: Codeunit UTwoDim;
    begin O.ShareTwoDim(Y); if Call then Y.Set(3); O.ClearAllGlobals(); exit('done'); end;
    procedure Reg(Call: Boolean): Text var O: Codeunit UOwner; Y: Codeunit UReg;
    begin O.SetReg(Y); if Call then Y.Set(3); O.ClearAllGlobals(); exit('done'); end;
    procedure Price(Call: Boolean): Text var O: Codeunit UOwner; Y: Codeunit UPrice;
    begin O.SharePrice(Y); if Call then Y.Set(3); O.ClearAllGlobals(); exit('done'); end;
}
";

/// What `name` of [`SOURCE`] gives with argument `call` and whether it ran a path of `clear-shared-codeunit`
/// ([`run_cell`]).
fn cell(name: &str, call: bool) -> (String, bool) {
    run_cell(SOURCE, name, vec![al_builtins::value::Value::Boolean(call)])
}

/// The stores whose shared instance no call made is measured, and the stores where every shared cell calls first.
const MEASURED_UNCALLED: [&str; 4] = ["Int", "Clb", "Arr", "Plus"];
const UNMEASURED_UNCALLED: [&str; 4] = ["Wrap", "TwoDim", "Reg", "Price"];

// A call made the shared instance (a scalar written, no row): one instance, flagged, for every store.
#[test]
fn clear_all_of_a_shared_store_a_call_made_runs() {
    for name in MEASURED_UNCALLED.into_iter().chain(UNMEASURED_UNCALLED) {
        assert_eq!(cell(name, true), ("done".to_string(), true), "{name}");
    }
}

// No call made the shared instance: measured for the one-Integer store, "CLB Store", the array store and the store with
// one more Integer only.
#[test]
fn clear_all_of_a_shared_store_no_call_made_runs_where_measured() {
    for name in MEASURED_UNCALLED {
        assert_eq!(cell(name, false), ("done".to_string(), true), "{name}");
    }
    for name in UNMEASURED_UNCALLED {
        let why = "ClearAll of a global codeunit variable whose instance another variable shares";
        assert_refused(name, &cell(name, false).0, why);
    }
}
