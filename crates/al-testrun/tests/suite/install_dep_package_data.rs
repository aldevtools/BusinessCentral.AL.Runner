//! al-runner issue 6649: `NavApp.LoadPackageData` in the install code of an app that is only a dependency of the run (AL
//! source passed with `--dep`). The install baseline runs its install code and loads the app's data files as the install of
//! an app under test does. Pinned by corpus tests/al-language/install-upgrade/install-triggers/Y85PackageData.al:3-6: the
//! install codeunit of the cross-app fixture A (tests/al-language-xapp-a/Y85XaPackageData.al), an app the test app depends
//! on, loads its TAB<id>.navxdata files per company and per database.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest};
use al_testrun::{CaseId, Outcome};

const DEP_ID: &str = "11111111-1111-1111-1111-111111166490";
const TEST_ID: &str = "22222222-2222-2222-2222-222222266490";

const DEP: &str = r#"
table 50300 "DP Company Row"
{
    fields { field(1; No; Code[20]) { } field(2; Desc; Text[30]) { } }
    keys { key(PK; No) { Clustered = true; } }
}

table 50301 "DP Shared Row"
{
    DataPerCompany = false;
    fields { field(1; No; Code[20]) { } field(2; Desc; Text[30]) { } }
    keys { key(PK; No) { Clustered = true; } }
}

codeunit 50300 "DP Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        Row: Record "DP Shared Row";
    begin
        if Row.IsEmpty() then
            NavApp.LoadPackageData(Database::"DP Shared Row");
    end;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "DP Company Row";
    begin
        if Row.IsEmpty() then
            NavApp.LoadPackageData(Database::"DP Company Row");
    end;
}
"#;

const TESTS: &str = r#"
codeunit 50310 "DP Tests"
{
    Subtype = Test;

    local procedure DumpCompany(): Text
    var
        Row: Record "DP Company Row";
        T: Text;
    begin
        if Row.FindSet() then
            repeat
                T += Row.No + '=' + Row.Desc + ';';
            until Row.Next() = 0;
        exit(T);
    end;

    local procedure DumpShared(): Text
    var
        Row: Record "DP Shared Row";
        T: Text;
    begin
        if Row.FindSet() then
            repeat
                T += Row.No + '=' + Row.Desc + ';';
            until Row.Next() = 0;
        exit(T);
    end;

    [Test]
    procedure BothParts()
    begin
        if DumpCompany() <> 'C1=one;C2=two;' then
            Error('company %1', DumpCompany());
        if DumpShared() <> 'S1=uno;' then
            Error('shared %1', DumpShared());
    end;

    [Test]
    procedure NoCompanyFile()
    var
        Row: Record "DP Company Row";
    begin
        if not Row.IsEmpty() then
            Error('company %1', DumpCompany());
        if DumpShared() <> 'S1=uno;' then
            Error('shared %1', DumpShared());
    end;

    [Test]
    procedure NoFiles()
    var
        Row: Record "DP Company Row";
        Shared: Record "DP Shared Row";
    begin
        if not Row.IsEmpty() then
            Error('company %1', DumpCompany());
        if not Shared.IsEmpty() then
            Error('shared %1', DumpShared());
    end;
}
"#;

fn string(out: &mut Vec<u8>, s: &str) {
    out.push(s.len() as u8);
    out.extend_from_slice(s.as_bytes());
}

/// A data file of `table` with the stored fields `No` (Code, the key) and `Desc` (Text), the layout
/// `install_load_package_data.rs` uses. Rows: (No, Desc, SystemId last byte).
fn data_file(table: u32, rows: &[(&str, &str, u8)]) -> Vec<u8> {
    let mut f = Vec::new();
    for n in [0xDA7Au32, 1, table, 2] {
        f.extend_from_slice(&n.to_le_bytes());
    }
    for name in ["No", "Desc"] {
        string(&mut f, name);
    }
    for (no, desc, id) in rows {
        f.push(1);
        string(&mut f, no);
        string(&mut f, desc);
        let mut guid = [0u8; 16];
        guid[15] = *id;
        f.extend_from_slice(&guid);
    }
    f.push(0);
    f
}

fn company_file() -> (String, Vec<u8>) {
    (
        "res/Tab50300.navxdata".into(),
        data_file(50300, &[("C1", "one", 1), ("C2", "two", 2)]),
    )
}

fn shared_file() -> (String, Vec<u8>) {
    ("res/Tab50301.navxdata".into(), data_file(50301, &[("S1", "uno", 3)]))
}

fn run(dep: Manifest, test: &str) -> Outcome {
    let apps = vec![
        AppSource {
            id: TEST_ID.into(),
            name: "DP Tests".into(),
            dependencies: vec![DEP_ID.into()],
            files: vec![("t.al".into(), TESTS.into())],
        },
        AppSource {
            id: DEP_ID.into(),
            name: "DP Dep".into(),
            dependencies: Vec::new(),
            files: vec![("d.al".into(), DEP.into())],
        },
    ];
    let kinds = vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency];
    let manifests = vec![Manifest::default(), dep];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &manifests, &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50310,
        name: test.into(),
    })
}

/// The install baseline loads the dependency app's data files: the per-company table in the per-company part, the shared
/// table in the per-database part (Y85PackageData.al:3-6).
#[test]
fn a_dependency_app_loads_its_package_data_in_both_install_parts() {
    let dep = Manifest {
        package_data: true,
        package_data_files: vec![company_file(), shared_file()],
        ..Default::default()
    };
    assert_eq!(run(dep, "BothParts"), Outcome::Pass);
}

/// A table without a data file in the dependency app loads nothing, as for an app under test
/// (metadata/Y64InstallLeads.al:81-96).
#[test]
fn a_dependency_app_without_a_file_for_the_table_loads_nothing_for_it() {
    let dep = Manifest {
        package_data: true,
        package_data_files: vec![shared_file()],
        ..Default::default()
    };
    assert_eq!(run(dep, "NoCompanyFile"), Outcome::Pass);
}

/// A dependency app that ships no data file at all loads nothing, per company and per database
/// (metadata/Y64InstallLeads.al:81-96).
#[test]
fn a_dependency_app_that_ships_no_package_data_loads_nothing() {
    assert_eq!(run(Manifest::default(), "NoFiles"), Outcome::Pass);
}

/// A compiled package (`.app`) that ships package data: where a package keeps its data files is not measured, so its files
/// are not read and the load stays refused. The test that reads the tables is not run.
#[test]
fn package_data_of_a_dependency_package_stays_refused() {
    let dep = Manifest {
        package_data: true,
        package_data_files: Vec::new(),
        ..Default::default()
    };
    match run(dep, "BothParts") {
        Outcome::NotRun(_) => {}
        other => panic!("expected not run, got {other:?}"),
    }
}
