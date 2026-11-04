//! Subscribers in apps that do not depend on each other, each guarded by `if Feature <> Feature::X then exit;`
//! (al-runner issue 6924, `guard_exit.rs`). BC pins ascending app id with each app's dependencies before it
//! (events/subscriber-order/BwSubscriberOrderTests.al); which apps the walk starts from is not pinned, so the runtime
//! refuses where the readings disagree. A subscriber whose guard exits for this call has no effect wherever it runs.
//!
//! The apps: `p` publishes the event; `a`, `b`, `c` subscribe and depend only on `p`; `z` calls the publisher and
//! depends on `p` and `a`. The pinned reading runs `a` before `b`; walking only from the apps no app depends on runs
//! `b` before `a` (`a` is reached through `z`).

use al_builtins::native::Signal;
use al_sema::workspace::AppSource;
use al_testkit::text;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

const PUBLISHER: &str = r#"
enum 50100 "GX Feature"
{
    Extensible = true;
    value(0; None) { }
    value(1; A1) { }
    value(2; A2) { }
    value(3; A3) { }
}

codeunit 50100 "GX Check"
{
    procedure Check(Feature: Enum "GX Feature"): Text
    var
        Valid: Boolean;
        Handled: Boolean;
        Who: Text;
    begin
        OnBeforeCheckLicense(Feature, Valid, Handled, Who);
        if Valid and Handled then
            exit(Who);
        exit('none:' + Who);
    end;

    [IntegrationEvent(false, false)]
    procedure OnBeforeCheckLicense(var Feature: Enum "GX Feature"; var LicenseValid: Boolean; var Handled: Boolean; var Who: Text)
    begin
    end;
}
"#;

/// A subscriber codeunit with body `body`, binding by name and leaving out `Who` unless the body names it.
fn subscriber(id: u32, body: &str) -> String {
    format!(
        r#"
codeunit {id} "GX Subs {id}"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"GX Check", 'OnBeforeCheckLicense', '', false, false)]
    local procedure OnBeforeCheckLicense(var Feature: Enum "GX Feature"; var Handled: Boolean; var LicenseValid: Boolean; var Who: Text)
    begin
{body}
    end;
}}
"#
    )
}

fn guarded(id: u32, value: &str) -> String {
    subscriber(
        id,
        &format!(
            "        if Feature <> Feature::{value} then exit;\n        LicenseValid := true;\n        Handled := true;\n        Who := '{value}';"
        ),
    )
}

const CALLER: &str = r#"
codeunit 50190 "GX Caller"
{
    procedure RunNone(): Text var C: Codeunit "GX Check"; begin exit(C.Check("GX Feature"::None)); end;
    procedure RunA1(): Text var C: Codeunit "GX Check"; begin exit(C.Check("GX Feature"::A1)); end;
    procedure RunA2(): Text var C: Codeunit "GX Check"; begin exit(C.Check("GX Feature"::A2)); end;
    procedure RunA3(): Text var C: Codeunit "GX Check"; begin exit(C.Check("GX Feature"::A3)); end;
}
"#;

fn built(a: String, b: String, c: String) -> al_testkit::Built {
    al_testkit::Built::new(al_testkit::compile_apps(vec![
        app("p", &[], PUBLISHER),
        app("a", &["p"], &a),
        app("b", &["p"], &b),
        app("c", &["p"], &c),
        app("z", &["p", "a"], CALLER),
    ]))
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains("whose order BC does not pin"), "{m}"),
        other => panic!("expected the order refusal, got {other:?}"),
    }
}

/// Three unrelated apps, guards on distinct values: at most one subscriber goes past its guard, so the order of the
/// apps cannot change the result. The one for the value raised sets the flags.
#[test]
fn mutually_exclusive_guards_in_unrelated_apps_run() {
    let mut b = built(guarded(50110, "A1"), guarded(50120, "A2"), guarded(50130, "A3"));
    assert_eq!(text(b.run_in("GX Caller", "RunA1")), "A1");
    assert_eq!(text(b.run_in("GX Caller", "RunA2")), "A2");
    assert_eq!(text(b.run_in("GX Caller", "RunA3")), "A3");
    assert_eq!(text(b.run_in("GX Caller", "RunNone")), "none:");
}

/// The equivalent guard forms: `if not (P = C) then exit;` and a body that is all `if P = C then begin ... end`.
#[test]
fn equivalent_guard_forms_run() {
    let not_eq = subscriber(
        50110,
        "        if not (Feature = Feature::A1) then exit;\n        LicenseValid := true;\n        Handled := true;\n        Who := 'A1';",
    );
    let wrapped = subscriber(
        50120,
        "        if Feature = Feature::A2 then begin\n            LicenseValid := true;\n            Handled := true;\n            Who := 'A2';\n        end;",
    );
    let case_form = subscriber(
        50130,
        "        case Feature of\n            Feature::A3:\n                begin\n                    LicenseValid := true;\n                    Handled := true;\n                    Who := 'A3';\n                end;\n        end;",
    );
    let mut b = built(not_eq, wrapped, case_form);
    assert_eq!(text(b.run_in("GX Caller", "RunA1")), "A1");
    assert_eq!(text(b.run_in("GX Caller", "RunA2")), "A2");
    assert_eq!(text(b.run_in("GX Caller", "RunA3")), "A3");
}

/// Two guards on the same value in apps whose order is not pinned: both go past the guard, so the last writer of
/// `Who` depends on the order. Refused.
#[test]
fn two_guards_on_the_same_value_stay_refused() {
    let mut b = built(guarded(50110, "A2"), guarded(50120, "A2"), guarded(50130, "A3"));
    refused(b.run_in("GX Caller", "RunA2"));
    // Raised for a value neither of them guards: every subscriber exits.
    assert_eq!(text(b.run_in("GX Caller", "RunA1")), "none:");
}

/// A subscriber that writes the guarded parameter: the guard of a subscriber after it sees another value, so where it
/// runs matters. Refused.
#[test]
fn a_subscriber_that_writes_the_guarded_parameter_stays_refused() {
    let writer = subscriber(
        50120,
        "        if Feature <> Feature::A2 then exit;\n        Feature := Feature::A1;\n        Who := Who + 'B';",
    );
    let mut b = built(guarded(50110, "A1"), writer, guarded(50130, "A3"));
    refused(b.run_in("GX Caller", "RunA2"));
}

/// A subscriber without a guard runs for every value: next to the one whose guard lets the value through, the order
/// of the two decides `Who`. Refused.
#[test]
fn an_unguarded_subscriber_stays_refused() {
    let unguarded = subscriber(50120, "        Who := Who + 'B';");
    let mut b = built(guarded(50110, "A1"), unguarded, guarded(50130, "A3"));
    refused(b.run_in("GX Caller", "RunA1"));
}
