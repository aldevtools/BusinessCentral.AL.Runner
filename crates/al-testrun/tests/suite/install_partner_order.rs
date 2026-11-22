//! al-runner issue 6644, ADR-046 amendment 7 (owner, 2026-11-08). Apps with no dependency path between them have no
//! order BC keeps (owner, 2026-10-30). The runner now orders them as follows:
//!
//! 1. Microsoft's apps install before partner apps: a container's database comes from the artifact with Microsoft's
//!    apps installed, and the partner apps are published and installed into it afterwards. The reporter's E-Document
//!    shape (Microsoft's E-Document Install inserts a Job Queue Entry; a partner app with no dependency path to
//!    E-Document has a table extension on Job Queue Entry with an OnBeforeInsert trigger) runs: the partner app is not
//!    installed yet while E-Document installs.
//! 2. Two partner apps whose install code may interact are installed in both orders and the database each order
//!    leaves is compared. The same database: the run goes on. Another: a test that touches a table that differs is
//!    not run, and the reason names both apps and the table.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest};
use al_testrun::{CaseId, Outcome};

const BASE_ID: &str = "10000000-0000-0000-0000-000000006644";
const EDOC_ID: &str = "20000000-0000-0000-0000-000000006644";
const PA_ID: &str = "30000000-0000-0000-0000-000000006644";
const PB_ID: &str = "40000000-0000-0000-0000-000000006644";
const PC_ID: &str = "50000000-0000-0000-0000-000000006644";
const PD_ID: &str = "60000000-0000-0000-0000-000000006644";
const PE_ID: &str = "70000000-0000-0000-0000-000000006644";
const MAIN_ID: &str = "90000000-0000-0000-0000-000000006644";

/// A Microsoft base app: Job Queue Entry with an enqueue codeunit, a log table, and a logger registry whose
/// OnRegister event hands its subscribers the registry as `var Sender` (System Application's "Telemetry Loggers").
const BASE: &str = r#"
table 70000 "JQ Entry"
{
    DataPerCompany = false;
    fields { field(1; "No."; Integer) { } field(2; Note; Text[50]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 70001 "Order Log"
{
    DataPerCompany = false;
    fields { field(1; Entry; Text[100]) { } }
    keys { key(PK; Entry) { Clustered = true; } }
}
table 70002 "Install Mark"
{
    DataPerCompany = false;
    fields { field(1; App; Text[30]) { } }
    keys { key(PK; App) { Clustered = true; } }
}
codeunit 70003 "JQ Enqueue"
{
    procedure Enqueue(No: Integer; Note: Text[50])
    var
        E: Record "JQ Entry";
    begin
        E.Init();
        E."No." := No;
        E.Note := Note;
        E.Insert(true);
    end;
}
codeunit 70004 "Loggers"
{
    SingleInstance = true;

    var
        Registered: List of [Text];

    procedure Register(Name: Text)
    begin
        Registered.Add(Name);
    end;

    procedure Count(): Integer
    begin
        exit(Registered.Count());
    end;

    procedure LogMessage(Msg: Text)
    var
        Self: Codeunit "Loggers";
    begin
        OnRegister(Self);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnRegister(var Sender: Codeunit "Loggers")
    begin
    end;
}
codeunit 70005 "Marks"
{
    procedure Mark(App: Text)
    var
        M: Record "Install Mark";
    begin
        M.App := CopyStr(App, 1, 30);
        if M.Insert() then;
    end;

    procedure Log(Entry: Text)
    var
        L: Record "Order Log";
    begin
        L.Entry := CopyStr(Entry, 1, 100);
        if L.Insert() then;
    end;
}
"#;

/// Microsoft's E-Document stand-in: its install code enqueues a job queue entry.
const EDOC: &str = r#"
codeunit 71000 "EDoc Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Q: Codeunit "JQ Enqueue";
    begin
        Q.Enqueue(1, 'edoc');
    end;
}
"#;

/// A partner app with no dependency path to E-Document: a table extension with an OnBeforeInsert trigger on Job Queue
/// Entry.
const PARTNER_EXT: &str = r#"
tableextension 72000 "PX JQ Ext" extends "JQ Entry"
{
    trigger OnBeforeInsert()
    begin
        Rec.Note := CopyStr(Rec.Note + '+px', 1, MaxStrLen(Rec.Note));
    end;
}
"#;

/// Partner app A: its install code logs a message, whose OnRegister event reaches partner subscribers, then marks
/// that it ran.
const PA: &str = r#"
codeunit 73000 "PA Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        T: Codeunit "Loggers";
        M: Codeunit "Marks";
    begin
        T.LogMessage('installed');
        M.Mark('A');
    end;
}
"#;

/// A partner app with no dependency path to A whose subscriber changes `Sender` only (registers its logger): the
/// database is the same in both orders.
fn registering(object: u32, name: &str) -> String {
    format!(
        r#"
codeunit {object} "{name} Logger Registration"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Loggers", 'OnRegister', '', false, false)]
    local procedure OnRegister(var Sender: Codeunit "Loggers")
    begin
        Sender.Register('{name}');
    end;
}}
"#
    )
}

