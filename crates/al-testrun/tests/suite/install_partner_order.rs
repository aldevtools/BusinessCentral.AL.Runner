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
    run_on(deps, Default::default())
}

/// [`run`] with the .NET host `dotnet`.
fn run_on(deps: Vec<(&str, &str, &str, Vec<&str>, String)>, dotnet: al_vm::dotnet::DotNetConfig) -> CorpusRuntime {
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
    CorpusRuntime::run_for_apps(apps, kinds, &manifests, &Default::default(), None, dotnet)
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

/// Partner app A2: A's install code under other object ids, marking "A2" (al-runner issue 8056: two apps install
/// through one event).
const PA2: &str = r#"
codeunit 73100 "PA2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        T: Codeunit "Loggers";
        M: Codeunit "Marks";
    begin
        T.LogMessage('installed');
        M.Mark('A2');
    end;
}
"#;

/// Issue 8056: A and A2 each raise an event that B and B2 subscribe to, and the subscribers change `Sender` only.
/// Four apps meet, more than every order tries (ADR-046 amendment 7); each install reads from B and B2 only, so each
/// runs once per subset of them and the results agree (ADR-079 decision 3).
#[test]
fn two_installs_that_each_meet_two_subscriber_apps_run() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PC_ID, "Partner A2", "Partner", vec![BASE_ID], PA2.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], registering(74000, "PB")),
        (PD_ID, "Partner B2", "Partner", vec![BASE_ID], registering(76000, "PD")),
    ]);
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
    let line = rt.phases.baseline_line().unwrap_or_default();
    assert!(line.contains("ran in 4 orders"), "{line}");
}

/// Issue 8056 with a subscriber that writes a row: A's install leaves another Order Log with B installed before it
/// than without, so a test that reads the log is not run, with the apps and the table named.
#[test]
fn an_install_whose_subset_runs_leave_different_rows_is_refused_for_that_table() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PC_ID, "Partner A2", "Partner", vec![BASE_ID], PA2.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], logging(74000, "PB")),
        (PD_ID, "Partner B2", "Partner", vec![BASE_ID], registering(76000, "PD")),
    ]);
    match outcome(&rt, "ReadsTheLog") {
        Outcome::NotRun(why) => assert!(why.contains("Partner B") && why.contains("Order Log"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
}

/// A partner install that reads Order Log and marks what it saw (ADR-079 decision 6: no meet, a data edge only).
const READER: &str = r#"
codeunit 73200 "Reader Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        L: Record "Order Log";
        M: Codeunit "Marks";
    begin
        if L.IsEmpty() then
            M.Mark('A')
        else
            M.Mark('A saw the log');
    end;
}
"#;

/// A partner install that writes one Order Log row of its own.
fn writer(object: u32, name: &str) -> String {
    format!(
        r#"
codeunit {object} "{name} Install"
{{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        M: Codeunit "Marks";
    begin
        M.Log('{name}');
    end;
}}
"#
    )
}

/// ADR-079 decision 6: two partner installs that meet nothing, one reading what the other writes. Main installs them
/// once; the runner now runs the reader with and without the writer before it, and Install Mark differs, so a test that
/// reads it is not run. The writer's Order Log row is the same in both runs.
#[test]
fn an_install_that_reads_what_another_install_wrote_is_refused_for_the_table_that_differs() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner Reader", "Partner", vec![BASE_ID], READER.into()),
        (PB_ID, "Partner W1", "Partner", vec![BASE_ID], writer(74000, "W1")),
    ]);
    match outcome(&rt, "PartnerInstallRan") {
        Outcome::NotRun(why) => assert!(why.contains("Partner Reader") && why.contains("Install Mark"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
    assert_eq!(outcome(&rt, "ReadsTheLog"), Outcome::Pass);
}

/// ADR-079 decision 5: an install that reads from four apps is beyond the three inputs whose every subset runs; the
/// tables the tied installs wrote are refused.
#[test]
fn an_install_that_reads_from_four_apps_is_refused() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner Reader", "Partner", vec![BASE_ID], READER.into()),
        (PB_ID, "Partner W1", "Partner", vec![BASE_ID], writer(74000, "W1")),
        (PC_ID, "Partner W2", "Partner", vec![BASE_ID], writer(75000, "W2")),
        (PD_ID, "Partner W3", "Partner", vec![BASE_ID], writer(76000, "W3")),
        (PE_ID, "Partner W4", "Partner", vec![BASE_ID], writer(77000, "W4")),
    ]);
    match outcome(&rt, "PartnerInstallRan") {
        Outcome::NotRun(why) => assert!(why.contains("reads from 4 apps"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
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

/// A partner app with no dependency path to A whose subscriber writes five rows: more differences than the refusal shows.
fn logging_five(object: u32, name: &str) -> String {
    format!(
        r#"
codeunit {object} "{name} Logging Subscriber"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Loggers", 'OnRegister', '', false, false)]
    local procedure OnRegister(var Sender: Codeunit "Loggers")
    var
        M: Codeunit "Marks";
        I: Integer;
    begin
        for I := 1 to 5 do
            M.Log('{name} saw a logger ' + Format(I));
    end;
}}
"#
    )
}

/// `--verbose` lists every reason of every install-order tie (al-runner issue 9058; CLI report text, no corpus pin): the
/// refusal of `test` shows the first differences and counts the rest, and every one of them reaches the notes the run
/// prints (`Phases::install_order_differences`).
fn assert_every_difference_noted(rt: &CorpusRuntime, test: &str) {
    let Outcome::NotRun(why) = outcome(rt, test) else {
        panic!("expected not run, got {:?}", outcome(rt, test));
    };
    let more: usize = (why
        .split("; ")
        .find_map(|p| p.strip_suffix(" more, --verbose lists them)")))
    .and_then(|n| n.parse().ok())
    .unwrap_or_else(|| panic!("no hidden differences: {why}"));
    let noted = &rt.phases.install_order_differences;
    let distinct: std::collections::BTreeSet<&String> = noted.iter().collect();
    assert!(distinct.len() >= 3 + more, "{more} hidden, noted: {noted:#?}\n{why}");
}

/// The install code of the apps under test runs once per run (`install_once.rs`).
#[test]
fn every_install_order_difference_of_partner_apps_under_test_is_noted() {
    let rt = run_under_test(vec![
        (PA_ID, "Partner A", PA.into()),
        (PB_ID, "Partner B", logging_five(74000, "PB")),
    ]);
    assert_every_difference_noted(&rt, "ReadsTheLog");
}

/// Partner dependency apps: the install baseline (`baseline.rs`).
#[test]
fn every_install_order_difference_of_partner_dependency_apps_is_noted() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], logging_five(74000, "PB")),
    ]);
    assert_every_difference_noted(&rt, "ReadsTheLog");
}

