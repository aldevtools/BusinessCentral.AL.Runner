//! al-runner issues 6972 and 6644 (reopened 2026-11-09): partner apps with no dependency path between them that meet
//! only through events where the install order cannot change the database. Each shape from the reporter is built with
//! more than three partner apps, so the comparison of every order (ADR-046 amendment 7, at most three apps) cannot
//! cover it; the install code must run to its end without trying the orders.
//!
//! 1. A `OnAfterGetDatabaseTableTriggerSetup` subscriber that answers from its own app's setup table through a
//!    SingleInstance cache, a table no install code writes (issue 6972).
//! 2. A subscriber that only registers a logger with `Sender`, whose logger keeps its messages in memory (issue 6644 a).
//! 3. Upgrade tags whose rows differ between the orders only in a timestamp read from the clock (issue 6644 b).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest};
use al_testrun::{CaseId, Outcome};

const BASE_ID: &str = "10000000-0000-0000-0000-000000006972";
const INSTALLER_ID: &str = "20000000-0000-0000-0000-000000006972";
const MAIN_ID: &str = "90000000-0000-0000-0000-000000006972";

/// Microsoft's side: a trigger setup event with `var` flags (Base Application's `GlobalTriggerManagement`), a logger
/// registry handed to subscribers as `Sender` (System Application's "Telemetry Loggers", not SingleInstance), and an
/// upgrade tag toolkit that stamps each new tag with the clock.
const BASE: &str = r#"
table 70000 "Install Mark"
{
    DataPerCompany = false;
    fields { field(1; App; Text[30]) { } }
    keys { key(PK; App) { Clustered = true; } }
}
table 70001 "Trigger Log"
{
    DataPerCompany = false;
    fields { field(1; TableId; Integer) { } }
    keys { key(PK; TableId) { Clustered = true; } }
}
table 70002 "Upgrade Tag"
{
    DataPerCompany = false;
    fields { field(1; Tag; Code[250]) { } field(2; "Tag Timestamp"; DateTime) { } }
    keys { key(PK; Tag) { Clustered = true; } }
}
table 70003 "Logged Row"
{
    DataPerCompany = false;
    fields { field(1; Msg; Text[100]) { } }
    keys { key(PK; Msg) { Clustered = true; } }
}
codeunit 70010 "Marks"
{
    procedure Mark(App: Text)
    var
        M: Record "Install Mark";
    begin
        M.App := CopyStr(App, 1, 30);
        if M.Insert() then;
    end;
}
codeunit 70011 "Global Trigger Mgt"
{
    procedure Setup(TableId: Integer)
    var
        OnDatabaseInsert: Boolean;
        OnDatabaseModify: Boolean;
        L: Record "Trigger Log";
    begin
        OnAfterGetDatabaseTableTriggerSetup(TableId, OnDatabaseInsert, OnDatabaseModify);
        if OnDatabaseInsert or OnDatabaseModify then begin
            L.TableId := TableId;
            if L.Insert() then;
        end;
    end;

    [IntegrationEvent(false, false)]
    local procedure OnAfterGetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean)
    begin
    end;
}
interface "Telemetry Logger"
{
    procedure LogMessage(Msg: Text);
}
codeunit 70012 "Telemetry Loggers"
{
    var
        Registered: List of [Interface "Telemetry Logger"];

    procedure Register(TelemetryLogger: Interface "Telemetry Logger")
    begin
        Registered.Add(TelemetryLogger);
    end;

    procedure LogAll(Msg: Text)
    var
        L: Interface "Telemetry Logger";
    begin
        foreach L in Registered do
            L.LogMessage(Msg);
    end;

    [IntegrationEvent(true, false)]
    procedure OnRegisterTelemetryLogger()
    begin
    end;
}
codeunit 70013 "Telemetry"
{
    procedure LogMessage(Msg: Text)
    var
        Loggers: Codeunit "Telemetry Loggers";
    begin
        Loggers.OnRegisterTelemetryLogger();
        Loggers.LogAll(Msg);
    end;
}
codeunit 70014 "Upgrade Tag Mgt"
{
    procedure SetAllUpgradeTags()
    var
        Tags: List of [Code[250]];
        T: Code[250];
        U: Record "Upgrade Tag";
    begin
        OnGetPerCompanyUpgradeTags(Tags);
        foreach T in Tags do
            if not U.Get(T) then begin
                U.Init();
                U.Tag := T;
                U."Tag Timestamp" := CreateDateTime(Today(), Time());
                U.Insert();
            end;
    end;

    [IntegrationEvent(false, false)]
    local procedure OnGetPerCompanyUpgradeTags(var PerCompanyUpgradeTags: List of [Code[250]])
    begin
    end;
}
"#;

/// The partner app whose install code meets the others: it sets up a table, logs telemetry, then marks that it ran.
const INSTALLER: &str = r#"
codeunit 73000 "Installer Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        G: Codeunit "Global Trigger Mgt";
        T: Codeunit "Telemetry";
        M: Codeunit "Marks";
    begin
        G.Setup(Database::"Install Mark");
        T.LogMessage('installed');
        M.Mark('Installer');
    end;
}
"#;

