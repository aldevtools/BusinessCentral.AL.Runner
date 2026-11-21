// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! An app under test's login subscriber that fills System Application's login memos (`login_memo.rs`, assumption
//! `login-own-memo-alone`, corpus PR 1490 P2A_06 and P2A_11). The dependency app `dep` (publisher Microsoft) stands in
//! for "System Initialization" (150), "System Initialization Impl." (151), the facades "Environment Information" (457)
//! and "Server Setting" (6723) and their memos 3702 and 3703. Its login code fills the memo and then stops with an
//! error, so a codeunit whose login is raised through Microsoft's code does not pass, and one that gets the app's own
//! subscribers only does.
use super::app_outcomes_tests::{OutcomesAndHits, outcomes_and_hits_with};
use super::*;

const DEP: &str = r#"
codeunit 150 "System Initialization" { procedure Raise() begin OnAfterLogin(); end;
    [IntegrationEvent(false, false, true)] procedure OnAfterLogin() begin end; }
codeunit 151 "System Initialization Impl."
{
    SingleInstance = true;
    var InProgress: Boolean;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany() var E: Codeunit "Environment Information"; S: Codeunit "System Initialization";
    begin InProgress := true; if E.IsSaaS() then; S.Raise(); Error('MS LOGIN'); end;
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
    var Known: Boolean; SaaS: Boolean;
    procedure IsSaaS(): Boolean begin if not Known then begin SaaS := false; Known := true; end; exit(SaaS); end;
    procedure IsSandbox(): Boolean begin exit(IsSaaS()); end;
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
codeunit 160 "Sys Helper" { procedure AskSaaS(): Boolean var E: Codeunit "Environment Information"; begin exit(E.IsSaaS()); end; }
"#;

/// The app under test: the OnAfterLogin subscriber with body `login`; `M Seen` calls the three measured procedures as
/// the probe's "LT P2 XA Env Seen".Record does (P2XaLoginMemos.al). The test codeunit: one test reads the memo, one
/// reads nothing.
fn main_app(login: &str) -> String {
    format!(
        "codeunit 61002 \"M Seen\" {{ procedure Record() var E: Codeunit \"Environment Information\"; S: Codeunit \"Server Setting\";
            begin if E.IsSaaS() then; if E.IsSandbox() then; if S.GetEnableMembershipEntitlement() then; end; }}
        codeunit 61001 \"M Login\" {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() {login} }}
        codeunit 61010 T {{ Subtype = Test;
            [Test] procedure ReadsMemo() var E: Codeunit \"Environment Information\"; begin if E.IsSaaS() then Error('saas'); end;
            [Test] procedure ReadsNothing() begin end; }}"
    )
}

const MEASURED: &str = "var S: Codeunit \"M Seen\"; begin S.Record(); end;";

fn run(publisher: &str, login: &str) -> OutcomesAndHits {
    outcomes_and_hits_with(
        &[("dep", &[], DEP), ("main", &["dep"], &main_app(login))],
        &[("dep", publisher)],
        "T",
        TestIsolation::Codeunit,
    )
}

// The measured shape: the subscriber calls IsSaaS(), IsSandbox() and GetEnableMembershipEntitlement() from its own app.
// On main the read of 3702 raised the login through Microsoft's code for every test codeunit (here: its error). Now the
// subscribers run alone; the test that reads the memo is flagged, the one that reads nothing is clean.
#[test]
fn an_own_login_subscriber_that_asks_is_saas_runs_alone_flagged() {
    let (o, hits) = run("Microsoft", MEASURED);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    let flagged: Vec<_> = hits.iter().filter(|(_, h)| !h.is_empty()).collect();
    assert_eq!(
        flagged,
        [&("ReadsMemo".to_string(), vec!["login-own-memo-alone"])],
        "{hits:?}"
    );
}

// Neighbors that keep main's result (the login raised through Microsoft's code, whose error stops the codeunit): another
// procedure of the facade, another SingleInstance global the login code sets, a read of the memo without the facade, a
// call of IsSaaS() from Microsoft's code,
// and the same codeunits in an app not published by Microsoft.
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
        let (o, _) = run(publisher, login);
        let raised = |r: &Outcome| matches!(r, Outcome::NotRun(m) if m.contains("MS LOGIN"));
        assert!(o.iter().all(|(_, r)| raised(r)), "{publisher} {login}: {o:?}");
    }
}