/// A reader whose five Install Mark rows depend on whether Order Log has rows.
const READER_FIVE: &str = r#"
codeunit 73200 "Reader Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        L: Record "Order Log";
        M: Codeunit "Marks";
        I: Integer;
        Seen: Text;
    begin
        if not L.IsEmpty() then
            Seen := ' saw the log';
        for I := 1 to 5 do
            M.Mark('A' + Format(I) + Seen);
    end;
}
"#;

/// The proof stops after a measured difference (`Unproven`: four apps collide on one row): the difference stays refused
/// and its every line is still noted.
#[test]
fn every_install_order_difference_is_noted_when_the_proof_stops() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner Reader", "Partner", vec![BASE_ID], READER_FIVE.into()),
        (PB_ID, "Partner W1", "Partner", vec![BASE_ID], writer(74000, "W1")),
        (PC_ID, "Partner C1", "Partner", vec![BASE_ID], colliding(75000, "C1")),
        (PD_ID, "Partner C2", "Partner", vec![BASE_ID], colliding(76000, "C2")),
        (PE_ID, "Partner C3", "Partner", vec![BASE_ID], colliding(77000, "C3")),
        (PG_ID, "Partner C4", "Partner", vec![BASE_ID], colliding(78000, "C4")),
    ]);
    assert_every_difference_noted(&rt, "PartnerInstallRan");
}

const PF_ID: &str = "a0000000-0000-0000-0000-000000006644";
const PG_ID: &str = "b0000000-0000-0000-0000-000000006644";

/// A partner install that copies what it sees in Order Log into Install Mark (`X` or `X saw the log`).
const COPIER: &str = r#"
codeunit 73300 "Copier Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        L: Record "Order Log";
        M: Codeunit "Marks";
    begin
        if L.IsEmpty() then
            M.Mark('X')
        else
            M.Mark('X saw the log');
    end;
}
"#;

