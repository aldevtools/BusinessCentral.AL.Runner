//! The login memos of al-runner issue 8945: what a partner dependency app's login subscribers left in Microsoft's caches
//! (stand-ins with the measured global names, in an app published as "Microsoft") and in a partner load-once memo, and
//! which entry a later test read marks (corpus PR 1571, open: cells M1, M3, E1, L1). Every other shape refuses the read.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

/// The stand-in "Microsoft" app: the login publisher, a setup subscriber that turns on every database event (as Microsoft's
/// Backup Management of the test libraries does), and memos with the globals of 423, 3702 and 9177.
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
table 405 "Change Log Setup" { fields { field(1; "Primary Key"; Code[10]) { } field(2; "Change Log Activated"; Boolean) { } }
    keys { key(PK; "Primary Key") { } } }
codeunit 423 "Change Log Management" { SingleInstance = true;
    var ChangeLogSetup: Record "Change Log Setup"; ChangeLogSetupRead: Boolean; MonitorSensitiveFieldSetupRead: Boolean;
    procedure Load() begin if ChangeLogSetupRead then exit; if ChangeLogSetup.Get() then; ChangeLogSetupRead := true; MonitorSensitiveFieldSetupRead := true; end;
    procedure IsRead(): Boolean begin exit(ChangeLogSetupRead); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DI(RecRef: RecordRef) begin Load(); end; }
codeunit 3702 "Environment Information Impl." { SingleInstance = true;
    var IsSaasInitialized: Boolean; IsSaaSConfig: Boolean; IsSandboxConfig: Boolean; IsSandboxInitialized: Boolean;
    procedure IsSaaS(): Boolean begin if not IsSaasInitialized then begin IsSaasInitialized := true; IsSaaSConfig := true; end; exit(IsSaaSConfig); end;
    procedure IsSandbox(): Boolean begin if not IsSandboxInitialized then begin IsSandboxInitialized := true; IsSandboxConfig := true; end; exit(IsSandboxConfig); end; }
codeunit 9177 "Application Area Cache" { SingleInstance = true;
    var ApplicationAreaCache: Dictionary of [Text, Text];
    procedure Fill() begin if not ApplicationAreaCache.ContainsKey('User:') then ApplicationAreaCache.Add('User:', '#Basic'); end;
    procedure Areas(): Text var T: Text; begin if ApplicationAreaCache.Get('User:', T) then; exit(T); end; }
"#;

/// The partner app: two login subscribers with bodies `first` and `second` (empty: none), a table they may write, and the
/// load-once memo of cell L1 reached through a wrapper's events, with `guard` at the start of its write subscriber and
/// `on_load` after it sets Loaded.
fn partner(first: &str, second: &str, guard: &str, on_load: &str) -> String {
    let sub = |id: u32, body: &str| {
        if body.is_empty() {
            return String::new();
        }
        format!(
            "codeunit {id} PLogin{id} {{
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
                local procedure L() var E: Codeunit \"Environment Information Impl.\"; C: Codeunit \"Change Log Management\";
                    A: Codeunit \"Application Area Cache\"; P: Record PTab; S: Record \"Change Log Setup\"; begin {body} end; }}"
        )
    };
    format!(
        r#"{}
{}
table 60800 PTab {{ fields {{ field(1; K; Integer) {{ }} }} keys {{ key(PK; K) {{ }} }} }}
table 60803 PSetup {{ fields {{ field(1; "Table Id"; Integer) {{ }} }} keys {{ key(PK; "Table Id") {{ }} }} }}
table 60804 PSub {{ fields {{ field(1; "Code"; Code[20]) {{ }} field(2; "Company Name"; Text[30]) {{ }} }} keys {{ key(PK; "Code", "Company Name") {{ }} }} }}
codeunit 60810 PWrapper {{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin OnWrapperSetup(TableId); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure I(RecRef: RecordRef) begin OnWrapperWrite(RecRef.Number); end;
    [IntegrationEvent(false, false)] procedure OnWrapperSetup(TableId: Integer) begin end;
    [IntegrationEvent(false, false)] procedure OnWrapperWrite(TableId: Integer) begin end; }}
codeunit 60811 PMemo {{ SingleInstance = true;
    var TempSetup: Record PSetup temporary; TempSub: Record PSub temporary; Checked: Boolean; Activated: Boolean; Loaded: Boolean; LoadTable: Integer;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::PWrapper, 'OnWrapperSetup', '', false, false)]
    local procedure WS(TableId: Integer) var Setup: Record PSetup;
    begin if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end; if not Activated then exit; end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::PWrapper, 'OnWrapperWrite', '', false, false)]
    local procedure WW(TableId: Integer)
    begin {guard} if not Loaded then begin Loaded := true; {on_load} LoadTemps(); end; if not TempSetup.Get(TableId) then exit; end;
    local procedure LoadTemps() var Setup: Record PSetup;
    begin TempSetup.DeleteAll(); if Setup.FindSet() then repeat TempSetup := Setup; TempSetup.Insert(); until Setup.Next() = 0;
        Clear(TempSub); TempSub.SetFilter("Company Name", '=%1', ''); end;
    procedure IsLoaded(): Boolean begin exit(Loaded and Checked); end;
    procedure Raw(): Text begin exit(Format(Loaded) + Format(Checked) + Format(Activated) + Format(TempSub.GetFilters())); end; }}
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
                A: Codeunit \"Application Area Cache\"; M: Codeunit PMemo; begin {body} end; }}"
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
        &partner("A.Fill();", "", "", ""),
        "if A.Areas() <> '#Basic' then Error('areas');",
    );
    assert_flagged(got, "partner-login-microsoft-cache");
}

