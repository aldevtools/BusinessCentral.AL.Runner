// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! An app under test's login subscriber that fills System Application's login memos (`login_memo.rs`, assumption
//! `login-own-memo-alone`, corpus PR 1490 P2A_06 and P2A_11). The dependency app `dep` (publisher Microsoft) stands in
//! for "System Initialization" (150), "System Initialization Impl." (151), the facades "Environment Information" (457)
//! and "Server Setting" (6723) and their memos 3702 and 3703. Its login code fills the memo and then stops: with an AL
//! error ([`ERRORS`]), which keeps main's result (not run), or with a refusal of the runner ([`REFUSES`]), after which
//! a codeunit that only the memos made raise Microsoft's code runs again with the app's own login subscribers alone.
use super::app_outcomes_tests::{OutcomesAndHits, outcomes_and_hits_with};
use super::*;

/// The dependency app; `{stop}` ends Microsoft's login code ([`ERRORS`], [`REFUSES`]).
const DEP: &str = r#"
codeunit 150 "System Initialization" { procedure Raise() begin OnAfterLogin(); end;
    [IntegrationEvent(false, false, true)] procedure OnAfterLogin() begin end; }
codeunit 151 "System Initialization Impl."
{
    SingleInstance = true;
    var InProgress: Boolean;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany() var E: Codeunit "Environment Information"; S: Codeunit "System Initialization"; H: Codeunit "Sys Halt"; Row: Record "Sys Login Row";
    begin InProgress := true; if E.IsSaaS() then; S.Raise(); Row.K := 1; Row.Insert(); {stop} end;
    procedure IsInProgress(): Boolean begin exit(InProgress); end;
}
codeunit 457 "Environment Information"
{
    SingleInstance = true;
    var Impl: Codeunit "Environment Information Impl.";
    procedure IsSaaS(): Boolean begin exit(Impl.IsSaaS()); end;
    procedure IsSandbox(): Boolean begin exit(Impl.IsSandbox()); end;
    procedure IsOnPrem(): Boolean begin exit(not Impl.IsSaaS()); end;
}
codeunit 3702 "Environment Information Impl."
{
    SingleInstance = true;
    var Known: Boolean; SaaS: Boolean; Fam: Text;
    procedure IsSaaS(): Boolean begin if not Known then begin SaaS := false; Known := true; end; exit(SaaS); end;
    procedure IsSandbox(): Boolean begin exit(IsSaaS()); end;
    procedure SetSaaS(Value: Boolean) begin SaaS := Value; end;
    procedure Family(): Text begin exit(Fam); end;
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
    var Known: Boolean; Ent: Boolean;
    procedure GetEnableMembershipEntitlement(): Boolean begin if not Known then begin Ent := false; Known := true; end; exit(Ent); end;
}
codeunit 1990 "Guided Experience" { procedure Ask(): Boolean var I: Codeunit "Environment Information Impl."; begin exit(I.IsSaaS()); end; }
codeunit 1992 Checklist { procedure Ask(): Boolean var I: Codeunit "Server Setting Impl."; begin exit(I.GetEnableMembershipEntitlement()); end; }
table 153 "Sys Login Row" { fields { field(1; K; Integer) { } } }
codeunit 152 "Sys Halt" { procedure Halt() begin Debugger.Deactivate(); end; }
codeunit 160 "Sys Helper" { procedure AskSaaS(): Boolean var E: Codeunit "Environment Information"; begin exit(E.IsSaaS()); end; }
"#;

/// Microsoft's login code stops with an AL error: the codeunit is not run, as on main.
const ERRORS: &str = "Error('MS LOGIN');";
/// Microsoft's login code calls code the runner refuses (as System Application's InitCompany reaches an unseeded G/L
/// Entry in the corpus run, ADR-047).
const REFUSES: &str = "H.Halt();";

/// The app under test: the OnAfterLogin subscriber with body `login`; `M Seen` calls the three measured procedures as
/// the probe's "LT P2 XA Env Seen".Record does (P2XaLoginMemos.al). The test codeunit: one test reads the memo, one
/// reads nothing, and the test `extra`.
fn main_app(login: &str, extra: &str) -> String {
    format!(
        "codeunit 61002 \"M Seen\" {{ procedure Record() var E: Codeunit \"Environment Information\"; S: Codeunit \"Server Setting\";
            begin if E.IsSaaS() then; if E.IsSandbox() then; if S.GetEnableMembershipEntitlement() then; end; }}
        codeunit 61001 \"M Login\" {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() {login} }}
        codeunit 61010 T {{ Subtype = Test;
            [Test] procedure ReadsMemo() var E: Codeunit \"Environment Information\"; begin if E.IsSaaS() then Error('saas'); end;
            [Test] procedure ReadsNothing() begin end;
            {extra} }}"
    )
}

const MEASURED: &str = "var S: Codeunit \"M Seen\"; begin S.Record(); end;";

fn run_with(stop: &str, publisher: &str, login: &str, extra: &str) -> OutcomesAndHits {
    outcomes_and_hits_with(
        &[
            ("dep", &[], &DEP.replace("{stop}", stop)),
            ("main", &["dep"], &main_app(login, extra)),
        ],
        &[("dep", publisher)],
        "T",
        TestIsolation::Codeunit,
    )
}

