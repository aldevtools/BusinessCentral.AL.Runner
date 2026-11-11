//! ADR-073 Phase 2: the listings of an engine that leaves an app uninstalled (`Engine::uninstalled_apps`, read through
//! `Engine::app_installed`) have no row of that app: AllObj, AllObjWithCaption, Field, and Aggregate Permission Set (NAV App
//! Installed App: al-vm tests/store/installed_app_published_as.rs). The corpus job runs the try-write suite's app in the main pass, and BC's main job does not have that app.
//!
//! Needs `System.app` of the pinned build in the artifact cache (`al_testfiles`).
use al_deps::package::read_package;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const MAIN: &str = r#"
codeunit 69990 T
{
    Subtype = Test;

    [Test]
    procedure Listings()
    var A: Record AllObj; W: Record AllObjWithCaption; F: Record Field; P: Record "Aggregate Permission Set";
    begin
        A.SetRange("Object ID", 81700, 81799);
        W.SetRange("Object ID", 81700, 81799);
        F.SetRange(TableNo, 81702);
        P.SetRange("Role ID", 'TW SET');
        Error('%1|%2|%3|%4', A.Count(), W.Count(), F.Count(), P.Count());
    end;
}
"#;

const TW: &str = r#"
table 81702 TwRow { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
codeunit 81703 TwRun { trigger OnRun() begin end; }
permissionset 81704 "TW SET" { Assignable = true; Permissions = tabledata TwRow = RIMD; }
"#;

/// What the test reports: the counts of the try-write app's rows in the listings.
fn counts(uninstalled: bool) -> String {
    use al_sema::workspace::{AppKind, AppSource};
    let bytes = std::fs::read(al_testfiles::system_app()).unwrap();
    let system = read_package(&bytes).expect("System.app reads").1;
    let main_id = "aa000000-0000-0000-0000-000000000001";
    let app = |id: &str, name: &str, deps: Vec<String>, files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: deps,
        files,
    };
    let apps = vec![
        app(main_id, "main", vec![], vec![("main.al".into(), MAIN.into())]),
        app(
            "bb000000-0000-0000-0000-000000000002",
            "tw",
            vec![main_id.into()],
            vec![("tw.al".into(), TW.into())],
        ),
        app(al_sema::workspace::PLATFORM_APP_ID, "System", vec![], system),
    ];
    let (_, l) = al_sema::lower::compile_apps_with(
        apps,
        vec![AppKind::Workspace, AppKind::Workspace, AppKind::Dependency],
        &Default::default(),
        None,
        Default::default(),
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let mut engine = Engine::new(l.program.clone(), blocked);
    if uninstalled {
        let tw = l.program.apps.iter().position(|a| a.name == "tw").unwrap() as u32;
        std::sync::Arc::get_mut(&mut engine).unwrap().uninstalled_apps = [tw].into();
    }
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("test codeunit T");
    match run_codeunit(&engine, &l, oi).pop() {
        Some((_, Outcome::Fail(f))) => f,
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_installed_app_has_rows_in_the_listings() {
    let c = counts(false);
    let n: Vec<i32> = c.split('|').map(|x| x.parse().unwrap()).collect();
    assert!(n.iter().all(|&x| x > 0), "{c}");
}

#[test]
fn an_uninstalled_app_has_no_row_in_the_listings() {
    assert_eq!(counts(true), "0|0|0|0");
}
