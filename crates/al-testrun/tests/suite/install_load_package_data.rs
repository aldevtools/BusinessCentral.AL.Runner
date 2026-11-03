//! al-runner issue 6649: `NavApp.LoadPackageData` in install code. An app that ships no package data loads nothing, per
//! company and per database (corpus install/lt-cm-install-leads/Y64InstallLeads.al:81-96, CmInstallObjects.al:270, :296).

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
