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
    procedure MarkMonitor() begin MonitorSensitiveFieldSetupRead := true; end;
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
    run_main(partner_src, &tests(test))
}

/// [`run`] with the whole source `main_src` of the app under test.
fn run_main(partner_src: &str, main_src: &str) -> (Outcome, Vec<&'static str>) {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![
            app("main", &["partner"], main_src),
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

// Review of al-runner PR 9028 (MED-1): 423 loaded by a direct call and reset before the insert event loads it. The watch
// compares values at each raise boundary, so the undone change leaves no trace: the insert event starts from the state
// cell M3 starts from, and the test reads what M3 measured.
#[test]
fn change_log_loaded_and_reset_directly_then_on_insert_is_kept() {
    let first = "C.Load(); C.Reset(); P.K := 1; P.Insert();";
    assert_flagged(run(&partner(first, ""), READ_423), "partner-login-microsoft-cache");
}

// Not measured (MED-1): one global of 423 changed by a direct call, outside any raise, the other on the insert event.
#[test]
fn change_log_changed_directly_then_on_insert_refuses() {
    let first = "C.MarkMonitor(); P.K := 1; P.Insert();";
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
    assert_refused(run(&partner("C.Load();", ""), READ_423), "not one of the caches");
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

// The setup-ask caches (assumption partner-login-setup-ask-cache, corpus probe pending): SingleInstance codeunits of the
// partner app changed only in the setup ask of the login's write of PAsk (60805), read by a test of an app under test
// with an install codeunit.

/// A partner cache codeunit 60820 PCache (`N: Integer`, `Last: Code[20]`) whose setup subscriber, of the event `event` of
/// `publisher`, fills it for PAsk; `bump` is called by nothing unless a body calls it.
fn ask_cache(publisher: &str, event: &str) -> String {
    format!(
        "codeunit 60820 PCache {{ SingleInstance = true; var N: Integer; Last: Code[20];
            [EventSubscriber(ObjectType::Codeunit, {publisher}, '{event}', '', false, false)]
            local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
            begin if TableId = 60805 then begin N += 1; Last := 'ASKED'; end; end;
            procedure Bump() begin N += 10; end;
            procedure Count(): Integer begin exit(N); end;
            procedure LastAsk(): Code[20] begin exit(Last); end; }}"
    )
}

/// The app under test: its install codeunit and test Read with body `body` (`P`: PCache) and the codeunits `more`.
fn ask_tests(body: &str, more: &str) -> String {
    format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60951 A {{ Subtype = Test; [Test] procedure Read() var P: Codeunit PCache; begin {body} end; }}
        {more}"
    )
}

const WRITE_ASK: &str = "Q.K := 1; Q.Insert();";
const READ_CACHE: &str = "if (P.Count() <> 1) or (P.LastAsk() <> 'ASKED') then Error('cache %1', P.Count());";

// A partner cache whose own Global Triggers setup subscriber fills it in the login's ask: the test reads what the ask left.
#[test]
fn ask_cache_filled_by_its_own_setup_subscriber_is_kept() {
    let src = partner(WRITE_ASK, "") + &ask_cache("Codeunit::\"Global Triggers\"", "GetDatabaseTableTriggerSetup");
    let got = run_main(&src, &ask_tests(READ_CACHE, ""));
    assert_flagged(got, "partner-login-setup-ask-cache");
}

// The same cache filled by a subscriber of the event GlobalTriggerManagement raises inside the ask.
#[test]
fn ask_cache_filled_through_global_trigger_management_is_kept() {
    let src = partner(WRITE_ASK, "")
        + &ask_cache(
            "Codeunit::GlobalTriggerManagement",
            "OnAfterGetDatabaseTableTriggerSetup",
        );
    let got = run_main(&src, &ask_tests(READ_CACHE, ""));
    assert_flagged(got, "partner-login-setup-ask-cache");
}

// A counter of the app under test whose setup subscriber counts the ask of the partner login's write.
#[test]
fn own_ask_counter_is_kept() {
    let own = "codeunit 60952 OwnCount { SingleInstance = true; var N: Integer;
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)]
        local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
        begin if TableId = 60805 then N += 1; end;
        procedure Count(): Integer begin exit(N); end; }";
    let src = partner(WRITE_ASK, "") + &ask_cache("Codeunit::\"Global Triggers\"", "GetDatabaseTableTriggerSetup");
    let main = ask_tests("", own).replace(
        "var P: Codeunit PCache; begin ",
        "var P: Codeunit PCache; O: Codeunit OwnCount; begin if O.Count() <> 1 then Error('own %1', O.Count()); ",
    );
    assert_flagged(run_main(&src, &main), "partner-login-setup-ask-cache");
}

// A later login subscriber of the same app bumps the cache the first one's ask filled: judged as if the fill had not
// happened (a change by one partner subscriber of its own app's Integer cache), the fill's mark stays.
#[test]
fn ask_cache_changed_by_a_later_login_subscriber_keeps_the_fill_mark() {
    let src = partner(WRITE_ASK, "") + &ask_cache("Codeunit::\"Global Triggers\"", "GetDatabaseTableTriggerSetup")
        + "codeunit 60821 PBumper { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() var P: Codeunit PCache; begin P.Bump(); end; }";
    let src = src
        .replace("var N: Integer; Last: Code[20];", "var N: Integer; Last: Text;")
        .replace("): Code[20]", "): Text");
    let got = run_main(
        &src,
        &ask_tests("if P.Count() <> 11 then Error('cache %1', P.Count());", ""),
    );
    assert_flagged(got, "partner-login-setup-ask-cache");
}

// A login subscriber of the app under test bumps the cache after the partner login's ask filled it.
#[test]
fn ask_cache_changed_by_an_own_login_subscriber_keeps_the_fill_mark() {
    let src = partner(WRITE_ASK, "") + &ask_cache("Codeunit::\"Global Triggers\"", "GetDatabaseTableTriggerSetup");
    let own = "codeunit 60954 OwnLogin { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
        local procedure L() var P: Codeunit PCache; begin P.Bump(); end; }";
    let got = run_main(&src, &ask_tests("if P.Count() < 0 then Error('cache');", own));
    assert_flagged(got, "partner-login-setup-ask-cache");
}

// The login subscriber itself changes the cache outside the ask as well: not a fill; the other facts refuse its Code
// global.
#[test]
fn ask_cache_changed_outside_the_ask_too_refuses() {
    let src = partner(&format!("{WRITE_ASK} PC.Bump();"), "")
        .replace("Q: Record PAsk;", "Q: Record PAsk; PC: Codeunit PCache;")
        + &ask_cache("Codeunit::\"Global Triggers\"", "GetDatabaseTableTriggerSetup");
    let got = run_main(&src, &ask_tests(READ_CACHE, ""));
    assert_refused(got, "of a type other than");
}

// A setup subscriber of the app under test fills the partner's cache: code of another app, not a fill; the other facts
// refuse its Code global.
#[test]
fn ask_cache_changed_by_another_apps_setup_subscriber_refuses() {
    let src = partner(WRITE_ASK, "")
        + "codeunit 60820 PCache { SingleInstance = true; var N: Integer; Last: Code[20];
            procedure Bump() begin N += 1; Last := 'ASKED'; end;
            procedure Count(): Integer begin exit(N); end;
            procedure LastAsk(): Code[20] begin exit(Last); end; }";
    let foreign = "codeunit 60953 Foreign {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)]
        local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
        var P: Codeunit PCache; begin if TableId = 60805 then P.Bump(); end; }";
    let got = run_main(&src, &ask_tests(READ_CACHE, foreign));
    assert_refused(got, "of a type other than");
}
