//! `setrange_written_id`: the id lowering gives a `FieldRef.SetRange`, `FieldRef.SetFilter` or `Record.SetRange` call by how
//! its values are written (al-vm `setrange_convert` converts a value of another type than the field's by it, al-runner
//! issue 8857).

use crate::workspace::{AppSource, Workspace};
use al_ir as ir;

/// The builtin id of the first statement of procedure `P` with locals `vars` and body `body`.
fn first_call_id(vars: &str, body: &str) -> String {
    let src = format!(
        "table 50100 T {{ fields {{ field(1; Id; Integer) {{ }} field(2; D; Date) {{ }} }} }}
codeunit 50100 C {{ procedure P() var R: RecordRef; F: FieldRef; F2: FieldRef; Rec: Record T; {vars} begin {body} end; }}"
    );
    let ws = Workspace::load(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    );
    let members = crate::members::build(&ws);
    let checked = crate::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    let l = super::lower_with_positions(&ws, &members, &checked, false);
    let p = l
        .program
        .objects
        .iter()
        .flat_map(|o| &o.routines)
        .find(|r| r.name == "P")
        .expect("P");
    match &p.body[0] {
        ir::Stmt::Expr(ir::Expr::Builtin { id, .. }) => id.name().to_string(),
        s => panic!("{s:?}"),
    }
}

#[test]
fn typed_values_take_the_typed_id() {
    for (vars, body) in [
        ("", "F.SetRange(20240101);"),
        ("X: Integer;", "F.SetRange(X, X);"),
        ("X: Text;", "F.SetRange(X);"),
        ("X: Date;", "F.SetRange(X, 20240101D);"),
    ] {
        assert_eq!(first_call_id(vars, body), "FieldRef.SetRangeTyped#1", "{body}");
    }
    assert_eq!(first_call_id("X: Code[20];", "F.SetFilter('%1', X);"), "FieldRef.SetFilterTyped#1");
}

#[test]
fn variant_values_take_the_variant_id() {
    for body in ["F.SetRange(V);", "F.SetRange(V, V);"] {
        assert_eq!(first_call_id("V: Variant;", body), "FieldRef.SetRangeVariant#1", "{body}");
    }
    assert_eq!(first_call_id("V: Variant;", "F.SetFilter('%1', V);"), "FieldRef.SetFilterVariant#1");
    assert_eq!(first_call_id("V: Variant;", "Rec.SetRange(D, V);"), "Record.SetRangeVariant#1");
}

#[test]
fn a_variant_and_a_typed_value_take_the_mixed_id() {
    for body in ["F.SetRange(V, 20240101);", "F.SetRange(20240101, V);"] {
        assert_eq!(first_call_id("V: Variant;", body), "FieldRef.SetRangeMixed#1", "{body}");
    }
}

#[test]
fn every_other_shape_keeps_the_catalog_id() {
    for (vars, body, id) in [
        ("", "F.SetRange();", "FieldRef.SetRange#1"),
        ("", "F.SetRange(F2.Value());", "FieldRef.SetRange#1"),
        ("", "F.SetRange(F2.Value);", "FieldRef.SetRange#1"),
        ("V: Variant;", "F.SetRange(V, F2.Value());", "FieldRef.SetRange#1"),
        ("", "F.SetRange(F2);", "FieldRef.SetRange#1"),
        ("V: Variant;", "F.SetFilter('%1|%2', V, V);", "FieldRef.SetFilter#1"),
        ("", "F.SetFilter('%1', F2.Value());", "FieldRef.SetFilter#1"),
        ("", "F.SetFilter('a');", "FieldRef.SetFilter#1"),
        ("X: Date;", "Rec.SetRange(D, X);", "Record.SetRange#1"),
        ("V: Variant;", "Rec.SetRange(D, V, V);", "Record.SetRange#1"),
        ("", "Rec.SetRange(D);", "Record.SetRange#1"),
    ] {
        assert_eq!(first_call_id(vars, body), id, "{body}");
    }
}
