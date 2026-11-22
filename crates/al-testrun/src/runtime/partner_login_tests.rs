//! The partner dependency apps' login subscribers in the test session (al-runner issue 8115, scenario s09;
//! `partner_login.rs`; pinned: G1sInstallSessionTests.al:98, :103 and PlLoginSingleTests.al:54-128).
use super::app_outcomes_tests::{OutcomesAndHits, outcomes_and_hits_with};
use super::*;

/// System Application's login code (app `dep`, publisher Microsoft): "System Initialization Impl." raises
/// "System Initialization".OnAfterLogin from Company Triggers OnCompanyOpenCompleted; `isolated` is the event's third
/// IntegrationEvent argument.
fn sys(isolated: bool) -> String {
    format!(
        "codeunit 150 \"System Initialization\" {{ procedure Raise() begin OnAfterLogin(); end;
            [IntegrationEvent(false, false, {isolated})] procedure OnAfterLogin() begin end; }}
        codeunit 151 \"System Initialization Impl.\" {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
            local procedure InitCompany() var S: Codeunit \"System Initialization\"; begin S.Raise(); end; }}
        codeunit 152 \"Sys Cache\" {{ SingleInstance = true; var N: Integer; procedure Set() begin N := 1; end;
            procedure Bump() begin N += 1; end; procedure Get(): Integer begin exit(N); end; }}"
    )
}

/// A partner dependency app (`dep2`): table `Other`, a SingleInstance `Cache` of the measured shape (two Integers, a list of
/// Text and a Text, PlXaLoginSingles.al:12-27), SingleInstance codeunits of other shapes: `Counter` (one Integer), `Wider`
/// (the measured globals and one Text more), `Flags` (a Boolean and a record global) and `Env Memo` (two Booleans, filled
/// on first use as Environment Information Impl. keeps IsSaaS),
/// a manual subscriber `Hook`, and the OnAfterLogin subscriber with body `login`.
fn partner(login: &str) -> String {
    format!(
        "table 60900 Other {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Blocked; Boolean) {{ }} }} }}
        codeunit 60904 Cache {{ SingleInstance = true; var Cnt: Integer; Log: List of [Text]; Txt: Text; Calls: Integer;
            procedure Set(N: Integer) begin Cnt := N; end; procedure Bump() begin Cnt += 1; end;
            procedure GetCount(): Integer begin exit(Cnt); end; procedure AddTxt(T: Text) begin Txt += T; end;
            procedure SetTxt(T: Text) begin Txt := T; end; procedure GetTxt(): Text begin exit(Txt); end;
            procedure AddLog(T: Text) begin Log.Add(T); end; procedure LogCount(): Integer begin exit(Log.Count()); end; }}
        codeunit 60909 Counter {{ SingleInstance = true; var N: Integer; procedure Put() begin N := 1; end;
            procedure Get(): Integer begin exit(N); end; }}
        codeunit 60903 Wider {{ SingleInstance = true; var Cnt: Integer; Log: List of [Text]; Txt: Text; Calls: Integer; More: Text;
            procedure Put() begin Cnt := 1; end; procedure Get(): Integer begin exit(Cnt); end; }}
        codeunit 60908 Flags {{ SingleInstance = true; var On: Boolean; Row: Record Other;
            procedure Put() begin On := true; end; procedure Sort() begin Row.SetCurrentKey(Blocked); end;
            procedure IsOn(): Boolean begin exit(On); end; }}
        codeunit 60905 \"Env Memo\" {{ SingleInstance = true; var Known: Boolean; SaaS: Boolean;
            procedure IsSaaS(): Boolean begin if not Known then begin SaaS := false; Known := true; end; exit(SaaS); end; }}
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
        [Test] procedure CacheIsZero() var C: Codeunit Cache; begin if C.GetCount() <> 0 then Error('count %1', C.GetCount()); end;
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

