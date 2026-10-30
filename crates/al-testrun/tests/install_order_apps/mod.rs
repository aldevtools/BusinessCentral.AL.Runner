//! The apps of the install order tests (`install_order.rs`, `install_order_listed.rs`).
#![allow(dead_code)]

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

pub const A_ID: &str = "11111111-1111-1111-1111-111111111111";
pub const B_ID: &str = "22222222-2222-2222-2222-222222222222";
pub const C_ID: &str = "33333333-3333-3333-3333-333333333333";
pub const MAIN_ID: &str = "44444444-4444-4444-4444-444444444444";

/// App A: its install code logs whether it sees app B, raises its own event and inserts a row of its own table.
pub const A: &str = r#"
table 61000 Log { DataPerCompany = false; fields { field(1; Entry; Text[100]) { } } keys { key(PK; Entry) { } } }
table 61001 Probe { DataPerCompany = false; fields { field(1; K; Text[50]) { } } keys { key(PK; K) { } } }
codeunit 61002 Events
{
    procedure Log(Entry: Text)
    var
        L: Record Log;
    begin
        L.Entry := CopyStr(Entry, 1, 100);
        if L.Insert() then;
    end;

    procedure Raise(Phase: Text)
    begin
        OnPhase(Phase);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnPhase(Phase: Text)
    begin
    end;
}
codeunit 61003 AInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        E: Codeunit Events;
        P: Record Probe;
        Info: ModuleInfo;
    begin
        E.Log('A-install');
        E.Log('A-seesB=' + Format(NavApp.GetModuleInfo('22222222-2222-2222-2222-222222222222', Info)));
        E.Raise('a-install');
        P.K := 'a-install';
        if P.Insert(true) then;
    end;
}
codeunit 61004 ASubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    var
        E: Codeunit Events;
    begin
        E.Log('A-sub|' + Phase);
    end;

    [EventSubscriber(ObjectType::Table, Database::Probe, 'OnAfterInsertEvent', '', false, false)]
    local procedure OnProbe(var Rec: Record Probe; RunTrigger: Boolean)
    var
        E: Codeunit Events;
    begin
        E.Log('A-tsub|' + Rec.K);
    end;
}
"#;

/// App B (depends on A): subscribers to A's event and table, and its own install code that raises A's event.
pub const B: &str = r#"
codeunit 62000 BSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    var
        E: Codeunit Events;
    begin
        E.Log('B-sub|' + Phase);
    end;

    [EventSubscriber(ObjectType::Table, Database::Probe, 'OnAfterInsertEvent', '', false, false)]
    local procedure OnProbe(var Rec: Record Probe; RunTrigger: Boolean)
    var
        E: Codeunit Events;
    begin
        E.Log('B-tsub|' + Rec.K);
    end;
}
codeunit 62001 BInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        E: Codeunit Events;
    begin
        E.Raise('b-install');
    end;
}
"#;

/// App C (depends on A only, no path to B): subscribes to A's event.
pub const C: &str = r#"
codeunit 63000 CSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    var
        E: Codeunit Events;
    begin
        E.Log('C-sub|' + Phase);
    end;
}
"#;

/// The main app (depends on A and B, and C when it is there): subscribers and the tests.
pub const MAIN: &str = r#"
codeunit 64000 MainSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    var
        E: Codeunit Events;
    begin
        E.Log('M-sub|' + Phase);
    end;
}
codeunit 64001 T
{
    Subtype = Test;

    local procedure Has(Entry: Text): Boolean
    var
        L: Record Log;
    begin
        exit(L.Get(Entry));
    end;

    local procedure Expect(Entry: Text; Want: Boolean)
    begin
        if Has(Entry) <> Want then
            Error('%1: expected %2', Entry, Want);
    end;

    [Test]
    procedure InstallSeesOnlyAppsInstalledBefore()
    begin
        Expect('A-install', true);
        Expect('A-seesB=No', true);
        Expect('A-sub|a-install', true);
        Expect('A-tsub|a-install', true);
        Expect('B-sub|a-install', false);
        Expect('B-tsub|a-install', false);
        Expect('M-sub|a-install', false);
        Expect('A-sub|b-install', true);
        Expect('B-sub|b-install', true);
        Expect('M-sub|b-install', false);
    end;

    [Test]
    procedure UnrelatedAppListedLaterIsNotInstalled()
    begin
        Expect('B-sub|b-install', true);
        Expect('C-sub|a-install', false);
        Expect('C-sub|b-install', false);
    end;

    [Test]
    procedure AtTestTimeAllSubscribersRun()
    var
        E: Codeunit Events;
    begin
        E.Raise('test');
        Expect('A-sub|test', true);
        Expect('B-sub|test', true);
        Expect('M-sub|test', true);
    end;
}
"#;

pub fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("app {}", &id[..1]),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{}.al", &id[..1]), src.into())],
    }
}

pub fn outcome(apps: Vec<AppSource>, test: &str) -> Outcome {
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 64001,
        name: test.into(),
    })
}

pub fn ordered() -> Vec<AppSource> {
    vec![
        app(A_ID, &[], A),
        app(B_ID, &[A_ID], B),
        app(MAIN_ID, &[A_ID, B_ID], MAIN),
    ]
}

pub fn with_c() -> Vec<AppSource> {
    vec![
        app(A_ID, &[], A),
        app(B_ID, &[A_ID], B),
        app(C_ID, &[A_ID], C),
        app(MAIN_ID, &[A_ID, B_ID, C_ID], MAIN),
    ]
}
