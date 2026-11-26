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
codeunit 50926 WyTnInner { TableNo = WyRow; var N: Integer; trigger OnRun() begin end; }
codeunit 50907 WyTableNo { TableNo = WyRow; var N: Integer; Inner: Codeunit WyTnInner;
    trigger OnRun() begin end;
    procedure Bump() begin N += 1; end;
    procedure Get(): Integer begin exit(N); end; }
interface WyFace { procedure Get(): Integer; }
codeunit 50908 WyByteLeaf implements WyFace { var B: Byte; N: Integer;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50909 WyFaceStore { var N: Integer; F: Interface WyFace;
    procedure Bump() var L: Codeunit WyByteLeaf; begin F := L; N += 1; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50916 WyJson { var N: Integer; J: JsonObject;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50917 WyRef { var N: Integer; R: RecordRef;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50918 WyLabel { var N: Integer; L: Label 'x', Comment = 'c';
    procedure Bump() begin N += 1; end;
    procedure Get(): Integer begin exit(N); end; }
codeunit 50919 WyLocked { var N: Integer; L: Label 'x', Locked = true;
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
codeunit 50921 OFace { var S: Codeunit WyFaceStore;
    procedure Shared(): Text var X: Codeunit WyFaceStore; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50922 OJson { var S: Codeunit WyJson;
    procedure Shared(): Text var X: Codeunit WyJson; begin X := S; X.Get(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50923 ORef { var S: Codeunit WyRef;
    procedure Shared(): Text var X: Codeunit WyRef; begin X := S; X.Get(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50924 OLabel { var S: Codeunit WyLabel;
    procedure Shared(): Text var X: Codeunit WyLabel; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50925 OLocked { var S: Codeunit WyLocked;
    procedure Shared(): Text var X: Codeunit WyLocked; begin X := S; X.Bump(); ClearAll(); exit(Format(S.Get())); end; }
codeunit 50920 T {
    procedure ByteShared(): Text var O: Codeunit OByte; begin exit(O.Shared()); end;
    procedure NestedShared(): Text var O: Codeunit OOuter; begin exit(O.Shared()); end;
    procedure CycleShared(): Text var O: Codeunit OCycle; begin exit(O.Shared()); end;
    procedure UnwrittenShared(): Text var O: Codeunit OUnwritten; begin exit(O.Shared()); end;
    procedure SingleShared(): Text var O: Codeunit OSingle; begin exit(O.Shared()); end;
    procedure TableNoShared(): Text var O: Codeunit OTableNo; begin exit(O.Shared()); end;
    procedure FaceShared(): Text var O: Codeunit OFace; begin exit(O.Shared()); end;
    procedure JsonShared(): Text var O: Codeunit OJson; begin exit(O.Shared()); end;
    procedure RefShared(): Text var O: Codeunit ORef; begin exit(O.Shared()); end;
    procedure LabelShared(): Text var O: Codeunit OLabel; begin exit(O.Shared()); end;
    procedure LockedShared(): Text var O: Codeunit OLocked; begin exit(O.Shared()); end;
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
    refused_naming("TableNoShared", &["store \"WyTableNo\"", "codeunit \"WyTnInner\" has a TableNo", "depth 1"]);
}

#[test]
fn an_untouched_handle_global_is_no_write() {
    for (name, owner) in [("JsonShared", "OJson"), ("RefShared", "ORef")] {
        refused_naming(name, &[&format!("owner \"{owner}\""), "called but nothing written"]);
    }
}

#[test]
fn a_label_with_a_property_is_named() {
    refused_naming(
        "LabelShared",
        &[
            "store \"WyLabel\"",
            "global \"L\" of \"WyLabel\" is a Label with a property",
        ],
    );
    refused_naming(
        "LockedShared",
        &[
            "store \"WyLocked\"",
            "global \"L\" of \"WyLocked\" is a Label with a property",
        ],
    );
}

#[test]
fn an_interface_holding_an_unmeasured_codeunit_is_named() {
    refused_naming(
        "FaceShared",
        &[
            "store \"WyFaceStore\"",
            "an Interface global holds an instance of a codeunit that is no store",
        ],
    );
}
