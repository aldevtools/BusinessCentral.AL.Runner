//! The partner dependency apps' login subscribers in the test session (al-runner issue 8115, scenario s09;
//! `partner_login.rs`, assumption `partner-login-subscriber-in-test-session`).
use super::app_outcomes_tests::{OutcomesAndHits, outcomes_and_hits_with};
use super::*;

const ID: &str = "partner-login-subscriber-in-test-session";

/// System Application's login code (app `dep`, publisher Microsoft): "System Initialization Impl." raises
/// "System Initialization".OnAfterLogin from Company Triggers OnCompanyOpenCompleted; `isolated` is the event's third
/// IntegrationEvent argument.
fn sys(isolated: bool) -> String {
    format!(
        "codeunit 150 \"System Initialization\" {{ procedure Raise() begin OnAfterLogin(); end;
            [IntegrationEvent(false, false, {isolated})] procedure OnAfterLogin() begin end; }}
        codeunit 151 \"System Initialization Impl.\" {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
            local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }}"
    )
}

/// A partner dependency app (`dep2`): table `Other`, a SingleInstance `Cache` (a Boolean and a record global), a manual subscriber `Hook`, and the
/// OnAfterLogin subscriber with body `login`.
fn partner(login: &str) -> String {
    format!(
        "table 60900 Other {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Blocked; Boolean) {{ }} }} }}
        codeunit 60904 Cache {{ SingleInstance = true; var On: Boolean; Row: Record Other;
            procedure Put() begin On := true; end; procedure Take(): Boolean begin exit(On); end;
            procedure Sort() begin Row.SetCurrentKey(Blocked); end; }}
        codeunit 60906 Pub {{ procedure Ask(): Integer var N: Integer; begin N := 1; OnAsk(N); exit(N); end;
            [IntegrationEvent(false, false)] local procedure OnAsk(var N: Integer) begin end; }}
        codeunit 60907 Hook {{ SingleInstance = true; EventSubscriberInstance = Manual;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnAsk', '', false, false)]
            local procedure H(var N: Integer) begin N := 2; end; }}
        codeunit 60901 DepLogin {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() {login} }}"
    )
}

/// A row of `Other` with a key of its own in each session (the install sessions raise the login too).
const INSERTS: &str = "var G: Record Other; begin G.No := Format(G.Count()); if G.Insert() then; end;";

/// Scenario s09 (G1sXaInstall.al:116-124): a group 1 filter on `Other`.
const SETS_FILTER: &str =
    "var G: Record Other; begin G.FilterGroup(1); G.SetRange(Blocked, true); G.FilterGroup(0); end;";

/// The app under test: install code (so the compile holds the login code, al-sema `reach_install_events`); a test that
/// counts `Other` with the filter it expects, one that reads the cache, one on its own table, one on no table.
const TESTS: &str = "table 60910 Mine { fields { field(1; K; Code[10]) { } } }
    codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
    codeunit 60912 T { Subtype = Test;
        [Test] procedure CountOther() var O: Record Other; begin O.No := 'A'; O.Insert(); O.No := 'B'; O.Blocked := true; O.Insert(); if O.Count() <> 1 then Error('rows %1', O.Count()); end;
        [Test] procedure CacheIsOff() var C: Codeunit Cache; begin if C.Take() then Error('cache on'); end;
        [Test] procedure OtherTable() var M: Record Mine; begin M.K := 'X'; M.Insert(); end;
        [Test] procedure NoTable() begin end; }";

fn run_with(publisher: &str, login: &str, tests: &str, isolated: bool, isolation: TestIsolation) -> OutcomesAndHits {
    run_codeunit(publisher, login, tests, isolated, isolation, "T")
}

fn run_codeunit(
    publisher: &str,
    login: &str,
    tests: &str,
    isolated: bool,
    isolation: TestIsolation,
    codeunit: &str,
) -> OutcomesAndHits {
    outcomes_and_hits_with(
        &[
            ("dep", &[], &sys(isolated)),
            ("dep2", &["dep"], &partner(login)),
            ("main", &["dep2"], tests),
        ],
        &[("dep", "Microsoft"), ("dep2", publisher)],
        codeunit,
        isolation,
    )
}

fn run(login: &str) -> OutcomesAndHits {
    run_with("Probe", login, TESTS, false, TestIsolation::Codeunit)
}

fn not_run_for(o: &Outcome, why: &str) -> bool {
    matches!(o, Outcome::NotRun(r) if r.contains(why))
}

