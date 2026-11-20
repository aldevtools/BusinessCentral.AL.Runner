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

/// A Company Triggers subscriber of an app under test that catches an error in a TryFunction and ends.
const CATCHES: &str = "codeunit 60915 Catches {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
    local procedure A() begin if not TryFail() then; end;
    [TryFunction] local procedure TryFail() begin Error('caught'); end; }";

/// A Company Triggers subscriber of an app under test that the runner refuses at lowering, without calling it.
const LOWERING: &str = "codeunit 60916 Lowering {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
    local procedure B() var v: Text; begin v := Debugger.GetLastErrorText(); end; }";

/// Every test is not run with a reason that names `who`, no stack is shown, and `--explain` names `who` as the routine
/// refused before the run.
fn assert_refused_before_the_run(rows: &[(String, Outcome, explain::TestDetail)], who: &str) {
    assert_eq!(rows.len(), 2, "{rows:?}");
    for (name, outcome, d) in rows {
        let Outcome::NotRun(why) = outcome else {
            panic!("{name}: {outcome:?}");
        };
        let routine = who
            .split_once(' ')
            .and_then(|(_, r)| r.split_once(' '))
            .map_or(who, |(_, r)| r);
        assert!(why.contains(routine) && !why.contains(" [in "), "{name}: {why}");
        assert!(d.raised_in.is_empty(), "{name}: {d:?}");
        assert_eq!(d.refused_in.as_ref().map(|l| l.who.as_str()), Some(who), "{d:?}");
    }
}

// Review of PR 8235, probe n5-stale: the caught error of a subscriber that ended is not the stack of a later subscriber
// refused at lowering.
#[test]
fn a_subscriber_refused_at_lowering_after_one_that_caught_an_error_shows_no_stack_of_the_other() {
    let main = format!("{CATCHES}\n{LOWERING}\n{TESTS}");
    let rows = explained(&[("main", &[], &main)], &[]);
    assert_refused_before_the_run(&rows, "codeunit 60916 \"Lowering\".B");
}

#[test]
fn a_subscriber_refused_at_lowering_names_its_routine_before_the_run() {
    let main = format!("{LOWERING}\n{TESTS}");
    let rows = explained(&[("main", &[], &main)], &[]);
    assert_refused_before_the_run(&rows, "codeunit 60916 \"Lowering\".B");
}

// Probe n2-nested: the stack ends in the helper of another codeunit that refused.
#[test]
fn a_refusal_in_a_helper_of_a_company_open_subscriber_names_the_helper() {
    let main = format!(
        "codeunit 60917 Helper {{ procedure H()
    {REFUSED} }}
codeunit 60914 OwnOpen {{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
    local procedure O() var h: Codeunit Helper; begin h.H(); end; }}
{TESTS}"
    );
    let rows = explained(&[("main", &[], &main)], &[]);
    assert_refused_in(&rows, "codeunit 60917 \"Helper\".H", "main: main.al:5:9");
    assert_eq!(rows[0].2.raised_in.len(), 2, "{rows:?}");
}

// Probe n1b-open-error: an AL error of a company-open subscriber keeps main's reason (no site) and shows its stack.
#[test]
fn an_error_in_a_company_open_subscriber_keeps_its_reason_and_shows_its_stack() {
    let main = format!(
        "codeunit 60914 OwnOpen {{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
    local procedure O() begin Error('boom'); end; }}
{TESTS}"
    );
    let rows = explained(&[("main", &[], &main)], &[]);
    for (name, outcome, d) in &rows {
        let Outcome::NotRun(why) = outcome else {
            panic!("{name}: {outcome:?}");
        };
        assert!(
            why.ends_with("failed before the first test (behaviour not pinned): boom"),
            "{why}"
        );
        assert_eq!(
            d.raised_in.first().map(|l| l.who.as_str()),
            Some("codeunit 60914 \"OwnOpen\".O"),
            "{d:?}"
        );
    }
}

// The reason of a refusal is the same with tracing off (no observer) as with --explain.
#[test]
fn the_reason_of_a_company_open_refusal_does_not_depend_on_tracing() {
    let main = format!(
        "codeunit 60914 OwnOpen {{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
    local procedure O()
    {REFUSED} }}
{TESTS}"
    );
    let traced: Vec<(String, Outcome)> = explained(&[("main", &[], &main)], &[])
        .into_iter()
        .map(|(n, o, _)| (n, o))
        .collect();
    let (engine, l) = compiled_apps(&[("main", &[], &main)], &[]);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let plain = run_codeunit_observed(&engine, &l, oi, None, TestIsolation::Codeunit, None);
    assert_eq!(plain, traced);
}

// Probe n4b-install-read: install code of an app under test that is refused names its routine and statement in the reason
// of a test that reads what it wrote.
#[test]
fn a_refusal_in_install_code_of_an_app_under_test_names_its_statement() {
    let main = "table 60920 X { fields { field(1; K; Integer) { } } }
codeunit 60921 Inst2 { Subtype = Install;
    trigger OnInstallAppPerCompany() var x: Record X; a: Integer; d: Date; begin
        x.K := 1; x.Insert();
        a := Date2DMY(d, 6);
    end; }
codeunit 60912 T { Subtype = Test;
    [Test] procedure First() var x: Record X; begin x.FindFirst(); end;
    [Test] procedure Second() begin end; }";
    let rows = explained(&[("main", &[], main)], &[]);
    let Outcome::NotRun(why) = &rows[0].1 else {
        panic!("{rows:?}");
    };
    assert!(
        why.ends_with(" [in codeunit 60921 \"Inst2\".OnInstallAppPerCompany (trigger) at main: main.al:5:9]"),
        "{why}"
    );
}
