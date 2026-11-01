//! Gap cycle 5, batch 5a: the platform-table and media shapes corpus writer Y48 pinned (corpus PR 980).
//! AL source -> compile against the System app of the corpus packages -> run as a test codeunit `T`; the values are the ones
//! real BC produced there.
//!
//! Needs the `corpus` submodule, including its `.alpackages` (`System.app`).

use al_deps::package::read_package;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;
use std::path::Path;

fn system_app_files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language/.alpackages");
    let bytes = std::fs::read(dir.join("System.app")).expect("corpus submodule not checked out");
    read_package(&bytes).expect("System.app reads").1
}

/// The outcome of each test of codeunit `T` in `main`, an app on the System app, run as BC 27.5.
fn outcomes(main: &str) -> Vec<(String, Outcome)> {
    use al_sema::workspace::{AppKind, AppSource};
    let app = |id: &str, name: &str, files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: vec![],
        files,
    };
    let apps = vec![
        app("main", "main", vec![("main.al".into(), main.into())]),
        app(al_sema::workspace::PLATFORM_APP_ID, "System", system_app_files()),
    ];
    let symbols = ["BC27PLUS".to_string()].into();
    let (_, l) = al_sema::lower::compile_apps_with(
        apps,
        vec![AppKind::Workspace, AppKind::Dependency],
        &symbols,
        None,
        Default::default(),
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut program = (*l.program).clone();
    for a in program.apps.iter_mut().filter(|a| a.version.is_empty()) {
        a.version = "1.0.0.0".into();
    }
    let engine = Engine::with_target(
        std::sync::Arc::new(program),
        blocked,
        Some(al_version::Target::for_bc(al_version::BcVersion::parse("27.5.0.0").unwrap())),
    );
    let oi = l.program.objects.iter().position(|o| o.name == "T").expect("test codeunit T");
    run_codeunit(&engine, &l, oi)
}

fn passes(src: &str) {
    let o = outcomes(src);
    assert!(!o.is_empty());
    let bad: Vec<_> = o.into_iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:?}");
}

// G5-02: Feature Key Get matches the ID case-insensitively and shows the listed casing; SetRange stays case-sensitive; Insert
// (with and without triggers, of an unknown or a listed ID) and Delete of a listed row raise "The Feature Key table is
// read-only." code DB:Ok and change nothing (record/lt-y48-feature-key-writes/Y48FeatureKeyWrites.al:104 FK_Get_OtherCasing,
// :126 FK_Insert_UnknownId, :151 FK_Insert_UnknownId_RunTrigger, :167 FK_Insert_ListedId, :188 FK_Delete_ListedRow).
#[test]
fn feature_key_get_ignores_case_and_writes_raise_read_only() {
    passes(
        r#"codeunit 50001 "Step" { var S: Text; K: Text[50]; R: Text;
  procedure SetStep(NS: Text; NK: Text[50]) begin S := NS; K := NK; R := ''; end;
  procedure Res(): Text begin exit(R); end;
  trigger OnRun() var F: Record "Feature Key"; begin
    case S of
      'insert', 'insert true': begin F.Init(); F.ID := K; F.Enabled := F.Enabled::"All Users"; R := Format(F.Insert(S = 'insert true')); end;
      'delete': begin F.Get(K); R := Format(F.Delete()); end;
    end; end; }
codeunit 50000 T { Subtype = Test;
  local procedure Run(S: Text; K: Text[50]): Text var C: Codeunit "Step"; begin
    C.SetStep(S, K); ClearLastError(); if C.Run() then exit('ok:' + C.Res()); exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode()); end;
  local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1, want %2', Got, Want); end;
  [Test] procedure GetOtherCasing() var F: Record "Feature Key"; begin
    if not F.Get('DisableSOAPwebservicesonMicrosoftUIpages') then Error('not found');
    Check(F.ID, 'DisableSoapWebServicesOnMicrosoftUIPages');
    Clear(F); if not F.Get('disablesoapwebservicesonmicrosoftuipages') then Error('lower not found');
    Clear(F); F.SetRange(ID, 'DisableSOAPwebservicesonMicrosoftUIpages'); Check(Format(F.Count()), '0'); end;
  [Test] procedure Writes() var F: Record "Feature Key"; Before: Integer; begin
    Before := F.Count();
    Check(Run('insert', 'LTProbeUnknownKey'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Run('insert true', 'LTProbeUnknownKey2'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Run('insert', 'DisableSoapWebServicesOnMicrosoftUIPages'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Run('insert', 'DisableSOAPwebservicesonMicrosoftUIpages'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Run('delete', 'DisableSoapWebServicesOnMicrosoftUIPages'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Format(F.Count() - Before), '0');
    if F.Get('LTProbeUnknownKey') then Error('inserted');
    if not F.Get('DisableSoapWebServicesOnMicrosoftUIPages') then Error('deleted'); end; }"#,
    );
}

// G5-02: Intelligent Cloud is an ordinary table: empty, Insert stores a row Get then finds, Modify of Enabled sticks, a second
// Insert answers false without an error, Delete empties it, another key is insertable (al-language-onprem/platform/
// lt-y48-intelligent-cloud/Y48IntelligentCloudWrites.al: IC_InsertModifyGet, IC_Standard).
#[test]
fn intelligent_cloud_is_an_ordinary_table() {
    passes(
        r#"codeunit 50000 T { Subtype = Test;
  [Test] procedure Writes() var IC: Record "Intelligent Cloud"; IC2: Record "Intelligent Cloud"; begin
    if not IC2.IsEmpty() then Error('not empty');
    if not IC.Get() then if not IC.Insert() then Error('insert');
    if not IC2.Get() then Error('get'); if IC2.Enabled then Error('enabled'); if IC2.Count() <> 1 then Error('count');
    IC.Enabled := true; IC.Modify(); Clear(IC2); IC2.Get(); if not IC2.Enabled then Error('modify');
    if IC.Insert() then Error('duplicate');
    IC.Delete(); if not IC2.IsEmpty() then Error('delete');
    IC."Primary Key" := 'LTY48'; IC.Insert(); if IC2.Count() <> 1 then Error('other key'); end; }"#,
    );
}
