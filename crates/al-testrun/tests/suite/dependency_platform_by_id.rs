//! A dependency app's subscriber written with the bare id of Global Triggers 2000000002, Caption Class 42, Company Triggers
//! 2000000003 or Extension Triggers 2000000010: corpus PR 1466 (SbiSubscriberByIdTests.al) measured bare-id subscribers of the
//! app under test only, so every test of the run refuses (al-vm `Registry::dependency_platform_by_id`). The same subscriber in
//! the app under test, and the name form in a dependency, keep main's pass. Extension Triggers: `install_subscriber_by_id`.

use super::install_subscriber_by_id::{app, run_apps};
use al_testrun::Outcome;

/// The test calls into the subscriber's codeunit, so a compile on demand compiles the subscriber too.
const TEST: &str = r#"
codeunit 50950 "DP Tests"
{
    Subtype = Test;
    [Test] procedure Runs() var S: Codeunit "DP Sub"; begin S.Touch(); end;
}
"#;

/// A subscriber codeunit to `event` (with the parameter list `params`) of the platform codeunit written as `publisher`.
fn subscriber(publisher: &str, event: &str, params: &str) -> String {
    format!(
        r#"
codeunit 50251 "DP Sub"
{{
    [EventSubscriber(ObjectType::Codeunit, {publisher}, '{event}', '', false, false)]
    local procedure S({params})
    begin
    end;

    procedure Touch()
    begin
    end;
}}
"#
    )
}

/// (bare id, name form, event, parameters) of the platform codeunits whose bare-id subscriber the probe measured.
const PLATFORM: [(&str, &str, &str, &str); 3] = [
    (
        "2000000002",
        "Codeunit::\"Global Triggers\"",
        "GetDatabaseTableTriggerSetup",
        "TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean",
    ),
    (
        "42",
        "Codeunit::\"Caption Class\"",
        "OnResolveCaptionClass",
        "CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean",
    ),
    ("2000000003", "Codeunit::\"Company Triggers\"", "OnCompanyOpen", ""),
];

/// The outcome of `Runs`; with `dependency` the subscriber is in a dependency app, else in the app under test.
fn run(sub: String, dependency: bool) -> Outcome {
    let (apps, kinds) = if dependency {
        (
            vec![app("test", &["lib"], TEST.into()), app("lib", &[], sub)],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        )
    } else {
        (
            vec![app("test", &[], format!("{sub}{TEST}"))],
            vec![al_sema::AppKind::Workspace],
        )
    };
    run_apps(apps, kinds, "Runs").0
}

#[test]
fn a_bare_id_platform_subscriber_of_a_dependency_refuses_every_test() {
    let wrong: Vec<_> = (PLATFORM.iter())
        .map(|(id, _, event, params)| (id, run(subscriber(id, event, params), true)))
        .filter(|(id, o)| !matches!(o, Outcome::NotRun(why) if why.contains(**id)))
        .collect();
    assert!(wrong.is_empty(), "{wrong:?}");
}

#[test]
fn a_bare_id_platform_subscriber_of_the_app_under_test_keeps_the_pass() {
    for (id, _, event, params) in PLATFORM {
        let o = run(subscriber(id, event, params), false);
        assert!(matches!(o, Outcome::Pass), "{id} {event}: {o:?}");
    }
}

#[test]
fn a_name_form_platform_subscriber_of_a_dependency_keeps_the_pass() {
    for (_, name, event, params) in PLATFORM {
        let o = run(subscriber(name, event, params), true);
        assert!(matches!(o, Outcome::Pass), "{name} {event}: {o:?}");
    }
}
