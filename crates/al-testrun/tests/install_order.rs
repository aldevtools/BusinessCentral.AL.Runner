//! BC installs apps one at a time in dependency order. While an app installs, only the apps installed before it and the app
//! itself are installed: subscribers of an app installed later do not run, and `NavApp.GetModuleInfo` does not find that app.
//! Measured on BC 29 (corpus branch al-runtime-rs-staging-y3: tests/al-language-xapp-a/Y3XaInstallEvents.al,
//! tests/al-language-xapp-b/Y3XbInstallEvents.al, tests/al-language/install/lt-install-order-subscribers/
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
