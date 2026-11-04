//! The Printer system table (2000000039) has no rows (metadata/platform-tables/Y21EnvColumns.al:30-40 Y21_Printer_NoRows;
//! al-runner issue 6916). RecordRef.Open stays refused in a Cloud app (Y21EnvColumns.al:42-50 Y21_Printer_RecordRefRefused).
//!
//! Needs the `corpus` submodule, including its `.alpackages` (`System.app`).

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

#[test]
fn printer_is_an_empty_table() {
    let got = outcomes(
        r#"codeunit 50000 T { Subtype = Test;
  [Test] procedure Empty() var P: Record Printer; begin
    if P.Count() <> 0 then Error('count %1', P.Count());
    if P.FindFirst() then Error('find first');
    if not P.IsEmpty() then Error('not empty');
    if P.FindSet() then Error('find set');
    if P.Get('x') then Error('get'); end;
  [Test] procedure ReportersFilter() var P: Record Printer; begin
    P.Reset(); P.SetFilter(Name, '@%1', 'x');
    if P.FindFirst() then Error('found'); if P.Count() <> 0 then Error('count'); end; }"#,
    );
    for (name, o) in &got {
        assert!(matches!(o, Outcome::Pass), "{name}: {o:?}");
    }
    assert_eq!(got.len(), 2);
}

/// An OnPrem server can list real printers and no corpus test under tests/al-language-onprem pins the table, so a read
/// under `--environment onprem` is refused (Y21EnvColumns.al:30 pins "no rows" for the Cloud suite only).
#[test]
fn printer_read_is_refused_on_an_onprem_server() {
    use al_sema::workspace::{AppKind, AppSource};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language/.alpackages");
    let system = al_deps::package::read_package(&std::fs::read(dir.join("System.app")).unwrap())
        .unwrap()
        .1;
    let app = |id: &str, name: &str, files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: vec![],
        files,
    };
    let src = "codeunit 50000 C { procedure Count(): Integer var P: Record Printer; begin exit(P.Count()); end; }";
    let (_, l) = al_sema::lower::compile_apps_with(
        vec![
            app("main", "main", vec![("main.al".into(), src.into())]),
            app(al_sema::workspace::PLATFORM_APP_ID, "System", system),
        ],
        vec![AppKind::Workspace, AppKind::Dependency],
        &["BC27PLUS".to_string()].into(),
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
    let oi = program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "Count")
        .unwrap();
    let engine = al_vm::Engine::with_target(
        std::sync::Arc::new(program),
        blocked,
        Some(al_version::Target::for_bc(
            al_version::BcVersion::parse("27.5.0.0").unwrap(),
        )),
    );
    let mut s = al_vm::Session::with_host(
        engine,
        al_host::Host::default().in_environment(al_host::Environment::OnPrem),
    );
    let inst = s.instance(oi);
    let err = s.call(oi, ri, inst, vec![]).expect_err("refused");
    let err = format!("{err:?}");
    assert!(err.contains("Printer") && err.contains("OnPrem"), "{err}");
}
