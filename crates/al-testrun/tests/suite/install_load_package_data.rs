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

/// A subscriber to the table's insert events that writes a log row.
const EVENT_SUBSCRIBER: &str = r#"
table 50201 "LD Log"
{
    fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; What; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50202 "LD Events"
{
    [EventSubscriber(ObjectType::Table, Database::"LD Row", 'OnBeforeInsertEvent', '', false, false)]
    local procedure OnBeforeInsert(var Rec: Record "LD Row"; RunTrigger: Boolean)
    var Log: Record "LD Log";
    begin
        Log.What := 'before ' + Rec.No;
        Log.Insert();
    end;

    [EventSubscriber(ObjectType::Table, Database::"LD Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsert(var Rec: Record "LD Row"; RunTrigger: Boolean)
    var Log: Record "LD Log";
    begin
        Log.What := 'after ' + Rec.No;
        Log.Insert();
    end;
}

codeunit 50203 "LD Event Tests"
{
    Subtype = Test;

    [Test]
    procedure NoEventDuringLoad()
    var Row: Record "LD Row"; Log: Record "LD Log";
    begin
        if Row.Count() <> 2 then
            Error('rows %1', Row.Count());
        if not Log.IsEmpty() then
            Error('events %1', Log.Count());
    end;
}
"#;

/// NavApp.LoadPackageData raises no table event; the subscribers still run for ordinary writes (corpus
/// install-upgrade/install-triggers/Y88PackageData.al, Y88_Events_LoadRaisesNoTableEvent,
/// Y88_Events_SubscribersWorkForOrdinaryWrites).
#[test]
fn load_package_data_raises_no_table_event() {
    let apps = vec![AppSource {
        id: "11111111-1111-1111-1111-111111116651".into(),
        name: "LD".into(),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), format!("{DATA_SRC}{EVENT_SUBSCRIBER}"))],
    }];
    let manifest = al_testrun::runtime::Manifest {
        package_data: true,
        package_data_files: vec![(
            "TAB50200.navxdata".into(),
            data_file(&[("A1", "alpha", 5, D2022, 1), ("B2", "", -3, D1753, 2)]),
        )],
        ..Default::default()
    };
    let kinds = vec![al_sema::AppKind::Workspace];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[manifest], &Default::default(), None, Default::default());
    let out = rt.first_outcome(&CaseId {
        codeunit: 50203,
        name: "NoEventDuringLoad".into(),
    });
    assert_eq!(out, Outcome::Pass);
}

/// The install code of DATA_SRC, with the load inside a TryFunction whose result is used.
const TRY_SRC: &str = r#"
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
}

codeunit 50200 "LD Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var Row: Record "LD Row";
    begin
        if Row.IsEmpty() then
            if not TryLoad() then
                Error('load failed');
    end;

    [TryFunction]
    local procedure TryLoad()
    begin
        NavApp.LoadPackageData(Database::"LD Row");
    end;

    [EventSubscriber(ObjectType::Table, Database::"LD Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsert(var Rec: Record "LD Row"; RunTrigger: Boolean)
    begin
    end;
}

codeunit 50201 "LD Tests"
{
    Subtype = Test;

    [Test]
    procedure Rows()
    var Row: Record "LD Row";
    begin
        if Row.Count() <> 2 then
            Error('rows %1', Row.Count());
    end;
}
"#;

/// NavApp.LoadPackageData inside a TryFunction whose result is used loads, also for a table with event subscribers (corpus
/// Y88PackageData.al: the loads of codeunit 94682 run in TryLoad and log `ok`).
#[test]
fn load_package_data_inside_a_used_try_function_loads() {
    let apps = vec![AppSource {
        id: "11111111-1111-1111-1111-111111116652".into(),
        name: "LD".into(),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), TRY_SRC.into())],
    }];
    let manifest = al_testrun::runtime::Manifest {
        package_data: true,
        package_data_files: vec![(
            "TAB50200.navxdata".into(),
            data_file(&[("A1", "alpha", 5, D2022, 1), ("B2", "", -3, D1753, 2)]),
        )],
        ..Default::default()
    };
    let kinds = vec![al_sema::AppKind::Workspace];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[manifest], &Default::default(), None, Default::default());
    let out = rt.first_outcome(&CaseId {
        codeunit: 50201,
        name: "Rows".into(),
    });
    assert_eq!(out, Outcome::Pass);
}

/// TRY_SRC with the table shared (`DataPerCompany = false`) or the load in the per-database part.
fn parts_src(shared: bool, trigger: &str) -> String {
    TRY_SRC
        .replacen(
            "table 50200 \"LD Row\"\n{\n",
            &format!("table 50200 \"LD Row\"\n{{\n    DataPerCompany = {};\n", !shared),
            1,
        )
        .replacen("OnInstallAppPerCompany", trigger, 1)
}

/// A per-company table cannot be read in the per-database part (CompanyName() is empty), so the load runs unguarded.
fn unguarded(src: String) -> String {
    src.replacen(
        "if Row.IsEmpty() then\n            if not TryLoad()",
        "if not TryLoad()",
        1,
    )
}

fn run_parts(src: String, app: &str) -> Outcome {
    let apps = vec![AppSource {
        id: format!("11111111-1111-1111-1111-11111111{app}"),
        name: "LD".into(),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), src)],
    }];
    let manifest = al_testrun::runtime::Manifest {
        package_data: true,
        package_data_files: vec![(
            "TAB50200.navxdata".into(),
            data_file(&[("A1", "alpha", 5, D2022, 1), ("B2", "", -3, D1753, 2)]),
        )],
        ..Default::default()
    };
    let kinds = vec![al_sema::AppKind::Workspace];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[manifest], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50201,
        name: "Rows".into(),
    })
}

/// A shared table loaded from OnInstallAppPerCompany loads its rows into the shared table (corpus
/// install-upgrade/install-triggers/PkgDataParts.al:40-42, LoadPackageData_SharedTableInCompanyPart: `load returned`,
/// rows `S1=one;S2=two;`).
#[test]
fn a_shared_table_loads_from_the_per_company_part() {
    let out = run_parts(parts_src(true, "OnInstallAppPerCompany"), "6653");
    assert_eq!(out, Outcome::Pass);
}

/// A per-company table loaded from OnInstallAppPerDatabase fails the publish of the app on BC (PkgDataParts.al:5,
/// tests/al-language-xapp-a/PkgDataPartsFixture.al:4); the error text is not pinned, so the install is refused.
#[test]
fn a_per_company_table_from_the_per_database_part_is_refused() {
    let out = run_parts(unguarded(parts_src(false, "OnInstallAppPerDatabase")), "6654");
    match out {
        Outcome::NotRun(why) => assert!(why.contains("fails the publish"), "{why}"),
        other => panic!("{other:?}"),
    }
}