// Cell M3 (ChangeLog_AfterLogin): 423 loads on the database insert event of the subscriber's write, the setup inactive.
#[test]
fn change_log_loaded_on_the_insert_event_is_kept() {
    let got = run(&partner("P.K := 1; P.Insert();", "", "", ""), READ_423);
    assert_flagged(got, "partner-login-microsoft-cache");
}

// Not measured: 423 loaded by a direct call, outside any Global Triggers raise.
#[test]
fn change_log_loaded_outside_a_raise_refuses() {
    assert_refused(
        run(&partner("C.Load();", "", "", ""), READ_423),
        "not one of the caches",
    );
}

// Not measured: a setup row at the load (its Get loads the record global, a third changed global).
#[test]
fn change_log_loaded_with_a_setup_row_refuses() {
    let first = "S.\"Primary Key\" := ''; S.Insert(); P.K := 1; P.Insert();";
    assert_refused(run(&partner(first, "", "", ""), READ_423), "not one of the caches");
}

// Cell E1 (EnvInfo_AfterLogin): two subscribers of one app, IsSandbox() first, IsSaaS() second; the reads mark the pair's
// entry only.
#[test]
fn env_info_split_over_two_subscribers_is_kept() {
    let got = run(
        &partner("E.IsSandbox();", "E.IsSaaS();", "", ""),
        "if not (E.IsSaaS() and E.IsSandbox()) then Error('env');",
    );
    assert_flagged(got, "partner-login-env-info-pair");
}

// Not measured: the reverse order.
#[test]
fn env_info_split_in_the_other_order_refuses() {
    let got = run(
        &partner("E.IsSaaS();", "E.IsSandbox();", "", ""),
        "if not E.IsSaaS() then Error('env');",
    );
    assert_refused(got, "two subscribers of the same app");
}

const READ_MEMO: &str = "if not M.IsLoaded() then Error('memo');";
const WRITE: &str = "P.K := 1; P.Insert();";

// Cell L1 (L1Memo_AfterLogin), without the asked table kept: Checked and Loaded true, the sub temp filtered and empty.
#[test]
fn load_once_memo_of_the_measured_shape_is_kept() {
    assert_flagged(
        run(&partner(WRITE, "", "", ""), READ_MEMO),
        "partner-login-load-once-memo",
    );
}

// L1's own LoadTable: an Integer that holds the asked table, which differs between the kinds of session.
#[test]
fn load_once_memo_that_keeps_the_asked_table_refuses() {
    let got = run(&partner(WRITE, "", "", "LoadTable := TableId;"), READ_MEMO);
    assert_refused(got, "global LoadTable changed other than measured");
}

// A guard the probe's memo has not: the company name.
#[test]
fn load_once_memo_with_a_company_guard_refuses() {
    let got = run(&partner(WRITE, "", "if CompanyName() = 'X' then exit;", ""), READ_MEMO);
    assert_refused(got, "Database.CompanyName");
}

// Not L1's change set: the setup ask changed no Boolean in this login (Loaded and the temp alone).
#[test]
fn load_once_memo_without_a_change_in_the_setup_ask_refuses() {
    let src = partner(WRITE, "", "", "").replace("Checked := true; ", "");
    assert_refused(run(&src, READ_MEMO), "0 Boolean in the setup ask");
}

// Not L1's change set: a second Boolean set in the insert event.
#[test]
fn load_once_memo_with_a_second_insert_boolean_refuses() {
    let src = partner(WRITE, "", "", "Extra := true;").replace("Loaded: Boolean;", "Loaded: Boolean; Extra: Boolean;");
    assert_refused(run(&src, READ_MEMO), "2 Boolean in the insert event");
}
