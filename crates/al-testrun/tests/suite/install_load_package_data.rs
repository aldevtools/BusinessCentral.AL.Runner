//! al-runner issue 6649: `NavApp.LoadPackageData` in install code. An app that ships no package data loads nothing, per
//! company and per database (corpus metadata/Y64InstallLeads.al:81-96, CmInstallObjects.al:270, :296).

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50100 "LP Row"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

table 50101 "LP Db Row"
{
    DataPerCompany = false;
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

codeunit 50100 "LP Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        Row: Record "LP Db Row";
    begin
        if Row.IsEmpty() then
            NavApp.LoadPackageData(Database::"LP Db Row");
    end;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "LP Row";
    begin
        if Row.IsEmpty() then
            NavApp.LoadPackageData(Database::"LP Row");
        Row.No := 7;
        Row.Insert();
    end;
}

codeunit 50101 "LP Tests"
{
    Subtype = Test;

    [Test]
    procedure InstallRanToItsEnd()
    var
        Row: Record "LP Row";
        DbRow: Record "LP Db Row";
    begin
        if Row.Count() <> 1 then
            Error('rows %1', Row.Count());
        if DbRow.Count() <> 0 then
            Error('db rows %1', DbRow.Count());
    end;
}
"#;

#[test]
fn load_package_data_of_an_own_table_loads_nothing_in_install_code() {
    let apps = vec![AppSource {
        id: "11111111-1111-1111-1111-111111116649".into(),
        name: "LP".into(),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), SRC.into())],
    }];
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    let out = rt.first_outcome(&CaseId {
        codeunit: 50101,
        name: "InstallRanToItsEnd".into(),
    });
    assert_eq!(out, Outcome::Pass);
}

// ---- an app that ships a data file (corpus install-upgrade/install-triggers/Y85PackageData.al, fixtures Y85XaPackageData.al) ----

fn string(out: &mut Vec<u8>, s: &str) {
    out.push(s.len() as u8);
    out.extend_from_slice(s.as_bytes());
}

/// A data file of table 50200 with the stored fields `No` (Code, the key), `Desc` (Text), `Amt` (Integer), `Day` (Date):
/// the layout of Y85XaPackageData.al. Rows: (No, Desc, Amt, ticks of Day, SystemId last byte).
fn data_file(rows: &[(&str, &str, i32, i64, u8)]) -> Vec<u8> {
    let mut f = Vec::new();
    for n in [0xDA7Au32, 1, 50200, 4] {
        f.extend_from_slice(&n.to_le_bytes());
    }
    for name in ["No", "Desc", "Amt", "Day"] {
        string(&mut f, name);
    }
    for (no, desc, amt, ticks, id) in rows {
        f.push(1);
        string(&mut f, no);
        string(&mut f, desc);
        f.extend_from_slice(&amt.to_le_bytes());
        f.extend_from_slice(&ticks.to_le_bytes());
        let mut guid = [0u8; 16];
        guid[15] = *id;
        f.extend_from_slice(&guid);
    }
    f.push(0);
    f
}

const DATA_SRC: &str = r#"
table 50200 "LD Row"
{
    fields
    {
        field(1; No; Code[20]) { }
        field(2; Desc; Text[30]) { }
        field(3; Amt; Integer) { }
        field(4; Day; Date) { }
    }
    keys { key(PK; No) { Clustered = true; } }
    trigger OnInsert() begin Desc := 'TRIGGER'; end;
}

codeunit 50200 "LD Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var Row: Record "LD Row";
    begin
        if Row.IsEmpty() then
            NavApp.LoadPackageData(Database::"LD Row");
    end;
}

codeunit 50201 "LD Tests"
{
    Subtype = Test;

    [Test]
    procedure Rows()
    var Row: Record "LD Row"; T: Text;
    begin
        if Row.FindSet() then
            repeat
                T += Row.No + '=' + Row.Desc + '/' + Format(Row.Amt) + '/' + Format(Row.Day, 0, 9) + '/' + Format(Row.SystemId) + ';';
            until Row.Next() = 0;
        if T <> 'A1=alpha/5/2022-01-01/{00000000-0000-0000-0000-000000000001};B2=/-3//{00000000-0000-0000-0000-000000000002};' then
            Error('rows %1', T);
    end;
}
"#;

fn run_with_data(file: Vec<u8>) -> Outcome {
    let apps = vec![AppSource {
        id: "11111111-1111-1111-1111-111111116650".into(),
        name: "LD".into(),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), DATA_SRC.into())],
    }];
    let manifest = al_testrun::runtime::Manifest {
        package_data: true,
        package_data_files: vec![("TAB50200.navxdata".into(), file)],
        ..Default::default()
    };
    let kinds = vec![al_sema::AppKind::Workspace];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[manifest], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50201,
        name: "Rows".into(),
    })
}

/// 2022-01-01 is day 738,155 since 0001-01-01 (Y85 T1: `2022-01-01`); 1753-01-01 (day 639,905) is the empty date.
const D2022: i64 = 738_155 * 864_000_000_000;
const D1753: i64 = 639_905 * 864_000_000_000;

#[test]
fn load_package_data_inserts_the_rows_of_the_data_file_without_the_table_trigger() {
    let out = run_with_data(data_file(&[("A1", "alpha", 5, D2022, 1), ("B2", "", -3, D1753, 2)]));
    assert_eq!(out, Outcome::Pass);
}

#[test]
fn a_data_file_that_is_not_pinned_is_refused() {
    // A Date before 1753-01-01: the platform raises an install error whose text is not pinned.
    let out = run_with_data(data_file(&[("A1", "alpha", 5, 1_000 * 864_000_000_000, 1)]));
    assert!(matches!(out, Outcome::NotRun(_)), "{out:?}");
}