/// ADR-079 decision 4, clarification of 2026-11-23: A's Order Log differs with B's subscriber installed before A or
/// not; X depends on A and copies what it sees into Install Mark, so Install Mark differs by the order of A and B too.
/// Every order of the group (ADR-046 amendment 7) shows both tables differ; the taint closure refuses Install Mark as
/// well as Order Log.
#[test]
fn a_table_written_from_a_table_that_differs_is_refused_too() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], logging(74000, "PB")),
        (PF_ID, "Partner X", "Partner", vec![BASE_ID, PA_ID], COPIER.into()),
    ]);
    match outcome(&rt, "PartnerInstallRan") {
        Outcome::NotRun(why) => assert!(why.contains("Partner X") && why.contains("Install Mark"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
    match outcome(&rt, "ReadsTheLog") {
        Outcome::NotRun(why) => assert!(why.contains("Order Log"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// Issue 8056 with A2 depending on A, both meeting B: in A2's subset runs A has its place in the fixed order, so its
/// meet with B is not of unknown order (ADR-079 decision 3, clarification of 2026-11-23).
#[test]
fn two_installs_with_a_dependency_path_that_meet_one_subscriber_app_run() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PC_ID, "Partner A2", "Partner", vec![BASE_ID, PA_ID], PA2.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], registering(74000, "PB")),
    ]);
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
}

/// A partner install that calls a builtin the footprint cannot attribute, then marks that it ran.
const UNATTRIBUTED: &str = r#"
codeunit 73400 "Unattributed Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        M: Codeunit "Marks";
        Installing: Boolean;
    begin
        Installing := NavApp.IsInstalling();
        M.Mark('A');
    end;
}
"#;

/// A partner app without install code or subscribers.
fn plain(object: u32, name: &str) -> String {
    format!(
        r#"
codeunit {object} "{name} Plain"
{{
    procedure Ping(): Integer
    begin
        exit(1);
    end;
}}
"#
    )
}

/// ADR-079 decision 1, clarification of 2026-11-23: an install that touches everything ties only apps with an install
/// session of their own, or that it meets. One install session and three apps without install code: no order can
/// matter, so nothing is refused.
#[test]
fn one_install_that_touches_everything_next_to_apps_without_install_code_runs() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner N1", "Partner", vec![BASE_ID], UNATTRIBUTED.into()),
        (PB_ID, "Partner L2", "Partner", vec![BASE_ID], plain(74000, "L2")),
        (PC_ID, "Partner L3", "Partner", vec![BASE_ID], plain(75000, "L3")),
        (PD_ID, "Partner L4", "Partner", vec![BASE_ID], plain(76000, "L4")),
    ]);
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
}

/// A partner install that writes its own JQ Entry row (`no`): no other app writes it.
fn own_row(object: u32, name: &str, no: u32) -> String {
    format!(
        r#"
codeunit {object} "{name} Install"
{{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        E: Record "JQ Entry";
    begin
        E."No." := {no};
        if E.Insert() then;
    end;
}}
"#
    )
}

