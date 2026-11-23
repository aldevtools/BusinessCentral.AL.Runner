//! `setrange_args_written`: the id lowering gives a `FieldRef.SetRange` call by how its arguments are written (al-vm
//! `fieldref_setrange_convert` converts a value of another type than the field's by it, al-runner issue 8857).

use crate::workspace::{AppSource, Workspace};
use al_ir as ir;

/// The builtin id of the first statement of procedure `P` with locals `vars` and body `body`.
fn first_call_id(vars: &str, body: &str) -> String {
    let src = format!(
        "table 50100 T {{ fields {{ field(1; Id; Integer) {{ }} field(2; D; Date) {{ }} }} }}
codeunit 50100 C {{ procedure P() var R: RecordRef; F: FieldRef; F2: FieldRef; {vars} begin {body} end; }}"
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
fn plain_value_arguments_keep_the_catalog_id() {
    for (vars, body) in [
        ("", "F.SetRange();"),
        ("", "F.SetRange(20240101);"),
        ("X: Integer;", "F.SetRange(X, X);"),
        ("X: Text;", "F.SetRange(X);"),
        ("X: Date;", "F.SetRange(X, 20240101D);"),
    ] {
        assert_eq!(first_call_id(vars, body), "FieldRef.SetRange#1", "{body}");
    }
}

#[test]
fn variant_arguments_take_the_variant_id() {
    assert_eq!(
        first_call_id("V: Variant;", "F.SetRange(V);"),
        "FieldRef.SetRangeVariant#1"
    );
    assert_eq!(
        first_call_id("V: Variant;", "F.SetRange(V, V);"),
        "FieldRef.SetRangeVariant#1"
    );
}

#[test]
fn a_mix_or_a_fieldref_value_takes_the_untyped_id() {
    for (vars, body) in [
        ("V: Variant;", "F.SetRange(V, 20240101);"),
        ("V: Variant;", "F.SetRange(20240101, V);"),
        ("", "F.SetRange(F2.Value());"),
        ("", "F.SetRange(F2.Value);"),
    ] {
        assert_eq!(first_call_id(vars, body), "FieldRef.SetRangeUntyped#1", "{body}");
    }
}
