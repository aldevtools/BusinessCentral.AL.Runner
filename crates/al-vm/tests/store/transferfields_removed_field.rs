//! TransferFields and a field declared ObsoleteState = Removed (al-runner issue 8494). The reporter's repro passes on
//! Business Central 29.0 (W1 OnPrem): a removed Code field of the target is skipped past a live Boolean field of the
//! source, both fields of one app (two tables of one app, or two table extensions of one app), with the default
//! arguments; the same field not removed raises the type mismatch. That skip runs on the assumption
//! transferfields-skips-removed-target-field; into a removed field of the very same type the skip is unflagged, and every
//! other shape past a removed target field is refused. A removed field of the source against a live field of the target
//! is refused where the field would be copied or raise an error, and skipped where it is skipped anyway.

use al_builtins::assumptions::{Assumption, take_hits};
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

const A: &str = r#"
table 95860 "RF Source" { fields { field(1; "No."; Code[20]) { } field(2; "My Flag"; Boolean) { } } keys { key(PK; "No.") { } } }
table 95861 "RF Target"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Old Code"; Code[5]) { ObsoleteState = Removed; ObsoleteReason = 'Not used.'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}
table 95862 "RF Target Live" { fields { field(1; "No."; Code[20]) { } field(2; "Old Code"; Code[5]) { } } keys { key(PK; "No.") { } } }
table 95863 "RF Ext Source" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 95864 "RF Ext Target" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 95865 "RF Removed Source"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Gone Flag"; Boolean) { ObsoleteState = Removed; ObsoleteReason = 'Not used.'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}
table 95866 "RF Kept Target" { fields { field(1; "No."; Code[20]) { } field(2; "Kept Flag"; Boolean) { } } keys { key(PK; "No.") { } } }
table 95870 "RF Text Source" { fields { field(1; "No."; Code[20]) { } field(2; "Note"; Text[30]) { } } keys { key(PK; "No.") { } } }
table 95871 "RF Int Target"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Old Number"; Integer) { ObsoleteState = Removed; ObsoleteReason = 'Not used.'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}
table 95872 "RF Code20 Source" { fields { field(1; "No."; Code[20]) { } field(2; "Long Code"; Code[20]) { } } keys { key(PK; "No.") { } } }
table 95873 "RF Code5 Source" { fields { field(1; "No."; Code[20]) { } field(2; "Short Code"; Code[5]) { } } keys { key(PK; "No.") { } } }
"#;

const B: &str = r#"
tableextension 95867 "RF Ext Source X" extends "RF Ext Source" { fields { field(95860; "My Flag"; Boolean) { } } }
tableextension 95868 "RF Ext Target X" extends "RF Ext Target"
{
    fields
    {
        field(95860; "Old Code"; Code[5]) { ObsoleteState = Removed; ObsoleteReason = 'Not used.'; ObsoleteTag = '1.0'; }
    }
}