/// Issue 6972's shape (the reporter's real code, guard first): a SingleInstance cache of the app's own setup table.
/// `writes_setup`: the app's install code adds a setup row for "Install Mark", so the flags differ between the orders.
fn trigger_setup_app(base: u32, name: &str, writes_setup: bool) -> String {
    let install = if writes_setup {
        format!(
            r#"
codeunit {i} "{name} Install"
{{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Record "{name} Setup";
    begin
        S."Table ID" := 70000;
        S.Insert();
    end;
}}
"#,
            i = base + 3
        )
    } else {
        String::new()
    };
    format!(
        r#"
table {t} "{name} Setup"
{{
    DataPerCompany = false;
    fields {{ field(1; "Table ID"; Integer) {{ }} }}
    keys {{ key(PK; "Table ID") {{ Clustered = true; }} }}
}}
codeunit {c} "{name} Cache"
{{
    SingleInstance = true;

    var
        Tables: List of [Integer];
        LastLoadAt: DateTime;
        SkipUpdate: Boolean;

    procedure HasSetupForTable(TableNo: Integer): Boolean
    begin
        RefreshSetupIfExpired();
        exit(Tables.Contains(TableNo));
    end;

    local procedure RefreshSetupIfExpired()
    begin
        if SkipUpdate then
            exit;
        if LastLoadAt = 0DT then begin
            Load();
            exit;
        end;
        if (CurrentDateTime() - LastLoadAt) >= 60000 then
            Load();
    end;

    local procedure Load()
    var
        S: Record "{name} Setup";
    begin
        Clear(Tables);
        if S.FindSet() then
            repeat
                Tables.Add(S."Table ID");
            until S.Next() = 0;
        LastLoadAt := CurrentDateTime();
    end;
}}
codeunit {s} "{name} Trigger Subscriber"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Trigger Mgt", 'OnAfterGetDatabaseTableTriggerSetup', '', false, false)]
    local procedure OnAfterGetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean)
    var
        Cache: Codeunit "{name} Cache";
    begin
        if not Cache.HasSetupForTable(TableId) then
            exit;
        OnDatabaseInsert := true;
        OnDatabaseModify := true;
    end;
}}
{install}"#,
        t = base,
        c = base + 1,
        s = base + 2,
    )
}

/// Issue 6644 (a): a subscriber that only registers its logger with `Sender`. `writes`: the logger writes a row.
fn logger_app(base: u32, name: &str, writes: bool) -> String {
    let body = if writes {
        "L.Msg := CopyStr(Msg, 1, 100); if L.Insert() then;"
    } else {
        "Messages.Add(Msg);"
    };
    format!(
        r#"
codeunit {c} "{name} Logger" implements "Telemetry Logger"
{{
    SingleInstance = true;

    var
        Messages: List of [Text];

    procedure LogMessage(Msg: Text)
    var
        L: Record "Logged Row";
    begin
        {body}
    end;
}}
codeunit {s} "{name} Logger Registration"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Telemetry Loggers", 'OnRegisterTelemetryLogger', '', true, true)]
    local procedure OnRegisterTelemetryLogger(var Sender: Codeunit "Telemetry Loggers")
    var
        MyLogger: Codeunit "{name} Logger";
    begin
        Sender.Register(MyLogger);
    end;
}}
"#,
        c = base,
        s = base + 1,
    )
}

/// Issue 6644 (b): an app that adds its constant upgrade tag and sets all tags when it installs.
fn upgrade_tag_app(base: u32, name: &str) -> String {
    format!(
        r#"
codeunit {c} "{name} Upgrade Tags"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Upgrade Tag Mgt", 'OnGetPerCompanyUpgradeTags', '', false, false)]
    local procedure OnGetPerCompanyUpgradeTags(var PerCompanyUpgradeTags: List of [Code[250]])
    begin
        PerCompanyUpgradeTags.Add('{name}-TAG');
    end;
}}
codeunit {i} "{name} Install"
{{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        U: Codeunit "Upgrade Tag Mgt";
        M: Codeunit "Marks";
    begin
        U.SetAllUpgradeTags();
        M.Mark('{name}');
    end;
}}
"#,
        c = base,
        i = base + 1,
    )
}

