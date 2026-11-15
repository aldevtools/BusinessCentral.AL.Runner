//! A filter in filter group 1 across install sessions (al-runner issue 8071; pinned by
//! tests/al-language/install-upgrade/G1sInstallSessionTests.al, owner al-vm `group1_sessions.rs`).
use super::tests::outcomes_and_hits_of_apps;
use super::*;

/// System Application's login, as the install session raises it: "System Initialization Impl." subscribes to Company
/// Triggers OnCompanyOpenCompleted and raises "System Initialization".OnAfterLogin.
const DEP: &str = "codeunit 150 \"System Initialization\" { procedure Raise() begin OnAfterLogin(); end;
        [IntegrationEvent(false, false)] procedure OnAfterLogin() begin end; }
    codeunit 151 \"System Initialization Impl.\" {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
        local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }";

/// The reporter's app A (issue 8071): a table, a subscriber that sets a group 1 filter on it, and an empty install
/// codeunit; `event` is the subscriber's attribute.
fn app_a(event: &str, body: &str) -> String {
    format!(
        "table 60950 Item {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Blocked; Boolean) {{ }} }} }}
        codeunit 60951 Setter {{
            [EventSubscriber(ObjectType::Codeunit, {event}, '', false, false)]
            local procedure Login() {body} }}
        codeunit 60952 InstA {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}"
    )
}

const LOGIN: &str = "Codeunit::\"System Initialization\", 'OnAfterLogin'";
const SET_BY_RECORD: &str =
    "var I: Record Item; begin I.FilterGroup(1); I.SetRange(Blocked, false); I.FilterGroup(0); end;";

/// App B: an install codeunit that writes a row, a test that reads it, and a test that touches no table.
const APP_B: &str = "table 60960 Setup { fields { field(1; K; Code[10]) { } } }
    codeunit 60961 InstB { Subtype = Install;
        trigger OnInstallAppPerCompany() var S: Record Setup; begin if S.Insert() then; end; }
    codeunit 60962 T { Subtype = Test;
        [Test] procedure InstallRan() var S: Record Setup; begin S.Get(); end;
        [Test] procedure NoTable() begin end; }";

/// App B, whose install code reads A's table.
const APP_B_READS: &str = "table 60960 Setup { fields { field(1; K; Code[10]) { } } }
    codeunit 60961 InstB { Subtype = Install;
        trigger OnInstallAppPerCompany() var I: Record Item; S: Record Setup; begin if I.IsEmpty() then; if S.Insert() then; end; }
    codeunit 60962 T { Subtype = Test;
        [Test] procedure InstallRan() var S: Record Setup; begin S.Get(); end; }";

/// The outcomes and hits of B's tests, with the dependency app `DEP` and the apps under test A and B.
fn run(a: &str, b: &str) -> super::tests::OutcomesAndHits {
    outcomes_and_hits_of_apps(&[("dep", &[], DEP), ("a", &["dep"], a), ("b", &["a"], b)], "T")
}

fn outcome(a: &str, b: &str) -> Outcome {
    run(a, b).0[0].1.clone()
}

/// Asserts B's tests passed clean: no assumption marks (the behavior is pinned).
fn passes_clean(a: &str, b: &str) {
    let (o, hits) = run(a, b);
    assert!(o.iter().all(|(_, out)| *out == Outcome::Pass), "{o:?}");
    assert!(hits.iter().all(|(_, h)| h.is_empty()), "{hits:?}");
}

// The reporter's repro (issue 8071; G1sXaInstall.al:116-124, G1sInstallSessionTests.al:87-105): a group 1 filter A's
// OnAfterLogin subscriber set in A's install session is not there in B's install session, where the subscriber sets it
// again. B's install code runs, and its tests pass clean.
#[test]
fn a_group1_filter_of_one_install_session_does_not_reach_the_next() {
    passes_clean(&app_a(LOGIN, SET_BY_RECORD), APP_B);
}

