//! BC installs apps one at a time in dependency order. While an app installs, only the apps installed before it and the app
//! itself are installed: subscribers of an app installed later do not run, and `NavApp.GetModuleInfo` does not find that app.
//! Measured on BC 29 (corpus branch al-runtime-rs-staging-y3: tests/al-language-xapp-a/Y3XaInstallEvents.al,
//! tests/al-language-xapp-b/Y3XbInstallEvents.al, tests/al-language/install-upgrade/install-order/
//! InstallOrderSubscriberTests.al:66-79, :90-91, :104-117, :139-144): during XApp A's install only A's own subscribers ran, for its integration event and for
//! the OnAfterInsertEvent of its table, and GetModuleInfo(B) was false; during B's install A's and B's ran, the main app's did
//! not; at test time all of them ran.
//!
//! Two apps with no dependency path between them have no known install order: an event raised in one's install code that
//! the other subscribes to is refused (the test that reads what that install code wrote is not run).

mod install_order_apps;

use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};
use install_order_apps::*;

#[test]
fn an_app_installing_sees_only_the_apps_installed_before_it() {
    assert_eq!(outcome(ordered(), "InstallSeesOnlyAppsInstalledBefore"), Outcome::Pass);
}

#[test]
fn at_test_time_every_subscriber_runs() {
    assert_eq!(outcome(ordered(), "AtTestTimeAllSubscribersRun"), Outcome::Pass);
}