/// ADR-079 decisions 1 and 5: an install that touches everything (`NavApp.IsInstalling`, not pinned) ties every other
/// app with an install session; four of them are more than the three whose orders the runner tries, so the group is
/// refused, though the three others write disjoint rows (main installs them once and passes; the review of PR 8865,
/// the neighbor shape "everything4").
#[test]
fn an_install_that_touches_everything_next_to_three_other_installs_is_refused() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner N1", "Partner", vec![BASE_ID], UNATTRIBUTED.into()),
        (PB_ID, "Partner N2", "Partner", vec![BASE_ID], own_row(74000, "N2", 2)),
        (PC_ID, "Partner N3", "Partner", vec![BASE_ID], own_row(75000, "N3", 3)),
        (PD_ID, "Partner N4", "Partner", vec![BASE_ID], own_row(76000, "N4", 4)),
    ]);
    match outcome(&rt, "PartnerInstallRan") {
        Outcome::NotRun(why) => assert!(
            why.contains("4 apps tied by their installs")
                && why.contains("touch everything")
                // al-runner issue 8954: the reason names the call site and the install that reached it.
                && why.contains("\"Unattributed Install\".OnInstallAppPerCompany")
                && why.contains("in the install of Partner N1"),
            "{why}"
        ),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A partner install that builds a local `System.Text.StringBuilder`, calls `call` on it (`B.Append('S1')`) and writes
/// its text into its own JQ Entry row (`no`); the first (`no` 1) also writes the Install Mark that `PartnerInstallRan`
/// reads.
fn string_builder(object: u32, name: &str, no: u32, call: &str) -> String {
    format!(
        r#"
dotnet {{ assembly("mscorlib") {{ type("System.Text.StringBuilder"; "{name} Builder") {{ }} }} }}

codeunit {object} "{name} Install"
{{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        B: DotNet "{name} Builder";
        E: Record "JQ Entry";
        M: Codeunit "Marks";
    begin
        B := B.StringBuilder();
        {call};
        E."No." := {no};
        E.Note := B.ToString();
        if E.Insert() then;
        if {no} = 1 then
            M.Mark('A');
    end;
}}
"#
    )
}

/// al-runner issue 8954, the outside reporter's shape: four installs that each build a local StringBuilder through members
/// served in process touch no lasting resource with it (the objects live no longer than the install session, ADR-079
/// fact 1); their disjoint rows tie nothing, so the group is proven.
#[test]
fn four_installs_with_a_local_string_builder_run() {
    let rt = run_on(
        vec![
            base(),
            (
                PA_ID,
                "Partner S1",
                "Partner",
                vec![BASE_ID],
                string_builder(73400, "S1", 1, "B.Append('S1')"),
            ),
            (
                PB_ID,
                "Partner S2",
                "Partner",
                vec![BASE_ID],
                string_builder(74000, "S2", 2, "B.Append('S2')"),
            ),
            (
                PC_ID,
                "Partner S3",
                "Partner",
                vec![BASE_ID],
                string_builder(75000, "S3", 3, "B.Append('S3')"),
            ),
            (
                PD_ID,
                "Partner S4",
                "Partner",
                vec![BASE_ID],
                string_builder(76000, "S4", 4, "B.Append('S4')"),
            ),
        ],
        al_vm::dotnet::DotNetConfig {
            host: std::sync::Arc::new(al_host::inproc::InprocHost::new(
                super::baseline_cache::with_dotnet().host,
            )),
            ..Default::default()
        },
    );
    assert_eq!(outcome(&rt, "PartnerInstallRan"), Outcome::Pass);
}

/// A .NET host that answers every construction with a new handle, `ToString` with a text and every other call with the
/// StringBuilder it was called on.
struct Answering(u64);

impl al_host::dotnet::DotNetSession for Answering {
    fn construct(&mut self, ty: &al_host::dotnet::DotNetType, _: Vec<al_host::dotnet::DotNetArg>) -> Answer {
        self.0 += 1;
        Ok(al_host::dotnet::DotNetReply::value(
            al_host::dotnet::DotNetValue::Handle {
                handle: self.0,
                ty: ty.name.clone(),
            },
        ))
    }
    fn invoke(&mut self, t: al_host::dotnet::DotNetTarget, m: &str, _: Vec<al_host::dotnet::DotNetArg>) -> Answer {
        let value = match (t, m) {
            (_, "ToString") => al_host::dotnet::DotNetValue::Str("text".into()),
            (al_host::dotnet::DotNetTarget::Instance(handle), _) => al_host::dotnet::DotNetValue::Handle {
                handle,
                ty: "System.Text.StringBuilder".into(),
            },
            _ => al_host::dotnet::DotNetValue::Null,
        };
        Ok(al_host::dotnet::DotNetReply::value(value))
    }
    fn get_property(&mut self, _: al_host::dotnet::DotNetTarget, _: &str) -> Answer {
        Ok(al_host::dotnet::DotNetReply::value(al_host::dotnet::DotNetValue::Null))
    }
    fn set_property(&mut self, _: al_host::dotnet::DotNetTarget, _: &str, _: al_host::dotnet::DotNetValue) -> Answer {
        Ok(al_host::dotnet::DotNetReply::value(al_host::dotnet::DotNetValue::Null))
    }
    fn release(&mut self, _: &[al_host::dotnet::DotNetHandle]) {}
    fn close(self: Box<Self>) {}
}

type Answer = Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure>;

struct AnsweringHost;

impl al_host::dotnet::DotNetHost for AnsweringHost {
    fn availability(&self) -> Result<(), al_host::dotnet::DotNetUnavailable> {
        Ok(())
    }
    fn open_session(&self, _: &al_host::dotnet::DotNetSessionConfig) -> Box<dyn al_host::dotnet::DotNetSession> {
        Box::new(Answering(0))
    }
}

/// al-runner issue 8954, ADR-079 amendment 1: `StringBuilder.Insert` is not in `inproc.txt`, so each of four installs
/// that call it touches everything; the four tie into one unit larger than three, which stays refused as on main
/// (a neighbor app cannot show it: no test runs on either side).
#[test]
fn four_installs_tied_by_a_dotnet_member_that_is_not_kept_are_not_run() {
    let insert = |o, n: &str, no| string_builder(o, n, no, &format!("B.Insert(0, '{n}')"));
    let rt = run_on(
        vec![
            base(),
            (PA_ID, "Partner I1", "Partner", vec![BASE_ID], insert(73400, "I1", 1)),
            (PB_ID, "Partner I2", "Partner", vec![BASE_ID], insert(74000, "I2", 2)),
            (PC_ID, "Partner I3", "Partner", vec![BASE_ID], insert(75000, "I3", 3)),
            (PD_ID, "Partner I4", "Partner", vec![BASE_ID], insert(76000, "I4", 4)),
        ],
        al_vm::dotnet::DotNetConfig {
            host: std::sync::Arc::new(al_host::inproc::InprocHost::new(std::sync::Arc::new(AnsweringHost))),
            ..Default::default()
        },
    );
    match outcome(&rt, "PartnerInstallRan") {
        Outcome::NotRun(why) => assert!(
            why.contains("4 apps tied by their installs")
                && why.contains("a .NET call (System.Text.StringBuilder.Insert)"),
            "{why}"
        ),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A partner install that writes JQ Entry 9 with its own note: four of them write the same row.
fn colliding(object: u32, name: &str) -> String {
    format!(
        r#"
codeunit {object} "{name} Install"
{{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        E: Record "JQ Entry";
    begin
        E."No." := 9;
        E.Note := '{name}';
        if not E.Insert() then
            E.Modify();
    end;
}}
"#
    )
}

/// ADR-079 decision 3: a reader whose Install Mark differs by the order stays refused when four other apps of the
/// group cannot be proven (they collide on one JQ Entry row).
#[test]
fn a_measured_difference_stays_refused_next_to_apps_that_cannot_be_proven() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner Reader", "Partner", vec![BASE_ID], READER.into()),
        (PB_ID, "Partner W1", "Partner", vec![BASE_ID], writer(74000, "W1")),
        (PC_ID, "Partner C1", "Partner", vec![BASE_ID], colliding(75000, "C1")),
        (PD_ID, "Partner C2", "Partner", vec![BASE_ID], colliding(76000, "C2")),
        (PE_ID, "Partner C3", "Partner", vec![BASE_ID], colliding(77000, "C3")),
        (PG_ID, "Partner C4", "Partner", vec![BASE_ID], colliding(78000, "C4")),
    ]);
    match outcome(&rt, "PartnerInstallRan") {
        Outcome::NotRun(why) => assert!(why.contains("Partner Reader"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A partner app whose subscriber sets an isolated storage key of its own.
fn storing(object: u32, name: &str) -> String {
    format!(
        r#"
codeunit {object} "{name} Storing Subscriber"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Loggers", 'OnRegister', '', false, false)]
    local procedure OnRegister(var Sender: Codeunit "Loggers")
    begin
        IsolatedStorage.Set('seen', 'yes', DataScope::Module);
    end;
}}
"#
    )
}

/// ADR-079 decision 3: A's install leaves an isolated storage key with B installed before it and none without. The
/// orders differ outside the table rows, so the proof stops and every table the installs wrote is refused.
#[test]
fn installs_whose_orders_differ_in_isolated_storage_are_refused() {
    let rt = run(vec![
        base(),
        (PA_ID, "Partner A", "Partner", vec![BASE_ID], PA.into()),
        (PB_ID, "Partner B", "Partner", vec![BASE_ID], storing(74000, "PB")),
    ]);
    let line = rt.phases.baseline_line().unwrap_or_default();
    assert!(line.contains("more than table rows"), "{line}");
}

/// Review round 2 of ADR-079, finding N1: the events app of the hidden-meet shape. `AskOne` and `AskTwo` raise an event
/// and give what its subscribers answered.
const ASKING: &str = r#"
codeunit 76500 "Asking"
{
    procedure AskOne(): Boolean
    var
        Seen: Boolean;
    begin
        OnOne(Seen);
        exit(Seen);
    end;

    procedure AskTwo(): Boolean
    var
        Seen: Boolean;
    begin
        OnTwo(Seen);
        exit(Seen);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnOne(var Seen: Boolean)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnTwo(var Seen: Boolean)
    begin
    end;
}
"#;

/// D's install raises `OnTwo` and marks that it ran.
const ASKS_TWO: &str = r#"
codeunit 76600 "D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        A: Codeunit "Asking";
        M: Codeunit "Marks";
    begin
        if A.AskTwo() then;
        M.Mark('D');
    end;
}
"#;

/// X's install raises `OnOne`, then logs what `OnTwo` answered.
const ASKS_BOTH: &str = r#"
codeunit 76700 "X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        A: Codeunit "Asking";
        M: Codeunit "Marks";
    begin
        if A.AskOne() then;
        if A.AskTwo() then
            M.Log('X SEEN')
        else
            M.Log('X EMPTY');
    end;
}
"#;

