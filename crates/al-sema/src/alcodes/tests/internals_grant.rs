//! AL0161 for a subscriber of an internal event of System.app, with and without the System package's grant (issue 7853).
//! Measured with alc 18.0.39.10160 against System.app 28.0.54946.56657, whose `InternalsVisibleTo` names Guided
//! Experience (`c64d75f0-...`, publisher Microsoft): the subscriber below compiles in an app with that id and
//! publisher (also with the id in upper case, another app name, or the publisher in lower case), and is AL0161 with
//! another id or with the id under another publisher.

use super::*;
use crate::workspace::AppKind;

const SYSTEM: &str = "codeunit 2000000004 \"System Action Triggers\"
{
    [InternalEvent(false)]
    procedure GetRoleCenterBannerPartID(var PartID: Integer)
    begin
    end;
}";

/// The subscriber of the reporter's repro (issue 7853), the `ivt-*` probe apps compiled with alc 18.0.39.10160.
const SUBSCRIBER: &str = "codeunit 50293 \"Probe Banner Subscriber\"
{
    EventSubscriberInstance = StaticAutomatic;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Action Triggers\", GetRoleCenterBannerPartID, '', true, true)]
    local procedure OnGetRoleCenterBannerPartID(var PartID: Integer)
    begin
        PartID := 42;
    end;
}";

const GUIDED_EXPERIENCE: &str = "c64d75f0-e9f1-4d0f-9949-cd453b9b1466";

fn codes(app_id: &str, granted: &[&str]) -> Vec<String> {
    let apps = vec![
        AppSource {
            id: app_id.into(),
            name: "Probe".into(),
            dependencies: vec!["8874ed3a-0643-4247-9ced-7a7002f7135d".into()],
            files: vec![("Probe.al".into(), SUBSCRIBER.into())],
        },
        AppSource {
            id: "8874ed3a-0643-4247-9ced-7a7002f7135d".into(),
            name: "System".into(),
            dependencies: vec![],
            files: vec![("System/S.al".into(), SYSTEM.into())],
        },
    ];
    let mut ws = Workspace::load_apps(apps, vec![AppKind::Workspace, AppKind::Dependency], &Default::default());
    ws.set_internals_granted(granted.iter().map(|s| s.to_string()).collect());
    check_workspace(&ws).into_iter().map(|d| d.code.to_string()).collect()
}

#[test]
fn a_subscriber_in_an_app_the_system_package_grants_is_not_al0161() {
    assert!(codes(GUIDED_EXPERIENCE, &[GUIDED_EXPERIENCE]).is_empty());
    // The app id compares in any case (alc accepts the upper-case id of the grant).
    assert!(codes(&GUIDED_EXPERIENCE.to_ascii_uppercase(), &[GUIDED_EXPERIENCE]).is_empty());
}

#[test]
fn a_subscriber_in_an_app_without_the_grant_keeps_al0161() {
    assert_eq!(
        codes("1b1b1b1b-0000-0000-0000-000000007853", &[GUIDED_EXPERIENCE]),
        ["AL0161"]
    );
    assert_eq!(codes(GUIDED_EXPERIENCE, &[]), ["AL0161"]);
}
