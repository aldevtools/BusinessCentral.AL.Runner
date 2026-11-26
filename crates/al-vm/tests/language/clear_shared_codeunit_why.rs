//! The reason `ClearAll()` gives when it refuses a store global whose instance another variable shares
//! (`Session::clear_all_in_place`): after the common text, it names the store codeunit, and the global kind, the nesting
//! or the (owner, holding) cell the corpus does not pin, so a reader finds which store blocks a test without a debugger.
//! A diagnostic only (ADR-009): which shapes refuse does not change.

use crate::clear_shared_codeunit::{assert_refused, run_cell};

const SOURCE: &str = "
table 50900 WyRow { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 50901 WyByte { var B: Byte; N: Integer;
    procedure Bump() begin N += 1; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50902 WyOuter { var Inner: Codeunit WyInner; N: Integer;
    procedure Bump() begin N += 1; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50903 WyInner { var F: DateFormula; }
codeunit 50904 WyA { var B: Codeunit WyB; N: Integer;
    procedure Bump() begin N += 1; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50905 WyB { var A: Codeunit WyA; T: Text; }
codeunit 50906 WyKinds { var N: Integer; T: Text;
    procedure Bump() begin N += 1; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50907 WyTableNo { TableNo = WyRow; var N: Integer; T: Text;
    trigger OnRun() begin end;
    procedure Bump() begin N += 1; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50910 OByte { var S: Codeunit WyByte;
    procedure Shared(): Text var X: Codeunit WyByte; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50911 OOuter { var S: Codeunit WyOuter;
    procedure Shared(): Text var X: Codeunit WyOuter; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50912 OCycle { var S: Codeunit WyA;
    procedure Shared(): Text var X: Codeunit WyA; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50913 OUnwritten { var S: Codeunit WyKinds;
    procedure Shared(): Text var X: Codeunit WyKinds; begin X := S; X.Get(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50914 OSingle { SingleInstance = true; var S: Codeunit WyKinds;
    procedure Shared(): Text var X: Codeunit WyKinds; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50915 OTableNo { var S: Codeunit WyTableNo;
    procedure Shared(): Text var X: Codeunit WyTableNo; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50920 T {
    procedure ByteShared(): Text var O: Codeunit OByte; begin exit(O.Shared()); end;
    procedure NestedShared(): Text var O: Codeunit OOuter; begin exit(O.Shared()); end;
    procedure CycleShared(): Text var O: Codeunit OCycle; begin exit(O.Shared()); end;
    procedure UnwrittenShared(): Text var O: Codeunit OUnwritten; begin exit(O.Shared()); end;
    procedure SingleShared(): Text var O: Codeunit OSingle; begin exit(O.Shared()); end;
    procedure TableNoShared(): Text var O: Codeunit OTableNo; begin exit(O.Shared()); end;
}";

/// Asserts that `name` of [`SOURCE`] refuses with the common text followed by each part of `parts`.
fn refused_naming(name: &str, parts: &[&str]) {
    let got = run_cell(SOURCE, name, vec![]).0;
    assert_refused(
        name,
        &got,
        "ClearAll of a global codeunit variable whose instance another variable shares, in a shape the corpus does not \
         pin: ",
    );
    for part in parts {
        assert!(got.contains(part), "{name}: {got} lacks {part}");
    }
}

#[test]
fn a_global_kind_outside_the_list_is_named() {
    refused_naming(
        "ByteShared",
        &[
            "store \"WyByte\"",
            "owner \"OByte\"",
            "global \"B\" of \"WyByte\" is a Byte",
            "depth 0",
        ],
    );
}

#[test]
fn a_nested_store_names_the_codeunit_and_its_depth() {
    refused_naming(
        "NestedShared",
        &[
            "store \"WyOuter\"",
            "global \"F\" of \"WyInner\" is a DateFormula",
            "depth 1",
        ],
    );
}

#[test]
fn a_cycle_names_the_codeunit_past_depth_three() {
    refused_naming("CycleShared", &["store \"WyA\"", "nests deeper than 3", "depth 4"]);
}

#[test]
fn an_unpinned_holding_names_the_cell() {
    refused_naming(
        "UnwrittenShared",
        &[
            "store \"WyKinds\"",
            "owner \"OUnwritten\"",
            "a plain codeunit",
            "called but nothing written",
        ],
    );
    refused_naming(
        "SingleShared",
        &["store \"WyKinds\"", "a SingleInstance codeunit", "written"],
    );
}

#[test]
fn a_tableno_store_is_named() {
    refused_naming("TableNoShared", &["store \"WyTableNo\"", "has a TableNo"]);
}