fn run(stop: &str, publisher: &str, login: &str) -> OutcomesAndHits {
    run_with(stop, publisher, login, "")
}

/// Every test not run, none flagged: main's result.
fn kept_not_run((o, hits): &OutcomesAndHits, what: &str) {
    assert!(o.iter().all(|(_, r)| matches!(r, Outcome::NotRun(_))), "{what}: {o:?}");
    assert!(hits.iter().all(|(_, h)| h.is_empty()), "{what}: {hits:?}");
}

// The measured shape: the subscriber calls IsSaaS(), IsSandbox() and GetEnableMembershipEntitlement() from its own app.
// Microsoft's login code runs first, as on main; it refuses, so the codeunit runs again with the subscriber alone: the
// test that reads the memo is flagged, the one that reads nothing is clean.
#[test]
fn a_refused_microsoft_login_falls_back_to_the_own_subscriber_flagged() {
    let (o, hits) = run(REFUSES, "Microsoft", MEASURED);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    let flagged: Vec<_> = hits.iter().filter(|(_, h)| !h.is_empty()).collect();
    assert_eq!(
        flagged,
        [&("ReadsMemo".to_string(), vec!["login-own-memo-alone"])],
        "{hits:?}"
    );
}

// The same shape where Microsoft's login code stops with an AL error: main's result, every test not run with it.
#[test]
fn an_al_error_of_microsofts_login_keeps_main() {
    let (o, _) = run(ERRORS, "Microsoft", MEASURED);
    assert!(
        o.iter()
            .all(|(_, r)| matches!(r, Outcome::NotRun(m) if m.contains("MS LOGIN"))),
        "{o:?}"
    );
}

// Neighbors that keep main's result (the login raised through Microsoft's code, whose refusal stops the codeunit):
// another procedure of the facade, another SingleInstance global the login code sets, a read of the memo without the
// facade, a call of IsSaaS() from Microsoft's code, and the same codeunits in an app not published by Microsoft.
#[test]
fn unmeasured_reads_of_the_memos_keep_the_login_raised() {
    let other_procedure = "var E: Codeunit \"Environment Information\"; begin if E.IsOnPrem() then; end;";
    let other_global = "var S: Codeunit \"M Seen\"; I: Codeunit \"System Initialization Impl.\"; begin S.Record(); if I.IsInProgress() then; end;";
    let direct = "var S: Codeunit \"M Seen\"; I: Codeunit \"Environment Information Impl.\"; begin S.Record(); if I.IsSaaS() then; end;";
    let from_microsoft = "var H: Codeunit \"Sys Helper\"; begin if H.AskSaaS() then; end;";
    for (publisher, login) in [
        ("Microsoft", other_procedure),
        ("Microsoft", other_global),
        ("Microsoft", direct),
        ("Microsoft", from_microsoft),
        ("Probe", MEASURED),
    ] {
        kept_not_run(&run(REFUSES, publisher, login), &format!("{publisher} {login}"));
    }
}

// Clause (c): a call of "Guided Experience" or "Checklist" (the keyed facades) whose reach reads a memo global keeps
// Microsoft's login, even when the subscriber also makes the measured calls.
#[test]
fn memo_reads_reached_through_guided_experience_or_checklist_keep_the_login_raised() {
    let guided =
        "var S: Codeunit \"M Seen\"; G: Codeunit \"Guided Experience\"; begin S.Record(); if G.Ask() then; end;";
    let checklist = "var S: Codeunit \"M Seen\"; C: Codeunit Checklist; begin S.Record(); if C.Ask() then; end;";
    for login in [guided, checklist] {
        kept_not_run(&run(REFUSES, "Microsoft", login), login);
    }
}

// The test side (review of al-runner PR 8639, MED 1): a test codeunit whose reach reads a global of 3702 that the
// measured procedures do not read, or touches a table Microsoft's login code writes, keeps main's refusal.
#[test]
fn a_test_codeunit_reading_beyond_the_measured_memo_globals_keeps_the_login_raised() {
    let family = "[Test] procedure ReadsFamily() var I: Codeunit \"Environment Information Impl.\"; begin if I.Family() = 'x' then; end;";
    kept_not_run(&run_with(REFUSES, "Microsoft", MEASURED, family), "family");
    let row = "[Test] procedure ReadsRow() var R: Record \"Sys Login Row\"; begin if R.FindFirst() then; end;";
    kept_not_run(&run_with(REFUSES, "Microsoft", MEASURED, row), "login row");
}

// The mark follows the gate (review of al-runner PR 8639, MED 2): a subscriber that only sets a memo global keeps
// main's result. The reach counts a global a routine names, assigned or read (`ReachFacts::globals_read`), so the
// setter reads a global Microsoft's login code sets and gets the login raised; it is not the measured shape, so its
// refusal stands and nothing is marked.
#[test]
fn a_subscriber_that_only_sets_a_memo_keeps_main() {
    let setter = "var I: Codeunit \"Environment Information Impl.\"; begin I.SetSaaS(false); end;";
    kept_not_run(&run(REFUSES, "Microsoft", setter), "setter");
}
