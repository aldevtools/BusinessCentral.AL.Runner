//! Rows install code left that the runner does not know (al-runner issue 8115): install code after a stopped install
//! routine, a not-run install routine's table read through a RecordRef, and a dependency app's login subscriber.
use super::app_outcomes::{outcomes_and_hits_of_apps, outcomes_and_hits_of_published_apps};
use super::*;

/// App A: table `Item`; its install trigger inserts two rows and then runs `tail`.
fn app_a(tail: &str) -> String {
    format!(
        "table 60950 Item {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Blocked; Boolean) {{ }} }} }}
        codeunit 60952 InstA {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var I: Record Item; T: Record Item temporary;
            begin I.No := 'I1'; if I.Insert() then; I.No := 'I2'; I.Blocked := true; if I.Insert() then; {tail} end; }}"
    )
}

/// The statement after A's inserts that the runner refuses (scenario s15: a group 1 filter on a temporary record).
const STOPS: &str = "T.FilterGroup(1); T.SetRange(Blocked, true); T.FilterGroup(0);";

/// App B: its install trigger stores the count of A's table; a test reports it, another touches no table.
const APP_B: &str = "table 60960 Setup { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
    codeunit 60961 InstB { Subtype = Install;
        trigger OnInstallAppPerCompany() var I: Record Item; S: Record Setup; begin S.N := I.Count(); if S.Insert() then; end; }
    codeunit 60962 T { Subtype = Test;
        [Test] procedure Seen() var S: Record Setup; begin S.Get(); if S.N <> 2 then Error('B saw %1', S.N); end;
        [Test] procedure NoTable() begin end; }";

fn run_ab(a: &str, b: &str) -> Vec<(String, Outcome)> {
    outcomes_and_hits_of_apps(&[("a", &[], a), ("b", &["a"], b)], "T").0
}

fn not_run(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(_))
}

// Scenario s15: A's install trigger stops after its inserts and is rolled back. On BC it runs to its end (2 rows), so the
// count B's install code takes is not known: B is not run, and the test that reads what B wrote is not run.
#[test]
fn install_code_that_reads_a_stopped_routines_table_is_not_run() {
    let o = run_ab(&app_a(STOPS), APP_B);
    assert!(not_run(&o[0].1), "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// The control: A runs to its end, and B counts both rows (main's result).
#[test]
fn install_code_after_a_routine_that_ran_counts_its_rows() {
    let o = run_ab(&app_a(""), APP_B);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// A neighbor that keeps main's result: B's install code touches no table A's stopped routine writes.
#[test]
fn install_code_that_does_not_touch_the_stopped_routines_tables_runs() {
    let b = "table 60960 Setup { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var S: Record Setup; begin S.N := 2; if S.Insert() then; end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var S: Record Setup; begin S.Get(); if S.N <> 2 then Error('B saw %1', S.N); end; }";
    let o = run_ab(&app_a(STOPS), b);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// B reads A's table through a RecordRef, which the static reach of B does not see: the read still makes B not run.
#[test]
fn install_code_that_reads_the_table_through_a_recordref_is_not_run() {
    let b = "table 60960 Setup { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var R: RecordRef; S: Record Setup; begin R.Open(60950); S.N := R.Count(); if S.Insert() then; end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var S: Record Setup; begin S.Get(); if S.N <> 2 then Error('B saw %1', S.N); end; }";
    let o = run_ab(&app_a(STOPS), b);
    assert!(not_run(&o[0].1), "{o:?}");
}

// Scenario s03b: B's install code stops after its insert and is rolled back, so the rows of its table are not known. A test
// that reads that table through a RecordRef is not run, as one through a Record is.
#[test]
fn a_recordref_read_of_a_not_run_routines_table_is_not_run() {
    let b = "table 60960 Setup { fields { field(1; K; Code[10]) { } field(2; Blocked; Boolean) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var S: Record Setup; T: Record Setup temporary;
            begin if S.Insert() then; T.FilterGroup(1); T.SetRange(Blocked, true); T.FilterGroup(0); end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure ViaRecordRef() var R: RecordRef; begin R.Open(60960); if R.Count() <> 1 then Error('rows %1', R.Count()); end;
            [Test] procedure NoTable() begin end; }";
    let o = run_ab(&app_a(""), b);
    assert!(not_run(&o[0].1), "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

/// A dependency app (scenario s09): table `Other` with two rows (one blocked) from its install code, and a "System
/// Initialization".OnAfterLogin subscriber with body `login`.
fn dep_login(login: &str) -> String {
    format!(
        "codeunit 150 \"System Initialization\" {{ procedure Raise() begin OnAfterLogin(); end;
            [IntegrationEvent(false, false)] procedure OnAfterLogin() begin end; }}
        codeunit 151 \"System Initialization Impl.\" {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
            local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }}
        table 60900 Other {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Blocked; Boolean) {{ }} }} }}
        codeunit 60901 DepLogin {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() {login} }}"
    )
}

const SETS_FILTER: &str =
    "var G: Record Other; begin G.FilterGroup(1); G.SetRange(Blocked, true); G.FilterGroup(0); end;";

/// The app under test: it has install code (so the compile holds the dependency apps' login code, al-sema
/// `reach_install_events`), a test that counts `Other`, one that reads it through a RecordRef, one on its own table.
const TESTS: &str = "table 60910 Mine { fields { field(1; K; Code[10]) { } } }
    codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
    codeunit 60912 T { Subtype = Test;
        [Test] procedure CountOther() var O: Record Other; begin O.No := 'A'; O.Insert(); O.No := 'B'; O.Blocked := true; O.Insert(); if O.Count() <> 1 then Error('rows %1', O.Count()); end;
        [Test] procedure ViaRecordRef() var R: RecordRef; begin R.Open(60900); if not R.IsEmpty() then Error('rows'); end;
        [Test] procedure OtherTable() var M: Record Mine; begin M.K := 'X'; M.Insert(); end; }";

fn run_dep(publisher: &str, login: &str) -> Vec<(String, Outcome)> {
    outcomes_and_hits_of_published_apps(
        &[("dep", &[], &dep_login(login)), ("main", &["dep"], TESTS)],
        &[("dep", publisher)],
        "T",
    )
    .0
}

// Scenario s09: a partner dependency app's OnAfterLogin subscriber sets a group 1 filter in every session, the test session
// too (corpus PR 1405, G1sInstallSessionTests.al:100), and the runner does not raise it there when no app under test
// subscribes. A test that touches the table, through a Record or a RecordRef, is not run; one that does not runs.
#[test]
fn a_test_that_touches_a_dependency_login_subscribers_table_is_not_run() {
    let o = run_dep("Probe", SETS_FILTER);
    assert!(not_run(&o[0].1), "{o:?}");
    assert!(not_run(&o[1].1), "{o:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
}

// A neighbor that keeps main's result: Microsoft's login subscribers (publisher Microsoft) are the documented gap of
// `login.rs`, not changed here.
#[test]
fn a_microsoft_login_subscribers_table_keeps_mains_result() {
    let o = run_dep("Microsoft", SETS_FILTER);
    assert!(matches!(o[0].1, Outcome::Fail(_)), "{o:?}");
}
