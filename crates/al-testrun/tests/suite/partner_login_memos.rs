//! The login memos of al-runner issue 8945: what a partner dependency app's login subscribers left in Microsoft's caches
//! (stand-ins with the measured global names, in an app published as "Microsoft"), and which entry a later test read marks
//! (corpus PR 1571, merged, not yet pinned: cells M1, M3, M4, E1). Every other shape refuses the read.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

/// The stand-in "Microsoft" app: the login publisher, GlobalTriggerManagement passing the Global Triggers events on, a setup subscriber that turns on every database event (as Microsoft's
/// Backup Management of the test libraries does), and memos with the globals of 423 (loaded in the setup ask of table 60805
/// and on every insert event), 3702, 6153 (asked on the insert event of table 60800) and 9177.
const MS: &str = r#"
codeunit 150 "System Initialization" { procedure Raise() begin OnAfterLogin(); end;
    [IntegrationEvent(false, false, false)] procedure OnAfterLogin() begin end; }
codeunit 151 "System Initialization Impl." {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany() var S: Codeunit "System Initialization"; begin S.Raise(); end; }
codeunit 130011 "Backup Stand-in" {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin OnDatabaseInsert := true; end; }
codeunit 49 GlobalTriggerManagement {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin OnAfterGetDatabaseTableTriggerSetup(TableId, OnDatabaseInsert, OnDatabaseModify, OnDatabaseDelete, OnDatabaseRename); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure I(RecRef: RecordRef) begin OnAfterOnDatabaseInsert(RecRef); end;
    [IntegrationEvent(false, false)]
    procedure OnAfterGetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) begin end;
    [IntegrationEvent(false, false)] procedure OnAfterOnDatabaseInsert(RecRef: RecordRef) begin end; }
table 405 "Change Log Setup" { fields { field(1; "Primary Key"; Code[10]) { } field(2; "Change Log Activated"; Boolean) { } }
    keys { key(PK; "Primary Key") { } } }
codeunit 423 "Change Log Management" { SingleInstance = true;
    var ChangeLogSetup: Record "Change Log Setup"; ChangeLogSetupRead: Boolean; MonitorSensitiveFieldSetupRead: Boolean;
    procedure Load() begin if ChangeLogSetupRead then exit; if ChangeLogSetup.Get() then; ChangeLogSetupRead := true; MonitorSensitiveFieldSetupRead := true; end;
    procedure IsRead(): Boolean begin exit(ChangeLogSetupRead); end;
    procedure Reset() begin ChangeLogSetupRead := false; MonitorSensitiveFieldSetupRead := false; end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::GlobalTriggerManagement, 'OnAfterGetDatabaseTableTriggerSetup', '', false, false)]
    local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin if TableId = 60805 then Load(); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DI(RecRef: RecordRef) begin Load(); end; }
codeunit 6153 "API Webhook Notification Mgt." { SingleInstance = true;
    var CachedApiSubscriptionEnabled: Boolean; UseCachedApiSubscriptionEnabled: Boolean;
    procedure Ask() begin if UseCachedApiSubscriptionEnabled then exit; UseCachedApiSubscriptionEnabled := true; CachedApiSubscriptionEnabled := true; end;
    procedure Asked(): Boolean begin exit(UseCachedApiSubscriptionEnabled); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DI(RecRef: RecordRef) begin if RecRef.Number = 60800 then Ask(); end; }
codeunit 3702 "Environment Information Impl." { SingleInstance = true;
    var IsSaasInitialized: Boolean; IsSaaSConfig: Boolean; IsSandboxConfig: Boolean; IsSandboxInitialized: Boolean;
    procedure IsSaaS(): Boolean begin if not IsSaasInitialized then begin IsSaasInitialized := true; IsSaaSConfig := true; end; exit(IsSaaSConfig); end;
    procedure IsSandbox(): Boolean begin if not IsSandboxInitialized then begin IsSandboxInitialized := true; IsSandboxConfig := true; end; exit(IsSandboxConfig); end;
    procedure ForgetSaaS() begin IsSaasInitialized := false; end; }
