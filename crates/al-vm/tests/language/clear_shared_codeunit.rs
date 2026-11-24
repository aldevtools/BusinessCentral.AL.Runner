//! `Clear` and `ClearAll` of a codeunit variable whose instance an interface, a second codeunit variable or a parameter
//! shares (al-runner issue 8912). The cells of open corpus pull request 1560
//! (tests/al-language/codeunits/clear-instance/ClearSharedCodeunit.al and ClearSharedCodeunitSI.al), measured on BC 29.0:
//! `Clear(X)` gives X a fresh instance whether X was unused or used once, and every other holder keeps the old one;
//! `ClearAll` in the codeunit that owns X resets X's instance in place, so a holder still shares it; a SingleInstance
//! codeunit variable is one instance whatever is cleared. Each cell runs with X unused and with X used once: both read
//! the same values on BC. The new paths run through assumption `clear-shared-codeunit` until the probe is pinned.

use al_builtins::assumptions::{Assumption, take_hits};

const OBJECTS: &str = "
interface IStore { procedure Set(V: Integer); procedure Get(): Integer; }
codeunit 50800 Store implements IStore { var N: Integer;
    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50801 Holder { var X: Codeunit Store; Count: Integer;
    procedure SetX(V: Integer) begin X.Set(V); end;
    procedure SetCount(V: Integer) begin Count := V; end;
    procedure GetCount(): Integer begin exit(Count); end;
    procedure UseX() begin X.Get(); end;
    procedure ShareIface(var I: Interface IStore) begin I := X; end;
    procedure ShareAlias(var Y: Codeunit Store) begin Y := X; end;
    procedure ClearX() begin Clear(X); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure ClearXViaVar() begin ClearVar(X); end;
    procedure GetX(): Integer begin exit(X.Get()); end;
    procedure ValueParam(UseClearAll: Boolean): Integer begin exit(ClearThenSet(X, UseClearAll)); end;
    local procedure ClearThenSet(P: Codeunit Store; UseClearAll: Boolean): Integer
    begin if UseClearAll then ClearAll() else Clear(X); P.Set(5); exit(P.Get()); end;
    local procedure ClearVar(var P: Codeunit Store) begin Clear(P); end; }
";

/// The test codeunit: each procedure answers `x|alias`. `@L` marks where a local X is used once, `@G` where the holder's
/// global X is.
const CELLS: &str = "
codeunit 50802 T {
    local procedure R(A: Integer; B: Integer): Text begin exit(Format(A) + '|' + Format(B)); end;
    local procedure ClearVar(var P: Codeunit Store) begin Clear(P); end;
    local procedure ClearThenSet(var P: Codeunit Store; Q: Codeunit Store): Integer begin Clear(P); Q.Set(5); exit(Q.Get()); end;
    local procedure SetThrough(P: Codeunit Store) begin P.Set(5); end;
    procedure ClearLocalIface(): Text var X: Codeunit Store; I: Interface IStore;
    begin @L I := X; Clear(X); I.Set(5); exit(R(X.Get(), I.Get())); end;
    procedure ClearLocalAlias(): Text var X: Codeunit Store; Y: Codeunit Store;
    begin @L Y := X; Clear(X); Y.Set(5); exit(R(X.Get(), Y.Get())); end;
    procedure ClearLocalAliasVar(): Text var X: Codeunit Store; Y: Codeunit Store;
    begin @L Y := X; ClearVar(X); Y.Set(5); exit(R(X.Get(), Y.Get())); end;
    procedure ClearLocalValueParam(): Text var X: Codeunit Store; A: Integer;
    begin @L A := ClearThenSet(X, X); exit(R(X.Get(), A)); end;
    procedure ClearGlobalIface(): Text var H: Codeunit Holder; I: Interface IStore;
    begin @G H.ShareIface(I); H.ClearX(); I.Set(5); exit(R(H.GetX(), I.Get())); end;
    procedure ClearGlobalAlias(): Text var H: Codeunit Holder; Y: Codeunit Store;
    begin @G H.ShareAlias(Y); H.ClearX(); Y.Set(5); exit(R(H.GetX(), Y.Get())); end;
    procedure ClearGlobalValueParam(): Text var H: Codeunit Holder; A: Integer;
    begin @G A := H.ValueParam(false); exit(R(H.GetX(), A)); end;
    procedure ClearGlobalIfaceWroteBefore(): Text var H: Codeunit Holder; I: Interface IStore;
    begin @G H.ShareIface(I); I.Set(3); H.ClearX(); exit(R(H.GetX(), I.Get())); end;
    procedure ClearGlobalAliasWroteBefore(): Text var H: Codeunit Holder; Y: Codeunit Store;
    begin @G H.ShareAlias(Y); Y.Set(3); H.ClearX(); exit(R(H.GetX(), Y.Get())); end;
    procedure ClearGlobalAliasVar(): Text var H: Codeunit Holder; Y: Codeunit Store;
    begin @G H.ShareAlias(Y); H.ClearXViaVar(); Y.Set(5); exit(R(H.GetX(), Y.Get())); end;
    procedure ClearAllGlobalIface(): Text var H: Codeunit Holder; I: Interface IStore;
    begin @G H.ShareIface(I); H.ClearAllGlobals(); I.Set(5); exit(R(H.GetX(), I.Get())); end;
    procedure ClearAllGlobalAlias(): Text var H: Codeunit Holder; Y: Codeunit Store;
    begin @G H.ShareAlias(Y); H.ClearAllGlobals(); Y.Set(5); exit(R(H.GetX(), Y.Get())); end;
    procedure ClearAllGlobalValueParam(): Text var H: Codeunit Holder; A: Integer;
    begin @G A := H.ValueParam(true); exit(R(H.GetX(), A)); end;
    procedure ClearAllGlobalIfaceWroteBefore(): Text var H: Codeunit Holder; I: Interface IStore;
    begin @G H.ShareIface(I); I.Set(3); H.ClearAllGlobals(); exit(R(H.GetX(), I.Get())); end;
    procedure ClearAllGlobalAliasWroteBefore(): Text var H: Codeunit Holder; Y: Codeunit Store;
    begin @G H.ShareAlias(Y); Y.Set(3); H.ClearAllGlobals(); exit(R(H.GetX(), Y.Get())); end;
    procedure ShareLocalIface(): Text var X: Codeunit Store; I: Interface IStore;
    begin @L I := X; I.Set(5); exit(R(X.Get(), I.Get())); end;
    procedure ShareLocalIfaceThroughX(): Text var X: Codeunit Store; I: Interface IStore;
    begin @L I := X; X.Set(5); exit(R(X.Get(), I.Get())); end;
    procedure ShareLocalAlias(): Text var X: Codeunit Store; Y: Codeunit Store;
    begin @L Y := X; Y.Set(5); exit(R(X.Get(), Y.Get())); end;
    procedure ShareLocalAliasThroughX(): Text var X: Codeunit Store; Y: Codeunit Store;
    begin @L Y := X; X.Set(5); exit(R(X.Get(), Y.Get())); end;
    procedure ShareLocalValueParam(): Text var X: Codeunit Store;
    begin @L SetThrough(X); exit(R(X.Get(), X.Get())); end;
    procedure ShareGlobalIface(): Text var H: Codeunit Holder; I: Interface IStore;
    begin @G H.ShareIface(I); I.Set(5); exit(R(H.GetX(), I.Get())); end;
    procedure SecondLocal(): Text var X: Codeunit Store; Z: Codeunit Store;
    begin @L Z.Set(7); exit(R(X.Get(), Z.Get())); end;
    procedure SecondGlobal(): Text var H: Codeunit Holder; Z: Codeunit Store;
    begin @G Z.Set(7); exit(R(H.GetX(), Z.Get())); end;
    procedure ClearAllSetXNoAlias(): Text var H: Codeunit Holder;
    begin H.SetX(3); H.SetCount(4); H.ClearAllGlobals(); exit(R(H.GetX(), H.GetCount())); end;
    procedure ClearSetXNoAlias(): Text var H: Codeunit Holder;
    begin H.SetX(3); H.ClearX(); exit(R(H.GetX(), 0)); end;
}
";

/// The cells of ClearSharedCodeunit.al with the values BC 29.0 gave, X unused and X used once alike; `true` where the cell
/// runs a path of assumption `clear-shared-codeunit` with X unused (a Clear of an X still unused that another holder
/// shares, a ClearAll of the holder whose X another holder shares; a write through the alias before the Clear uses X).
const EXPECTED: &[(&str, &str, bool)] = &[
    ("ClearLocalIface", "0|5", true),       // ClearSharedCodeunit.al Clear_LocalUnusedIface
    ("ClearLocalAlias", "0|5", true),       // Clear_LocalUnusedAlias
    ("ClearLocalAliasVar", "0|5", true),    // Clear_LocalUnusedAliasClearedThroughVarParam
    ("ClearLocalValueParam", "0|5", true),  // Clear_LocalUnusedValueParam
    ("ClearGlobalIface", "0|5", true),      // Clear_GlobalUnusedIface
    ("ClearGlobalAlias", "0|5", true),      // Clear_GlobalUnusedAlias
    ("ClearGlobalValueParam", "0|5", true), // Clear_GlobalUnusedValueParam
    ("ClearGlobalIfaceWroteBefore", "0|3", false), // Clear_GlobalUnusedIface_AliasWroteBefore
    ("ClearGlobalAliasWroteBefore", "0|3", false), // Clear_GlobalUnusedAlias_AliasWroteBefore
    ("ClearGlobalAliasVar", "0|5", true),   // Clear_GlobalUnusedAliasClearedThroughVarParam
    ("ClearAllGlobalIface", "5|5", true),   // ClearAll_GlobalUnusedIface
    ("ClearAllGlobalAlias", "5|5", true),   // ClearAll_GlobalUnusedAlias
    ("ClearAllGlobalValueParam", "5|5", true), // ClearAll_GlobalUnusedValueParam
    ("ClearAllGlobalIfaceWroteBefore", "0|0", true), // ClearAll_GlobalUnusedIface_AliasWroteBefore
    ("ClearAllGlobalAliasWroteBefore", "0|0", true), // ClearAll_GlobalUnusedAlias_AliasWroteBefore
    ("ShareLocalIface", "5|5", false),      // Share_LocalUnusedIface_WriteThroughAlias
    ("ShareLocalIfaceThroughX", "5|5", false), // Share_LocalUnusedIface_WriteThroughX
    ("ShareLocalAlias", "5|5", false),      // Share_LocalUnusedAlias_WriteThroughAlias
    ("ShareLocalAliasThroughX", "5|5", false), // Share_LocalUnusedAlias_WriteThroughX
    ("ShareLocalValueParam", "5|5", false), // Share_LocalUnusedValueParam_WriteThroughParam
    ("ShareGlobalIface", "5|5", false),     // Share_GlobalUnusedIface_WriteThroughAlias
    ("SecondLocal", "0|7", false),          // Second_LocalUnusedX_OtherVariableWrites
    ("SecondGlobal", "0|7", false),         // Second_GlobalUnusedX_OtherVariableWrites
    ("ClearAllSetXNoAlias", "0|0", false),  // ClearAll_GlobalSetX_NoAlias (nothing shares X: main's fresh instance)
    ("ClearSetXNoAlias", "0|0", false),     // Clear_GlobalSetX_NoAlias (X used: main's fresh instance)
];

fn source(used: bool) -> String {
    let cells = CELLS
        .replace("@L", if used { "X.Get();" } else { "" })
        .replace("@G", if used { "H.UseX();" } else { "" });
    format!("{OBJECTS}{cells}")
}

#[test]
fn clear_and_clear_all_of_a_shared_codeunit_variable_give_the_measured_values() {
    for used in [false, true] {
        let src = source(used);
        let mut wrong = Vec::new();
        for (cell, want, _) in EXPECTED {
            let got = al_testkit::text(al_testkit::run_in(&src, "T", cell));
            if got != *want {
                wrong.push(format!("{cell} (X used: {used}): got {got}, BC {want}"));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}

#[test]
fn the_new_paths_are_flagged_and_the_others_are_not() {
    let src = source(false);
    for (cell, _, flagged) in EXPECTED {
        take_hits();
        al_testkit::text(al_testkit::run_in(&src, "T", cell));
        let hit = take_hits().contains(&Assumption::ClearSharedCodeunit);
        assert_eq!(hit, *flagged, "{cell}");
    }
}

const SI: &str = "
interface IStore { procedure Set(V: Integer); procedure Get(): Integer; }
codeunit 50810 Store implements IStore { SingleInstance = true; var N: Integer;
    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50811 Holder { var X: Codeunit Store; Count: Integer;
    procedure ShareIface(var I: Interface IStore) begin I := X; end;
    procedure ClearX() begin Clear(X); end;
    procedure SetCount(V: Integer) begin Count := V; end;
    procedure GetCount(): Integer begin exit(Count); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure GetX(): Integer begin exit(X.Get()); end; }
codeunit 50812 T {
    local procedure R(A: Integer; B: Integer): Text begin exit(Format(A) + '|' + Format(B)); end;
    procedure ClearLocal(): Text var R0: Codeunit Store; X: Codeunit Store; I: Interface IStore;
    begin R0.Set(0); I := X; Clear(X); I.Set(5); exit(R(X.Get(), I.Get())); end;
    procedure ClearGlobal(): Text var R0: Codeunit Store; H: Codeunit Holder; I: Interface IStore;
    begin R0.Set(0); H.ShareIface(I); I.Set(3); H.ClearX(); exit(R(H.GetX(), I.Get())); end;
    procedure ClearAllGlobal(): Text var R0: Codeunit Store; H: Codeunit Holder; I: Interface IStore;
    begin R0.Set(0); H.ShareIface(I); I.Set(3); H.SetCount(4); H.ClearAllGlobals(); exit(R(H.GetX(), H.GetCount())); end;
}
";

// ClearSharedCodeunitSI.al: Clear and ClearAll change nothing for a SingleInstance codeunit variable; ClearAll still resets
// the owner's Integer global. The runner's SingleInstance variable routes every call to the one instance (main's result).
#[test]
fn a_single_instance_codeunit_variable_keeps_its_one_instance() {
    for (cell, want) in [("ClearLocal", "5|5"), ("ClearGlobal", "3|3"), ("ClearAllGlobal", "3|0")] {
        take_hits();
        assert_eq!(al_testkit::text(al_testkit::run_in(SI, "T", cell)), want, "{cell}");
        assert!(!take_hits().contains(&Assumption::ClearSharedCodeunit), "{cell}");
    }
}

const SHAPES: &str = "
interface IStore { procedure Set(V: Integer); procedure Get(): Integer; }
codeunit 50820 Store implements IStore { EventSubscriberInstance = Manual; var N: Integer;
    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end;
    procedure CallBack(H: Codeunit Holder) begin H.ClearAllGlobals(); end; }
codeunit 50821 Store2 implements IStore { var N: Integer; M: Integer;
    procedure Set(V: Integer) begin N := V; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50822 Holder { var X: Codeunit Store;
    procedure ShareIface(var I: Interface IStore) begin I := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure RunCallBack() var Me: Codeunit Holder; begin Me := this; X.CallBack(Me); end;
    procedure GetX(): Integer begin exit(X.Get()); end; }
codeunit 50823 Holder2 { var X: Codeunit Store2;
    procedure ShareIface(var I: Interface IStore) begin I := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50824 SiHolder { SingleInstance = true; var X: Codeunit Store;
    procedure ShareIface(var I: Interface IStore) begin I := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50825 ArrHolder { var A: array[2] of Codeunit Store;
    procedure ShareIface(var I: Interface IStore) begin I := A[1]; end;
    procedure ClearA() begin Clear(A); end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50827 GlobalChain { var X: Codeunit Store;
    local procedure Reassign(var P: Codeunit Store): Integer var Z: Codeunit Store; begin X := Z; Clear(P); exit(0); end;
    local procedure Keep(var P: Codeunit Store): Integer begin Clear(P); P.Set(5); exit(0); end;
    procedure ReassignedShared(): Text var Y: Codeunit Store; begin Y := X; exit(Format(Reassign(X))); end;
    procedure KeptShared(): Text var Y: Codeunit Store; begin Y := X; Keep(X); exit(Format(X.Get()) + '|' + Format(Y.Get())); end;
    procedure KeptUnshared(): Text begin Keep(X); exit(Format(X.Get())); end; }
codeunit 50826 T {
    local procedure ClearVal(P: Codeunit Store; Q: Codeunit Store): Integer begin Clear(P); P.Set(5); exit(Q.Get()); end;
    local procedure ClearVar(var P: Codeunit Store) begin Clear(P); end;
    local procedure ClearVarVar(var P: Codeunit Store) begin ClearVar(P); end;
    local procedure AssignClear(var P: Codeunit Store): Integer var Y: Codeunit Store;
    begin P := Y; Clear(P); P.Set(5); exit(Y.Get()); end;
    local procedure AssignClearVar(var P: Codeunit Store): Integer var Y: Codeunit Store;
    begin P := Y; ClearVar(P); P.Set(5); exit(Y.Get()); end;
    procedure ClearVarAssignedShared(): Text var X: Codeunit Store; begin exit(Format(AssignClear(X))); end;
    procedure ClearVarVarAssignedShared(): Text var X: Codeunit Store; begin exit(Format(AssignClearVar(X))); end;
    procedure ClearLocalUnshared(): Text var X: Codeunit Store; begin Clear(X); X.Set(5); exit(Format(X.Get())); end;
    procedure ClearVarUnshared(): Text var X: Codeunit Store; begin ClearVar(X); X.Set(5); exit(Format(X.Get())); end;
    procedure ClearVarVarUnshared(): Text var X: Codeunit Store; begin ClearVarVar(X); X.Set(5); exit(Format(X.Get())); end;
    procedure ClearVarVarShared(): Text var X: Codeunit Store; Y: Codeunit Store;
    begin Y := X; ClearVarVar(X); Y.Set(5); exit(Format(X.Get()) + '|' + Format(Y.Get())); end;
    procedure ClearValueParam(): Text var X: Codeunit Store; begin exit(Format(ClearVal(X, X))); end;
    procedure ClearValueParamUsed(): Text var X: Codeunit Store; begin X.Get(); exit(Format(ClearVal(X, X))); end;
    procedure ClearElementShared(): Text var A: array[2] of Codeunit Store; I: Interface IStore; begin I := A[1]; Clear(A[1]); exit('done'); end;
    procedure ClearInterfaceShared(): Text var X: Codeunit Store; I: Interface IStore; begin I := X; Clear(I); X.Set(5); exit(Format(X.Get())); end;
    procedure ClearVariantShared(): Text var X: Codeunit Store; V: Variant; begin V := X; Clear(V); X.Set(5); exit(Format(X.Get())); end;
    procedure ClearArrayShared(): Text var H: Codeunit ArrHolder; I: Interface IStore; begin H.ShareIface(I); H.ClearA(); exit('done'); end;
    procedure ClearArrayUnshared(): Text var H: Codeunit ArrHolder; begin H.ClearA(); exit('done'); end;
    procedure ClearAllArrayShared(): Text var H: Codeunit ArrHolder; I: Interface IStore; begin H.ShareIface(I); H.ClearAllGlobals(); exit('done'); end;
    procedure ClearAllTwoGlobalsShared(): Text var H: Codeunit Holder2; I: Interface IStore; begin H.ShareIface(I); H.ClearAllGlobals(); exit('done'); end;
    procedure ClearAllTwoGlobalsUnshared(): Text var H: Codeunit Holder2; begin H.ClearAllGlobals(); exit('done'); end;
    procedure ClearAllSiOwnerShared(): Text var H: Codeunit SiHolder; I: Interface IStore; begin H.ShareIface(I); H.ClearAllGlobals(); exit('done'); end;
    procedure ClearAllSiOwnerUnshared(): Text var H: Codeunit SiHolder; begin H.ClearAllGlobals(); exit('done'); end;
    procedure ClearAllWhileXRuns(): Text var H: Codeunit Holder; begin H.RunCallBack(); exit('done'); end;
    procedure ClearBoundNeverCalled(): Text var X: Codeunit Store; Y: Codeunit Store;
    begin BindSubscription(X); Y := X; Clear(X); Y.Set(5); exit(Format(X.Get()) + '|' + Format(Y.Get())); end;
}
";

/// What `cell` of [`SHAPES`] gives: its text, or `refused`; and whether it ran a path of assumption `clear-shared-codeunit`.
fn shape(cell: &str, track: bool) -> (String, bool) {
    take_hits();
    let mut b = al_testkit::Built::from_source(SHAPES);
    if track {
        b.session.track_single_instances();
    }
    let (owner, cell) = cell.split_once('.').unwrap_or(("T", cell));
    let got = match b.run_in(owner, cell) {
        Err(al_builtins::native::Signal::Unsupported(why)) => format!("refused: {why}"),
        other => al_testkit::text(other),
    };
    (got, take_hits().contains(&Assumption::ClearSharedCodeunit))
}

// A codeunit variable nothing shares reads the same after main's answer as after the measured one: no mark.
#[test]
fn an_instance_nothing_shares_keeps_main_with_no_mark() {
    for track in [false, true] {
        for (cell, want) in [
            ("ClearLocalUnshared", "5"),
            ("ClearInterfaceShared", "5"),
            ("ClearVariantShared", "5"),
            ("ClearVarUnshared", "5"),
            ("ClearArrayUnshared", "done"),
            ("ClearAllTwoGlobalsUnshared", "done"),
            ("ClearAllSiOwnerUnshared", "done"),
        ] {
            assert_eq!(
                shape(cell, track),
                (want.to_string(), false),
                "{cell} (tracked: {track})"
            );
        }
    }
}

// The shapes the probe does not measure refuse when another holder shares the instance: Clear of a by-value parameter
// nothing used yet, Clear or ClearAll of an array whose unused element is shared, ClearAll of a shared instance of a codeunit
// with two globals, ClearAll while a routine of X runs. A used by-value parameter
// keeps main's fresh instance.
#[test]
fn a_shared_instance_in_a_shape_the_probe_does_not_measure_refuses() {
    for (cell, why) in [
        (
            "ClearValueParam",
            "Clear of a codeunit variable that nothing used yet and another variable shares",
        ),
        (
            "ClearElementShared",
            "Clear of a codeunit variable that nothing used yet and another variable shares",
        ),
        ("ClearArrayShared", "Clear of an array whose unused codeunit element"),
        (
            "ClearAllArrayShared",
            "ClearAll of a global array whose codeunit element",
        ),
        (
            "ClearAllTwoGlobalsShared",
            "ClearAll of a global codeunit variable whose instance",
        ),
        (
            "ClearAllWhileXRuns",
            "ClearAll of a global codeunit variable whose instance",
        ),
    ] {
        let got = shape(cell, false).0;
        assert!(got.starts_with("refused: ") && got.contains(why), "{cell}: {got}");
    }
    assert_eq!(shape("ClearValueParamUsed", false), ("0".to_string(), false));
    // The Integer store under a SingleInstance owner runs since corpus PR 1564 (ClearSharedCodeunitOwnerKinds.al
    // SICLX_ClearAll_UnusedAlias :1472, shared through a second variable; the sharer's kind is not read).
    assert_eq!(shape("ClearAllSiOwnerShared", false), ("done".to_string(), true));
}

// BindSubscription makes the instance (al-vm `dispatch.rs`, `ensure_initialized` before the binding), so a bound X is never
// unused: Clear takes main's arm for a used X, with no mark.
// A var parameter handed on as a var parameter keeps one more copy of the caller's variable, which is not another holder: a
// Clear nothing else shares keeps main's answer with no mark, as Microsoft's facades hand a var parameter to their
// implementation (Data Compression to Data Compression Impl., corpus streams/blob-tempblob/ZipBlobFieldUpdate.al); one
// another holder shares gets the measured fresh instance.
#[test]
fn a_var_parameter_handed_on_counts_only_other_holders() {
    assert_eq!(shape("ClearVarVarUnshared", false), ("5".to_string(), false));
    assert_eq!(shape("ClearVarVarShared", false), ("0|5".to_string(), true));
}

// A var parameter handed a global: the global counts as the chain's own while it holds the instance. Assigned another
// instance by its name while the routine runs, the parameter no longer follows it, which the probe does not measure: refused
// when another holder shares the instance.
#[test]
fn a_var_parameter_of_a_global_counts_the_global_while_it_holds_the_instance() {
    assert_eq!(shape("GlobalChain.KeptShared", false), ("5|0".to_string(), true));
    assert_eq!(shape("GlobalChain.KeptUnshared", false), ("5".to_string(), false));
    let got = shape("GlobalChain.ReassignedShared", false).0;
    assert!(
        got.starts_with("refused: Clear of a codeunit variable that nothing used yet"),
        "{got}"
    );
}

// A var parameter that an assignment made share another variable's instance is shared like a local
// (ClearSharedCodeunit.al Clear_LocalUnusedAlias): Clear gives it a fresh instance and the other variable keeps the old one.
#[test]
fn a_var_parameter_assigned_another_instance_counts_that_holder() {
    assert_eq!(shape("ClearVarAssignedShared", false), ("0".to_string(), true));
    assert_eq!(shape("ClearVarVarAssignedShared", false), ("0".to_string(), true));
}

#[test]
fn a_bound_codeunit_variable_is_never_unused() {
    assert_eq!(shape("ClearBoundNeverCalled", false), ("0|5".to_string(), false));
}

/// The store of open corpus pull request 1561 (ClearSharedCodeunitTempRef.al:55-104, "CLB Store": a temporary record, an
/// array of RecordRef of two dimensions, an array of Integer) and its owners: a SingleInstance codeunit
/// (ClearSharedCodeunitSIOwner.al "CLS SI Owner A3"), a plain holder ("CLB Holder") and a test codeunit ("CLS TG ClearAll
/// 6"), each also with the store shared by a variable; and the array store "CLS Arr Store" (ClearSharedCodeunitSIOwner.al:42)
/// with its plain holder. Next to them the shapes 1561 does not measure: a store with one more global or a Label, a store
/// inside a codeunit the owner holds, an array of stores, a report and a codeunit with a Subtype other than Test as the
/// owner, the array store never written, unshared or under a SingleInstance owner, the Integer store under a test codeunit.
const TEMP_STORE: &str = "
table 50830 Row { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 50831 TStore { var Rows: Record Row temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer;
    procedure Write(V: Integer) begin Rows.Init(); Rows.No := V; Rows.Insert();
        if Refs[1, 1].Number = 0 then Refs[1, 1].Open(50830, true);
        Refs[1, 1].Init(); Refs[1, 1].Field(1).Value := V; Refs[1, 1].Insert(); Ints[1] := V; end;
    procedure SetInt(V: Integer) begin Ints[1] := V; end;
    procedure State(): Text var RefRows: Text;
    begin RefRows := '-'; if Refs[1, 1].Number <> 0 then RefRows := Format(Refs[1, 1].Count());
        exit(StrSubstNo('rows=%1 ref11=%2 refrows=%3 int1=%4', Rows.Count(), Refs[1, 1].Number, RefRows, Ints[1])); end; }
codeunit 50832 TStoreExtra { var Rows: Record Row temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer; N: Integer;
    procedure Write(V: Integer) begin Rows.No := V; Rows.Insert(); end; }
codeunit 50833 TWrap { var Inner: Codeunit TStore; procedure Write(V: Integer) begin Inner.Write(V); end; }
codeunit 50834 TOwner { SingleInstance = true; var X: Codeunit TStore; Cnt: Integer;
    procedure WriteX(V: Integer) begin X.Write(V); end;
    procedure StateX(): Text begin exit(X.State() + ' cnt=' + Format(Cnt)); end;
    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure Share(var Y: Codeunit TStore) begin Y := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50835 THolder { var X: Codeunit TStore; Count: Integer; E: Codeunit TStoreExtra; W: Codeunit TWrap; A: array[2] of Codeunit TStore;
    procedure WriteX(V: Integer) begin X.Write(V); end;
    procedure SetIntX(V: Integer) begin X.SetInt(V); end;
    procedure WriteE() begin E.Write(1); end;
    procedure WriteW() begin W.Write(1); end;
    procedure WriteA() begin A[1].Write(1); end;
    procedure StateX(): Text begin exit(X.State() + ' count=' + Format(Count)); end;
    procedure SetCount(V: Integer) begin Count := V; end;
    procedure Share(var Y: Codeunit TStore) begin Y := X; end;
    procedure ValueParam(): Text begin exit(ClearThenWrite(X)); end;
    local procedure ClearThenWrite(P: Codeunit TStore): Text begin ClearAll(); P.Write(5); exit(X.State()); end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50836 TTest { Subtype = Test; var GX: Codeunit TStore;
    procedure GlobalWritten(): Text begin GX.Write(3); ClearAll(); exit(GX.State()); end;
    procedure GlobalShared(): Text var Y: Codeunit TStore;
    begin Y := GX; Y.Write(3); ClearAll(); exit(GX.State() + '|' + Y.State()); end; }
codeunit 50837 TNormal { Subtype = Normal; var GX: Codeunit TStore;
    procedure GlobalWritten(): Text begin GX.Write(3); ClearAll(); exit(GX.State()); end; }
report 50838 TRep { ProcessingOnly = true; var GX: Codeunit TStore;
    procedure GlobalWritten(): Text begin GX.Write(3); ClearAll(); exit(GX.State()); end; }
codeunit 50840 TArrSelf { var Rows: array[2] of Record Row temporary;
    procedure Run(): Text begin Rows[1].No := 1; Rows[1].Insert(); ClearAll(); exit(Format(Rows[2].Count())); end; }
report 50841 TArrRep { ProcessingOnly = true; var Rows: array[2] of Record Row temporary;
    procedure Run(): Text begin Rows[1].No := 1; Rows[1].Insert(); ClearAll(); exit(Format(Rows[2].Count())); end; }
codeunit 50842 TArrStore { var Rows: array[2] of Record Row temporary; Refs: array[2] of RecordRef; Ints: array[3] of Integer;
    procedure Write(E: Integer; V: Integer) begin Rows[E].No := V; Rows[E].Insert();
        if Refs[1].Number = 0 then Refs[1].Open(50830, true); Ints[1] := V; end;
    procedure State(): Text
    begin exit(StrSubstNo('rows1=%1 rows2=%2 ref1=%3 int1=%4', Rows[1].Count(), Rows[2].Count(), Refs[1].Number, Ints[1])); end; }
codeunit 50843 TArrHolder { var X: Codeunit TArrStore;
    procedure WriteX(E: Integer; V: Integer) begin X.Write(E, V); end;
    procedure Share(var Y: Codeunit TArrStore) begin Y := X; end;
    procedure StateX(): Text begin exit(X.State()); end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50844 TArrSiHolder { SingleInstance = true; var X: Codeunit TArrStore;
    procedure Share(var Y: Codeunit TArrStore) begin Y := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50845 TStoreLabel { var Rows: Record Row temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer; L: Label 'x';
    procedure Write(V: Integer) begin Rows.No := V; Rows.Insert(); Ints[1] := V; end; }
codeunit 50846 TLabelHolder { var X: Codeunit TStoreLabel;
    procedure WriteX(V: Integer) begin X.Write(V); end;
    procedure Share(var Y: Codeunit TStoreLabel) begin Y := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50847 TInt { var N: Integer; procedure Set(V: Integer) begin N := V; end; }
codeunit 50849 TLabelSi { SingleInstance = true; var X: Codeunit TStoreLabel;
    procedure Share(var Y: Codeunit TStoreLabel) begin Y := X; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50850 TLabelTest { Subtype = Test; var GX: Codeunit TStoreLabel;
    procedure GlobalWritten(): Text begin GX.Write(3); ClearAll(); exit('done'); end; }
codeunit 50851 TLabelVp { var X: Codeunit TStoreLabel;
    procedure ValueParam(): Text begin exit(ClearThenWrite(X)); end;
    local procedure ClearThenWrite(P: Codeunit TStoreLabel): Text begin ClearAll(); P.Write(5); exit('done'); end; }
codeunit 50852 TStoreLabel2 { var Rows: Record Row temporary; Refs: array[2, 3] of RecordRef; L: Label 'x'; Ints: array[3] of Integer;
    procedure Write(V: Integer) begin Rows.No := V; Rows.Insert(); end; }
codeunit 50853 TStoreLocked { var Rows: Record Row temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer; L: Label 'x', Locked = true;
    procedure Write(V: Integer) begin Rows.No := V; Rows.Insert(); end; }
codeunit 50854 TOddHolder { var L2: Codeunit TStoreLabel2; Lk: Codeunit TStoreLocked;
    procedure WriteL2() begin L2.Write(1); end;
    procedure WriteLk() begin Lk.Write(1); end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50855 TArrNormal { Subtype = Normal; var X: Codeunit TArrStore;
    procedure GlobalWritten(): Text begin X.Write(1, 3); ClearAll(); exit('done'); end; }
codeunit 50856 TPlusSi { SingleInstance = true; var X: Codeunit TStoreExtra;
    procedure GlobalWritten(): Text begin X.Write(3); ClearAll(); exit('done'); end; }
codeunit 50857 TWrapN { var Inner: Codeunit TStore; N: Integer;
    procedure Share(var Y: Codeunit TStore) begin Y := Inner; end; }
codeunit 50858 TDeepHolder { var W: Codeunit TWrapN; A: array[2] of Codeunit TStore;
    procedure ShareInner(var Y: Codeunit TStore) begin W.Share(Y); end;
    procedure ShareElement(var Y: Codeunit TStore) begin Y := A[1]; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50848 TIntTest { Subtype = Test; var GX: Codeunit TInt;
    procedure Shared(): Text var Y: Codeunit TInt; begin Y := GX; Y.Set(3); ClearAll(); exit('done'); end; }
codeunit 50839 T {
    procedure HolderSharedWroteBefore(): Text var H: Codeunit THolder; Y: Codeunit TStore; A: Text;
    begin H.Share(Y); Y.Write(3); H.ClearAllGlobals(); A := H.StateX() + '|' + Y.State(); Y.Write(5); exit(A + '|' + H.StateX()); end;
    procedure HolderSharedUnused(): Text var H: Codeunit THolder; Y: Codeunit TStore;
    begin H.Share(Y); H.ClearAllGlobals(); Y.Write(5); exit(H.StateX()); end;
    procedure HolderValueParam(): Text var H: Codeunit THolder; begin exit(H.ValueParam()); end;
    procedure SiOwnerShared(): Text var S: Codeunit TOwner; Y: Codeunit TStore;
    begin S.Share(Y); Y.Write(3); S.SetCnt(4); S.ClearAllGlobals(); exit(S.StateX() + '|' + Y.State()); end;
    procedure TestOwnerShared(): Text var C: Codeunit TTest; begin exit(C.GlobalShared()); end;
    procedure ArrHolderShared(): Text var H: Codeunit TArrHolder; Y: Codeunit TArrStore;
    begin H.Share(Y); Y.Write(1, 3); Y.Write(2, 4); H.ClearAllGlobals(); exit(H.StateX() + '|' + Y.State()); end;
    procedure ArrHolderSharedUnused(): Text var H: Codeunit TArrHolder; Y: Codeunit TArrStore;
    begin H.Share(Y); H.ClearAllGlobals(); exit('done'); end;
    procedure ArrHolderUnsharedWritten(): Text var H: Codeunit TArrHolder; begin H.WriteX(1, 3); H.ClearAllGlobals(); exit('done'); end;
    procedure ArrSiShared(): Text var S: Codeunit TArrSiHolder; Y: Codeunit TArrStore;
    begin S.Share(Y); Y.Write(1, 3); S.ClearAllGlobals(); exit('done'); end;
    procedure LabelHolderWritten(): Text var H: Codeunit TLabelHolder; begin H.WriteX(3); H.ClearAllGlobals(); exit('done'); end;
    procedure LabelHolderShared(): Text var H: Codeunit TLabelHolder; Y: Codeunit TStoreLabel;
    begin H.Share(Y); Y.Write(3); H.ClearAllGlobals(); exit('done'); end;
    procedure IntTestShared(): Text var C: Codeunit TIntTest; begin exit(C.Shared()); end;
    procedure LabelSiShared(): Text var S: Codeunit TLabelSi; Y: Codeunit TStoreLabel;
    begin S.Share(Y); Y.Write(3); S.ClearAllGlobals(); exit('done'); end;
    procedure LabelTestWritten(): Text var C: Codeunit TLabelTest; begin exit(C.GlobalWritten()); end;
    procedure LabelValueParam(): Text var H: Codeunit TLabelVp; begin exit(H.ValueParam()); end;
    procedure LabelBetweenArrays(): Text var H: Codeunit TOddHolder; begin H.WriteL2(); H.ClearAllGlobals(); exit('done'); end;
    procedure LabelLocked(): Text var H: Codeunit TOddHolder; begin H.WriteLk(); H.ClearAllGlobals(); exit('done'); end;
    procedure ArrNormalWritten(): Text var C: Codeunit TArrNormal; begin exit(C.GlobalWritten()); end;
    procedure PlusSiWritten(): Text var C: Codeunit TPlusSi; begin exit(C.GlobalWritten()); end;
    procedure WrapperInnerSharedUnwritten(): Text var H: Codeunit TDeepHolder; Y: Codeunit TStore;
    begin H.ShareInner(Y); H.ClearAllGlobals(); exit('done'); end;
    procedure ElementSharedNoRows(): Text var H: Codeunit TDeepHolder; Y: Codeunit TStore;
    begin H.ShareElement(Y); Y.SetInt(1); H.ClearAllGlobals(); exit('done'); end;
    procedure ArrayOfTempRecordsInReport(): Text var R: Report TArrRep; begin exit(R.Run()); end;
    procedure ArrayOfTempRecordsInOwner(): Text var C: Codeunit TArrSelf; begin exit(C.Run()); end;
    procedure SiOwner(): Text var S: Codeunit TOwner;
    begin S.WriteX(5); S.SetCnt(4); S.ClearAllGlobals(); exit(S.StateX()); end;
    procedure SiOwnerThenWrites(): Text var S: Codeunit TOwner;
    begin S.WriteX(5); S.ClearAllGlobals(); S.WriteX(6); exit(S.StateX()); end;
    procedure Holder(): Text var H: Codeunit THolder;
    begin H.WriteX(3); H.SetCount(4); H.ClearAllGlobals(); exit(H.StateX()); end;
    procedure HolderTempNotWritten(): Text var H: Codeunit THolder;
    begin H.SetIntX(3); H.ClearAllGlobals(); exit(H.StateX()); end;
    procedure TestOwner(): Text var C: Codeunit TTest; begin exit(C.GlobalWritten()); end;
    procedure NormalOwner(): Text var C: Codeunit TNormal; begin exit(C.GlobalWritten()); end;
    procedure ReportOwner(): Text var R: Report TRep; begin exit(R.GlobalWritten()); end;
    procedure ExtraStore(): Text var H: Codeunit THolder; begin H.WriteE(); H.ClearAllGlobals(); exit('done'); end;
    procedure WrappedStore(): Text var H: Codeunit THolder; begin H.WriteW(); H.ClearAllGlobals(); exit('done'); end;
    procedure StoreArray(): Text var H: Codeunit THolder; begin H.WriteA(); H.ClearAllGlobals(); exit('done'); end;
}
";

/// What `cell` of [`TEMP_STORE`] gives (its text, or `refused: ...`) and whether it ran a path of assumption
/// `clear-shared-codeunit`.
fn temp_store(cell: &str) -> (String, bool) {
    take_hits();
    let got = match al_testkit::Built::from_source(TEMP_STORE).run_in("T", cell) {
        Err(al_builtins::native::Signal::Unsupported(why)) => format!("refused: {why}"),
        other => al_testkit::text(other),
    };
    (got, take_hits().contains(&Assumption::ClearSharedCodeunit))
}

// ClearAll in the owner of a store whose temporary table holds rows, nothing sharing the store: BC resets the store in
// place and keeps the rows, closes the RecordRef and sets the Integers to 0 (ClearSharedCodeunitSIOwner.al
// A3_Step3_ClearAllInOwner and A3_Step5_WriteAgain, ClearAll_TestGlobalWritten_NoAlias; ClearSharedCodeunitTempRef.al
// ClearAll_GlobalWritten_NoAlias :1362). Main gave the store a fresh instance, which lost the rows.
#[test]
fn clear_all_of_a_temp_store_keeps_its_rows() {
    for (cell, want) in [
        ("SiOwner", "rows=1 ref11=0 refrows=- int1=0 cnt=0"),
        ("SiOwnerThenWrites", "rows=2 ref11=50830 refrows=1 int1=6 cnt=0"),
        ("Holder", "rows=1 ref11=0 refrows=- int1=0 count=0"),
        ("TestOwner", "rows=1 ref11=0 refrows=- int1=0"),
    ] {
        assert_eq!(temp_store(cell), (want.to_string(), true), "{cell}");
    }
    // A temporary table nothing wrote: a fresh store reads the same, main's answer with no mark.
    assert_eq!(
        temp_store("HolderTempNotWritten"),
        ("rows=0 ref11=0 refrows=- int1=0 count=0".to_string(), false)
    );
    // ClearAll in a plain codeunit whose own global is an array of temporary records keeps the one table's rows
    // (ClearSharedCodeunitSIOwner.al ArrStore_ClearAll_InsideStore :1808); main gave the array fresh tables.
    assert_eq!(temp_store("ArrayOfTempRecordsInOwner"), ("1".to_string(), true));
}

// The shapes 1561 does not measure, whose temporary rows main's fresh instance would lose, refuse.
#[test]
fn clear_all_of_an_unmeasured_store_with_rows_refuses() {
    let got = temp_store("ArrayOfTempRecordsInReport").0;
    assert!(
        got.starts_with("refused: ClearAll of a global array of temporary records whose table holds rows"),
        "{got}"
    );
    // A report that does not state `UseRequestPage = false` (ClearSharedCodeunitOwnerKinds.al:138-141 measures one that
    // does), a store holding only a store (no Integer, as "CLQ Wrapper" :268 has).
    for cell in ["ReportOwner", "WrappedStore"] {
        let got = temp_store(cell).0;
        assert!(
            got.starts_with("refused: ClearAll of a global codeunit variable whose instance holds temporary rows"),
            "{cell}: {got}"
        );
    }
}

// The owners and stores corpus PR 1564 adds: a codeunit with Subtype = Normal (ClearSharedCodeunitOwnerKinds.al
// NormalSubtypeOwner_Written_NoAlias :470), "CLB Store" with one more Integer (StorePlus_Written_NoAlias :644), an array of
// "CLB Store" whose element 1 was written (Arr2_Element1Written_NoAlias :770): rows kept, RecordRef closed, Integers 0.
#[test]
fn clear_all_of_a_store_in_the_owners_and_shapes_1564_measures_keeps_its_rows() {
    assert_eq!(
        temp_store("NormalOwner"),
        ("rows=1 ref11=0 refrows=- int1=0".to_string(), true)
    );
    assert_eq!(temp_store("ExtraStore"), ("done".to_string(), true));
    assert_eq!(temp_store("StoreArray"), ("done".to_string(), true));
}

// ClearAll in the owner of a shared store of 1561's shapes: BC keeps the store and every holder on one instance and resets it
// in place (rows kept, RecordRef closed, Integers 0), for the measured (owner, store) pairs: a plain holder, a SingleInstance
// owner and a test codeunit with "CLB Store" (ClearSharedCodeunitTempRef.al ClearAll_GlobalUnusedAlias :403,
// ClearAll_GlobalUnusedAlias_AliasWroteBefore :456, ClearAll_GlobalUnusedValueParam :424; ClearSharedCodeunitSIOwner.al
// ClearAll_SIOwnerUnused_Alias_AliasWroteBefore :1459, ClearAll_TestGlobalUnused_AliasWroteBefore :2090), a plain holder with
// "CLS Arr Store" whose rows a sharer wrote (ArrStore_ClearAll_Alias_AliasWroteBefore :1759).
#[test]
fn clear_all_of_a_shared_temp_store_resets_it_in_place() {
    for (cell, want) in [
        (
            "HolderSharedWroteBefore",
            "rows=1 ref11=0 refrows=- int1=0 count=0|rows=1 ref11=0 refrows=- int1=0|rows=2 ref11=50830 refrows=1 int1=5 count=0",
        ),
        ("HolderSharedUnused", "rows=1 ref11=50830 refrows=1 int1=5 count=0"),
        ("HolderValueParam", "rows=1 ref11=50830 refrows=1 int1=5"),
        (
            "SiOwnerShared",
            "rows=1 ref11=0 refrows=- int1=0 cnt=0|rows=1 ref11=0 refrows=- int1=0",
        ),
        (
            "TestOwnerShared",
            "rows=1 ref11=0 refrows=- int1=0|rows=1 ref11=0 refrows=- int1=0",
        ),
        (
            "ArrHolderShared",
            "rows1=2 rows2=2 ref1=0 int1=0|rows1=2 rows2=2 ref1=0 int1=0",
        ),
    ] {
        assert_eq!(temp_store(cell), (want.to_string(), true), "{cell}");
    }
}

// The cells corpus PR 1564 adds to 1561's pairs run, flagged: "CLS Arr Store" shared and never written
// (ClearSharedCodeunitOwnerKinds.al ArrStore_ClearAll_UnusedAlias :418), written and unshared in its holder
// (ArrStore_ClearAll_InHolder_NoAlias :439), shared under a SingleInstance owner (SIArr_ClearAll_AliasWroteBefore :987);
// "CLB Store" with a Label in a plain holder (ClearSharedCodeunitLabelStore.al LabelLast_ClearAllInHolder_Written_NoAlias
// :640, LabelLast_ClearAllInHolder_AliasWroteBefore :672); the Integer store under a test codeunit
// (ClearSharedCodeunitOwnerKinds.al ClearAll_TestGlobalCLX_LocalAliasWroteBefore :1568).
#[test]
fn clear_all_of_a_temp_store_in_a_pair_1564_measures_runs() {
    for cell in [
        "ArrHolderSharedUnused",
        "ArrHolderUnsharedWritten",
        "ArrSiShared",
        "LabelHolderWritten",
        "LabelHolderShared",
        "IntTestShared",
    ] {
        assert_eq!(temp_store(cell), ("done".to_string(), true), "{cell}");
    }
}

// The cells no probe measures refuse: the Label store shared under a SingleInstance owner, as a test codeunit global,
// behind a by-value parameter of the routine that runs ClearAll; a Label between the RecordRef array and the Integer
// array, a Label with Locked; "CLS Arr Store" under a codeunit with Subtype = Normal; "CLB Store" with one more Integer
// under a SingleInstance owner; a wrapper never written whose store an alias shares; an array of stores whose shared
// element holds no rows.
#[test]
fn clear_all_of_a_temp_store_in_an_unmeasured_pair_refuses() {
    for (cell, why) in [
        ("LabelSiShared", "whose instance another variable shares"),
        ("LabelTestWritten", "whose instance holds temporary rows"),
        ("LabelValueParam", "whose instance another variable shares"),
        ("LabelBetweenArrays", "whose instance holds temporary rows"),
        ("LabelLocked", "whose instance holds temporary rows"),
        ("ArrNormalWritten", "whose instance holds temporary rows"),
        ("PlusSiWritten", "whose instance holds temporary rows"),
        (
            "WrapperInnerSharedUnwritten",
            "whose inner store another variable shares",
        ),
    ] {
        let got = temp_store(cell).0;
        assert!(
            got.starts_with("refused: ClearAll of a global codeunit variable") && got.contains(why),
            "{cell}: {got}"
        );
    }
    let got = temp_store("ElementSharedNoRows").0;
    assert!(
        got.starts_with("refused: ClearAll of a global array whose codeunit element another variable shares"),
        "{got}"
    );
}