codeunit 95869 T
{
    [TryFunction]
    local procedure TryPlain(var Tgt: Record "RF Target"; var Src: Record "RF Source"; Mode: Integer)
    begin
        case Mode of
            1: Tgt.TransferFields(Src);
            2: Tgt.TransferFields(Src, false);
            3: Tgt.TransferFields(Src, true, true);
        end;
    end;

    procedure Plain(Mode: Integer): Text
    var Src: Record "RF Source"; Tgt: Record "RF Target";
    begin
        Src."No." := 'A'; Src."My Flag" := true;
        Tgt."No." := 'T';
        if TryPlain(Tgt, Src, Mode) then exit('ok: ' + Tgt."No.");
        exit('error: ' + GetLastErrorText() + ' | ' + GetLastErrorCode());
    end;

    [TryFunction]
    local procedure TryLive(var Tgt: Record "RF Target Live"; var Src: Record "RF Source")
    begin
        Tgt.TransferFields(Src);
    end;

    procedure Live(): Text
    var Src: Record "RF Source"; Tgt: Record "RF Target Live";
    begin
        Src."No." := 'A'; Src."My Flag" := true;
        if TryLive(Tgt, Src) then exit('ok: ' + Tgt."No.");
        exit('error: ' + GetLastErrorText() + ' | ' + GetLastErrorCode());
    end;

    [TryFunction]
    local procedure TryExtension(var Tgt: Record "RF Ext Target"; var Src: Record "RF Ext Source")
    begin
        Tgt.TransferFields(Src);
    end;

    procedure Extension(): Text
    var Src: Record "RF Ext Source"; Tgt: Record "RF Ext Target";
    begin
        Src."No." := 'A'; Src."My Flag" := true;
        if TryExtension(Tgt, Src) then exit('ok: ' + Tgt."No.");
        exit('error: ' + GetLastErrorText() + ' | ' + GetLastErrorCode());
    end;

    procedure RemovedSource(): Text
    var Src: Record "RF Removed Source"; Tgt: Record "RF Kept Target";
    begin
        Src."No." := 'A';
        Tgt."No." := 'T'; Tgt."Kept Flag" := true;
        Tgt.TransferFields(Src);
        exit('ok: ' + Tgt."No." + ' ' + Format(Tgt."Kept Flag"));
    end;

    procedure Direct(Mode: Integer): Text
    var Src: Record "RF Source"; Tgt: Record "RF Target";
    begin
        Src."No." := 'A'; Src."My Flag" := true;
        Tgt."No." := 'T';
        case Mode of
            1: Tgt.TransferFields(Src, true);
            2: Tgt.TransferFields(Src, false);
            3: Tgt.TransferFields(Src, true, true);
        end;
        exit('ok: ' + Tgt."No.");
    end;

    procedure IntFromText(): Text
    var Src: Record "RF Text Source"; Tgt: Record "RF Int Target";
    begin
        Src."No." := 'A'; Src.Note := 'x';
        Tgt.TransferFields(Src);
        exit('ok: ' + Tgt."No.");
    end;

    procedure CodeTooLong(): Text
    var Src: Record "RF Code20 Source"; Tgt: Record "RF Target";
    begin
        Src."No." := 'A'; Src."Long Code" := 'ABCDEFGHIJ';
        Tgt.TransferFields(Src);
        exit('ok: ' + Tgt."No.");
    end;

    procedure SameType(): Text
    var Src: Record "RF Code5 Source"; Tgt: Record "RF Target";
    begin
        Src."No." := 'A'; Src."Short Code" := 'ABC';
        Tgt.TransferFields(Src);
        exit('ok: ' + Tgt."No.");
    end;

    procedure BothRemoved(): Text
    var Src: Record "RF Removed Source"; Tgt: Record "RF Target";
    begin
        Src."No." := 'A';
        Tgt.TransferFields(Src);
        exit('ok: ' + Tgt."No.");
    end;

    procedure OtherApp(): Text
    var Src: Record "RF B Source"; Tgt: Record "RF Target";
    begin
        Src."No." := 'A'; Src."My Flag" := true;
        Tgt.TransferFields(Src);
        exit('ok: ' + Tgt."No.");
    end;

    procedure RemovedSourceSkipped(): Text
    var Src: Record "RF Removed Source"; Tgt: Record "RF Target Live";
    begin
        Src."No." := 'A';
        Tgt."No." := 'T'; Tgt."Old Code" := 'KEEP';
        Tgt.TransferFields(Src, true, true);
        exit('ok: ' + Tgt."No." + ' ' + Tgt."Old Code");
    end;

    procedure RemovedSourceMismatch(): Text
    var Src: Record "RF Removed Source"; Tgt: Record "RF Target Live";
    begin
        Src."No." := 'A';
        Tgt."No." := 'T'; Tgt."Old Code" := 'KEEP';
        Tgt.TransferFields(Src);
        exit('ok: ' + Tgt."No." + ' ' + Tgt."Old Code");
    end;
}

table 95874 "RF B Source" { fields { field(1; "No."; Code[20]) { } field(2; "My Flag"; Boolean) { } } keys { key(PK; "No.") { } } }
"#;

fn built() -> al_testkit::Built {
    al_testkit::Built::new(al_testkit::compile_apps(vec![app("a", &[], A), app("b", &["a"], B)]))
}

fn call(b: &mut al_testkit::Built, routine: &str, args: Vec<Value>) -> String {
    let (oi, ri) = b.routine_in("T", routine);
    let (v, _) = b.call(oi, ri, args).unwrap_or_else(|e| panic!("{routine}: {e:?}"));
    al_testkit::text(Ok(v))
}

