//! Rows install code left that the runner does not know (al-runner issue 8115): install code after a stopped install
//! routine, a not-run install routine's table read through a RecordRef, and a dependency app's login subscriber.
use super::app_outcomes_tests::{outcomes_and_hits_of_apps, outcomes_and_hits_of_published_apps};
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

/// Whether `o` is not run for a reason that holds `why`.
fn not_run_for(o: &Outcome, why: &str) -> bool {
    matches!(o, Outcome::NotRun(r) if r.contains(why))
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

/// App B of the SingleInstance probes (review of al-runner PR 8119, probe n1b): `Keep` holds a number for the install
/// session; `InstB` (body `first`) runs before `InstB2`, which writes `Keep.Take()` to `Setup`. A test reports it.
fn app_b_keep(first: &str) -> String {
    format!(
        "table 60960 Setup {{ fields {{ field(1; K; Code[10]) {{ }} field(2; N; Integer) {{ }} }} }}
        table 60964 Mark {{ fields {{ field(1; K; Code[10]) {{ }} }} }}
        codeunit 60965 Keep {{ SingleInstance = true; var N: Integer;
            procedure Put(V: Integer) begin N := V; end; procedure Take(): Integer begin exit(N); end; }}
        codeunit 60961 InstB {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var I: Record Item; M: Record Mark; K: Codeunit Keep; begin {first} if M.Insert() then; end; }}
        codeunit 60963 InstB2 {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var S: Record Setup; K: Codeunit Keep; begin S.N := K.Take() + 7; if S.Insert() then; end; }}
        codeunit 60962 T {{ Subtype = Test;
            [Test] procedure Seen() var S: Record Setup; begin S.Get(); if S.N <> 7 then Error('B2 saw %1', S.N); end;
            [Test] procedure NoTable() begin end; }}"
    )
}