/// C and B do not depend on each other. During B's install, C's subscriber to A's event runs only if C installs first:
/// that order is not known, so what B's install code wrote is not read.
#[test]
fn a_subscriber_of_an_app_with_no_order_to_the_installing_app_is_refused() {
    match outcome(with_c(), "InstallSeesOnlyAppsInstalledBefore") {
        Outcome::NotRun(why) => assert!(why.contains("install order"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// The refusal names the subscriber and what its trial run showed (al-vm `order_trial.rs`): C's subscriber writes table
/// Log. An outside suite saw only the event and the app (al-runner issue 6924) and could not tell what to change.
#[test]
fn the_install_order_refusal_names_the_subscriber_and_what_its_trial_showed() {
    match outcome(with_c(), "InstallSeesOnlyAppsInstalledBefore") {
        Outcome::NotRun(why) => {
            assert!(
                why.contains(
                    "event onphase: subscriber codeunit \"CSubs\".OnPhase in app app 3, which has no dependency path to \
                     the app that installs; the install order of the two is not known; running it once showed: it writes Log"
                ),
                "{why}"
            );
        }
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A trial run that stopped on an error: the refusal names the error text it stopped on.
#[test]
fn the_install_order_refusal_names_the_error_the_trial_stopped_on() {
    const C_ERRS: &str = r#"
codeunit 63000 CSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    begin
        Error('C stops here');
    end;
}
"#;
    let mut apps = with_c();
    apps[2] = app(C_ID, &[A_ID], C_ERRS);
    match outcome(apps, "InstallSeesOnlyAppsInstalledBefore") {
        Outcome::NotRun(why) => {
            assert!(
                why.contains("running it once showed: it stopped with an error or a refusal (C stops here)"),
                "{why}"
            )
        }
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A trial run that entered several codeunits of other apps that hold state names the one with the lowest object index,
/// the same in every run (the reason gates compare it across processes).
#[test]
fn the_trial_names_the_first_stateful_codeunit_by_object_index() {
    const A_STATE: &str = r#"
codeunit 61010 S1 { var N: Integer; procedure Touch() begin N += 1; end; }
codeunit 61011 S2 { var N: Integer; procedure Touch() begin N += 1; end; }
codeunit 61012 S3 { var N: Integer; procedure Touch() begin N += 1; end; }
codeunit 61013 S4 { var N: Integer; procedure Touch() begin N += 1; end; }
"#;
    const C_STATE: &str = r#"
codeunit 63000 CSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    var
        S1: Codeunit S1;
        S2: Codeunit S2;
        S3: Codeunit S3;
        S4: Codeunit S4;
    begin
        S4.Touch();
        S3.Touch();
        S2.Touch();
        S1.Touch();
    end;
}
"#;
    for _ in 0..3 {
        let mut apps = with_c();
        apps[0] = app(A_ID, &[], &format!("{A}{A_STATE}"));
        apps[2] = app(C_ID, &[A_ID], C_STATE);
        match outcome(apps, "InstallSeesOnlyAppsInstalledBefore") {
            Outcome::NotRun(why) => assert!(
                why.contains("running it once showed: it enters S1, which holds state"),
                "{why}"
            ),
            other => panic!("expected not run, got {other:?}"),
        }
    }
}

/// The refusal at pick time (no trial run): C's subscriber, run under watch, raises an event that D handles, and D's
/// order is not known either. D is refused when it is picked, and C's trial shows that refusal.
#[test]
fn a_refusal_at_pick_time_shows_in_the_trial_reason_of_the_outer_subscriber() {
    const D_ID: &str = "55555555-5555-5555-5555-555555555555";
    const C_RAISES: &str = r#"
codeunit 63000 CSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    begin
        OnCPhase(Phase);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnCPhase(Phase: Text)
    begin
    end;
}
"#;
    const D: &str = r#"
codeunit 65000 DSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::CSubs, 'OnCPhase', '', false, false)]
    local procedure OnCPhase(Phase: Text)
    var
        E: Codeunit Events;
    begin
        E.Log('D-sub|' + Phase);
    end;
}
"#;
    let apps = vec![
        app(A_ID, &[], A),
        app(B_ID, &[A_ID], B),
        app(C_ID, &[A_ID], C_RAISES),
        app(D_ID, &[A_ID, C_ID], D),
        app(MAIN_ID, &[A_ID, B_ID, C_ID, D_ID], MAIN),
    ];
    match outcome(apps, "InstallSeesOnlyAppsInstalledBefore") {
        Outcome::NotRun(why) => {
            assert!(
                why.contains(
                    "event onphase: subscriber codeunit \"CSubs\".OnPhase in app app 3, which has no dependency path to the \
                     app that installs; the install order of the two is not known; running it once showed: it stopped with \
                     an error or a refusal (event oncphase: subscriber codeunit \"DSubs\".OnCPhase in app app 5, which has no \
                     dependency path to the app that installs; the install order of the two is not known)"
                ),
                "{why}"
            );
            assert_eq!(why.matches("running it once showed").count(), 1, "{why}");
        }
        other => panic!("expected not run, got {other:?}"),
    }
}

/// When the tables install code writes are not known, the trial reason names the install routine that holds a RecordRef
/// or a DataTransfer (al-testrun `install.rs`, `tables_install_code_of_may_write`).
#[test]
fn the_trial_names_the_install_routine_that_makes_the_written_tables_unknown() {
    const B_RECREF: &str = r#"
codeunit 62001 BInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        E: Codeunit Events;
        RR: RecordRef;
    begin
        E.Raise('b-install');
    end;
}
"#;
    const C_READS: &str = r#"
codeunit 63000 CSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    var
        P: Record Probe;
    begin
        if P.FindFirst() then;
    end;
}
"#;
    let mut apps = with_c();
    apps[1] = app(B_ID, &[A_ID], B_RECREF);
    apps[2] = app(C_ID, &[A_ID], C_READS);
    match outcome(apps, "InstallSeesOnlyAppsInstalledBefore") {
        Outcome::NotRun(why) => assert!(
            why.contains(
                "running it once showed: it reads a table and the tables install code writes are not known (codeunit \
                 \"BInstall\".OnInstallAppPerCompany holds a RecordRef or a DataTransfer)"
            ),
            "{why}"
        ),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A Company Triggers subscriber of an app with no dependency path to the installing app: the same sentence and the same
/// subscriber name as an event subscriber (al-vm `order_trial.rs`, `order_unknown_refusal`).
#[test]
fn the_company_triggers_refusal_names_the_subscriber_like_the_event_refusal() {
    const C_OPEN: &str = r#"
codeunit 63000 CSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure OnOpen()
    var
        E: Codeunit Events;
    begin
        E.Log('C-open');
    end;
}
"#;
    let mut apps = with_c();
    apps[2] = app(C_ID, &[A_ID], C_OPEN);
    match outcome(apps, "InstallSeesOnlyAppsInstalledBefore") {
        Outcome::NotRun(why) => assert!(
            why.contains(
                "Company Triggers subscriber codeunit \"CSubs\".OnOpen in app app 3, which has no dependency path to the \
                 app that installs; the install order of the two is not known"
            ),
            "{why}"
        ),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A table extension of an app with no dependency path to the installing app: the same sentence.
#[test]
fn the_table_extension_refusal_uses_the_same_sentence() {
    const B_INSERTS: &str = r#"
codeunit 62001 BInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        P: Record Probe;
    begin
        P.K := 'b-install';
        if P.Insert(true) then;
    end;
}
"#;
    const C_EXT: &str = r#"
tableextension 63001 CExt extends Probe
{
    trigger OnBeforeInsert()
    begin
        Rec.K := CopyStr(Rec.K + '+C', 1, MaxStrLen(Rec.K));
    end;
}
"#;
    let mut apps = with_c();
    apps[1] = app(B_ID, &[A_ID], B_INSERTS);
    apps[2] = app(C_ID, &[A_ID], C_EXT);
    match outcome(apps, "InstallSeesOnlyAppsInstalledBefore") {
        Outcome::NotRun(why) => assert!(
            why.contains(
                "OnBeforeInsert of table Probe: tableextension \"CExt\" in app app 3, which has no dependency path to the \
                 app that installs; the install order of the two is not known"
            ),
            "{why}"
        ),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// The dependency apps install one at a time too: during A's install B's subscribers do not run, during B's they do.
#[test]
fn dependency_apps_install_one_at_a_time() {
    let apps = vec![
        app(MAIN_ID, &[A_ID, B_ID], MAIN),
        app(B_ID, &[A_ID], B),
        app(A_ID, &[], A),
    ];
    let kinds = vec![
        al_sema::AppKind::Workspace,
        al_sema::AppKind::Dependency,
        al_sema::AppKind::Dependency,
    ];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    let got = rt.first_outcome(&CaseId {
        codeunit: 64001,
        name: "InstallSeesOnlyAppsInstalledBefore".into(),
    });
    assert_eq!(got, Outcome::Pass);
}

/// A test-toolkit app (here with System Application Test Library's id) and an application app with no dependency path
/// between them: the application app installs first, the toolkit app after it (owner decision 2026-10-30, BcContainerHelper's
/// `Import-TestToolkitToBcContainer`). During the application app's install the toolkit's subscriber does not run; during the
/// toolkit's install the application app's subscriber runs. Nothing is refused.
#[test]
fn test_toolkit_apps_install_after_the_application_apps() {
    const TOOLKIT_ID: &str = "9856ae4f-d1a7-46ef-89bb-6ef056398228";
    const BASE_ID: &str = "55555555-5555-5555-5555-555555555555";
    // The application app: its own subscriber to A's event and an install codeunit that raises it.
    const BASE: &str = r#"
codeunit 65000 BaseSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    var
        E: Codeunit Events;
    begin
        E.Log('Base-sub|' + Phase);
    end;
}
codeunit 65001 BaseInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        E: Codeunit Events;
    begin
        E.Raise('base-install');
    end;
}
"#;
    const TOOLKIT: &str = r#"
codeunit 66000 KitSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Events, 'OnPhase', '', false, false)]
    local procedure OnPhase(Phase: Text)
    var
        E: Codeunit Events;
    begin
        E.Log('Kit-sub|' + Phase);
    end;
}
codeunit 66001 KitInstall
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        E: Codeunit Events;
    begin
        E.Raise('kit-install');
    end;
}
"#;
    const MAIN_T: &str = r#"