/// A subscriber to `event` of "Asking" that answers true.
fn answering(object: u32, name: &str, event: &str) -> String {
    format!(
        r#"
codeunit {object} "{name} Answer"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Asking", '{event}', '', false, false)]
    local procedure Answer(var Seen: Boolean)
    begin
        Seen := true;
    end;
}}
"#
    )
}

/// ADR-079 decision 3, clarification of 2026-11-23 (review round 2, N1): X depends on D. D's install meets Z (`OnTwo`);
/// X's install meets B (`OnOne`) first, then Z, and logs Z's answer. Z is in X's run set because D reads from it, and the
/// plan places it after X. With Z installed before X the log holds "X SEEN", after X "X EMPTY". The graph has no edge
/// "X reads from Z" before X's runs, so Z placed after X stays of unknown order: the meet is noted, and the subset runs
/// measure the difference.
#[test]
fn a_meet_with_an_app_placed_after_the_install_is_noted() {
    let rt = run(vec![
        base(),
        (PE_ID, "Partner E", "Partner", vec![BASE_ID], ASKING.into()),
        (PD_ID, "Partner D", "Partner", vec![BASE_ID, PE_ID], ASKS_TWO.into()),
        (
            PC_ID,
            "Partner X",
            "Partner",
            vec![BASE_ID, PE_ID, PD_ID],
            ASKS_BOTH.into(),
        ),
        (
            PB_ID,
            "Partner B",
            "Partner",
            vec![BASE_ID, PE_ID],
            answering(76800, "B", "OnOne"),
        ),
        (
            PA_ID,
            "Partner Z",
            "Partner",
            vec![BASE_ID, PE_ID],
            answering(76900, "Z", "OnTwo"),
        ),
    ]);
    match outcome(&rt, "ReadsTheLog") {
        Outcome::NotRun(why) => assert!(why.contains("Order Log"), "{why}"),
        other => panic!(
            "expected not run, got {other:?}; {}",
            rt.phases.baseline_line().unwrap_or_default()
        ),
    }
}