/// No test relied on an assumption.
fn clean(hits: &[(String, Vec<&'static str>)]) -> bool {
    hits.iter().all(|(_, h)| h.is_empty())
}

// Scenario s09: the partner subscriber's group 1 filter is in the test session (G1sInstallSessionTests.al:103). On main the
// test counted 2 rows and failed; the filter another record set is now there, and the runtime refuses a record without it
// (al-vm group1_sessions.rs `Session::group1_use`). The tests that do not touch the table pass clean.
#[test]
fn a_partner_login_subscribers_group1_filter_reaches_the_test_session() {
    let (o, hits) = run(SETS_FILTER);
    assert!(not_run_for(&o[0].1, "filter group 1"), "{o:?}");
    for k in 1..4 {
        assert_eq!(o[k].1, Outcome::Pass, "{o:?}");
    }
    assert!(clean(&hits), "{hits:?}");
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
    assert!(clean(&hits), "{hits:?}");
}

// A SingleInstance global of the measured shape the subscriber sets is in the test session (PlLoginSingleTests.al:54-74):
// the test that reads it sees 10, clean.
#[test]
fn a_test_that_reads_a_single_instance_global_the_partner_subscriber_set_sees_it() {
    let (o, hits) = run("var C: Codeunit Cache; begin C.Set(10); C.AddLog('x'); C.AddTxt('L'); end;");
    assert!(matches!(&o[1].1, Outcome::Fail(m) if m.contains("count 10")), "{o:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[3].1, Outcome::Pass, "{o:?}");
    assert!(clean(&hits), "{hits:?}");
}

/// Test codeunits after a login that changes a SingleInstance codeunit outside the measured shape: the first test of `T`
/// reads one of its globals through `read` (a call of the codeunit `single`), its second test and `U` read none.
fn single_reader(single: &str, read: &str) -> String {
    format!(
        "table 60910 Mine {{ fields {{ field(1; K; Code[10]) {{ }} }} }}
        codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60912 T {{ Subtype = Test;
            [Test] procedure Reads() var S: Codeunit {single}; begin if S.{read} then; end;
            [Test] procedure ReadsNothing() begin end; }}
        codeunit 60915 U {{ Subtype = Test;
            [Test] procedure OtherTable() var M: Record Mine; begin M.K := 'X'; M.Insert(); end;
            [Test] procedure NoTable() begin end; }}"
    )
}

/// The test that reads `single` is not run at its read; the test of the same codeunit and the codeunit that read none
/// of its globals run (al-runner issue 8239: they cannot see what BC leaves there).
fn assert_only_the_reader_is_refused(login: &str, single: &str, read: &str, name: &str) {
    let tests = single_reader(single, read);
    let (o, _) = run_codeunit("Probe", login, &tests, false, TestIsolation::Codeunit, "T");
    assert!(
        not_run_for(&o[0].1, &format!("a global of SingleInstance codeunit {name}")),
        "{login}: {o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{login}: {o:?}");
    let (o, _) = run_codeunit("Probe", login, &tests, false, TestIsolation::Codeunit, "U");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{login}: {o:?}");
}

// A SingleInstance codeunit with a global of a type the probe did not measure (a Boolean, a record): its reads refuse.
#[test]
fn a_partner_subscriber_that_sets_an_unmeasured_single_instance_refuses_its_reads() {
    for login in [
        "var F: Codeunit Flags; begin F.Put(); end;",
        "var F: Codeunit Flags; begin F.Sort(); end;",
    ] {
        assert_only_the_reader_is_refused(login, "Flags", "IsOn()", "Flags");
    }
}

const MEMO_KEPT: &str = "partner-login-memo-kept";

/// The assumption entries test `test` passed through (`hits` lists only the tests that passed through one).
fn marks<'a>(hits: &'a [(String, Vec<&'static str>)], test: &str) -> &'a [&'static str] {
    hits.iter().find(|(n, _)| n == test).map_or(&[], |(_, h)| h.as_slice())
}

/// The tests of `single_reader` after `login` run, and only the reader passes through the assumption entry
/// `partner-login-memo-kept` (corpus PR 1490).
fn assert_the_reader_runs_flagged(login: &str, single: &str, read: &str) {
    let tests = single_reader(single, read);
    let (o, hits) = run_codeunit("Probe", login, &tests, false, TestIsolation::Codeunit, "T");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{login}: {o:?}");
    assert_eq!(marks(&hits, "Reads"), [MEMO_KEPT], "{login}: {hits:?}");
    assert!(marks(&hits, "ReadsNothing").is_empty(), "{login}: {hits:?}");
}

// Globals of the measured types (Boolean, Integer, Text, List of [Text]; P2XaLoginMemos.al codeunits 98490-98494) but not
// the pinned set (one Integer only; the pinned set and one Text more): the reader sees what the login left, flagged.
#[test]
fn a_partner_subscriber_that_sets_a_single_instance_of_measured_types_keeps_it_flagged() {
    for (login, name) in [
        ("var C: Codeunit Counter; begin C.Put(); end;", "Counter"),
        ("var W: Codeunit Wider; begin W.Put(); end;", "Wider"),
    ] {
        assert_the_reader_runs_flagged(login, name, "Get() = 1");
    }
}

// A SingleInstance codeunit of a partner app the subscriber's app depends on (`Counter` of `dep2`, changed by the
// subscriber of `dep3`; P2A_03, P2A_04): the reader sees what the login left, flagged.
#[test]
fn a_partner_subscriber_that_sets_a_single_instance_of_another_app_keeps_it_flagged() {
    let login3 = "var C: Codeunit Counter; begin C.Put(); end;";
    let (o, hits) = run_two_partner_apps("begin end;", login3, "", &single_reader("Counter", "Get() = 1"));
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert_eq!(marks(&hits, "Reads"), [MEMO_KEPT], "{hits:?}");
    assert!(marks(&hits, "ReadsNothing").is_empty(), "{hits:?}");
}

// A neighbor that refuses the read: a memo of measured types (one Integer) of an app published by Microsoft at an id other
// than 3702 and 3703 (`Sys Cache`, 152). The probe measured Microsoft memos only at 3702 and 3703 (P2A_06, P2A_11).
#[test]
fn a_memo_of_a_microsoft_app_other_than_3702_and_3703_refuses_its_reads() {
    let login = "var S: Codeunit \"Sys Cache\"; begin S.Set(); end;";
    let tests = single_reader("\"Sys Cache\"", "Get() = 1");
    let (o, _) = run_codeunit("Probe", login, &tests, false, TestIsolation::Codeunit, "T");
    assert!(
        not_run_for(
            &o[0].1,
            "it is a codeunit of an app published by Microsoft other than 3702 and 3703"
        ),
        "{o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

/// The app under test of al-runner issue 8239's invented repro: a table whose OnInsert reads `Env Memo`, a test that
/// inserts a row with Insert(true), one that asks the memo itself, one that reads nothing.
const MEMO_TESTS: &str = "codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
    table 60975 \"PX Row\" { fields { field(1; No; Code[20]) { } field(2; SaaS; Boolean) { } field(3; Ran; Boolean) { } }
        keys { key(PK; No) { } }
        trigger OnInsert() var M: Codeunit \"Env Memo\"; begin SaaS := M.IsSaaS(); Ran := true; end; }
    codeunit 60970 \"PX Env Test\" { Subtype = Test;
        [Test] procedure InsertReadsTheMemo() var R: Record \"PX Row\"; begin R.No := 'PX1'; R.Insert(true); R.Get('PX1');
            if R.SaaS or not R.Ran then Error('saas %1 ran %2', R.SaaS, R.Ran); end;
        [Test] procedure IsSaaSAfterLogin() var M: Codeunit \"Env Memo\"; begin if M.IsSaaS() then Error('saas'); end;
        [Test] procedure ReadsNothing() begin end; }";

// Al-runner issue 8239, the invented repro: the partner login calls a memo of its own app (`Env Memo`, as Environment
// Information Impl. keeps IsSaaS); OnInsert of Insert(true) and the test read what the login left (P2A_02, P2A_07), the
// test that reads nothing runs clean (P2A_01). On main both readers were not run.
#[test]
fn the_issue_8239_repro_reads_what_the_login_left_flagged() {
    let login = "var M: Codeunit \"Env Memo\"; begin if M.IsSaaS() then; end;";
    let (o, hits) = run_codeunit(
        "Probe",
        login,
        MEMO_TESTS,
        false,
        TestIsolation::Codeunit,
        "PX Env Test",
    );
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert_eq!(marks(&hits, "InsertReadsTheMemo"), [MEMO_KEPT], "{hits:?}");
    assert_eq!(marks(&hits, "IsSaaSAfterLogin"), [MEMO_KEPT], "{hits:?}");
    assert!(marks(&hits, "ReadsNothing").is_empty(), "{hits:?}");
}

/// A Microsoft stand-in (`dep`, codeunits `id`) with a memo of a Boolean and a Decimal, `Dec Memo`; the partner login fills
/// it, test `T` reads it.
fn run_dec_memo(publisher: &str, id: u32) -> OutcomesAndHits {
    let memo = format!(
        "codeunit {id} \"Dec Memo\" {{ SingleInstance = true; var Known: Boolean; Rate: Decimal;
            procedure Get(): Decimal begin if not Known then begin Rate := 1.5; Known := true; end; exit(Rate); end; }}"
    );
    let tests = "codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
        codeunit 60912 T { Subtype = Test;
            [Test] procedure Reads() var M: Codeunit \"Dec Memo\"; begin if M.Get() <> 1.5 then Error('rate'); end;
            [Test] procedure ReadsNothing() begin end; }";
    let login = "var M: Codeunit \"Dec Memo\"; begin if M.Get() = 0 then; end;";
    let (sys_src, partner_src) = match publisher {
        "Microsoft" => (format!("{}\n{memo}", sys(false)), partner(login)),
        _ => (sys(false), format!("{}\n{memo}", partner(login))),
    };
    outcomes_and_hits_with(
        &[
            ("dep", &[], &sys_src),
            ("dep2", &["dep"], &partner_src),
            ("main", &["dep2"], tests),
        ],
        &[("dep", "Microsoft"), ("dep2", "Probe")],
        "T",
        TestIsolation::Codeunit,
    )
}

// Codeunits 3702 and 3703 of an app published by Microsoft (Environment Information Impl. and Server Setting Impl.,
// P2A_06, P2A_11: filled by IsSaaS() in the login) keep what the login left whatever their globals, flagged.
#[test]
fn microsofts_codeunit_3703_keeps_what_the_login_left_flagged() {
    for id in [3702, 3703] {
        let (o, hits) = run_dec_memo("Microsoft", id);
        assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{id}: {o:?}");
        assert_eq!(marks(&hits, "Reads"), [MEMO_KEPT], "{id}: {hits:?}");
    }
}

// Neighbors that refuse the read: the same memo at another id of the Microsoft app (only 3702 and 3703 are measured), and
// at id 3703 of the partner app (a Decimal global, a type no probe measured).
#[test]
fn a_memo_with_a_decimal_global_refuses_its_reads() {
    let microsoft = "it is a codeunit of an app published by Microsoft other than 3702 and 3703";
    let decimal = "its global Rate is of a type other than";
    for (publisher, id, what) in [("Microsoft", 154, microsoft), ("Probe", 3703, decimal)] {
        let (o, _) = run_dec_memo(publisher, id);
        let why = format!(
            "Dec Memo: OnAfterLogin raised to the login subscribers of the partner dependency apps changed it before the first test, and {what}"
        );
        assert!(not_run_for(&o[0].1, &why), "{publisher} {id}: {o:?}");
        assert_eq!(o[1].1, Outcome::Pass, "{publisher} {id}: {o:?}");
    }
}

/// A partner app `dep3` that depends on `dep2` with an OnAfterLogin subscriber of body `login3`, and `extra` objects;
/// the app under test depends on both.
fn run_two_partner_apps(login: &str, login3: &str, extra: &str, tests: &str) -> OutcomesAndHits {
    let third = format!(
        "codeunit 60980 DepLogin3 {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() {login3} }}
        {extra}"
    );
    outcomes_and_hits_with(
        &[
            ("dep", &[], &sys(false)),
            ("dep2", &["dep"], &partner(login)),
            ("dep3", &["dep2"], &third),
            ("main", &["dep3"], tests),
        ],
        &[("dep", "Microsoft"), ("dep2", "Probe"), ("dep3", "Probe3")],
        "T",
        TestIsolation::Codeunit,
    )
}

/// Adds one to `Cache`: every run is a change.
const CACHE_BUMP: &str = "var C: Codeunit Cache; begin C.Bump(); end;";

/// A reader of `Cache`'s count and text after the subscribers of `dep2` (`login`) and `dep3` (`login3`) ran.
fn run_cache_of_two_apps(login: &str, login3: &str, count: i32, txt: &str) -> OutcomesAndHits {
    let tests = format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60912 T {{ Subtype = Test;
            [Test] procedure Reads() var C: Codeunit Cache; begin
                if C.GetCount() <> {count} then Error('count %1', C.GetCount());
                if C.GetTxt() <> '{txt}' then Error('txt %1', C.GetTxt()); end;
            [Test] procedure ReadsNothing() begin end; }}"
    );
    run_two_partner_apps(login, login3, "", &tests)
}

// Two subscribers of different apps change other globals of the same memo, one of them of the memo's own app (P2A_05: XApp
// A sets FromA, XApp B sets FromB): both changes are there, flagged. The memo has the pinned globals (`Cache`), but the pin
// has one subscriber.
#[test]
fn two_subscribers_of_different_apps_that_change_other_globals_of_one_memo_keep_both_changes_flagged() {
    let set = "var C: Codeunit Cache; begin C.Set(1); end;";
    let add = "var C: Codeunit Cache; begin C.AddTxt('B'); end;";
    let (o, hits) = run_cache_of_two_apps(set, add, 1, "B");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert_eq!(marks(&hits, "Reads"), [MEMO_KEPT], "{hits:?}");
    assert!(marks(&hits, "ReadsNothing").is_empty(), "{hits:?}");
}

// Two subscribers of different apps that both change the same global of a memo: what is left can depend on their order,
// which no probe measured for OnAfterLogin, so its reads refuse. That holds for P2A_05's own counter cell too (both add one
// to `Cnt`), and for two assignments that do not commute (`Txt` set to 'A', then to 'B').
#[test]
fn two_subscribers_that_change_the_same_global_of_one_memo_refuse_its_reads() {
    let set_a = "var C: Codeunit Cache; begin C.SetTxt('A'); end;";
    let set_b = "var C: Codeunit Cache; begin C.SetTxt('B'); end;";
    for (login, login3, count, txt, global) in [(CACHE_BUMP, CACHE_BUMP, 2, "", "Cnt"), (set_a, set_b, 0, "B", "Txt")] {
        let (o, hits) = run_cache_of_two_apps(login, login3, count, txt);
        let why = format!(
            "two subscribers changed its global {global}: the order of two subscribers whose changes do not commute is not measured"
        );
        assert!(not_run_for(&o[0].1, &why), "{login3}: {o:?}");
        assert_eq!(o[1].1, Outcome::Pass, "{login3}: {o:?}");
        assert!(marks(&hits, "ReadsNothing").is_empty(), "{hits:?}");
    }
}

// Two subscribers of the same app change the same memo: not measured, its reads refuse.
#[test]
fn two_subscribers_of_the_same_app_that_change_one_memo_refuse_its_reads() {
    let second = format!(
        "codeunit 60981 DepLogin3b {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() {CACHE_BUMP} }}"
    );
    let tests = single_reader("Cache", "GetCount() = 2");
    let (o, _) = run_two_partner_apps("begin end;", CACHE_BUMP, &second, &tests);
    assert!(
        not_run_for(&o[0].1, "two subscribers of the same app changed it"),
        "{o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// Neighbors of P2A_05 (two subscribers, one of them of the memo's own app): a third subscriber, and two subscribers of
// apps other than the memo's (`Sys Cache` of `dep`, changed by `dep2` and `dep3`). Not measured, their reads refuse.
#[test]
fn a_memo_changed_by_three_subscribers_or_by_two_of_other_apps_refuses_its_reads() {
    let second = format!(
        "codeunit 60981 DepLogin3b {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() {CACHE_BUMP} }}"
    );
    let tests = single_reader("Cache", "GetCount() = 3");
    let (o, _) = run_two_partner_apps(CACHE_BUMP, CACHE_BUMP, &second, &tests);
    assert!(not_run_for(&o[0].1, "more than two subscribers changed it"), "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    let set = "var C: Codeunit \"Sys Cache\"; begin C.Bump(); end;";
    let tests = single_reader("\"Sys Cache\"", "Get() = 2");
    let (o, _) = run_two_partner_apps(set, set, "", &tests);
    assert!(
        not_run_for(&o[0].1, "two subscribers changed it, neither of them of its own app"),
        "{o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// A memo the login changes through an event whose subscriber is in an app the subscriber's app does not depend on (`dep3`
// subscribes to `dep2`'s Pub.OnAsk): not measured, its reads refuse; so does a memo of the app under test changed the
// same way.
#[test]
fn a_memo_of_an_app_the_subscribers_app_does_not_depend_on_refuses_its_reads() {
    let ask = "var P: Codeunit Pub; begin if P.Ask() = 0 then; end;";
    let memo = |id: u32| {
        format!(
            "codeunit {id} \"Far Memo\" {{ SingleInstance = true; var N: Integer; procedure Put() begin N := 1; end;
                procedure Get(): Integer begin exit(N); end; }}
            codeunit {} FarSub {{
                [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnAsk', '', false, false)]
                local procedure H(var N: Integer) var M: Codeunit \"Far Memo\"; begin M.Put(); end; }}",
            id + 1
        )
    };
    let tests = single_reader("\"Far Memo\"", "Get() = 1");
    let (o, _) = run_two_partner_apps(ask, "begin end;", &memo(60982), &tests);
    assert!(
        not_run_for(
            &o[0].1,
            "its app is neither the subscriber's app nor an app the subscriber's app depends on"
        ),
        "{o:?}"
    );
    let tests = format!("{tests}\n{}", memo(60990));
    let (o, _) = run_two_partner_apps(ask, "begin end;", "", &tests);
    assert!(not_run_for(&o[0].1, "it is a codeunit of an app under test"), "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

/// Test codeunit `C` after a login that sets `Flags` (outside the measured shape): `objects` copy it into `Mine`, a
/// SingleInstance codeunit of the app under test, which `MineIs10` reads; `Nothing` reads nothing.
fn run_copied_flags(objects: &str) -> OutcomesAndHits {
    let tests = format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60950 Mine {{ SingleInstance = true; var V: Integer;
            procedure Put(N: Integer) begin V := N; end; procedure Take(): Integer begin exit(V); end; }}
        codeunit 60953 C {{ Subtype = Test;
            [Test] procedure MineIs10() var M: Codeunit Mine; begin if M.Take() <> 10 then Error('mine %1', M.Take()); end;
            [Test] procedure Nothing() begin end; }}
        {objects}"
    );
    let login = "var F: Codeunit Flags; begin F.Put(); end;";
    run_codeunit("Probe", login, &tests, false, TestIsolation::Codeunit, "C")
}

/// The body that copies `Flags` into `Mine`.
const COPY_FLAGS: &str = "var F: Codeunit Flags; M: Codeunit Mine; begin if F.IsOn() then M.Put(10); end;";

// Review of al-runner PR 8241, set B: the app's own session-open code after the login reads the unmeasured codeunit: the
// session does not open, no test runs.
#[test]
fn session_open_code_that_reads_an_unmeasured_single_instance_refuses_the_codeunit() {
    let (o, _) = run_copied_flags(&format!(
        "codeunit 60960 Cot {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
            local procedure L() {COPY_FLAGS} }}"
    ));
    assert!(
        o.iter()
            .all(|(_, x)| not_run_for(x, "a global of SingleInstance codeunit Flags")),
        "{o:?}"
    );
}

// Review of al-runner PR 8241, set F: the runner runs codeunit 130301 before every test (runner_reset.rs); when it reads
// the unmeasured codeunit no test runs.
#[test]
fn the_runners_reset_step_that_reads_an_unmeasured_single_instance_refuses_every_test() {
    let (o, _) = run_copied_flags(&format!(
        "codeunit 130301 \"Reset State Before Test Run\" {{ trigger OnRun() {COPY_FLAGS} }}"
    ));
    assert!(
        o.iter()
            .all(|(_, x)| not_run_for(x, "a global of SingleInstance codeunit Flags")),
        "{o:?}"
    );
}

// Review of al-runner PR 8241, set A2: a RecordRef insert reaches a table event subscriber that reads the unmeasured
// codeunit: that test is not run at the read, the other test runs.
#[test]
fn a_table_event_behind_a_recordref_insert_that_reads_an_unmeasured_single_instance_refuses_the_test() {
    let tests = "codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
        table 60971 RefRow { fields { field(1; K; Code[10]) { } field(2; Seen; Boolean) { } } keys { key(PK; K) { } } }
        codeunit 60972 RefSub {
            [EventSubscriber(ObjectType::Table, Database::RefRow, OnBeforeInsertEvent, '', false, false)]
            local procedure OnIns(var Rec: Record RefRow) var F: Codeunit Flags; begin Rec.Seen := F.IsOn(); end; }
        codeunit 60973 R { Subtype = Test;
            [Test] procedure RecRefInsert() var RR: RecordRef; FR: FieldRef;
            begin RR.Open(60971); FR := RR.Field(1); FR.Value := 'A'; RR.Insert(); end;
            [Test] procedure Nothing() begin end; }";
    let login = "var F: Codeunit Flags; begin F.Put(); end;";
    let (o, _) = run_codeunit("Probe", login, tests, false, TestIsolation::Codeunit, "R");
    assert!(
        not_run_for(&o[0].1, "a global of SingleInstance codeunit Flags"),
        "{o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// A write of a global of the unmeasured codeunit reads nothing of it: the test that only sets it runs.
#[test]
fn a_write_of_an_unmeasured_single_instance_runs() {
    let tests = "codeunit 60911 Inst { Subtype = Install; trigger OnInstallAppPerCompany() begin end; }
        codeunit 60974 W { Subtype = Test;
            [Test] procedure Puts() var F: Codeunit Flags; begin F.Put(); end; }";
    let login = "var F: Codeunit Flags; begin F.Put(); end;";
    let (o, _) = run_codeunit("Probe", login, tests, false, TestIsolation::Codeunit, "W");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
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
    assert!(clean(&hits), "{hits:?}");
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

/// A partner app `dep3` (on `dep2`) whose automatic subscriber to `Pub.OnAsk` sets `Cache`'s count to 5: an own subscriber
/// that calls `Pub.Ask()` changes `Cache` through it, which the walk of the own subscriber's reach does not follow (it
/// follows the subscribers in apps under test only, `written::login_subscriber_reach_facts`), so the login stays Own.
/// `sys_src` is the Microsoft stand-in, `login` the body of `dep2`'s OnAfterLogin subscriber, `own` the own subscriber
/// codeunit of the app under test; test `T` reads `Cache` and `Counter`.
fn run_own_and_partner(sys_src: &str, login: &str, own: &str) -> OutcomesAndHits {
    let third = "codeunit 60983 AskSub {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnAsk', '', false, false)]
            local procedure H(var N: Integer) var C: Codeunit Cache; begin C.Set(5); end; }";
    let tests = format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        {own}
        codeunit 60912 T {{ Subtype = Test;
            [Test] procedure ReadsCache() var C: Codeunit Cache; begin if C.GetCount() = -1 then; end;
            [Test] procedure ReadsCounter() var C: Codeunit Counter; begin if C.Get() = -1 then; end;
            [Test] procedure ReadsNothing() begin end; }}"
    );
    outcomes_and_hits_with(
        &[
            ("dep", &[], sys_src),
            ("dep2", &["dep"], &partner(login)),
            ("dep3", &["dep2"], third),
            ("main", &["dep3"], &tests),
        ],
        &[("dep", "Microsoft"), ("dep2", "Probe"), ("dep3", "Probe3")],
        "T",
        TestIsolation::Codeunit,
    )
}

// A memo a partner subscriber changed and an own OnAfterLogin subscriber of the app under test changed after it: no probe
// has an own subscriber, so its reads refuse. A memo only the partner subscriber changed keeps its flag.
#[test]
fn an_own_login_subscriber_that_changes_a_memo_after_a_partner_one_refuses_its_reads() {
    let login = "var C: Codeunit Cache; Ctr: Codeunit Counter; begin C.Set(1); Ctr.Put(); end;";
    let own = "codeunit 60913 OwnLogin {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() var P: Codeunit Pub; begin if P.Ask() = 0 then; end; }";
    let (o, hits) = run_own_and_partner(&sys(false), login, own);
    assert!(
        not_run_for(
            &o[0].1,
            "a subscriber of an app under test changed it after a partner subscriber did"
        ),
        "{o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    assert_eq!(marks(&hits, "ReadsCounter"), [MEMO_KEPT], "{hits:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
}

// A memo an own login subscriber of the app under test changed before a partner subscriber changed it (the own subscriber
// of "Guided Experience".OnRegisterAssistedSetup runs before OnAfterLogin): not measured, its reads refuse.
#[test]
fn a_partner_subscriber_that_changes_a_memo_after_an_own_one_refuses_its_reads() {
    let guided = format!(
        "{}
        codeunit 1990 \"Guided Experience\" {{ procedure Register() begin OnRegisterAssistedSetup(); end;
            [IntegrationEvent(false, false)] local procedure OnRegisterAssistedSetup() begin end; }}
        codeunit 1997 \"Checklist Events\" {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Initialization\", 'OnAfterLogin', '', false, false)]
            local procedure L() var G: Codeunit \"Guided Experience\"; begin G.Register(); end; }}",
        sys(false)
    );
    let own = "codeunit 60913 OwnSetup {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Guided Experience\", 'OnRegisterAssistedSetup', '', false, false)]
            local procedure R() var P: Codeunit Pub; begin if P.Ask() = 0 then; end; }";
    let (o, _) = run_own_and_partner(&guided, "var C: Codeunit Cache; begin C.AddTxt('L'); end;", own);
    assert!(
        not_run_for(&o[0].1, "a subscriber of an app under test changed it too"),
        "{o:?}"
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
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
            [Test] procedure Sequence() begin if NumberSequence.Exists('S') then if NumberSequence.Current('S') = 0 then; end;
            [Test] procedure CacheIsZero() var C: Codeunit Cache; begin if C.GetCount() <> 0 then Error('count %1', C.GetCount()); end;
            [Test] procedure SetFirst() var C: Codeunit Cache; begin C.Set(5); if C.GetCount() <> 5 then Error('count %1', C.GetCount()); end; }}
        codeunit 60915 U {{ Subtype = Test;
            [Test] procedure OtherTable() var M: Record Mine; begin M.K := 'X'; M.Insert(); end; }}"
    )
}

const NOTHING_EARLY: &str = "var N: Integer; begin N := 1; end;";

fn run_readers(login: &str, early: &str, codeunit: &str) -> OutcomesAndHits {
    let tests = readers(early);
    run_codeunit("Probe", login, &tests, false, TestIsolation::Codeunit, codeunit)
}

// The subscriber makes the SingleInstance codeunit's instance and leaves its globals as declared: the test codeunit that
// reads it runs as on main.
#[test]
fn a_new_single_instance_with_declared_globals_is_no_change() {
    let (o, hits) = run_readers(
        "var C: Codeunit Cache; begin if C.GetCount() = 0 then; end;",
        NOTHING_EARLY,
        "T",
    );
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert!(clean(&hits), "{hits:?}");
}

/// Readers of `T` and `U` after a login that sets the cache to 10: the reader sees 10, the others pass, all clean.
fn assert_the_reader_sees_the_login_value(early: &str) {
    let login = "var C: Codeunit Cache; begin C.Set(10); end;";
    let (o, hits) = run_readers(login, early, "T");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(matches!(&o[1].1, Outcome::Fail(m) if m.contains("count 10")), "{o:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
    assert!(clean(&hits), "{hits:?}");
    let (o, hits) = run_readers(login, early, "U");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert!(clean(&hits), "{hits:?}");
}

// The subscriber makes the instance and sets a global.
#[test]
fn a_new_single_instance_with_a_set_global_reaches_the_reader() {
    assert_the_reader_sees_the_login_value(NOTHING_EARLY);
}

// The instance was there before the login (an OnCompanyOpen subscriber of the app under test made it) and the subscriber
// changes it.
#[test]
fn a_changed_existing_single_instance_reaches_the_reader() {
    assert_the_reader_sees_the_login_value("var C: Codeunit Cache; begin if C.GetCount() = 0 then; end;");
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

/// The login sets `Cnt` to 10 and adds one entry to `Log` (probe p1 of the review of al-runner PR 8138).
const COUNTS: &str = "var C: Codeunit Cache; begin C.Set(10); C.AddLog('login'); end;";

/// Test codeunits after a login that leaves `COUNTS`: `body` is their objects (`Mine` is a SingleInstance codeunit of the
/// app under test).
fn run_counts(tests: &str, codeunit: &str) -> OutcomesAndHits {
    let tests = format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60950 Mine {{ SingleInstance = true; var V: Integer;
            procedure Put(N: Integer) begin V := N; end; procedure Take(): Integer begin exit(V); end; }}
        {tests}"
    );
    run_codeunit("Probe", COUNTS, &tests, false, TestIsolation::Codeunit, codeunit)
}

// The value the login left carries from test to test of the codeunit: the next test after `Cnt += 1` reads 11
// (PlLoginSingleTests.al:79-98), and a list keeps the login's entry.
#[test]
fn the_login_value_carries_to_the_later_tests_of_the_codeunit() {
    let tests = "codeunit 60951 A { Subtype = Test;
        [Test] procedure A1() var C: Codeunit Cache; begin C.Bump(); if C.GetCount() <> 11 then Error('count %1', C.GetCount()); end;
        [Test] procedure A2() var C: Codeunit Cache; begin C.Bump(); if C.GetCount() <> 12 then Error('count %1', C.GetCount()); end;
        [Test] procedure A3() var C: Codeunit Cache; begin C.AddLog('x'); if C.LogCount() <> 2 then Error('log %1', C.LogCount()); end; }";
    let (o, hits) = run_counts(tests, "A");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert!(clean(&hits), "{hits:?}");
}

/// An OnCompanyOpenCompleted subscriber of the app under test (it runs after the login) with body `body`, and the test
/// codeunit `C`.
fn company_open_completed(body: &str) -> String {
    format!(
        "codeunit 60960 Cot {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
            local procedure L() {body} }}
        codeunit 60953 C {{ Subtype = Test;
            [Test] procedure MineIs10() var M: Codeunit Mine; begin if M.Take() <> 10 then Error('mine %1', M.Take()); end;
            [Test] procedure Nothing() begin end; }}"
    )
}

// The app's own session-open code after the login reads the login's value (PlLoginSingleTests.al:66-74: it reads 10) and
// copies it into its own SingleInstance codeunit.
#[test]
fn own_session_open_code_after_the_login_reads_its_global() {
    let tests = company_open_completed("var C: Codeunit Cache; M: Codeunit Mine; begin M.Put(C.GetCount()); end;");
    let (o, hits) = run_counts(&tests, "C");
    assert!(o.iter().all(|(_, x)| *x == Outcome::Pass), "{o:?}");
    assert!(clean(&hits), "{hits:?}");
}

// Probe p2/cotrows: the app's own session-open code after the login counts the table the login wrote: what it finds there
// is not measured, the codeunit is not run.
#[test]
fn own_session_open_code_after_the_login_that_reads_its_table_refuses_the_codeunit() {
    let tests = format!(
        "codeunit 60911 Inst {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}
        codeunit 60950 Mine {{ SingleInstance = true; var V: Integer;
            procedure Put(N: Integer) begin V := N; end; procedure Take(): Integer begin exit(V); end; }}
        {}",
        company_open_completed("var O: Record Other; M: Codeunit Mine; begin M.Put(O.Count()); end;")
    );
    let (o, _) = run_codeunit("Probe", INSERTS, &tests, false, TestIsolation::Codeunit, "C");
    assert!(
        o.iter()
            .all(|(_, x)| not_run_for(x, "partner dependency apps ran before the first test and wrote")),
        "{o:?}"
    );
}