// Cell b (G1sInstallSessionTests.al:55-63; G1sXaInstall.al:151-153): a filter A's install routine left set, read with a new
// record in B's install session: both rows, no filter.
#[test]
fn the_next_install_session_reads_the_table_without_the_filter() {
    let a = "table 60950 Item { fields { field(1; No; Code[20]) { } field(2; Qty; Integer) { } } }
        codeunit 60952 InstA { Subtype = Install;
            trigger OnInstallAppPerCompany() var I: Record Item; S: Record Item;
            begin S.No := 'R1'; S.Qty := 1; S.Insert(); S.No := 'R2'; S.Qty := 5; S.Insert();
                I.FilterGroup(1); I.SetFilter(Qty, '>=3'); I.FilterGroup(0); end; }";
    let b = "table 60960 Log { fields { field(1; K; Code[10]) { } field(2; Seen; Text[100]) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var I: Record Item; L: Record Log; G1: Text;
            begin I.FilterGroup(1); G1 := I.GetFilters(); I.FilterGroup(0); L.Seen := StrSubstNo('count=%1;g1=%2', I.Count(), G1); L.Insert(); end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var L: Record Log; begin L.Get(); if L.Seen <> 'count=2;g1=' then Error('%1', L.Seen); end; }";
    passes_clean(a, b);
}

// A dependency app's install session (the install baseline, `baseline_roots.rs`): a filter its install routine left set
// is not there in the app under test's install session (G1sInstallSessionTests.al:65-73). The tests pass clean on the
// baseline.
#[test]
fn a_dependency_apps_install_session_ends_its_group1_filter() {
    let dep = "table 60940 Feature { fields { field(1; K; Code[10]) { } field(2; On; Boolean) { } } }
        codeunit 60941 DepInst { Subtype = Install;
            trigger OnInstallAppPerCompany() var F: Record Feature; G: Record Feature;
            begin F.K := 'A'; F.Insert(); F.K := 'B'; F.On := true; F.Insert(); G.FilterGroup(1); G.SetRange(On, true); G.FilterGroup(0); end; }";
    let main = "table 60960 Log { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var F: Record Feature; L: Record Log; begin L.N := F.Count(); L.Insert(); end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var L: Record Log; begin L.Get(); if L.N <> 2 then Error('%1', L.N); end;
            [Test] procedure NoTable() begin end; }";
    let o = on_dependency_baseline(dep, main);
    assert_eq!(o, [(Outcome::Pass, false), (Outcome::Pass, false)]);
}

fn refused_by_group1(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(r) if r.contains("filter group 1"))
}

// Out of the measured shape, main's refusal stays: a group 1 filter a Company Triggers subscriber set (not an
// OnAfterLogin subscriber).
#[test]
fn a_filter_of_a_company_triggers_subscriber_keeps_mains_refusal() {
    let a = app_a(
        "Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted'",
        SET_BY_RECORD,
    );
    assert!(refused_by_group1(&outcome(&a, APP_B)));
}

// Out of the measured shape, main's refusal stays: a group 1 filter an "Extension Triggers" install subscriber set (not
// the install codeunit's trigger).
#[test]
fn a_filter_of_an_extension_triggers_subscriber_keeps_mains_refusal() {
    let a = app_a(
        "Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany'",
        SET_BY_RECORD,
    );
    let o = outcome(&a, APP_B_READS);
    assert!(refused_by_group1(&o), "{o:?}");
}

// Out of the measured shape, main's refusal stays: a group 1 filter set in the per-database part of A's install session.
#[test]
fn a_filter_of_the_per_database_part_keeps_mains_refusal() {
    let a = "table 60950 Item { DataPerCompany = false; fields { field(1; No; Code[20]) { } field(2; Blocked; Boolean) { } } }
        codeunit 60952 InstA { Subtype = Install;
            trigger OnInstallAppPerDatabase() var I: Record Item; begin I.FilterGroup(1); I.SetRange(Blocked, false); I.FilterGroup(0); end; }";
    assert!(refused_by_group1(&outcome(a, APP_B_READS)));
}

// Out of the measured shape, main's refusal stays: a group 1 filter an OnAfterLogin subscriber set through a FieldRef.
#[test]
fn a_filter_set_through_a_fieldref_keeps_mains_refusal() {
    let a = app_a(
        LOGIN,
        "var R: RecordRef; F: FieldRef; begin R.Open(60950); R.FilterGroup(1); F := R.Field(2); F.SetRange(false); end;",
    );
    assert!(refused_by_group1(&outcome(&a, APP_B)));
}

