//! More than two partner login subscribers that change one SingleInstance codeunit (al-runner issue 8945,
//! `login_shared_memo.rs`; assumption `login-subscribers-shared-memo`, corpus PR 1567 cells A and B).
use super::app_outcomes_tests::{OutcomesAndHits, outcomes_and_hits_with};
use super::*;

const SHARED: &str = "login-subscribers-shared-memo";

/// System Application's login code (publisher Microsoft): "System Initialization Impl." raises
/// "System Initialization".OnAfterLogin from Company Triggers OnCompanyOpenCompleted.
const SYS: &str = "codeunit 150 \"System Initialization\" { procedure Raise() begin OnAfterLogin(); end;
        [IntegrationEvent(false, false, false)] procedure OnAfterLogin() begin end; }
    codeunit 151 \"System Initialization Impl.\" {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
        local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }";

/// XApp A's memo of the State cells (LsXaLoginSubscribers.al, State One and State Many): two temporary records of its
/// own app's tables, three Booleans, two Texts and two Integers; `extra` declares more globals. `Touch` loads the setup
/// rows once, sets the Boolean, the Text of the last caller, `Num * 10 + K` and adds a log row.
fn memo(extra: &str) -> String {
    format!(
        "table 60930 XSetup {{ fields {{ field(1; Code; Code[20]) {{ }} }} }}
        table 60934 XLog {{ fields {{ field(1; No; Integer) {{ }} field(2; Tag; Text[30]) {{ }} }} }}
        table 60931 XWrite {{ fields {{ field(1; Tag; Code[20]) {{ }} }} }}
        codeunit 60936 Memo {{ SingleInstance = true;
            var TempSetup: Record XSetup temporary; TempLog: Record XLog temporary; Loaded: Boolean; Activated: Boolean;
                Flag: Boolean; LoadedBy: Text; Txt: Text; Num: Integer; Touches: Integer; {extra}
            procedure Touch(Tag: Text; K: Integer) begin
                if not Loaded then begin Loaded := true; LoadedBy := Tag; Activated := true;
                    TempSetup.Code := 'S1'; TempSetup.Insert(); TempSetup.Code := 'S2'; TempSetup.Insert(); end;
                Flag := true; Txt := Tag; Num := Num * 10 + K; Touches += 1;
                TempLog.No := Touches; TempLog.Tag := CopyStr(Tag, 1, 30); TempLog.Insert(); end;
            procedure Summary(): Text begin exit(LoadedBy + ':' + Txt + ':' + Format(Num) + ':' + Format(TempLog.Count())); end; }}"
    )
}

/// An OnAfterLogin subscriber codeunit `id` that runs `body`.
fn login(id: u32, body: &str) -> String {
    format!(
        "codeunit {id} \"Login {id}\" {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() {body} }}"
    )
}

/// A login subscriber that touches the memo with `tag` and `k`.
fn touch(id: u32, tag: &str, k: i32) -> String {
    login(id, &format!("var M: Codeunit Memo; begin M.Touch('{tag}', {k}); end;"))
}

/// The app under test: install code (so the compile holds the login code), a test that reads the memo's summary and wants
/// `want`, and a test that reads nothing.
fn tests(want: &str) -> String {
    format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60912 T {{ Subtype = Test;
            [Test] procedure Reads() var M: Codeunit Memo; begin if M.Summary() <> '{want}' then Error('summary %1', M.Summary()); end;
            [Test] procedure ReadsNothing() begin end; }}"
    )
}

/// XApp A (`depa`, the memo and `a`), XApp B (`depb`, on `b_deps`, with `b`) and XApp C (`depc`, on `c_deps`, with `c`), all
/// of publisher Probe, and the app under test on all three.
fn run(a: &str, b: &str, b_deps: &[&str], c: &str, c_deps: &[&str], want: &str) -> OutcomesAndHits {
    outcomes_and_hits_with(
        &[
            ("dep", &[], SYS),
            ("depa", &["dep"], a),
            ("depb", b_deps, b),
            ("depc", c_deps, c),
            ("main", &["depa", "depb", "depc"], &tests(want)),
        ],
        &[
            ("dep", "Microsoft"),
            ("depa", "Probe"),
            ("depb", "Probe"),
            ("depc", "Probe"),
        ],
        "T",
        TestIsolation::Codeunit,
    )
}

