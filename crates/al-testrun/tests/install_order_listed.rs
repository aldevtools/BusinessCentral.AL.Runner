//! With the publish order known (`host::set_publish_order_known`, the corpus run: corpus `.github/workflows/ci.yml`
//! `app_dirs` publishes the apps one after the other in a fixed order), two apps under test with no dependency path
//! between them install in the order the run lists them. A process of its own: the setting is global to the process.

mod install_order_apps;

use al_testrun::Outcome;

#[test]
fn an_unrelated_app_listed_later_is_not_installed_yet() {
    al_testrun::host::set_publish_order_known(true);
    let got = install_order_apps::outcome(install_order_apps::with_c(), "UnrelatedAppListedLaterIsNotInstalled");
    assert_eq!(got, Outcome::Pass);
}
