// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! `--explain` for a codeunit the code of the test session's company open refused before its first test (al-runner
//! issue 8142): the reason names the routine and statement the refusal left from, and every test of the codeunit shows the
//! stack, as for a refusal inside a test. Runner output only: no BC behavior is decided here.
use super::app_outcomes_tests::compiled_apps;
use super::*;
use crate::explain;

/// System Application's login code (app `dep`, publisher Microsoft), in the shape of `partner_login_tests.rs`.
const SYS: &str = "codeunit 150 \"System Initialization\" { procedure Raise() begin OnAfterLogin(); end;
        [IntegrationEvent(false, false, false)] procedure OnAfterLogin() begin end; }
    codeunit 151 \"System Initialization Impl.\" {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
        local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }";

/// A statement the runtime refuses while it runs (`explain_site.rs`, ReadsTheUndefinedDate), on line 7 of its app's file when it starts on line 4.
const REFUSED: &str = "var a: Integer; d: Date;
    begin
        a := 0;
        a := Date2DMY(d, 6);
    end;";

/// Two tests, so the reason and the stack reach every test of the codeunit; install code so the compile holds the login code.
const TESTS: &str = "codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
    codeunit 60912 T { Subtype = Test;
        [Test] procedure First() begin end;
        [Test] procedure Second() begin end; }";

/// Each test of codeunit `T` with its outcome and what `--explain` keeps of it.
fn explained(
    apps: &[(&str, &[&str], &str)],
    publishers: &[(&str, &str)],
) -> Vec<(String, Outcome, explain::TestDetail)> {
    let (engine, l) = compiled_apps(apps, publishers);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let mut details = Vec::new();
    let mut observe = |name: &str, _: &Outcome, seen: &Observed| {
        details.push((name.to_string(), super::corpus::observed_detail(&l, seen)));
    };
    let observer: &mut Observer = &mut observe;
    let outcomes = run_codeunit_observed(&engine, &l, oi, None, TestIsolation::Codeunit, Some(observer));
    assert_eq!(outcomes.len(), details.len(), "{outcomes:?} {details:?}");
    outcomes
        .into_iter()
        .zip(details)
        .map(|((name, outcome), (n, d))| {
            assert_eq!(name, n);
            (name, outcome, d)
        })
        .collect()
}

/// Every test is not run with a reason that ends with the site `who` at `at`, and `--explain` shows `who` innermost.
fn assert_refused_in(rows: &[(String, Outcome, explain::TestDetail)], who: &str, at: &str) {
    assert_eq!(rows.len(), 2, "{rows:?}");
    for (name, outcome, d) in rows {
        let Outcome::NotRun(why) = outcome else {
            panic!("{name}: {outcome:?}");
        };
        assert!(why.ends_with(&format!(" [in {who} at {at}]")), "{name}: {why}");
        assert_eq!(
            d.raised_in.first().map(|l| (l.who.as_str(), l.at.as_str())),
            Some((who, at)),
            "{d:?}"
        );
        let text = explain::render_without_oracle("head", outcome, Some(d));
        assert!(
            text.contains("refused while running, in:") && !text.contains("where: not recorded"),
            "{text}"
        );
    }
}

#[test]
fn a_refusal_in_a_login_subscriber_of_an_app_under_test_names_its_statement() {
    let main = format!(
        "codeunit 60913 OwnLogin {{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
    local procedure L()
    {REFUSED} }}
{TESTS}"
    );
    let rows = explained(&[("dep", &[], SYS), ("main", &["dep"], &main)], &[("dep", "Microsoft")]);
    assert_refused_in(&rows, "codeunit 60913 \"OwnLogin\".L", "main: main.al:7:9");
}

#[test]
fn a_refusal_in_a_login_subscriber_of_a_partner_dependency_names_its_statement() {
    let partner = format!(
        "codeunit 60901 DepLogin {{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
    local procedure L()
    {REFUSED} }}"
    );
    let rows = explained(
        &[
            ("dep", &[], SYS),
            ("dep2", &["dep"], &partner),
            ("main", &["dep2"], TESTS),
        ],
        &[("dep", "Microsoft"), ("dep2", "Probe")],
    );
    assert_refused_in(&rows, "codeunit 60901 \"DepLogin\".L", "dep2: dep2.al:7:9");
}

#[test]
fn a_refusal_in_a_company_open_subscriber_of_an_app_under_test_names_its_statement() {
    let main = format!(
        "codeunit 60914 OwnOpen {{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
    local procedure O()
    {REFUSED} }}
{TESTS}"
    );
    let rows = explained(&[("main", &[], &main)], &[]);
    assert_refused_in(&rows, "codeunit 60914 \"OwnOpen\".O", "main: main.al:7:9");
}

// Neighbor: a refusal inside a test keeps main's reason (no site suffix) and main's stack.
#[test]
fn a_refusal_inside_a_test_keeps_its_reason_and_stack() {
    let main = format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
codeunit 60912 T {{ Subtype = Test;
    [Test] procedure First()
    {REFUSED}
    [Test] procedure Second() begin end; }}"
    );
    let rows = explained(&[("main", &[], &main)], &[]);
    let Outcome::NotRun(why) = &rows[0].1 else {
        panic!("{rows:?}");
    };
    assert!(!why.contains(" [in "), "{why}");
    assert_eq!(rows[0].2.raised_in.len(), 1, "{rows:?}");
    assert_eq!(rows[0].2.raised_in[0].at, "main: main.al:7:9");
    assert_eq!(rows[1].1, Outcome::Pass);
}
