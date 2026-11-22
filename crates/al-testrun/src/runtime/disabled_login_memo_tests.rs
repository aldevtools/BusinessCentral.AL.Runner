// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! An app under test's login subscriber that fills System Application's login memos, in a session under
//! RequiredTestIsolation = Disabled (`login::watched_state`). System Application's memos ask the service tier through
//! `DotNet` server classes the runtime serves itself: "Environment Information Impl." calls
//! `NavTenantSettingsHelper.IsSandbox()`, "Server Setting Impl." keeps `ALConfigSettings.Instance()` and calls its
//! `IsSaaS()`. Those three members answer and change nothing (al-vm `server_classes::ANSWERS_ONLY`), so the reach
//! analysis does not count them as code it does not follow; any other member of the same classes still counts.
//! The dependency app `dep` (publisher Microsoft) is the stand-in of `login_memo_tests.rs`, with those calls.
use super::app_outcomes_tests::{OutcomesAndHits, outcomes_and_hits_with};
use super::*;

/// The dependency app. `{env}` is the body of the 3702 stand-in's `IsSandbox`, `{srv}` the call of the 3703 stand-in.
const DEP: &str = r#"
dotnet
{
    assembly("Microsoft.Dynamics.Nav.NavUserAccount")
    { type("Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper"; "NavTenantSettingsHelper") { } }
    assembly("Microsoft.Dynamics.Nav.Types")
    { type("Microsoft.Dynamics.Nav.Types.ALConfigSettings"; "ALConfigSettings") { } }
}
codeunit 150 "System Initialization" { procedure Raise() begin OnAfterLogin(); end;
    [IntegrationEvent(false, false, true)] procedure OnAfterLogin() begin end; }
codeunit 151 "System Initialization Impl."
{
    SingleInstance = true;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany() var E: Codeunit "Environment Information"; S: Codeunit "System Initialization"; H: Codeunit "Sys Halt";
    begin if E.IsSaaS() then; S.Raise(); H.Halt(); end;
}
codeunit 457 "Environment Information"
{
    SingleInstance = true;
    var Impl: Codeunit "Environment Information Impl.";
    procedure IsSaaS(): Boolean begin exit(Impl.IsSaaS()); end;
    procedure IsSandbox(): Boolean begin exit(Impl.IsSandbox()); end;
}
codeunit 3702 "Environment Information Impl."
{
    SingleInstance = true;
    var Helper: DotNet NavTenantSettingsHelper; SaaSKnown: Boolean; SaaS: Boolean; SandboxKnown: Boolean; Sandbox: Boolean;
    procedure IsSaaS(): Boolean var S: Codeunit "Server Setting";
    begin if not SaaSKnown then begin SaaS := IsSandbox() or S.GetEnableMembershipEntitlement(); SaaSKnown := true; end; exit(SaaS); end;
    procedure IsSandbox(): Boolean begin if not SandboxKnown then begin {env} SandboxKnown := true; end; exit(Sandbox); end;
}
codeunit 6723 "Server Setting"
{
    SingleInstance = true;
    var Impl: Codeunit "Server Setting Impl.";
    procedure GetEnableMembershipEntitlement(): Boolean begin exit(Impl.GetEnableMembershipEntitlement()); end;
}
codeunit 3703 "Server Setting Impl."
{
    SingleInstance = true;
    var Config: DotNet ALConfigSettings; Known: Boolean;
    procedure GetEnableMembershipEntitlement(): Boolean
    begin if not Known then begin Config := Config.Instance(); Known := true; end; exit({srv}); end;
}
codeunit 152 "Sys Halt" { procedure Halt() begin Debugger.Deactivate(); end; }
"#;

/// System Application's calls (BC 29.0 System Application, Environment Information Impl. IsSandbox, Server Setting Impl.
/// GetEnableMembershipEntitlement).
const ENV: &str = "Sandbox := Helper.IsSandbox();";
const SRV: &str = "Config.IsSaaS()";

/// The app under test: the OnAfterLogin subscriber calls the three measured procedures from its own codeunit, as the
/// probe's "LT P2 XA Env Seen".Record does (corpus PR 1490, P2XaLoginMemos.al). The test codeunit holds `{tests}`.
const MAIN: &str = r#"
codeunit 61002 "M Seen" { procedure Record() var Id: Integer; E: Codeunit "Environment Information"; S: Codeunit "Server Setting";
    begin if E.IsSaaS() then; if E.IsSandbox() then; if S.GetEnableMembershipEntitlement() then; {extra} end; }
codeunit 61001 "M Login" {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure L() var S: Codeunit "M Seen"; begin S.Record(); end; }
table 61020 "M Row" { fields { field(1; K; Integer) { } } }
codeunit 61010 T { Subtype = Test; {tests} }
"#;

/// A test that reads nothing, and one that writes and reads a table of its own.
const NO_MEMO: &str = "[Test] procedure ReadsNothing() begin end;
    [Test] procedure OwnRow() var R: Record \"M Row\"; begin R.K := 1; R.Insert(); R.FindFirst(); end;";