/// A partner app with no dependency path to A whose subscriber writes a row: the database differs between the orders.
fn logging(object: u32, name: &str) -> String {
    format!(
        r#"
codeunit {object} "{name} Logging Subscriber"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Loggers", 'OnRegister', '', false, false)]
    local procedure OnRegister(var Sender: Codeunit "Loggers")
    var
        M: Codeunit "Marks";
    begin
        M.Log('{name} saw a logger');
    end;
}}
"#
    )
}

const TESTS: &str = r#"
codeunit 79000 "Order Tests"
{
    Subtype = Test;

    [Test]
    procedure EDocRowWithoutPartnerTrigger()
    var
        E: Record "JQ Entry";
    begin
        E.Get(1);
        if E.Note <> 'edoc' then
            Error('note %1', E.Note);
    end;

    [Test]
    procedure PartnerInstallRan()
    var
        M: Record "Install Mark";
    begin
        M.Get('A');
    end;

    [Test]
    procedure ReadsTheLog()
    var
        L: Record "Order Log";
    begin
        if L.IsEmpty() then
            Error('empty');
    end;
}
"#;

fn app(id: &str, name: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{name}.al"), src.into())],
    }
}

fn manifest(publisher: &str) -> Manifest {
    Manifest {
        publisher: publisher.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    }
}

/// Runs the test app (under test) over the dependency apps `deps` (id, name, publisher, dependencies, source).
fn run(deps: Vec<(&str, &str, &str, Vec<&str>, String)>) -> CorpusRuntime {
    let mut apps = Vec::new();
    let mut kinds = Vec::new();
    let mut manifests = Vec::new();
    let mut all = Vec::new();
    for (id, name, publisher, d, src) in &deps {
        apps.push(app(id, name, d, src));
        kinds.push(al_sema::AppKind::Dependency);
        manifests.push(manifest(publisher));
        all.push(*id);
    }
    apps.push(app(MAIN_ID, "Order Tests", &all, TESTS));
    kinds.push(al_sema::AppKind::Workspace);
    manifests.push(manifest("Partner"));
    CorpusRuntime::run_for_apps(apps, kinds, &manifests, &Default::default(), None, Default::default())
}

fn outcome(rt: &CorpusRuntime, test: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit: 79000,
        name: test.into(),
    })
}

fn base() -> (&'static str, &'static str, &'static str, Vec<&'static str>, String) {
    (BASE_ID, "Base", "Microsoft", vec![], BASE.into())
}

/// The reporter's E-Document shape: Microsoft's app installs before the partner app, so the partner's OnBeforeInsert
/// does not run on the row E-Document's install code inserts.
#[test]
fn microsoft_apps_install_before_partner_apps() {
    let rt = run(vec![
        base(),
        // The partner app is listed first, so only the Microsoft-first rule puts E-Document before it.
        (PB_ID, "Partner Ext", "Partner", vec![BASE_ID], PARTNER_EXT.into()),
        (EDOC_ID, "E-Document", "Microsoft", vec![BASE_ID], EDOC.into()),
    ]);
    assert_eq!(outcome(&rt, "EDocRowWithoutPartnerTrigger"), Outcome::Pass);
}

