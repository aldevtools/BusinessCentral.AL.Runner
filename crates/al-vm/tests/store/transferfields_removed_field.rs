//! TransferFields and a field declared ObsoleteState = Removed (al-runner issue 8494). The reporter's repro passes on
//! Business Central 29.0 (W1 OnPrem): a removed field of the target is skipped, also when the source's field of that
//! number has another type, on two tables of one app and on two table extensions of one app; the same field not removed
//! raises the type mismatch. The skip runs on the assumption transferfields-skips-removed-target-field. A removed field
//! of the source against a live field of the target is not measured: refused.

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
}
"#;

fn built() -> al_testkit::Built {
    al_testkit::Built::new(al_testkit::compile_apps(vec![app("a", &[], A), app("b", &["a"], B)]))
}

fn call(b: &mut al_testkit::Built, routine: &str, args: Vec<Value>) -> String {
    let (oi, ri) = b.routine_in("T", routine);
    let (v, _) = b.call(oi, ri, args).unwrap_or_else(|e| panic!("{routine}: {e:?}"));
    al_testkit::text(Ok(v))
}

/// The repro's TransferFieldsIntoTableWithRemovedFieldOfOtherType: field 2 of the target is a removed Code[5], the
/// source's a Boolean; TransferFields copies the primary key (unless InitPrimaryKeyFields is false) and raises nothing.
/// The repro's BaseAppTransferFieldsWithRemovedFieldOfOtherType in its shape: the two fields are table extension fields of
/// one app. Each call marks the assumption.
#[test]
fn a_removed_target_field_is_skipped_on_an_assumption() {
    let mut b = built();
    let a = Assumption::TransferFieldsSkipsRemovedTargetField;
    for (mode, pk) in [(1, "A"), (2, "T"), (3, "A")] {
        take_hits();
        assert_eq!(
            call(&mut b, "Plain", vec![Value::Integer(mode)]),
            format!("ok: {pk}"),
            "mode {mode}"
        );
        assert!(take_hits().contains(&a), "mode {mode} not flagged");
    }
    take_hits();
    assert_eq!(call(&mut b, "Extension", vec![]), "ok: A");
    assert!(take_hits().contains(&a), "extension fields not flagged");
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

/// A removed field of the source against a live field of the target: whether the target keeps its value is not measured.
#[test]
fn a_removed_source_field_into_a_live_field_is_refused() {
    let mut b = built();
    let (oi, ri) = b.routine_in("T", "RemovedSource");
    match b.call(oi, ri, vec![]) {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("Removed"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