// Probe n1b: B's first install routine reads A's stopped table and puts a number into a SingleInstance codeunit. It is
// rolled back and refused, but the global stays: B's second routine, which reads it, is not run either.
#[test]
fn install_code_after_a_refused_routine_that_wrote_a_global_is_not_run() {
    let o = run_ab(&app_a(STOPS), &app_b_keep("K.Put(I.Count() + 100);"));
    assert!(
        not_run_for(&o[0].1, "can change a codeunit global or the session state"),
        "{o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// The neighbor: the refused routine's reach writes no global and no session state, so the routine after it runs (main's
// result for it).
#[test]
fn install_code_after_a_refused_routine_that_left_nothing_runs() {
    let o = run_ab(&app_a(STOPS), &app_b_keep("if I.IsEmpty() then;"));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// The same through B's Company Triggers subscriber in its install session: it reads A's stopped table and its reach
// writes a global, so the rest of B's install code (its per-database routine) is not run. The read is only in a session
// without GuiAllowed (the install session), so that the test session's run of the subscriber does not decide.
#[test]
fn install_code_after_a_refused_company_triggers_subscriber_is_not_run() {
    let b = app_b_keep("")
        .replace("table 60960 Setup {", "table 60960 Setup { DataPerCompany = false;")
        .replace("codeunit 60963 InstB2 { Subtype = Install;\n            trigger OnInstallAppPerCompany()", "codeunit 60963 InstB2 { Subtype = Install;\n            trigger OnInstallAppPerDatabase()")
        + "
        codeunit 60966 Open {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
            local procedure O() var I: Record Item; K: Codeunit Keep; begin if not GuiAllowed() then K.Put(I.Count() + 100); end; }";
    assert!(b.contains("OnInstallAppPerDatabase"));
    let o = run_ab(&app_a(STOPS), &b);
    assert!(
        not_run_for(&o[0].1, "can change a codeunit global or the session state"),
        "{o:?}"
    );
}

/// App B of probes n15 and n16: `Keep` is set by a subscriber (`subscriber`, an attribute and a procedure that reads A's
/// table) that runs when the test session opens the company; a test reports it.
fn app_b_open(subscriber: &str) -> String {
    format!(
        "codeunit 60965 Keep {{ SingleInstance = true; var N: Integer;
            procedure Put(V: Integer) begin N := V; end; procedure Take(): Integer begin exit(N); end; }}
        codeunit 60966 Open {{ {subscriber} local procedure O() var I: Record Item; K: Codeunit Keep; begin K.Put(I.Count() + 100); end; }}
        codeunit 60962 T {{ Subtype = Test;
            [Test] procedure KeepSaw() var K: Codeunit Keep; begin if K.Take() <> 102 then Error('open saw %1', K.Take()); end;
            [Test] procedure NoTable() begin end; }}"
    )
}

// Probe n15: B's Company Triggers subscriber runs when the test session opens the company and reads A's stopped table:
// the codeunit is not run.
#[test]
fn a_test_session_company_triggers_subscriber_that_reads_a_stopped_table_refuses_the_codeunit() {
    let b = app_b_open(
        "[EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]",
    );
    let o = run_ab(&app_a(STOPS), &b);
    assert!(not_run(&o[0].1), "{o:?}");
    assert!(not_run(&o[1].1), "{o:?}");
}

// The neighbor: A ran to its end, and the subscriber counts its two rows (main's result).
#[test]
fn a_test_session_company_triggers_subscriber_after_install_code_that_ran_runs() {
    let b = app_b_open(
        "[EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]",
    );
    let o = run_ab(&app_a(""), &b);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// Probe n16: the same through B's own "System Initialization".OnAfterLogin subscriber, which the test session's login runs.
#[test]
fn a_test_session_login_subscriber_that_reads_a_stopped_table_refuses_the_codeunit() {
    let b = app_b_open(
        "[EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]",
    );
    let o = outcomes_and_hits_of_apps(
        &[
            ("dep", &[], &dep_login("begin end;")),
            ("a", &["dep"], &app_a(STOPS)),
            ("b", &["a", "dep"], &b),
        ],
        "T",
    )
    .0;
    assert!(not_run(&o[0].1), "{o:?}");
    assert!(not_run(&o[1].1), "{o:?}");
}

/// A partner dependency app (publisher Probe) with a SingleInstance `Cache` its OnAfterLogin subscriber (body `login`)
/// may set, and the app under test's tests: one reads the cache, one counts `Other`, one touches nothing (probe n5).
fn run_dep_cache(login: &str) -> Vec<(String, Outcome)> {
    let dep = format!(
        "{}
        codeunit 60904 Cache {{ SingleInstance = true; var On: Boolean;
            procedure Put() begin On := true; end; procedure Take(): Boolean begin exit(On); end; }}
        codeunit 60906 Pub {{ procedure Ask(): Integer var N: Integer; begin N := 1; OnAsk(N); exit(N); end;
            [IntegrationEvent(false, false)] local procedure OnAsk(var N: Integer) begin end; }}
        codeunit 60907 Hook {{ SingleInstance = true; EventSubscriberInstance = Manual;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnAsk', '', false, false)]
            local procedure H(var N: Integer) begin N := 2; end; }}",
        dep_login(login)
    );
    let tests = "codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
        codeunit 60912 T { Subtype = Test;
            [Test] procedure CacheIsOn() var C: Codeunit Cache; begin if not C.Take() then Error('cache off'); end;
            [Test] procedure CountOther() var O: Record Other; begin O.No := 'A'; O.Insert(); if O.Count() <> 1 then Error('rows %1', O.Count()); end;
            [Test] procedure NoTable() begin end; }";
    outcomes_and_hits_of_published_apps(
        &[("dep", &[], &dep), ("main", &["dep"], tests)],
        &[("dep", "Probe")],
        "T",
    )
    .0
}

// Probe n5: the dependency subscriber only sets a SingleInstance global. The test that reads it is not run; the two that
// cannot see it keep main's result.
#[test]
fn only_a_test_that_reads_a_dependency_login_subscribers_global_is_not_run() {
    let o = run_dep_cache("var C: Codeunit Cache; begin C.Put(); end;");
    assert!(
        not_run_for(&o[0].1, "write a codeunit global this test codeunit reads"),
        "{o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
}

// Probe n11: the dependency subscriber binds a manual subscriber, which any event can reach: no test of the codeunit runs.
#[test]
fn a_dependency_login_subscriber_that_binds_refuses_every_test() {
    let o = run_dep_cache("var H: Codeunit Hook; begin if BindSubscription(H) then; end;");
    assert!(
        o.iter().all(|(_, x)| not_run_for(x, "bind a manual event subscriber")),
        "{o:?}"
    );
}

// A dependency subscriber whose reach is not known (a codeunit run by an id known only at run time) may leave anything:
// no test of the codeunit runs.
#[test]
fn a_dependency_login_subscriber_whose_reach_is_not_known_refuses_every_test() {
    let o = run_dep_cache("var Id: Integer; begin Id := 60904; Codeunit.Run(Id); end;");
    assert!(o.iter().all(|(_, x)| not_run_for(x, "is not known")), "{o:?}");
}

// A dependency subscriber that sets WorkDate: every test reads it (a posting date defaults to it), so none runs.
#[test]
fn a_dependency_login_subscriber_that_sets_workdate_refuses_every_test() {
    let o = run_dep_cache("begin WorkDate(20300101D); end;");
    assert!(o.iter().all(|(_, x)| not_run_for(x, "which this test reads")), "{o:?}");
}

// A dependency subscriber that writes through a RecordRef may write any table: a test that touches a table is not run, one
// that touches none keeps main's result.
#[test]
fn a_dependency_login_subscriber_that_writes_an_unknown_table_refuses_the_tests_with_tables() {
    let o = run_dep_cache("var R: RecordRef; begin R.Open(60900); R.Init(); if R.Insert() then; end;");
    assert!(not_run_for(&o[1].1, "a table not known before the run"), "{o:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
}

// Probe n8: an app under test with its own login subscriber (Login::Own) and a partner dependency's subscriber that sets a
// group 1 filter: a test that touches the dependency's table is not run, one that does not runs.
#[test]
fn a_dependency_login_subscriber_next_to_an_own_one_still_refuses_its_table() {
    let tests = "table 60910 Mine { fields { field(1; K; Code[10]) { } } }
        codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
        codeunit 60913 OwnLogin {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() var M: Record Mine; begin M.K := 'L'; if M.Insert() then; end; }
        codeunit 60912 T { Subtype = Test;
            [Test] procedure CountOther() var O: Record Other; begin O.No := 'A'; O.Insert(); O.No := 'B'; O.Blocked := true; O.Insert(); if O.Count() <> 1 then Error('rows %1', O.Count()); end;
            [Test] procedure NoTable() begin end; }";
    let o = outcomes_and_hits_of_published_apps(
        &[("dep", &[], &dep_login(SETS_FILTER)), ("main", &["dep"], tests)],
        &[("dep", "Probe")],
        "T",
    )
    .0;
    assert!(not_run(&o[0].1), "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// Probe n10: the test codeunit's OnRun reads A's stopped table through a RecordRef: the codeunit is not run.
#[test]
fn an_onrun_that_reads_a_stopped_table_refuses_the_codeunit() {
    let b = "codeunit 60962 T { Subtype = Test;
        trigger OnRun() var R: RecordRef; begin R.Open(60950); if R.IsEmpty() then; end;
        [Test] procedure NoTable() begin end; }";
    let o = run_ab(&app_a(STOPS), b);
    assert!(not_run(&o[0].1), "{o:?}");
}

// Probe n3_temp_only: A's stopped routine uses `Other` only as a temporary record, which adds no row to the table. B's
// install code that counts the table runs (main's result).
#[test]
fn install_code_that_reads_a_table_the_stopped_routine_holds_only_as_temporary_runs() {
    let a = app_a(STOPS).replace(
        "codeunit 60952",
        "table 60951 Other { fields { field(1; No; Code[20]) { } field(2; Blocked; Boolean) { } } }
        codeunit 60952",
    );
    let a = a.replace("T: Record Item temporary", "T: Record Other temporary");
    let b = "table 60960 Setup { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var O: Record Other; S: Record Setup; begin S.N := O.Count() + 50; if S.Insert() then; end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var S: Record Setup; begin S.Get(); if S.N <> 50 then Error('B saw %1', S.N); end; }";
    let o = run_ab(&a, b);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// Probe n2_write_only: B only inserts into A's stopped table: a duplicate key changes what its Insert does, so B is not run.
#[test]
fn install_code_that_only_writes_a_stopped_table_is_not_run() {
    let b = "table 60960 Setup { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var I: Record Item; S: Record Setup; begin I.No := 'X9'; if I.Insert() then; S.N := 7; if S.Insert() then; end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var S: Record Setup; begin S.Get(); if S.N <> 7 then Error('B saw %1', S.N); end; }";
    let o = run_ab(&app_a(STOPS), b);
    assert!(not_run(&o[0].1), "{o:?}");
}