fn hit(hits: &[(String, Vec<&'static str>)], name: &str) -> bool {
    hits.iter().any(|(n, h)| n == name && h.contains(&ID))
}

// Scenario s09: the partner subscriber's group 1 filter is in the test session (G1sInstallSessionTests.al:103). On main the
// test counted 2 rows and failed; the filter another record set is now there, and the runtime refuses a record without it
// (al-vm records.rs `global_group_check`). The tests that do not touch the table pass clean.
#[test]
fn a_partner_login_subscribers_group1_filter_reaches_the_test_session() {
    let (o, hits) = run(SETS_FILTER);
    assert!(not_run_for(&o[0].1, "filter group 1"), "{o:?}");
    for k in 1..4 {
        assert_eq!(o[k].1, Outcome::Pass, "{o:?}");
        assert!(!hit(&hits, &o[k].0), "{hits:?}");
    }
}

// A neighbor that keeps main's result: the subscriber of a dependency app published by Microsoft is not run here (only the
// Raise path runs Microsoft's login code).
#[test]
fn a_microsoft_dependency_login_subscriber_keeps_mains_result() {
    let (o, _) = run_with("Microsoft", SETS_FILTER, TESTS, false, TestIsolation::Codeunit);
    assert!(matches!(&o[0].1, Outcome::Fail(m) if m.contains("rows 2")), "{o:?}");
}

// A neighbor that keeps main's result: a dependency app whose manifest names no publisher.
#[test]
fn a_dependency_without_a_publisher_keeps_mains_result() {
    let (o, _) = run_with("", SETS_FILTER, TESTS, false, TestIsolation::Codeunit);
    assert!(matches!(&o[0].1, Outcome::Fail(m) if m.contains("rows 2")), "{o:?}");
}

// A neighbor that keeps main's result: under Disabled isolation the probe did not measure the session.
#[test]
fn under_disabled_isolation_the_partner_subscriber_keeps_mains_result() {
    let (o, _) = run_with("Probe", SETS_FILTER, TESTS, false, TestIsolation::Disabled);
    assert!(matches!(&o[0].1, Outcome::Fail(m) if m.contains("rows 2")), "{o:?}");
}

// Rows the subscriber writes are not measured: a test that touches the table is not run, the others run clean.
#[test]
fn a_test_that_touches_a_table_the_partner_subscriber_wrote_is_not_run() {
    let (o, hits) = run(INSERTS);
    assert!(not_run_for(&o[0].1, "partner dependency apps"), "{o:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[3].1, Outcome::Pass, "{o:?}");
    assert!(!hit(&hits, "NoTable"), "{hits:?}");
}

// A SingleInstance global the subscriber sets is in the test session, as an own login subscriber's is
// (LgnLoginChecklistTests.al:93, :132): the test that reads it sees it and relies on the assumption; the tests that do not
// read it run clean.
#[test]
fn a_test_that_reads_a_single_instance_global_the_partner_subscriber_set_sees_it() {
    let (o, hits) = run("var C: Codeunit Cache; begin C.Put(); end;");
    assert!(matches!(&o[1].1, Outcome::Fail(m) if m.contains("cache on")), "{o:?}");
    assert!(hit(&hits, "CacheIsOff"), "{hits:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[3].1, Outcome::Pass, "{o:?}");
    assert!(!hit(&hits, "OtherTable") && !hit(&hits, "NoTable"), "{hits:?}");
}

// A manual subscriber the subscriber binds through a SingleInstance codeunit: no test runs.
#[test]
fn a_partner_subscriber_that_binds_refuses_the_codeunit() {
    let (o, _) = run("var H: Codeunit Hook; begin if BindSubscription(H) then; end;");
    assert!(o.iter().all(|(_, x)| matches!(x, Outcome::NotRun(_))), "{o:?}");
}

// WorkDate the subscriber moves on in each session: no test runs.
#[test]
fn a_partner_subscriber_that_sets_workdate_refuses_the_codeunit() {
    let (o, _) = run("begin WorkDate(CalcDate('<+1D>', WorkDate())); end;");
    assert!(o.iter().all(|(_, x)| not_run_for(x, "WorkDate")), "{o:?}");
}

// A subscriber that leaves nothing: every test runs clean, as on main.
#[test]
fn a_partner_subscriber_that_leaves_nothing_keeps_mains_result() {
    let (o, hits) = run("var N: Integer; begin N := 1; end;");
    assert_eq!(o[0].1, Outcome::Fail("rows 2".into()), "{o:?}");
    assert!(o[1..].iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert!(hits.iter().all(|(_, h)| !h.contains(&ID)), "{hits:?}");
}

// An error in the subscriber of a non-isolated event stops the login: the codeunit is not run.
#[test]
fn a_partner_subscriber_error_refuses_the_codeunit() {
    let (o, _) = run("begin Error('login failed'); end;");
    assert!(o.iter().all(|(_, x)| not_run_for(x, "login failed")), "{o:?}");
}

// In an isolated OnAfterLogin (as System Application declares it) the error is rolled back and the session goes on
// (codeunits/TestIsolatedEventSubscriberError.al): the tests run as on main.
#[test]
fn a_partner_subscriber_error_in_an_isolated_event_is_rolled_back() {
    let (o, _) = run_with(
        "Probe",
        "begin Error('login failed'); end;",
        TESTS,
        true,
        TestIsolation::Codeunit,
    );
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[3].1, Outcome::Pass, "{o:?}");
}

// A subscriber the runtime cannot run: the codeunit is not run.
#[test]
fn a_partner_subscriber_that_cannot_run_refuses_the_codeunit() {
    let (o, _) = run("var G: Record Other temporary; begin G.FilterGroup(1); G.SetRange(Blocked, true); end;");
    assert!(
        o.iter().all(|(_, x)| not_run_for(x, "partner dependency apps")),
        "{o:?}"
    );
}

// The test codeunit's OnRun reads the table the subscriber wrote: the codeunit is not run.
#[test]
fn an_onrun_that_reads_a_table_the_partner_subscriber_wrote_refuses_the_codeunit() {
    let tests = "codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
        codeunit 60912 T { Subtype = Test;
            trigger OnRun() var O: Record Other; begin if O.IsEmpty() then; end;
            [Test] procedure NoTable() begin end; }";
    let (o, _) = run_with("Probe", INSERTS, tests, false, TestIsolation::Codeunit);
    assert!(
        not_run_for(&o[0].1, "OnRun: the login subscribers of the partner"),
        "{o:?}"
    );
}

// Probe n8 of the review of al-runner PR 8119: an app under test with its own OnAfterLogin subscriber (Login::Own) next to
// the partner one. Both run, in the order the event reaches them; the own subscriber's row is answered as on main, the
// partner's filter refuses the test that counts its table.
#[test]
fn a_partner_login_subscriber_next_to_an_own_one() {
    let tests = "table 60910 Mine { fields { field(1; K; Code[10]) { } } }
        codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
        codeunit 60913 OwnLogin {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() var M: Record Mine; begin M.K := 'L'; if M.Insert() then; end; }
        codeunit 60912 T { Subtype = Test;
            [Test] procedure CountOther() var O: Record Other; begin O.No := 'A'; O.Insert(); O.No := 'B'; O.Blocked := true; O.Insert(); if O.Count() <> 1 then Error('rows %1', O.Count()); end;
            [Test] procedure OwnRow() var M: Record Mine; begin M.Get('L'); end;
            [Test] procedure NoTable() begin end; }";
    let (o, _) = run_with("Probe", SETS_FILTER, tests, false, TestIsolation::Codeunit);
    assert!(not_run_for(&o[0].1, "filter group 1"), "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
}

/// Two test codeunits: `T` reads the SingleInstance `Cache` and the number sequence `S`, `U` reads neither. `early` is the
/// body of an OnCompanyOpen subscriber of the app under test, which runs before the login.
fn readers(early: &str) -> String {
    format!(
        "table 60910 Mine {{ fields {{ field(1; K; Code[10]) {{ }} }} }}
        codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60914 Early {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
            local procedure O() {early} }}
        codeunit 60912 T {{ Subtype = Test;
            [Test] procedure CacheIsOff() var C: Codeunit Cache; begin if C.Take() then Error('cache on'); end;
            [Test] procedure Sequence() begin if NumberSequence.Exists('S') then if NumberSequence.Current('S') = 0 then; end;
            [Test] procedure PutFirst() var C: Codeunit Cache; begin C.Put(); if not C.Take() then Error('cache off'); end; }}
        codeunit 60915 U {{ Subtype = Test;
            [Test] procedure OtherTable() var M: Record Mine; begin M.K := 'X'; M.Insert(); end; }}"
    )
}

const NOTHING_EARLY: &str = "var N: Integer; begin N := 1; end;";

fn run_readers(login: &str, early: &str, codeunit: &str) -> OutcomesAndHits {
    let tests = readers(early);
    run_codeunit("Probe", login, &tests, false, TestIsolation::Codeunit, codeunit)
}

// The subscriber makes the SingleInstance codeunit's instance and leaves its globals as declared: what a reader finds is
// what a fresh instance holds, so the test codeunit that reads it runs as on main.
#[test]
fn a_new_single_instance_with_declared_globals_is_no_change() {
    let (o, _) = run_readers(
        "var C: Codeunit Cache; begin if C.Take() then; end;",
        NOTHING_EARLY,
        "T",
    );
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
}

/// Readers of `T` after a login that sets the cache: the test that reads it first sees it (and relies on the assumption),
/// the one that never reads it and the one that writes it before it reads it run clean; `U` runs clean.
fn assert_only_the_first_reader_relies_on_it(early: &str) {
    let login = "var C: Codeunit Cache; begin C.Put(); end;";
    let (o, hits) = run_readers(login, early, "T");
    assert!(matches!(&o[0].1, Outcome::Fail(m) if m.contains("cache on")), "{o:?}");
    assert!(hit(&hits, "CacheIsOff"), "{hits:?}");
    assert_eq!((&o[1].1, &o[2].1), (&Outcome::Pass, &Outcome::Pass), "{o:?}");
    assert!(!hit(&hits, "Sequence") && !hit(&hits, "PutFirst"), "{hits:?}");
    let (o, hits) = run_readers(login, early, "U");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert!(hits.iter().all(|(_, h)| !h.contains(&ID)), "{hits:?}");
}

// The subscriber makes the instance and sets a global.
#[test]
fn a_new_single_instance_with_a_set_global_marks_only_the_test_that_reads_it_first() {
    assert_only_the_first_reader_relies_on_it(NOTHING_EARLY);
}

// The instance was there before the login (an OnCompanyOpen subscriber of the app under test made it) and the subscriber
// changes it.
#[test]
fn a_changed_existing_single_instance_marks_only_the_test_that_reads_it_first() {
    assert_only_the_first_reader_relies_on_it("var C: Codeunit Cache; begin if C.Take() then; end;");
}

// The OnRun trigger reads the global the subscriber set before the first test: what every test finds may rest on it.
#[test]
fn an_onrun_that_reads_the_set_global_marks_every_test() {
    let tests = readers(NOTHING_EARLY).replace(
        "codeunit 60915 U { Subtype = Test;",
        "codeunit 60915 U { Subtype = Test; trigger OnRun() var C: Codeunit Cache; begin if C.Take() then; end;",
    );
    let login = "var C: Codeunit Cache; begin C.Put(); end;";
    let (o, hits) = run_codeunit("Probe", login, &tests, false, TestIsolation::Codeunit, "U");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert!(hit(&hits, "OtherTable"), "{hits:?}");
}

// A number sequence the subscriber moves on: only the test codeunit whose reach uses number sequences is refused.
#[test]
fn a_number_sequence_the_subscriber_moves_refuses_only_its_readers() {
    let login = "begin if not NumberSequence.Exists('S') then NumberSequence.Insert('S'); if NumberSequence.Next('S') = 0 then; end;";
    let (o, _) = run_readers(login, NOTHING_EARLY, "T");
    assert!(o.iter().all(|(_, x)| not_run_for(x, "a number sequence")), "{o:?}");
    let (o, _) = run_readers(login, NOTHING_EARLY, "U");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
}

// WorkDate every code reads: it keeps refusing every test codeunit, also one that does not name it.
#[test]
fn workdate_the_subscriber_moves_still_refuses_every_codeunit() {
    let (o, _) = run_readers(
        "begin WorkDate(CalcDate('<+1D>', WorkDate())); end;",
        NOTHING_EARLY,
        "U",
    );
    assert!(o.iter().all(|(_, x)| not_run_for(x, "WorkDate")), "{o:?}");
}

// A record global whose view the subscriber changes, with its fields as declared: the instance is not as declared. The
// mark goes by codeunit, not by global: the test that reads another global of it first relies on the assumption too.
#[test]
fn a_record_global_whose_key_the_subscriber_sets_marks_the_first_reader_of_the_codeunit() {
    let (o, hits) = run_readers("var C: Codeunit Cache; begin C.Sort(); end;", NOTHING_EARLY, "T");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert!(hit(&hits, "CacheIsOff") && !hit(&hits, "Sequence"), "{hits:?}");
}
