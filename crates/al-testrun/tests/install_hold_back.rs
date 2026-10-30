//! The apps under test are not installed while the dependency apps install and Company-Initialize runs: their subscribers
//! to `OnCompanyInitialize` and to the events Company-Initialize raises (Upgrade Tag `OnGetPerCompanyUpgradeTags`) do not
//! run there. A freshly installed app's per-company install code sees no row its `OnCompanyInitialize` subscriber writes
//! and `HasUpgradeTag` = No for its own tag (install/lt-bf-install-probes/BfUpgradeTagInstallTests.al:42-43).

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

/// A dependency app: Company-Initialize raises `OnCompanyInitialize` and, as Base App's `UpgradeTag.SetAllUpgradeTags()`
/// does, an event that collects tags from every subscriber and stores them.
const DEP: &str = r#"
table 50100 Tags { DataPerCompany = false; fields { field(1; Tag; Code[50]) { } } keys { key(PK; Tag) { } } }
codeunit 50101 "Tag Store"
{
    procedure SetAll()
    var
        L: List of [Code[50]];
        T: Code[50];
        R: Record Tags;
    begin
        OnGetTags(L);
        foreach T in L do begin
            R.Tag := T;
            if R.Insert() then;
        end;
    end;

    procedure Has(T: Code[50]): Boolean
    var
        R: Record Tags;
    begin
        exit(R.Get(T));
    end;

    [IntegrationEvent(false, false)]
    local procedure OnGetTags(var L: List of [Code[50]])
    begin
    end;
}
codeunit 2 "Company-Initialize"
{
    trigger OnRun()
    var
        S: Codeunit "Tag Store";
    begin
        OnCompanyInitialize();
        S.SetAll();
    end;

    [IntegrationEvent(false, false)]
    local procedure OnCompanyInitialize()
    begin
    end;
}
"#;

const APP: &str = r#"
table 60100 InitLog { DataPerCompany = false; fields { field(1; Id; Integer) { AutoIncrement = true; } } keys { key(PK; Id) { } } }
table 60101 InstallLog
{
    DataPerCompany = false;
    fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; HasTag; Boolean) { } field(3; InitRows; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 60102 Subs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company-Initialize", 'OnCompanyInitialize', '', false, false)]
    local procedure OnInit()
    var
        R: Record InitLog;
    begin
        R.Insert();
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Tag Store", 'OnGetTags', '', false, false)]
    local procedure AddTag(var L: List of [Code[50]])
    begin
        L.Add('APP-TAG');
    end;
}
codeunit 60103 Inst
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Codeunit "Tag Store";
        I: Record InitLog;
        R: Record InstallLog;
    begin
        R.HasTag := S.Has('APP-TAG');
        R.InitRows := I.Count();
        R.Insert();
    end;
}
codeunit 60104 T
{
    Subtype = Test;

    [Test]
    procedure InstallCodeSawNoTagAndNoInitRow()
    var
        R: Record InstallLog;
    begin
        R.FindFirst();
        if R.HasTag or (R.InitRows <> 0) then
            Error('has tag=%1 init rows=%2', R.HasTag, R.InitRows);
    end;
}
"#;

fn outcome() -> Outcome {
    outcome_of(APP, "InstallCodeSawNoTagAndNoInitRow")
}

fn outcome_of(src: &str, test: &str) -> Outcome {
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    // The run builds the install baseline: the dependency installs, Company-Initialize, then the app's install code.
    let rt = CorpusRuntime::run_for_apps(
        vec![app("app", &["dep"], src), app("dep", &[], DEP)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[],
        &Default::default(),
        None,
        Default::default(),
    );
    rt.first_outcome(&CaseId {
        codeunit: 60104,
        name: test.into(),
    })
}

// BfUpgradeTagInstallTests.al:42-43: has tag before=No, init row exists=No, init rows all companies=0.
#[test]
fn install_code_of_the_app_under_test_sees_no_tag_and_no_company_initialize_row() {
    assert_eq!(outcome(), Outcome::Pass);
}

/// The app's install code raises an event of a dependency app; the app's own subscriber answers it.
const APP_OWN: &str = r#"
table 60100 InstallLog
{
    DataPerCompany = false;
    fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; HasTag; Boolean) { } }
    keys { key(PK; Id) { } }
}
codeunit 60102 Subs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Tag Store", 'OnGetTags', '', false, false)]
    local procedure AddTag(var L: List of [Code[50]])
    begin
        L.Add('APP-TAG');
    end;
}
codeunit 60103 Inst
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Codeunit "Tag Store";
        R: Record InstallLog;
    begin
        S.SetAll();
        R.HasTag := S.Has('APP-TAG');
        R.Insert();
    end;
}
codeunit 60104 T
{
    Subtype = Test;

    [Test]
    procedure OwnSubscriberRanInInstall()
    var
        R: Record InstallLog;
    begin
        R.FindFirst();
        if not R.HasTag then
            Error('own subscriber did not run in install');
    end;
}
"#;

// The app's own subscribers run while its own install code runs: the platform asks the app's trigger setup subscriber
// once per company during its OnInstallAppPerCompany
// (install/lt-ug-install-trigger-setup/UgInstallTests.al:37-41 TriggerSetup_AskedPerCompanyInInstall_NeverInPerDatabasePart).
// The apps under test install after the baseline (`install::plan` with `dependency` false), when every app is installed:
// the hold-back covers only the dependency installs, Company-Initialize and the company build. A guard, green before and
// after the hold-back.
#[test]
fn own_subscribers_of_the_app_under_test_run_in_its_install_code() {
    assert_eq!(outcome_of(APP_OWN, "OwnSubscriberRanInInstall"), Outcome::Pass);
}