/// Two Microsoft apps with no dependency path between them keep no order: the partner rule does not apply to them.
#[test]
fn two_unrelated_microsoft_apps_stay_refused() {
    let rt = run(vec![
        base(),
        (PB_ID, "Microsoft Ext", "Microsoft", vec![BASE_ID], PARTNER_EXT.into()),
        (EDOC_ID, "E-Document", "Microsoft", vec![BASE_ID], EDOC.into()),
    ]);
    // The install code that stopped is named in the baseline notes; the test sees an unseeded table.
    assert!(matches!(
        outcome(&rt, "EDocRowWithoutPartnerTrigger"),
        Outcome::NotRun(_)
    ));
}

/// The reporter's telemetry shape: a partner subscriber that only changes `Sender`. Both orders leave the same
/// database, so partner A's install runs to its end.
#[test]
fn partner_apps_whose_orders_give_the_same_database_run() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], registering(74000, "PB")),
    ]);
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
    // `--timings` says that the install code ran twice, and for which apps.
    let line = rt.phases.baseline_line().unwrap_or_default();
    assert!(
        line.contains("install order: Partner A, Partner B") && line.contains("ran in 2 orders"),
        "{line}"
    );
}

/// A partner subscriber that writes a row: the two orders leave different rows in Order Log. A test that reads it is
/// not run, and the reason names both apps and the table; a test that does not touch it runs.
#[test]
fn partner_apps_whose_orders_give_different_databases_are_refused() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], logging(74000, "PB")),
    ]);
    match outcome(&rt, "ReadsTheLog") {
        Outcome::NotRun(why) => {
            assert!(why.contains("Partner A") && why.contains("Partner B"), "{why}");
            assert!(why.contains("Order Log"), "{why}");
            assert!(why.contains("install order"), "{why}");
        }
        other => panic!("expected not run, got {other:?}"),
    }
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
}

/// Three interacting partner apps: every order of the three is tried, and they all leave the same database.
#[test]
fn three_interacting_partner_apps_are_tried_in_every_order() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], registering(74000, "PB")),
        (PC_ID, "Partner C", "Partner", vec![BASE_ID], registering(75000, "PC")),
    ]);
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
}

/// More than three interacting partner apps: the runner does not try every order and refuses as before.
#[test]
fn more_than_three_interacting_partner_apps_are_refused() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], registering(74000, "PB")),
        (PC_ID, "Partner C", "Partner", vec![BASE_ID], registering(75000, "PC")),
        (PD_ID, "Partner D", "Partner", vec![BASE_ID], registering(76000, "PD")),
        (PE_ID, "Partner E", "Partner", vec![BASE_ID], registering(77000, "PE")),
    ]);
    assert!(matches!(outcome(&rt, "PartnerInstallRan"), Outcome::NotRun(_)));
}

/// [`run`] with the partner apps under test too (`al-runner test A B Tests`): the Microsoft base app is a dependency.
fn run_under_test(partners: Vec<(&str, &str, String)>) -> CorpusRuntime {
    let mut apps = vec![app(BASE_ID, "Base", &[], BASE)];
    let mut kinds = vec![al_sema::AppKind::Dependency];
    let mut manifests = vec![manifest("Microsoft")];
    let mut all = vec![BASE_ID];
    for (id, name, src) in &partners {
        apps.push(app(id, name, &[BASE_ID], src));
        kinds.push(al_sema::AppKind::Workspace);
        manifests.push(manifest("Partner"));
        all.push(*id);
    }
    apps.push(app(MAIN_ID, "Order Tests", &all, TESTS));
    kinds.push(al_sema::AppKind::Workspace);
    manifests.push(manifest("Partner"));
    CorpusRuntime::run_for_apps(apps, kinds, &manifests, &Default::default(), None, Default::default())
}

/// The same rule for two partner apps under test with no dependency path between them.
#[test]
fn partner_apps_under_test_whose_orders_give_the_same_database_run() {
    let rt = run_under_test(vec![
        (PA_ID, "Partner A", PA.into()),
        (PB_ID, "Partner B", registering(74000, "PB")),
    ]);
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
}

#[test]
fn partner_apps_under_test_whose_orders_give_different_databases_are_refused() {
    let rt = run_under_test(vec![
        (PA_ID, "Partner A", PA.into()),
        (PB_ID, "Partner B", logging(74000, "PB")),
    ]);
    match outcome(&rt, "ReadsTheLog") {
        Outcome::NotRun(why) => assert!(why.contains("Partner B") && why.contains("Order Log"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
}