fn refusal(b: &mut al_testkit::Built, routine: &str, args: Vec<Value>) -> String {
    let (oi, ri) = b.routine_in("T", routine);
    match b.call(oi, ri, args) {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

/// The repro's TransferFieldsIntoTableWithRemovedFieldOfOtherType: field 2 of the target is a removed Code[5], the
/// source's a live Boolean of the same app; TransferFields with the default arguments copies the primary key and raises
/// nothing, also with InitPrimaryKeyFields true passed. The repro's BaseAppTransferFieldsWithRemovedFieldOfOtherType in its
/// shape: the two fields are table extension fields of one app. Each call marks the assumption.
#[test]
fn the_repro_shape_is_skipped_on_an_assumption() {
    let mut b = built();
    let a = Assumption::TransferFieldsSkipsRemovedTargetField;
    for (routine, args) in [
        ("Plain", vec![Value::Integer(1)]),
        ("Direct", vec![Value::Integer(1)]),
        ("Extension", vec![]),
    ] {
        take_hits();
        assert_eq!(call(&mut b, routine, args), "ok: A", "{routine}");
        assert!(take_hits().contains(&a), "{routine} not flagged");
    }
}

/// Into a removed field of the very same type (Code[5] from Code[5]) the copy could not fail and no reader sees the field:
/// skipped, unflagged.
#[test]
fn a_removed_target_field_of_the_same_type_is_skipped_unflagged() {
    let mut b = built();
    take_hits();
    assert_eq!(call(&mut b, "SameType", vec![]), "ok: A");
    assert!(!take_hits().contains(&Assumption::TransferFieldsSkipsRemovedTargetField));
}

/// Past a removed target field of another type, every shape the repro did not measure is refused: other types (an Integer
/// field from a Text field, a Code[20] value too long for the removed Code[5]), InitPrimaryKeyFields false,
/// SkipFieldsNotMatchingType true, a source field that is removed too, and fields of two apps.
#[test]
fn other_shapes_past_a_removed_target_field_are_refused() {
    let mut b = built();
    for (routine, args) in [
        ("IntFromText", vec![]),
        ("CodeTooLong", vec![]),
        ("Direct", vec![Value::Integer(2)]),
        ("Direct", vec![Value::Integer(3)]),
        ("BothRemoved", vec![]),
        ("OtherApp", vec![]),
    ] {
        let why = refusal(&mut b, routine, args);
        assert!(why.contains("declared ObsoleteState = Removed"), "{routine}: {why}");
    }
}

/// The repro's control TransferFieldsIntoTableWithLiveFieldOfOtherType: the same field, not removed, raises the type
/// mismatch.
#[test]
fn a_live_field_of_another_type_still_raises_the_type_mismatch() {
    let mut b = built();
    take_hits();
    let r = call(&mut b, "Live", vec![]);
    assert!(
        r.starts_with("error: The following fields must have the same type:") && r.ends_with("| DB:TypeMisMatch"),
        "{r}"
    );
    assert!(!take_hits().contains(&Assumption::TransferFieldsSkipsRemovedTargetField));
}

/// A removed field of the source against a live field of the target, where the field would be copied (the same type) or
/// raise the type mismatch (another type, the default arguments): whether the target keeps its value is not measured.
#[test]
fn a_removed_source_field_that_would_move_or_raise_is_refused() {
    let mut b = built();
    for routine in ["RemovedSource", "RemovedSourceMismatch"] {
        let why = refusal(&mut b, routine, vec![]);
        assert!(why.contains("declared ObsoleteState = Removed"), "{routine}: {why}");
    }
}

/// With SkipFieldsNotMatchingType a field of another type is skipped whether or not the source's field is removed: the
/// target keeps its value, as before the removed field was looked at.
#[test]
fn a_removed_source_field_of_another_type_stays_skipped_with_skip_fields_not_matching_type() {
    let mut b = built();
    take_hits();
    assert_eq!(call(&mut b, "RemovedSourceSkipped", vec![]), "ok: A KEEP");
    assert!(!take_hits().contains(&Assumption::TransferFieldsSkipsRemovedTargetField));
}
