//! Gaps a partner app hit (np-blockers-5). Each test names the corpus pin it relies on.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

/// Runs the test codeunit `T` of the workspace app; `base` is a dependency app.
fn run_with_base(base: &str, objects: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile_apps(
        vec![
            AppSource {
                id: "base".into(),
                name: "base".into(),
                dependencies: vec![],
                files: vec![("b.al".into(), base.into())],
            },
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["base".into()],
                files: vec![("m.al".into(), objects.into())],
            },
        ],
        vec![AppKind::Dependency, AppKind::Workspace],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

// An enum field added by a table extension of another app: its value is an enum value, so a Variant
// reports IsOption. Corpus types/TestVariantExtended.al:261 pins IsOption for an enum in a Variant.
#[test]
fn enum_field_of_extension_in_another_app_reads_as_enum() {
    let base = "table 69400 \"NB5 Base\" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }";
    let main = "
enum 69401 \"NB5 Kind\" { Extensible = false; value(0; \" \") { } value(1; One) { } }
tableextension 69401 \"NB5 Ext\" extends \"NB5 Base\" { fields { field(69401; \"NB5 Source\"; Enum \"NB5 Kind\") { } } }
codeunit 69402 T { Subtype = Test;
  [Test] procedure P() var R: Record \"NB5 Base\"; V: Variant; begin
    V := R.\"NB5 Source\"; if not V.IsOption then Error('declared');
    R.Code := 'A'; R.Insert(); Clear(R); R.Get('A');
    V := R.\"NB5 Source\"; if not V.IsOption then Error('after get');
  end; }";
    for (name, outcome) in run_with_base(base, main) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// ErrorInfo.SystemId is assignable and reads back: errorinfo/lt-errnotif/EnTErrorInfoProps.al:106-115 (a row's
// SystemId round-trips) and :259-262 (a null Guid reads as null); :42 (a fresh ErrorInfo holds a null Guid).
#[test]
fn error_info_system_id_assigns_and_reads_back() {
    let main = "
table 69410 \"NB5 Row\" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 69411 T { Subtype = Test;
  [Test] procedure P() var R: Record \"NB5 Row\"; E: ErrorInfo; G: Guid; begin
    E := ErrorInfo.Create('x');
    if not IsNullGuid(E.SystemId) then Error('fresh');
    R.Code := 'A'; R.Insert();
    E.SystemId := R.SystemId;
    if E.SystemId <> R.SystemId then Error('roundtrip');
    E.SystemId := G;
    if not IsNullGuid(E.SystemId) then Error('null');
  end; }";
    for (name, outcome) in run_with_base("", main) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}