codeunit 9177 "Application Area Cache" { SingleInstance = true;
    var ApplicationAreaCache: Dictionary of [Text, Text];
    procedure Fill() begin if not ApplicationAreaCache.ContainsKey('User:') then ApplicationAreaCache.Add('User:', '#Basic'); end;
    procedure Areas(): Text var T: Text; begin if ApplicationAreaCache.Get('User:', T) then; exit(T); end; }
"#;

/// The partner app: two login subscribers with bodies `first` and `second` (empty: none) and the tables they may write:
/// PTab (60800), PAsk (60805, whose setup ask loads 423).
fn partner(first: &str, second: &str) -> String {
    let sub = |id: u32, body: &str| {
        if body.is_empty() {
            return String::new();
        }
        format!(
            "codeunit {id} PLogin{id} {{
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
                local procedure L() var E: Codeunit \"Environment Information Impl.\"; C: Codeunit \"Change Log Management\";
                    A: Codeunit \"Application Area Cache\"; W: Codeunit \"API Webhook Notification Mgt.\"; P: Record PTab;
                    Q: Record PAsk; S: Record \"Change Log Setup\"; begin {body} end; }}"
        )
    };
    format!(
        r#"{}
{}
table 60800 PTab {{ fields {{ field(1; K; Integer) {{ }} }} keys {{ key(PK; K) {{ }} }} }}
table 60805 PAsk {{ fields {{ field(1; K; Integer) {{ }} }} keys {{ key(PK; K) {{ }} }} }}
"#,
        sub(60801, first),
        sub(60802, second)
    )
}

