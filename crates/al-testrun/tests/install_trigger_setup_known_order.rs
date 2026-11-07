//! Install-time trigger events with the install order of the apps known (`host::set_publish_order_known`, as in a
//! corpus run). Two apps with no dependency path between them: the subscribers of app B to `Global Triggers`
//! (`GetGlobalTableTriggerMask`, `GetDatabaseTableTriggerSetup`, `OnDatabaseInsert`) run while app A installs exactly
//! when B was installed first, and never in the per-database part of the install. Measured on BC 27.5, 28.5 and 29
//! (corpus branch al-runtime-rs-staging-y79, install-upgrade/install-triggers/Y79InstallUnrelated.al:
//! Y79_PerCompany_TriggerSubscribersOfInstalledApps, Y79_PerDatabase_NoTriggerSubscriberAsked; needs corpus bump).
//! A process of its own: the setting is global to the process.

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const A_ID: &str = "11111111-1111-1111-1111-111111117901";
const B_ID: &str = "22222222-2222-2222-2222-222222227901";

const A: &str = r#"
table 50100 "OA Seen"
{
    DataPerCompany = false;
    fields { field(1; Seq; Integer) { } field(2; Tag; Text[100]) { } }
    keys { key(PK; Seq) { Clustered = true; } }
}
table 50101 "OA Company Data"
{
    fields { field(1; K; Code[20]) { } }
    keys { key(PK; K) { Clustered = true; } }
}
table 50102 "OA Database Data"
{
    DataPerCompany = false;
    fields { field(1; K; Code[20]) { } }
    keys { key(PK; K) { Clustered = true; } }
}
codeunit 50100 "OA Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        D: Record "OA Company Data";
    begin
        D.K := 'C';
        D.Insert(true);
    end;

    trigger OnInstallAppPerDatabase()
    var
        D: Record "OA Database Data";
    begin
        D.K := 'D';
        D.Insert(true);
    end;
}
codeunit 50101 "OA Tests"
{
    Subtype = Test;

    local procedure Tags(): Text
    var
        S: Record "OA Seen";
        T: Text;
    begin
        if S.FindSet() then
            repeat
                T += S.Tag + '|';
            until S.Next() = 0;
        exit(T);
    end;

    [Test]
    procedure NothingOfBRanDuringAInstall()
    begin
        if Tags() <> '' then
            Error(Tags());
    end;

    [Test]
    procedure BRanForTheCompanyTableOnly()
    begin
        if Tags() <> 'mask50101|setup50101|insert50101|' then
            Error(Tags());
    end;
}
"#;

const B: &str = r#"
codeunit 50200 "OB Subscriber"
{
    local procedure Note(Tag: Text)
    var
        R: RecordRef;
    begin
        R.Open(50100);
        R.Init();
        R.Field(1).Value := R.Count() + 1;
        R.Field(2).Value := Tag;
        R.Insert();
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetGlobalTableTriggerMask, '', false, false)]
    local procedure Mask(TableID: Integer; var TableTriggerMask: Integer)
    begin
        if TableID in [50101, 50102] then
            Note('mask' + Format(TableID));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean)
    begin
        if TableId in [50101, 50102] then begin
            Note('setup' + Format(TableId));
            OnDatabaseInsert := true;
        end;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", OnDatabaseInsert, '', false, false)]
    local procedure Inserted(RecRef: RecordRef)
    begin
        if RecRef.Number in [50101, 50102] then
            Note('insert' + Format(RecRef.Number));
    end;
}
"#;

fn app(id: &str, name: &str, src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), src.into())],
    }
}

/// The outcome of `case` with the apps listed in the order of `b_first`.
fn outcome(b_first: bool, case: &str) -> Outcome {
    al_testrun::host::set_publish_order_known(true);
    let (a, b) = (app(A_ID, "Order A", A), app(B_ID, "Order B", B));
    let apps = if b_first { vec![b, a] } else { vec![a, b] };
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50101,
        name: case.into(),
    })
}

/// B is installed after A: its subscribers are not active while A installs.
#[test]
fn a_subscriber_of_an_app_installed_later_is_not_asked() {
    assert_eq!(outcome(false, "NothingOfBRanDuringAInstall"), Outcome::Pass);
}

/// B is installed first: its subscribers run for the table the per-company install writes, in the order mask, setup, event;
/// the per-database part asks none of the three.
#[test]
fn a_subscriber_of_an_app_installed_earlier_is_asked_per_company_only() {
    assert_eq!(outcome(true, "BRanForTheCompanyTableOnly"), Outcome::Pass);
}