/// A test that reads WorkDate and isolated storage.
const READS_STATE: &str = "[Test] procedure ReadsState() var V: Text; begin if WorkDate() = 0D then; if IsolatedStorage.Get('k', V) then; end;";
/// A test that reads the memo through the facade.
const READS_MEMO: &str =
    "[Test] procedure ReadsMemo() var E: Codeunit \"Environment Information\"; begin if E.IsSaaS() then; end;";

fn run(env: &str, srv: &str, tests: &str, isolation: TestIsolation) -> OutcomesAndHits {
    run_with(env, srv, "", tests, isolation)
}

/// [`run`] with `extra`, more statements of the subscriber's procedure `M Seen.Record`.
fn run_with(env: &str, srv: &str, extra: &str, tests: &str, isolation: TestIsolation) -> OutcomesAndHits {
    outcomes_and_hits_with(
        &[
            ("dep", &[], &DEP.replace("{env}", env).replace("{srv}", srv)),
            (
                "main",
                &["dep"],
                &MAIN.replace("{tests}", tests).replace("{extra}", extra),
            ),
        ],
        &[("dep", "Microsoft")],
        "T",
        isolation,
    )
}

/// Every test not run, none flagged: main's result.
fn kept_not_run((o, hits): &OutcomesAndHits, what: &str) {
    assert!(o.iter().all(|(_, r)| matches!(r, Outcome::NotRun(_))), "{what}: {o:?}");
    assert!(hits.iter().all(|(_, h)| h.is_empty()), "{what}: {hits:?}");
}

// The subscriber of corpus 5da42777 (tests/al-language-xapp-a/TiXaIsSaaSLogin.al) under Disabled isolation, for a test
// codeunit that reads no memo: the service tier calls of the memos leave nothing, so Microsoft's login code runs first
// and, when it refuses, the codeunit runs again with the subscriber alone, watched (`login_memo.rs`). Main refused it:
// "run code the analysis does not follow". Nothing the tests read relies on the assumption: no mark.
#[test]
fn a_disabled_codeunit_that_reads_no_memo_runs_with_the_subscriber_alone() {
    let (o, hits) = run(ENV, SRV, NO_MEMO, TestIsolation::Disabled);
    assert!(o.iter().all(|(_, r)| *r == Outcome::Pass), "{o:?}");
    assert!(hits.iter().all(|(_, h)| h.is_empty()), "{hits:?}");
}

// A Disabled test codeunit that reads the memo keeps main's refusal: the subscribers write a SingleInstance global it
// reads, and how that meets what an earlier codeunit left is not pinned (`login::DISABLED`).
#[test]
fn a_disabled_codeunit_that_reads_the_memo_keeps_the_refusal() {
    kept_not_run(&run(ENV, SRV, READS_MEMO, TestIsolation::Disabled), "reads memo");
}

// Under Codeunit isolation both run, as on main (`login_memo_tests.rs`).
#[test]
fn under_codeunit_isolation_both_run_as_on_main() {
    for tests in [NO_MEMO, READS_MEMO] {
        let (o, _) = run(ENV, SRV, tests, TestIsolation::Codeunit);
        assert!(o.iter().all(|(_, r)| *r == Outcome::Pass), "{o:?}");
    }
}

// Neighbors that keep main's refusal under Disabled: another member of each served class (IsProduction; a member that
// changes the service tier, which the runtime does not serve; a config flag the memos do not read).
#[test]
fn other_members_of_the_server_classes_keep_the_refusal_under_disabled() {
    for (env, srv) in [
        ("Sandbox := not Helper.IsProduction();", SRV),
        ("Helper.EnableM365Collaboration(); Sandbox := Helper.IsSandbox();", SRV),
        (ENV, "Config.TestAutomationEnabled()"),
    ] {
        kept_not_run(
            &run(env, srv, NO_MEMO, TestIsolation::Disabled),
            &format!("{env} {srv}"),
        );
    }
}

// The review of PR 8653: the memos' DotNet calls are ignored only when the subscribers' reach is fully known and changes
// no other session state (`login::watched_state`). A subscriber that also runs a codeunit by an id known only at run
// time, or writes WorkDate or isolated storage a test reads, keeps main's refusal, for a test that reads nothing of the
// memos. Main refused these (the memo calls were `OTHER`).
#[test]
fn a_subscriber_with_unknown_reach_or_other_session_state_keeps_the_refusal() {
    for extra in [
        "Codeunit.Run(Id);",
        "WorkDate(DMY2Date(1, 1, 2030));",
        "IsolatedStorage.Set('k', 'v');",
    ] {
        kept_not_run(&run_with(ENV, SRV, extra, READS_STATE, TestIsolation::Disabled), extra);
    }
}

// Clean allowed case: the subscriber's reach is known and holds nothing but the memos' calls and a table write
// (the first test above), for a test that reads WorkDate and isolated storage.
#[test]
fn a_known_subscriber_that_only_fills_the_memos_runs_for_a_test_that_reads_session_state() {
    let (o, hits) = run(ENV, SRV, READS_STATE, TestIsolation::Disabled);
    assert!(o.iter().all(|(_, r)| *r == Outcome::Pass), "{o:?}");
    assert!(hits.iter().all(|(_, h)| h.is_empty()), "{hits:?}");
}