/// P's install raises `OnOne` and logs what it answered.
const ASKS_ONE: &str = r#"
codeunit 77100 "P Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        A: Codeunit "Asking";
        M: Codeunit "Marks";
    begin
        if A.AskOne() then
            M.Log('P SEEN')
        else
            M.Log('P EMPTY');
    end;
}
"#;

/// ADR-079 decision 3, clarification of 2026-11-23 (review round 3, N2): A depends on P. P's install meets G (`OnOne`)
/// and logs G's answer; G's install meets A (`OnTwo`). G and P have no dependency path, and P comes first in the plan.
/// With G installed before P the log holds "P SEEN", after P "P EMPTY". The read order (A, G, P) runs against the
/// dependency order (P before A), so the run with G before P cannot be built: the proof stops and the log stays refused.
#[test]
fn a_read_order_against_the_dependency_order_is_refused() {
    let rt = run(vec![
        base(),
        (PE_ID, "Partner E", "Partner", vec![BASE_ID], ASKING.into()),
        (PA_ID, "Partner P", "Partner", vec![BASE_ID, PE_ID], ASKS_ONE.into()),
        (
            PB_ID,
            "Partner G",
            "Partner",
            vec![BASE_ID, PE_ID],
            format!(
                "{}{}",
                ASKS_TWO.replace("76600 \"D Install\"", "77200 \"G Install\""),
                answering(77300, "G", "OnOne")
            ),
        ),
        (
            PC_ID,
            "Partner Q",
            "Partner",
            vec![BASE_ID, PE_ID, PA_ID],
            answering(77400, "Q", "OnTwo"),
        ),
    ]);
    let note = rt.phases.baseline_line().unwrap_or_default();
    match outcome(&rt, "ReadsTheLog") {
        Outcome::NotRun(why) => assert!(why.contains("Order Log"), "{why}"),
        other => panic!("expected not run, got {other:?}; {note}"),
    }
    assert!(note.contains("an order their dependencies rule out"), "{note}");
}

