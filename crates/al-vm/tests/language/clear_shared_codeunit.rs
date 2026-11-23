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
codeunit 50826 T {
    local procedure ClearVal(P: Codeunit Store; Q: Codeunit Store): Integer begin Clear(P); P.Set(5); exit(Q.Get()); end;
    local procedure ClearVar(var P: Codeunit Store) begin Clear(P); end;
    local procedure ClearVarVar(var P: Codeunit Store) begin ClearVar(P); end;
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
    let got = match b.run_in("T", cell) {
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
// with two globals, ClearAll in a SingleInstance codeunit, ClearAll while a routine of X runs. A used by-value parameter
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
            "ClearAllSiOwnerShared",
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
}

// BindSubscription makes the instance (al-vm `dispatch.rs`, `ensure_initialized` before the binding), so a bound X is never
// unused: Clear takes main's arm for a used X, with no mark.
// A var parameter handed on as a var parameter keeps one more copy of the caller's variable: it counts as shared, so the
// Clear is marked even when nothing else holds the instance (the value is main's either way); one another holder shares
// gets the measured fresh instance.
#[test]
fn a_var_parameter_of_a_var_parameter_counts_its_copies_as_holders() {
    assert_eq!(shape("ClearVarVarUnshared", false), ("5".to_string(), true));
    assert_eq!(shape("ClearVarVarShared", false), ("0|5".to_string(), true));
}

#[test]
fn a_bound_codeunit_variable_is_never_unused() {
    assert_eq!(shape("ClearBoundNeverCalled", false), ("0|5".to_string(), false));
}