/// App B, whose install code stores the count of A's table `Other`, and a test that expects `n`.
fn app_b_counts(n: i32) -> String {
    format!(
        "table 60960 Setup {{ fields {{ field(1; K; Code[10]) {{ }} field(2; N; Integer) {{ }} }} }}
        codeunit 60961 InstB {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var O: Record Other; S: Record Setup; begin S.N := O.Count(); if S.Insert() then; end; }}
        codeunit 60962 T {{ Subtype = Test;
            [Test] procedure Seen() var S: Record Setup; begin S.Get(); if S.N <> {n} then Error('%1', S.N); end; }}"
    )
}

/// App A: table `Other`, whose install trigger writes two rows (one blocked) and then runs `tail`; `more` adds objects.
fn app_a_other(tail: &str, more: &str) -> String {
    format!(
        "table 60950 Other {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Blocked; Boolean) {{ }} }} }}
        codeunit 60952 InstA {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var O: Record Other; G: Record Other; F: Record Other; H: Codeunit Helper; L: Record Log;
            begin O.No := 'A'; O.Blocked := true; O.Insert(); O.No := 'B'; O.Blocked := false; O.Insert(); {tail} end; }}
        codeunit 60953 Helper {{ procedure SetIt() var G: Record Other; begin G.FilterGroup(1); G.SetRange(Blocked, true); G.FilterGroup(0); end;
            [IntegrationEvent(false, false)] procedure OnSomething() begin end; }}
        table 60954 Log {{ fields {{ field(1; No; Code[20]) {{ }} }}
            trigger OnInsert() var H: Codeunit Helper; begin H.SetIt(); end; }}
        {more}"
    )
}

/// In A's install trigger: the filter set in the trigger's own body (G1sXaInstall.al:151-153).
const SET_IN_TRIGGER: &str = "G.FilterGroup(1); G.SetRange(Blocked, true); G.FilterGroup(0);";

// The probe measured a setter routine that ran to its end (G1sXaInstall.al:131-155). A's install trigger sets the filter
// and then stops (a new record of the table in the same session, which main refuses): its rows are rolled back, and its
// filter keeps main's refusal in B's install session.
#[test]
fn a_filter_of_a_setter_that_stops_keeps_mains_refusal() {
    let a = app_a_other(&format!("{SET_IN_TRIGGER} if F.IsEmpty() then;"), "");
    let o = outcome(&a, &app_b_counts(0));
    assert!(refused_by_group1(&o), "{o:?}");
}

// The control: the same setter run to its end. B's install code counts both rows, and its test passes clean.
#[test]
fn a_filter_of_a_setter_that_ran_to_its_end_ends() {
    passes_clean(&app_a_other(SET_IN_TRIGGER, ""), &app_b_counts(2));
}

// An install routine that reads A's table after its filter ended and writes nothing: it runs, and the tests pass clean.
#[test]
fn a_routine_that_reads_the_ended_table_runs() {
    let b = "table 60960 Setup { fields { field(1; K; Code[10]) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var O: Record Other; S: Record Setup; begin O.SetRange(Blocked, false); if O.IsEmpty() then S.Insert(); end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure SetupEmpty() var S: Record Setup; begin if not S.IsEmpty() then Error('B wrote'); end;
            [Test] procedure NoTable() begin end; }";
    passes_clean(&app_a_other(SET_IN_TRIGGER, ""), b);
}

// Out of the measured shape (the probe sets the filter in the trigger's body), main's refusal stays: a procedure the
// install trigger calls sets it.
#[test]
fn a_filter_of_a_procedure_the_trigger_calls_keeps_mains_refusal() {
    let o = outcome(&app_a_other("H.SetIt();", ""), &app_b_counts(2));
    assert!(refused_by_group1(&o), "{o:?}");
}

// Out of the measured shape, main's refusal stays: a table's OnInsert trigger that the install trigger runs sets it.
#[test]
fn a_filter_of_a_table_trigger_keeps_mains_refusal() {
    let o = outcome(&app_a_other("L.No := 'X'; L.Insert(true);", ""), &app_b_counts(2));
    assert!(refused_by_group1(&o), "{o:?}");
}