/// The test app: one test with body `body` (its variables as in [`partner`]'s subscribers, plus the memo `M`).
fn tests(body: &str) -> String {
    format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60951 A {{ Subtype = Test;
            [Test] procedure Read() var E: Codeunit \"Environment Information Impl.\"; C: Codeunit \"Change Log Management\";
                A: Codeunit \"Application Area Cache\"; W: Codeunit \"API Webhook Notification Mgt.\"; begin {body} end; }}"
    )
}

/// The outcome of test Read and the ids of the assumption entries it marked.
fn run(partner_src: &str, test: &str) -> (Outcome, Vec<&'static str>) {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![
            app("main", &["partner"], &tests(test)),
            app("partner", &["ms"], partner_src),
            app("ms", &[], MS),
        ],
        vec![
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Dependency,
            al_sema::AppKind::Dependency,
        ],
        &[manifest("Me"), manifest("Probe"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    let r = rt
        .results()
        .iter()
        .find(|r| r.test.ends_with("Read"))
        .expect("Read ran")
        .clone();
    (r.outcome, r.assumptions.iter().map(|a| a.info().id).collect())
}

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn manifest(p: &str) -> Manifest {
    Manifest {
        publisher: p.into(),
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

fn assert_flagged(got: (Outcome, Vec<&'static str>), id: &str) {
    assert_eq!(got.0, Outcome::Pass, "{got:?}");
    assert_eq!(got.1, vec![id], "{got:?}");
}

fn assert_refused(got: (Outcome, Vec<&'static str>), why: &str) {
    assert!(matches!(&got.0, Outcome::NotRun(m) if m.contains(why)), "{got:?}");
}

const READ_423: &str = "if not C.IsRead() then Error('not read');";

// Cell M1 (AppAreas_AfterLogin): 9177 filled by a direct call; the test reads the login's areas.
#[test]
fn application_area_cache_filled_directly_is_kept() {
    let got = run(
        &partner("A.Fill();", ""),
        "if A.Areas() <> '#Basic' then Error('areas');",
    );
    assert_flagged(got, "partner-login-microsoft-cache");
}

// Cell M3 (ChangeLog_AfterLogin): 423 loads on the database insert event of the subscriber's write, the setup inactive.
#[test]
fn change_log_loaded_on_the_insert_event_is_kept() {
    let got = run(&partner("P.K := 1; P.Insert();", ""), READ_423);
    assert_flagged(got, "partner-login-microsoft-cache");
}

// Cell M3 in the plain job (LmLoginCacheTests.al:570-581): 423 loads in the setup ask of the login's first write; the test
// session asks it again after the install sessions did (Y69InstallTriggerSetup.al:134).
#[test]
fn change_log_loaded_in_the_setup_ask_is_kept() {
    let got = run(&partner("Q.K := 1; Q.Insert();", ""), READ_423);
    assert_flagged(got, "partner-login-microsoft-cache");
}

// Review of al-runner PR 9028: a cache the setup ask loads, with a test that loads it again and checks it is read. The
// login loaded it (the ask), so the read is kept, not refused.
#[test]
fn change_log_loaded_in_the_ask_then_loaded_by_the_test_is_kept() {
    let got = run(
        &partner("Q.K := 1; Q.Insert();", ""),
        "C.Load(); if not C.IsRead() then Error('not read');",
    );
    assert_flagged(got, "partner-login-microsoft-cache");
}

// Not measured: 423 loaded by a direct call, reset, then loaded again on the insert event (changed outside a raise too).
#[test]
fn change_log_loaded_directly_then_on_insert_refuses() {
    let first = "C.Load(); C.Reset(); P.K := 1; P.Insert();";
    assert_refused(run(&partner(first, ""), READ_423), "not one of the caches");
}

// Cell M4 (ApiCache_CountingSubscriber): 6153 asked on the database insert event of the login's first write.
#[test]
fn api_cache_asked_on_the_insert_event_is_kept() {
    let got = run(&partner(WRITE, ""), "if not W.Asked() then Error('api');");
    assert_flagged(got, "partner-login-microsoft-cache");
}

// Not measured: 6153 asked by a direct call, outside any Global Triggers raise.
#[test]
fn api_cache_asked_outside_a_raise_refuses() {
    let got = run(&partner("W.Ask();", ""), "if not W.Asked() then Error('api');");
    assert_refused(got, "not one of the caches");
}

// Not measured: 423 loaded by a direct call, outside any Global Triggers raise.
#[test]
fn change_log_loaded_outside_a_raise_refuses() {
    assert_refused(
        run(&partner("C.Load();", ""), READ_423),
        "not one of the caches",
    );
}

// Not measured: a setup row at the load (its Get loads the record global, a third changed global).
#[test]
fn change_log_loaded_with_a_setup_row_refuses() {
    let first = "S.\"Primary Key\" := ''; S.Insert(); P.K := 1; P.Insert();";
    assert_refused(run(&partner(first, ""), READ_423), "not one of the caches");
}

// Cell E1 (EnvInfo_AfterLogin): two subscribers of one app, IsSandbox() first, IsSaaS() second; the reads mark the pair's
// entry only.
#[test]
fn env_info_split_over_two_subscribers_is_kept() {
    let got = run(
        &partner("E.IsSandbox();", "E.IsSaaS();"),
        "if not (E.IsSaaS() and E.IsSandbox()) then Error('env');",
    );
    assert_flagged(got, "partner-login-env-info-pair");
}

// Not measured: the reverse order.
#[test]
fn env_info_split_in_the_other_order_refuses() {
    let got = run(
        &partner("E.IsSaaS();", "E.IsSandbox();"),
        "if not E.IsSaaS() then Error('env');",
    );
    assert_refused(got, "two subscribers of the same app");
}

const WRITE: &str = "P.K := 1; P.Insert();";

/// A Global Triggers subscriber of the partner app that calls IsSaaS() on the insert event.
const SAAS_ON_INSERT: &str = "codeunit 60812 PSaaSOnInsert {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseInsert', '', false, false)]
    local procedure I(RecRef: RecordRef) var E: Codeunit \"Environment Information Impl.\"; begin E.IsSaaS(); end; }";

// E1's pair, but the second change is made inside a Global Triggers raise.
#[test]
fn env_info_pair_with_a_change_inside_a_raise_refuses() {
    let src = partner("E.IsSandbox();", WRITE) + SAAS_ON_INSERT;
    let got = run(&src, "if not (E.IsSaaS() and E.IsSandbox()) then Error('env');");
    assert_refused(got, "two subscribers of the same app");
}

// A third login subscriber of the same app that changes 3702.
#[test]
fn env_info_changed_by_a_third_subscriber_refuses() {
    let third = "codeunit 60813 PLogin60813 {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
        local procedure L() var E: Codeunit \"Environment Information Impl.\"; begin E.ForgetSaaS(); end; }";
    let src = partner("E.IsSandbox();", "E.IsSaaS();") + third;
    let got = run(&src, "if not (E.IsSaaS() and E.IsSandbox()) then Error('env');");
    assert_refused(got, "subscribers");
}
