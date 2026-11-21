//! TransferFields and a field declared ObsoleteState = Removed: the shapes next to the measured ones (corpus PR 1486,
//! record/TransferFieldsRemovedField.al). A measured type pair of one app skips the removed field on the assumption
//! transferfields-removed-field; any other shape keeps main's skip, keeps main's copy into a removed target field, and
//! refuses where main copied a removed source field or raised an error.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::AppSource;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

const REMOVED: &str = "ObsoleteState = Removed; ObsoleteReason = 'Not used.'; ObsoleteTag = '1.0';";

const A: &str = r#"
table 95880 "N Src"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; F2; Boolean) { }
        field(3; F3; Code[20]) { }
        field(4; F4; Text[30]) { }
        field(5; F5; Integer) { }
    }
    keys { key(PK; "No.") { } }
}
table 95881 "N T IntFromText" { fields { field(1; "No."; Code[20]) { } field(4; F4; Integer) { #R } } keys { key(PK; "No.") { } } }
table 95882 "N T LongCode" { fields { field(1; "No."; Code[20]) { } field(3; F3; Code[5]) { #R } } keys { key(PK; "No.") { } } }
table 95883 "N T IntSame" { fields { field(1; "No."; Code[20]) { } field(5; F5; Integer) { #R } } keys { key(PK; "No.") { } } }
table 95884 "N T DecFromInt" { fields { field(1; "No."; Code[20]) { } field(5; F5; Decimal) { #R } } keys { key(PK; "No.") { } } }
table 95885 "N T TextFromText" { fields { field(1; "No."; Code[20]) { } field(4; F4; Text[30]) { #R } } keys { key(PK; "No.") { } } }
table 95886 "N S Removed"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; F2; Boolean) { #R }
        field(5; F5; Integer) { #R }
    }
    keys { key(PK; "No.") { } }
}
table 95887 "N T Live" { fields { field(1; "No."; Code[20]) { } field(2; F2; Integer) { } field(5; F5; Integer) { } } keys { key(PK; "No.") { } } }
table 95888 "N S RemovedDec" { fields { field(1; "No."; Code[20]) { } field(6; F6; Decimal) { #R } } keys { key(PK; "No.") { } } }
table 95889 "N T LiveDec" { fields { field(1; "No."; Code[20]) { } field(6; F6; Decimal) { } } keys { key(PK; "No.") { } } }
table 95890 "N S RemovedBool" { fields { field(1; "No."; Code[20]) { } field(7; F7; Boolean) { #R } } keys { key(PK; "No.") { } } }
table 95896 "N S RemovedInt" { fields { field(1; "No."; Code[20]) { } field(5; F5; Integer) { #R } } keys { key(PK; "No.") { } } }
table 95891 "N T LiveDate" { fields { field(1; "No."; Code[20]) { } field(7; F7; Date) { } } keys { key(PK; "No.") { } } }
"#;

const B: &str = r#"
table 95892 "X T BoolToCode" { fields { field(1; "No."; Code[20]) { } field(2; F2; Code[5]) { #R } } keys { key(PK; "No.") { } } }
table 95893 "X T LongCode" { fields { field(1; "No."; Code[20]) { } field(3; F3; Code[5]) { #R } } keys { key(PK; "No.") { } } }
table 95894 "X S RemovedInt" { fields { field(1; "No."; Code[20]) { } field(5; F5; Integer) { #R } } keys { key(PK; "No.") { } } }

codeunit 95895 T
{
    local procedure Source(var Src: Record "N Src")
    begin
        Src."No." := 'A'; Src.F2 := true; Src.F3 := 'ABCDEFGHIJ'; Src.F4 := '12'; Src.F5 := 4;
    end;

    procedure IntFromText(Skip: Boolean): Text
    var Src: Record "N Src"; Tgt: Record "N T IntFromText";
    begin
        Source(Src); Tgt."No." := 'T'; Tgt.TransferFields(Src, true, Skip); exit(Tgt."No.");
    end;

    procedure LongCode(): Text
    var Src: Record "N Src"; Tgt: Record "N T LongCode";
    begin
        Source(Src); Tgt.TransferFields(Src); exit(Tgt."No.");
    end;

    procedure IntSame(Skip: Boolean): Text
    var Src: Record "N Src"; Tgt: Record "N T IntSame";
    begin
        Source(Src); Tgt.TransferFields(Src, true, Skip); exit(Tgt."No.");
    end;

    procedure DecFromInt(): Text
    var Src: Record "N Src"; Tgt: Record "N T DecFromInt";
    begin
        Source(Src); Tgt.TransferFields(Src); exit(Tgt."No.");
    end;

    procedure TextFromText(): Text
    var Src: Record "N Src"; Tgt: Record "N T TextFromText";
    begin
        Source(Src); Tgt.TransferFields(Src); exit(Tgt."No.");
    end;

    procedure RemovedIntoLive(Skip: Boolean): Text
    var Src: Record "N S Removed"; Tgt: Record "N T Live";
    begin
        Src."No." := 'A'; Tgt."No." := 'T'; Tgt.F2 := 8; Tgt.F5 := 7;
        Tgt.TransferFields(Src, true, Skip);
        exit(Tgt."No." + ' ' + Format(Tgt.F2) + ' ' + Format(Tgt.F5));
    end;

    procedure RemovedInt(): Text
    var Src: Record "N S RemovedInt"; Tgt: Record "N T Live";
    begin
        Src."No." := 'A'; Tgt.F5 := 7; Tgt.TransferFields(Src); exit(Format(Tgt.F5));
    end;

    procedure RemovedDec(): Text
    var Src: Record "N S RemovedDec"; Tgt: Record "N T LiveDec";
    begin
        Src."No." := 'A'; Tgt.F6 := 7; Tgt.TransferFields(Src); exit(Format(Tgt.F6));
    end;

    procedure RemovedBoolSkip(): Text
    var Src: Record "N S RemovedBool"; Tgt: Record "N T LiveDate";
    begin
        Src."No." := 'A'; Tgt.F7 := 20240131D; Tgt.TransferFields(Src, true, true);
        exit(Tgt."No." + ' ' + Format(Tgt.F7, 0, 9));
    end;

    procedure CrossBoolToCode(): Text
    var Src: Record "N Src"; Tgt: Record "X T BoolToCode";
    begin
        Source(Src); Tgt.TransferFields(Src); exit(Tgt."No.");
    end;

    procedure CrossLongCode(): Text
    var Src: Record "N Src"; Tgt: Record "X T LongCode";
    begin
        Source(Src); Tgt.TransferFields(Src); exit(Tgt."No.");
    end;

    procedure CrossRemovedInt(): Text
    var Src: Record "X S RemovedInt"; Tgt: Record "N T Live";
    begin
        Src."No." := 'A'; Tgt.F5 := 7; Tgt.TransferFields(Src); exit(Format(Tgt.F5));
    end;
}
"#;

fn built() -> al_testkit::Built {
    let (a, b) = (A.replace("#R", REMOVED), B.replace("#R", REMOVED));
    al_testkit::Built::new(al_testkit::compile_apps(vec![app("a", &[], &a), app("b", &["a"], &b)]))
}

const FLAG: Assumption = Assumption::TransferFieldsRemovedField;

/// The result of `routine` and whether it marked the assumption.
fn run(b: &mut al_testkit::Built, routine: &str, args: Vec<Value>) -> (String, bool) {
    let (oi, ri) = b.routine_in("T", routine);
    take_hits();
    let (v, _) = b.call(oi, ri, args).unwrap_or_else(|e| panic!("{routine}: {e:?}"));
    (al_testkit::text(Ok(v)), take_hits().contains(&FLAG))
}

fn refused(b: &mut al_testkit::Built, routine: &str) -> String {
    let (oi, ri) = b.routine_in("T", routine);
    match b.call(oi, ri, vec![]) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

/// Measured pairs of one app: Text into a removed Integer with and without SkipFieldsNotMatchingType
/// (TransferFieldsRemovedField.al:548-568), a 10 character Code value into a removed Code[5] (:608), a removed Integer
/// and a removed Boolean source into live Integer fields, which keep their values (:710-802).
#[test]
fn measured_pairs_of_one_app_skip_the_removed_field_flagged() {
    let mut b = built();
    for skip in [false, true] {
        assert_eq!(
            run(&mut b, "IntFromText", vec![Value::Boolean(skip)]),
            ("A".into(), true)
        );
        assert_eq!(
            run(&mut b, "RemovedIntoLive", vec![Value::Boolean(skip)]),
            ("A 8 7".into(), true),
            "skip {skip}"
        );
    }
    assert_eq!(run(&mut b, "LongCode", vec![]), ("A".into(), true));
}

/// A removed Integer source field into a live Integer field: the target keeps its value
/// (TransferFieldsRemovedField.al:779 RemovedSource_SameType_Default). Main copied the removed field's 0 over it.
#[test]
fn a_removed_integer_source_leaves_a_live_integer_target_as_it_was() {
    let mut b = built();
    assert_eq!(run(&mut b, "RemovedInt", vec![]), ("7".into(), true));
}

/// Out of the measured pairs, main's answer stays: a copy into a removed target field of the very same type
/// (Integer with SkipFieldsNotMatchingType true, Text[30]) cannot fail and no reader sees the field; a removed Boolean source
/// against a live Date field with SkipFieldsNotMatchingType true is skipped as any field of another type
/// (record/TestRecordTransferFields.al:14-95); across apps a removed Code[5] target against a Boolean source is skipped
/// as any field of another kind (record/transferfields-cross-app/Y38TransferXApp.al:449-499). None of them is flagged.
#[test]
fn unmeasured_shapes_keep_main_s_answer_unflagged() {
    let mut b = built();
    assert_eq!(run(&mut b, "IntSame", vec![Value::Boolean(true)]), ("A".into(), false));
    assert_eq!(run(&mut b, "TextFromText", vec![]), ("A".into(), false));
    assert_eq!(run(&mut b, "RemovedBoolSkip", vec![]), ("A 2024-01-31".into(), false));
    assert_eq!(run(&mut b, "CrossBoolToCode", vec![]), ("A".into(), false));
}

/// Out of the measured pairs, where main copied a removed source field or raised an error: refused. Integer into a
/// removed Decimal (main: DB:TypeMisMatch), a removed Decimal source into a live Decimal (main: copied 0), across apps a
/// 10 character Code value into a removed Code[5] (main: the length error) and a removed Integer source into a live Integer
/// (main: copied 0).
#[test]
fn unmeasured_shapes_that_main_copied_or_failed_are_refused() {
    let mut b = built();
    for routine in ["DecFromInt", "RemovedDec", "CrossLongCode", "CrossRemovedInt"] {
        let why = refused(&mut b, routine);
        assert!(why.contains("ObsoleteState = Removed"), "{routine}: {why}");
    }
}