// Out of the measured shape, main's refusal stays: a subscriber to the app's own event, raised by the install trigger.
#[test]
fn a_filter_of_an_own_event_subscriber_keeps_mains_refusal() {
    let sub = format!(
        "codeunit 60955 Sub {{ [EventSubscriber(ObjectType::Codeunit, Codeunit::Helper, 'OnSomething', '', false, false)]
            local procedure S() {SET_OTHER_BODY} }}"
    );
    let o = outcome(&app_a_other("H.OnSomething();", &sub), &app_b_counts(2));
    assert!(refused_by_group1(&o), "{o:?}");
}

const SET_OTHER_BODY: &str =
    "var G: Record Other; begin G.FilterGroup(1); G.SetRange(Blocked, true); G.FilterGroup(0); end;";

// Out of the measured shape (the probe sets the filter in the OnAfterLogin subscriber's body, G1sXaInstall.al:121-123), main's
// refusal stays: a procedure the subscriber calls sets it (the reporter's layout otherwise).
#[test]
fn a_filter_of_a_procedure_the_login_subscriber_calls_keeps_mains_refusal() {
    let a = app_a(LOGIN, "var H: Codeunit Helper; begin H.SetIt(); end;")
        + "codeunit 60953 Helper { procedure SetIt() var I: Record Item; begin I.FilterGroup(1); I.SetRange(Blocked, false); I.FilterGroup(0); end; }";
    let o = outcome(&a, APP_B);
    assert!(refused_by_group1(&o), "{o:?}");
}

/// The tests of `main` (codeunit `T`) on the install baseline of dependency app `dep`, as the baseline cache keeps it:
/// the outcomes and, per test, whether it carries an assumption mark.
fn on_dependency_baseline(dep: &str, main: &str) -> Vec<(Outcome, bool)> {
    let (o, hits) = super::app_outcomes_tests::outcomes_on_dependency_baseline(dep, main);
    (o.into_iter())
        .map(|(n, out)| {
            let hit = hits.iter().any(|(m, h)| *m == n && !h.is_empty());
            (out, hit)
        })
        .collect()
}

// Cell d (G1sInstallSessionTests.al:75-85): the test session reads the table whose filter a dependency app's install
// session ended, with no install code after it, on the install baseline.
#[test]
fn a_test_reads_the_ended_table_on_the_baseline() {
    let dep = "table 60940 Feature { fields { field(1; K; Code[10]) { } field(2; On; Boolean) { } } }
        codeunit 60941 DepInst { Subtype = Install;
            trigger OnInstallAppPerCompany() var F: Record Feature; G: Record Feature;
            begin F.K := 'A'; F.Insert(); F.K := 'B'; F.On := true; F.Insert(); G.FilterGroup(1); G.SetRange(On, true); G.FilterGroup(0); end; }";
    let main = "codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var F: Record Feature; begin if F.Count() <> 2 then Error('%1', F.Count()); end;
            [Test] procedure NoTable() begin end; }";
    let o = on_dependency_baseline(dep, main);
    assert_eq!(o, [(Outcome::Pass, false), (Outcome::Pass, false)]);
}

// A dependency app's install routine (the install baseline, `baseline_roots.rs`) that sets the filter and then stops keeps
// main's refusal: the app under test's install code that reads the table is not run, nor the test that reads its result.
#[test]
fn a_filter_of_a_dependency_setter_that_stops_keeps_mains_refusal() {
    let dep = "table 60940 Feature { fields { field(1; K; Code[10]) { } field(2; On; Boolean) { } } }
        codeunit 60941 DepInst { Subtype = Install;
            trigger OnInstallAppPerCompany() var F: Record Feature; G: Record Feature; N: Record Feature;
            begin F.K := 'A'; F.Insert(); F.K := 'B'; F.On := true; F.Insert(); G.FilterGroup(1); G.SetRange(On, true); G.FilterGroup(0);
                if N.IsEmpty() then; end; }";
    let main = "table 60960 Log { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
        codeunit 60961 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var F: Record Feature; L: Record Log; begin L.N := F.Count(); L.Insert(); end; }
        codeunit 60962 T { Subtype = Test;
            [Test] procedure Seen() var L: Record Log; begin L.Get(); if L.N <> 0 then Error('%1', L.N); end; }";
    let o = on_dependency_baseline(dep, main);
    assert!(
        matches!(&o[0], (Outcome::NotRun(r), false) if r.contains("filter group 1")),
        "{o:?}"
    );
}