const TESTS: &str = r#"
codeunit 79000 "Independent Order Tests"
{
    Subtype = Test;

    [Test]
    procedure InstallerRan()
    var
        M: Record "Install Mark";
    begin
        M.Get('Installer');
    end;

    [Test]
    procedure TriggerLogIsEmpty()
    var
        L: Record "Trigger Log";
    begin
        if not L.IsEmpty() then
            Error('trigger log has rows');
    end;

    [Test]
    procedure LoggedRowsAreEmpty()
    var
        L: Record "Logged Row";
    begin
        if not L.IsEmpty() then
            Error('logged rows');
    end;

    [Test]
    procedure BothTagsAreSet()
    var
        U: Record "Upgrade Tag";
    begin
        U.Get('TA-TAG');
        U.Get('TB-TAG');
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

fn partner_id(n: usize) -> String {
    format!("{:08}-0000-0000-0000-000000006972", 30_000_000 + n)
}

/// Runs the test app over the Microsoft base app and the partner apps `partners` (name, source), all of which depend on
/// the base app only. `under_test`: the partner apps are apps under test, else dependency apps (the install baseline).
fn run(partners: &[(String, String)], under_test: bool) -> CorpusRuntime {
    let mut apps = vec![app(BASE_ID, "Base", &[], BASE)];
    let mut kinds = vec![al_sema::AppKind::Dependency];
    let mut manifests = vec![manifest("Microsoft")];
    let mut all = vec![BASE_ID.to_string()];
    for (n, (name, src)) in partners.iter().enumerate() {
        let id = if name == "Installer" {
            INSTALLER_ID.to_string()
        } else {
            partner_id(n)
        };
        apps.push(app(&id, name, &[BASE_ID], src));
        kinds.push(if under_test {
            al_sema::AppKind::Workspace
        } else {
            al_sema::AppKind::Dependency
        });
        manifests.push(manifest("Partner"));
        all.push(id);
    }
    let deps: Vec<&str> = all.iter().map(String::as_str).collect();
    apps.push(app(MAIN_ID, "Independent Order Tests", &deps, TESTS));
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

fn installer() -> (String, String) {
    ("Installer".into(), INSTALLER.into())
}

/// Four apps with issue 6972's subscriber and the installer: five interacting apps, more than the orders tried.
fn trigger_setup_apps() -> Vec<(String, String)> {
    let mut v = vec![installer()];
    for (k, name) in ["SA", "SB", "SC", "SD"].iter().enumerate() {
        v.push((name.to_string(), trigger_setup_app(74000 + 10 * k as u32, name, false)));
    }
    v
}

#[test]
fn trigger_setup_subscriber_reading_its_own_unwritten_table_does_not_depend_on_the_order() {
    for under_test in [false, true] {
        let rt = run(&trigger_setup_apps(), under_test);
        assert_eq!(outcome(&rt, "InstallerRan"), Outcome::Pass, "under test: {under_test}");
        assert_eq!(
            outcome(&rt, "TriggerLogIsEmpty"),
            Outcome::Pass,
            "under test: {under_test}"
        );
    }
}

/// The same subscriber, but its app's install code writes the setup row for the table set up: the flags depend on the
/// order, so the orders are compared and the trigger log differs.
#[test]
fn trigger_setup_subscriber_whose_table_install_code_writes_still_depends_on_the_order() {
    let rt = run(
        &[installer(), ("SA".into(), trigger_setup_app(74000, "SA", true))],
        false,
    );
    match outcome(&rt, "TriggerLogIsEmpty") {
        Outcome::NotRun(why) => assert!(why.contains("Trigger Log"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

#[test]
fn subscriber_that_only_registers_a_logger_writing_nothing_does_not_depend_on_the_order() {
    let mut v = vec![installer()];
    for (k, name) in ["LA", "LB", "LC", "LD"].iter().enumerate() {
        v.push((name.to_string(), logger_app(75000 + 10 * k as u32, name, false)));
    }
    for under_test in [false, true] {
        let rt = run(&v, under_test);
        assert_eq!(outcome(&rt, "InstallerRan"), Outcome::Pass, "under test: {under_test}");
        assert_eq!(
            outcome(&rt, "LoggedRowsAreEmpty"),
            Outcome::Pass,
            "under test: {under_test}"
        );
    }
}

/// A registered logger that writes a row when the installer logs: the rows depend on the order.
#[test]
fn registered_logger_that_writes_a_row_still_depends_on_the_order() {
    let rt = run(&[installer(), ("LA".into(), logger_app(75000, "LA", true))], false);
    match outcome(&rt, "LoggedRowsAreEmpty") {
        Outcome::NotRun(why) => assert!(why.contains("Logged Row"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// Issue 6644 (b): both orders insert the same two tags; only "Tag Timestamp", read from the clock, differs.
#[test]
fn rows_that_differ_only_in_values_read_from_the_clock_count_as_the_same() {
    for under_test in [false, true] {
        let rt = run(
            &[
                ("TA".into(), upgrade_tag_app(76000, "TA")),
                ("TB".into(), upgrade_tag_app(76010, "TB")),
            ],
            under_test,
        );
        assert_eq!(
            outcome(&rt, "BothTagsAreSet"),
            Outcome::Pass,
            "under test: {under_test}"
        );
    }
}

/// A difference that is not a clock value is named with its table, key and field.
#[test]
fn a_difference_names_its_table_key_and_field() {
    let rt = run(
        &[installer(), ("SA".into(), trigger_setup_app(74000, "SA", true))],
        false,
    );
    match outcome(&rt, "TriggerLogIsEmpty") {
        Outcome::NotRun(why) => assert!(why.contains("Trigger Log, key 70000"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}