codeunit 64001 T
{
    Subtype = Test;

    local procedure Expect(Entry: Text; Want: Boolean)
    var
        L: Record Log;
    begin
        if L.Get(Entry) <> Want then
            Error('%1: expected %2', Entry, Want);
    end;

    [Test]
    procedure ToolkitAfterApplication()
    begin
        Expect('Base-sub|base-install', true);
        Expect('Kit-sub|base-install', false);
        Expect('Base-sub|kit-install', true);
        Expect('Kit-sub|kit-install', true);
    end;
}
"#;
    // The toolkit app is listed first, so only the toolkit rule puts it after the application app.
    let apps = vec![
        app(MAIN_ID, &[A_ID, BASE_ID, TOOLKIT_ID], MAIN_T),
        app(TOOLKIT_ID, &[A_ID], TOOLKIT),
        app(BASE_ID, &[A_ID], BASE),
        app(A_ID, &[], A),
    ];
    let kinds = vec![
        al_sema::AppKind::Workspace,
        al_sema::AppKind::Dependency,
        al_sema::AppKind::Dependency,
        al_sema::AppKind::Dependency,
    ];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    let got = rt.first_outcome(&CaseId {
        codeunit: 64001,
        name: "ToolkitAfterApplication".into(),
    });
    assert_eq!(got, Outcome::Pass);
}