/// The assumption entries test `test` passed through.
fn marks<'a>(hits: &'a [(String, Vec<&'static str>)], test: &str) -> &'a [&'static str] {
    hits.iter().find(|(n, _)| n == test).map_or(&[], |(_, h)| h.as_slice())
}

fn assert_flagged((o, hits): OutcomesAndHits) {
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert_eq!(marks(&hits, "Reads"), [SHARED], "{hits:?}");
    assert!(marks(&hits, "ReadsNothing").is_empty(), "{hits:?}");
}

fn assert_refused((o, _): OutcomesAndHits, why: &str) {
    assert!(matches!(&o[0].1, Outcome::NotRun(r) if r.contains(why)), "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

/// State One: the three subscribers of XApp A (ids ascending) change the memo; B and C have a subscriber that does not.
fn state_one(extra: &str, b_deps: &[&str]) -> OutcomesAndHits {
    let a = format!(
        "{}\n{}\n{}\n{}",
        memo(extra),
        touch(60940, "A1", 1),
        touch(60941, "A2", 2),
        touch(60942, "A3", 3)
    );
    let b = login(60950, "begin end;");
    let c = login(60960, "begin end;");
    run(&a, &b, b_deps, &c, &["dep", "depa", "depb"], "A1:A3:123:3")
}

/// State Many: one subscriber of each app, A1 (60940), B1 (`b_id`) and C1 (60960), XApp C on `c_deps`.
fn state_many(b_id: u32, c_deps: &[&str]) -> OutcomesAndHits {
    let a = format!("{}\n{}", memo(""), touch(60940, "A1", 1));
    let b = touch(b_id, "B1", 2);
    let c = touch(60960, "C1", 3);
    run(&a, &b, &["dep", "depa"], &c, c_deps, "A1:C1:123:3")
}

// Cell B, State One (corpus PR 1567, LsLoginSubscriberTests.al:259-278): three subscribers of the memo's app change it;
// the reader sees what they left, flagged.
#[test]
fn three_subscribers_of_the_memos_app_keep_their_changes_flagged() {
    assert_flagged(state_one("", &["dep", "depa"]));
}

// Cell B, State Many: A1, B1 and C1 of three apps along a dependency chain, lowest subscriber ids ascending.
#[test]
fn one_subscriber_of_each_of_three_chained_apps_keep_their_changes_flagged() {
    assert_flagged(state_many(60950, &["dep", "depa", "depb"]));
}

// A global of a type the cell did not have (a Decimal more): refused, as on main.
#[test]
fn a_memo_with_another_global_refuses_its_reads() {
    assert_refused(
        state_one("Rate: Decimal;", &["dep", "depa"]),
        "more than two subscribers changed it",
    );
}

// XApp C does not depend on XApp B: BC's order between B and C is not measured.
#[test]
fn apps_that_are_no_dependency_chain_refuse_the_reads() {
    assert_refused(
        state_many(60950, &["dep", "depa"]),
        "more than two subscribers changed it",
    );
}

// XApp B's subscriber has a lower id than XApp A's: dependency order and lowest-id order disagree, which the probe did
// not tell apart.
#[test]
fn apps_whose_lowest_subscriber_ids_do_not_ascend_refuse_the_reads() {
    assert_refused(
        state_many(60939, &["dep", "depa", "depb"]),
        "more than two subscribers changed it",
    );
}

// Four subscribers of the memo's app: no cell has four.
#[test]
fn four_subscribers_of_the_memos_app_refuse_the_reads() {
    let a = format!(
        "{}\n{}\n{}\n{}\n{}",
        memo(""),
        touch(60940, "A1", 1),
        touch(60941, "A2", 2),
        touch(60942, "A3", 3),
        touch(60943, "A4", 4)
    );
    let r = run(
        &a,
        &login(60950, "begin end;"),
        &["dep", "depa"],
        &login(60960, "begin end;"),
        &["dep", "depa", "depb"],
        "",
    );
    assert_refused(r, "more than two subscribers changed it");
}
