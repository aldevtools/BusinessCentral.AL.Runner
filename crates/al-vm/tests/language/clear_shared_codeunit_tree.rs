//! ClearAll of a global codeunit variable whose store is a tree ("CWT", ClearSharedCodeunitWhseTree.al, corpus PR 1620,
//! merged, not pinned; assumption clear-all-register-tree): a node RecordRef or FieldRef another variable shares refuses.
//! The neighbor app tests/neighbors/fix-9194-tree runs the same shapes against a main binary.

use al_builtins::assumptions::{Assumption, take_hits};

use super::clear_shared_codeunit::assert_refused;

const TREE: &str = r#"
table 50100 Row { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
codeunit 50101 Node
{
    var RR: RecordRef;
    procedure Set() begin RR.Open(50100); end;
    procedure GetRR(var X: RecordRef) begin X := RR; end;
    procedure GetFR(var X: FieldRef) begin X := RR.Field(1); end;
}
codeunit 50102 Reg
{
    TableNo = Row;
    var Tmp: Record Row temporary; N: Codeunit Node;
    procedure Set() begin Tmp."No." := 1; Tmp.Insert(); N.Set(); end;
    procedure GetRR(var X: RecordRef) begin N.GetRR(X); end;
    procedure GetFR(var X: FieldRef) begin N.GetFR(X); end;
}
codeunit 50103 Owner
{
    var Reg: Codeunit Reg;
    procedure Prime() begin Reg.Set(); end;
    procedure GetRR(var X: RecordRef) begin Reg.GetRR(X); end;
    procedure GetFR(var X: FieldRef) begin Reg.GetFR(X); end;
    procedure DoClearAll() begin ClearAll(); end;
}
codeunit 50104 T
{
    procedure Alone(): Text var O: Codeunit Owner; begin O.Prime(); O.DoClearAll(); exit('done'); end;
    procedure RecordRefShared(): Text var O: Codeunit Owner; X: RecordRef; begin O.Prime(); O.GetRR(X); O.DoClearAll(); exit('done'); end;
    procedure FieldRefShared(): Text var O: Codeunit Owner; X: FieldRef; begin O.Prime(); O.GetFR(X); O.DoClearAll(); exit('done'); end;
}
"#;

/// What routine `cell` of [`TREE`] gives (its text, or `refused: ...`) and whether it ran a path of assumption
/// clear-all-register-tree.
fn tree(cell: &str) -> (String, bool) {
    take_hits();
    let got = match al_testkit::run_with(TREE, "T", cell, vec![]).map(|(v, _)| v) {
        Err(al_builtins::native::Signal::Unsupported(why)) => format!("refused: {why}"),
        other => al_testkit::text(other),
    };
    (got, take_hits().contains(&Assumption::ClearAllRegisterTree))
}

// The node's RecordRef only opened and held by the node alone: the shape the probe measures runs, flagged.
#[test]
fn a_node_record_ref_nothing_shares_runs_flagged() {
    assert_eq!(tree("Alone"), ("done".to_string(), true));
}

// `X := RR` and `X := RR.Field(1)` give `X` the node's record buffer, which the probe does not measure: refused, as main.
#[test]
fn a_node_record_ref_or_field_ref_another_variable_shares_refuses() {
    for cell in ["RecordRefShared", "FieldRefShared"] {
        let (got, hit) = tree(cell);
        assert_refused(cell, &got, "in a shape the corpus does not pin");
        assert!(!hit, "{cell}");
    }
}
