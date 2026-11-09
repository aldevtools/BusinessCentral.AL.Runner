//! Al-runner issue 7359: an app's install routine calls `NavApp.LoadPackageData` and the load is refused (a data file the
//! runtime does not read). A test that reads the table through a library codeunit finds no row and raises its own error.
//! On BC the load runs and the rows exist, so "no row" is not BC's answer: the test is not run, as for the tables a stopped
//! install writes with a record (al-runner issue 6973, `install_stopped_rows.rs`).

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const APP_ID: &str = "11111111-1111-1111-1111-111111117359";
const TEST_ID: &str = "22222222-2222-2222-2222-222222227359";

const APP: &str = r#"
table 50100 "My File Demo Data"
{
    DataPerCompany = false;
    fields { field(1; Code; Code[20]) { } field(2; Description; Text[50]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 50100 "My Upload Demo File"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    begin
        NavApp.LoadPackageData(Database::"My File Demo Data");
    end;
}
"#;

const TESTS: &str = r#"
codeunit 50200 "My Demo Library"
{
    procedure FirstDemo(): Code[20]
    var
        DemoData: Record "My File Demo Data";
    begin
        DemoData.FindFirst();
        exit(DemoData.Code);
    end;
}

codeunit 50201 "My Demo Tests"
{
    Subtype = Test;

    [Test]
    procedure ReadsTheDemoDataThroughTheLibrary()
    var
        Library: Codeunit "My Demo Library";
    begin
        Library.FirstDemo();
    end;
}
"#;

fn string(out: &mut Vec<u8>, s: &str) {
    out.push(s.len() as u8);
    out.extend_from_slice(s.as_bytes());
}

/// A data file of table 50100 that lists three fields for a table with two stored fields: the load is refused.
fn data_file() -> Vec<u8> {
    let mut f = Vec::new();
    for n in [0xDA7Au32, 1, 50100, 3] {
        f.extend_from_slice(&n.to_le_bytes());
    }
    for name in ["Code", "Description", "Extra"] {
        string(&mut f, name);
    }
    f.push(0);
    f
}

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("Demo {}", &id[..1]),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![("f.al".into(), src.into())],
    }
}

fn outcome(app_kind: al_sema::AppKind) -> Outcome {
    let apps = vec![app(TEST_ID, &[APP_ID], TESTS), app(APP_ID, &[], APP)];
    let kinds = vec![al_sema::AppKind::Workspace, app_kind];
    let manifests = vec![
        al_testrun::runtime::Manifest::default(),
        al_testrun::runtime::Manifest {
            package_data: true,
            package_data_files: vec![("Tab50100.navxdata".into(), data_file())],
            ..Default::default()
        },
    ];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &manifests, &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50201,
        name: "ReadsTheDemoDataThroughTheLibrary".into(),
    })
}

/// The app is under test: its install runs after the baseline, and the load of its data file is refused.
#[test]
fn a_test_that_reads_rows_a_refused_load_package_data_would_have_loaded_is_not_run() {
    match outcome(al_sema::AppKind::Workspace) {
        Outcome::NotRun(why) => assert!(
            why.contains("My Upload Demo File") && why.contains("LoadPackageData of table 50100"),
            "{why}"
        ),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// The app is a dependency: the install baseline runs its install, and its LoadPackageData is refused whatever it ships. The
/// table counts as unseeded, so the read that finds no row is refused.
#[test]
fn a_test_that_reads_rows_a_dependency_app_would_have_loaded_is_not_run() {
    match outcome(al_sema::AppKind::Dependency) {
        Outcome::NotRun(why) => assert!(why.contains("My File Demo Data"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}
