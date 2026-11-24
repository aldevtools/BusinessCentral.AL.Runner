//! al-runner issue 9022 (repro and analysis by SShadowS): a codeunit whose globals name its own codeunit type, directly
//! or through other codeunits, has no store shape the corpus measures (`Session::store_kind`), so reading its shape ends.
//! Each shape gives what its twin with a `Decimal` in place of the `Integer` gives, which main already read as no store:
//! `ClearAll()` of a global holding one keeps main's answer when nothing else holds it (a fresh instance), refuses when
//! another variable shares it, and the login check (`left_stamp`) counts it as no store. Each shape overflowed the stack
//! before the fix. Every owner has one codeunit global, so each test reaches its own call site.

use crate::clear_shared_codeunit::{assert_refused, run_cell};

const SOURCE: &str = "
table 50970 CyRow { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 50971 CySelf { var Next: Codeunit CySelf; Count: Integer;
    procedure Bump() begin Count += 1; end;
    procedure LinkSelf() begin Next := this; end;
    procedure Get(): Decimal begin exit(Count); end; }
codeunit 50972 CySelfDec { var Next: Codeunit CySelfDec; Count: Decimal;
    procedure Bump() begin Count += 1; end;
    procedure LinkSelf() begin Next := this; end;
    procedure Get(): Decimal begin exit(Count); end; }
codeunit 50973 CyA { var B: Codeunit CyB; Count: Integer;
    procedure Bump() begin Count += 1; end;
    procedure Get(): Decimal begin exit(Count); end; }
codeunit 50974 CyB { var A: Codeunit CyA; Count: Integer; }
codeunit 50975 CyADec { var B: Codeunit CyBDec; Count: Decimal;
    procedure Bump() begin Count += 1; end;
    procedure Get(): Decimal begin exit(Count); end; }
codeunit 50976 CyBDec { var A: Codeunit CyADec; Count: Decimal; }
codeunit 50977 CyRegSelf { TableNo = CyRow; var T: Record CyRow temporary; R: Record CyRow; D: Decimal; B: Boolean;
    C: Code[20]; Next: Codeunit CyRegSelf;
    trigger OnRun() begin end;
    procedure Bump() begin D += 1; end;
    procedure Get(): Decimal begin exit(D); end; }
codeunit 50978 CyRegCycle { TableNo = CyRow; var T: Record CyRow temporary; R: Record CyRow; D: Decimal; B: Boolean;
    C: Code[20]; W: Codeunit CyRegWrap;
    trigger OnRun() begin end;
    procedure Bump() begin D += 1; end;
    procedure Get(): Decimal begin exit(D); end; }
codeunit 50979 CyRegWrap { var Reg: Codeunit CyRegCycle; N: Integer; }
codeunit 50980 CyRegDec { TableNo = CyRow; var T: Record CyRow temporary; R: Record CyRow; D: Decimal; B: Boolean;
    C: Code[20]; Next: Codeunit CySelfDec;
    trigger OnRun() begin end;
    procedure Bump() begin D += 1; end;
    procedure Get(): Decimal begin exit(D); end; }
codeunit 50981 OSelf { var S: Codeunit CySelf;
    procedure Unshared(): Text begin S.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Shared(): Text var X: Codeunit CySelf; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Linked(): Text begin S.Bump(); S.LinkSelf(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50982 OSelfDec { var S: Codeunit CySelfDec;
    procedure Unshared(): Text begin S.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Shared(): Text var X: Codeunit CySelfDec; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Linked(): Text begin S.Bump(); S.LinkSelf(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50983 OCycle { var S: Codeunit CyA;
    procedure Unshared(): Text begin S.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Shared(): Text var X: Codeunit CyA; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50984 OCycleDec { var S: Codeunit CyADec;
    procedure Unshared(): Text begin S.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Shared(): Text var X: Codeunit CyADec; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50985 ORegSelf { var S: Codeunit CyRegSelf;
    procedure Unshared(): Text begin S.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Shared(): Text var X: Codeunit CyRegSelf; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50986 ORegCycle { var S: Codeunit CyRegCycle;
    procedure Unshared(): Text begin S.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Shared(): Text var X: Codeunit CyRegCycle; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50987 ORegDec { var S: Codeunit CyRegDec;
    procedure Unshared(): Text begin S.Bump(); ClearAll(); exit(Format(S.Get())); end;
    procedure Shared(): Text var X: Codeunit CyRegDec; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50988 OArr { var Arr: array[2] of Codeunit CySelf;
    procedure Shared(): Text var X: Codeunit CySelf; begin X := Arr[1]; X.Bump(); ClearAll(); exit(Format(Arr[1].Get())); end; }
codeunit 50989 OArrDec { var Arr: array[2] of Codeunit CySelfDec;
    procedure Shared(): Text var X: Codeunit CySelfDec; begin X := Arr[1]; X.Bump(); ClearAll(); exit(Format(Arr[1].Get())); end; }
codeunit 50990 SiSelf { SingleInstance = true; var S: Codeunit CySelf; procedure Touch() begin S.Get(); end; }
codeunit 50991 SiSelfDec { SingleInstance = true; var S: Codeunit CySelfDec; procedure Touch() begin S.Get(); end; }
codeunit 50992 SiCycle { SingleInstance = true; var S: Codeunit CyA; procedure Touch() begin S.Get(); end; }
codeunit 50993 SiCycleDec { SingleInstance = true; var S: Codeunit CyADec; procedure Touch() begin S.Get(); end; }
codeunit 50994 T {
    procedure SelfUnshared(): Text var O: Codeunit OSelf; begin exit(O.Unshared()); end;
    procedure SelfShared(): Text var O: Codeunit OSelf; begin exit(O.Shared()); end;
    procedure SelfLinked(): Text var O: Codeunit OSelf; begin exit(O.Linked()); end;
    procedure SelfDecUnshared(): Text var O: Codeunit OSelfDec; begin exit(O.Unshared()); end;
    procedure SelfDecShared(): Text var O: Codeunit OSelfDec; begin exit(O.Shared()); end;
    procedure SelfDecLinked(): Text var O: Codeunit OSelfDec; begin exit(O.Linked()); end;
    procedure CycleUnshared(): Text var O: Codeunit OCycle; begin exit(O.Unshared()); end;
    procedure CycleShared(): Text var O: Codeunit OCycle; begin exit(O.Shared()); end;
    procedure CycleDecUnshared(): Text var O: Codeunit OCycleDec; begin exit(O.Unshared()); end;
    procedure CycleDecShared(): Text var O: Codeunit OCycleDec; begin exit(O.Shared()); end;
    procedure RegSelfUnshared(): Text var O: Codeunit ORegSelf; begin exit(O.Unshared()); end;
    procedure RegSelfShared(): Text var O: Codeunit ORegSelf; begin exit(O.Shared()); end;
    procedure RegCycleUnshared(): Text var O: Codeunit ORegCycle; begin exit(O.Unshared()); end;
    procedure RegCycleShared(): Text var O: Codeunit ORegCycle; begin exit(O.Shared()); end;
    procedure RegDecUnshared(): Text var O: Codeunit ORegDec; begin exit(O.Unshared()); end;
    procedure RegDecShared(): Text var O: Codeunit ORegDec; begin exit(O.Shared()); end;
    procedure ArrShared(): Text var O: Codeunit OArr; begin exit(O.Shared()); end;
    procedure ArrDecShared(): Text var O: Codeunit OArrDec; begin exit(O.Shared()); end;
    procedure SiSelf() var O: Codeunit SiSelf; begin O.Touch(); end;
    procedure SiSelfDec() var O: Codeunit SiSelfDec; begin O.Touch(); end;
    procedure SiCycle() var O: Codeunit SiCycle; begin O.Touch(); end;
    procedure SiCycleDec() var O: Codeunit SiCycleDec; begin O.Touch(); end;
}
";

/// What `name` of [`SOURCE`] gives and whether it ran a path of `clear-shared-codeunit` ([`run_cell`]).
fn cell(name: &str) -> (String, bool) {
    run_cell(SOURCE, name, vec![])
}

/// Asserts that `name` and its `Decimal` twin `twin` both give `want` with no mark.
fn same_as_twin(name: &str, twin: &str, want: &str) {
    assert_eq!(cell(twin), (want.to_string(), false), "{twin}");
    assert_eq!(cell(name), (want.to_string(), false), "{name}");
}

/// Asserts that `name` and its `Decimal` twin `twin` both refuse with a reason that contains `why`.
fn refused_as_twin(name: &str, twin: &str, why: &str) {
    assert_refused(twin, &cell(twin).0, why);
    assert_refused(name, &cell(name).0, why);
}

const SHARED: &str = "whose instance another variable shares";

// The issue's repro: nothing else holds the instance (`wrapped_store_shared`), so main's fresh instance, with no mark.
#[test]
fn clear_all_of_an_unshared_self_holder_keeps_mains_answer() {
    same_as_twin("SelfUnshared", "SelfDecUnshared", "0");
}

// Another variable shares the instance, or the instance holds itself (`measured_cell`): no measured cell, so it refuses.
#[test]
fn clear_all_of_a_shared_self_holder_refuses() {
    refused_as_twin("SelfShared", "SelfDecShared", SHARED);
    refused_as_twin("SelfLinked", "SelfDecLinked", SHARED);
}

// Two codeunits that hold each other.
#[test]
fn clear_all_of_a_two_codeunit_cycle_ends() {
    same_as_twin("CycleUnshared", "CycleDecUnshared", "0");
    refused_as_twin("CycleShared", "CycleDecShared", SHARED);
}

// A TableNo codeunit with the register's globals (`register_kinds`) whose codeunit global is its own type, or a codeunit
// that holds it back.
#[test]
fn clear_all_of_a_register_that_holds_itself_ends() {
    same_as_twin("RegSelfUnshared", "RegDecUnshared", "0");
    same_as_twin("RegCycleUnshared", "RegDecUnshared", "0");
    refused_as_twin("RegSelfShared", "RegDecShared", SHARED);
    refused_as_twin("RegCycleShared", "RegDecShared", SHARED);
}

// A global array of self holders whose element another variable shares (`clear_all_store_array`).
#[test]
fn clear_all_of_an_array_of_self_holders_refuses() {
    refused_as_twin(
        "ArrShared",
        "ArrDecShared",
        "whose codeunit element another variable shares",
    );
}

/// What the login check reads after `name` made the store a SingleInstance codeunit holds directly (`login_store`): the
/// changed globals and whether one counted through its declared form.
fn login_change(name: &str) -> (Vec<std::collections::BTreeSet<usize>>, usize) {
    let mut b = al_testkit::Built::from_source(SOURCE);
    let before = b.session.left_stamp();
    b.run_in("T", name).expect("made");
    let change = before.changed(&b.session.left_stamp());
    (change.singles.into_values().collect(), change.declared_form.len())
}

#[test]
fn the_login_check_counts_a_self_holder_as_no_store() {
    assert_eq!(login_change("SiSelf"), login_change("SiSelfDec"));
}

#[test]
fn the_login_check_counts_a_two_codeunit_cycle_as_no_store() {
    assert_eq!(login_change("SiCycle"), login_change("SiCycleDec"));
}