/// The table extension of an app installed later is not active while an earlier app installs: XApp A's install inserts a row
/// with Insert(true) and XApp B's OnBeforeInsert does not run on it nor does B's field InitValue apply; during B's own install
/// and at test time both do (install-upgrade/install-order/Y8InstallOrderTests.al:88-110, fixtures tests/al-language-xapp-a/
/// Y8XaInstallOrder.al and tests/al-language-xapp-b/Y8XbInstallOrder.al).
#[test]
fn a_later_apps_table_extension_is_not_active_during_an_earlier_install() {
    const XA: &str = r#"
table 65100 Data
{
    DataPerCompany = false;
    fields { field(1; "No."; Integer) { } field(2; Note; Text[50]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 65101 XaInstall
{
    Subtype = Install;
    trigger OnInstallAppPerDatabase()
    var D: Record Data;
    begin
        D.Init(); D."No." := 1; D.Note := 'A'; D.Insert(true);
    end;
}
"#;
    const XB: &str = r#"
tableextension 65102 DataExt extends Data
{
    fields { field(65102; Mark; Text[10]) { InitValue = 'B init'; } }
    trigger OnBeforeInsert()
    begin
        Rec.Note := CopyStr(Rec.Note + '+BX', 1, MaxStrLen(Rec.Note));
    end;
}
codeunit 65103 XbInstall
{
    Subtype = Install;
    trigger OnInstallAppPerDatabase()
    var D: Record Data;
    begin
        D.Init(); D."No." := 20; D.Note := 'B'; D.Insert(true);
    end;
}
"#;
    const T: &str = r#"
codeunit 64001 T
{
    Subtype = Test;
    [Test]
    procedure Rows()
    var D: Record Data; Tmp: Record Data temporary;
    begin
        D.Get(1);
        if (D.Note <> 'A') or (D.Mark <> '') then Error('row 1: %1 %2', D.Note, D.Mark);
        D.Get(20);
        if (D.Note <> 'B+BX') or (D.Mark <> 'B init') then Error('row 20: %1 %2', D.Note, D.Mark);
        Tmp.Init(); Tmp."No." := 99; Tmp.Note := 'T'; Tmp.Insert(true);
        if (Tmp.Note <> 'T+BX') or (Tmp.Mark <> 'B init') then Error('temp: %1 %2', Tmp.Note, Tmp.Mark);
    end;
}
"#;
    let apps = vec![
        app(A_ID, &[], XA),
        app(B_ID, &[A_ID], XB),
        app(MAIN_ID, &[A_ID, B_ID], T),
    ];
    assert_eq!(outcome(apps, "Rows"), Outcome::Pass);
}