/// P's install raises `OnOne` and logs a constant: what it answered does not reach the database.
const ASKS_ONE_CONST: &str = r#"
codeunit 77500 "P Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        A: Codeunit "Asking";
        M: Codeunit "Marks";
    begin
        if A.AskOne() then;
        M.Log('P CONST');
    end;
}
"#;

/// The apps of review round 4 (R1): P's install is `p_install`; L and X answer `OnOne`, and X depends on L (a base app
/// and its extension). L and X are two inputs of P with a dependency path between them.
fn dependent_inputs(p_install: &str) -> CorpusRuntime {
    run(vec![
        base(),
        (PE_ID, "Partner E", "Partner", vec![BASE_ID], ASKING.into()),
        (PA_ID, "Partner P", "Partner", vec![BASE_ID, PE_ID], p_install.into()),
        (
            PB_ID,
            "Partner L",
            "Partner",
            vec![BASE_ID, PE_ID],
            answering(77600, "L", "OnOne"),
        ),
        (
            PC_ID,
            "Partner X",
            "Partner",
            vec![BASE_ID, PE_ID, PB_ID],
            answering(77700, "X", "OnOne"),
        ),
    ])
}

/// ADR-079 decision 3, clarification of 2026-11-23 (review round 4, R1): the subset "X before P, L after P" is no real
/// order, since X needs L first. The proof skips it; the other subsets cover every real order and leave the same log.
#[test]
fn an_install_whose_inputs_depend_on_each_other_runs() {
    let rt = dependent_inputs(ASKS_ONE_CONST);
    assert_eq!(
        outcome(&rt, "ReadsTheLog"),
        Outcome::Pass,
        "{}",
        rt.phases.baseline_line().unwrap_or_default()
    );
}

/// The same apps, with P logging what `OnOne` answered: L before P gives "P SEEN", L after P "P EMPTY".
#[test]
fn an_install_whose_dependent_inputs_change_its_rows_is_refused() {
    let rt = dependent_inputs(ASKS_ONE);
    match outcome(&rt, "ReadsTheLog") {
        Outcome::NotRun(why) => assert!(why.contains("Order Log"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// ADR-079 decision 3, clarification of 2026-11-23 (review round 4, the final install): B's install meets C; A depends
/// on B and comes before C in the list of units. The final install puts A after B, so no order the dependencies rule
/// out is requested.
#[test]
fn the_final_install_puts_an_app_after_the_install_it_depends_on() {
    let rt = run(vec![
        base(),
        (PE_ID, "Partner E", "Partner", vec![BASE_ID], ASKING.into()),
        (
            PA_ID,
            "Partner B",
            "Partner",
            vec![BASE_ID, PE_ID],
            ASKS_ONE_CONST.into(),
        ),
        (
            PB_ID,
            "Partner A",
            "Partner",
            vec![BASE_ID, PE_ID, PA_ID],
            answering(77800, "A", "OnTwo"),
        ),
        (
            PC_ID,
            "Partner C",
            "Partner",
            vec![BASE_ID, PE_ID],
            answering(77900, "C", "OnOne"),
        ),
    ]);
    assert_eq!(
        outcome(&rt, "ReadsTheLog"),
        Outcome::Pass,
        "{}",
        rt.phases.baseline_line().unwrap_or_default()
    );
}
