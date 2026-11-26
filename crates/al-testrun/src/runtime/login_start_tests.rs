// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! Replaying the raised login (`login_start.rs`, al-runner issue 8211): the first session of a key runs the
//! login and keeps what it left (a start); a later session of the key takes the start in place of running the login. The
//! tests run over the stand-in of Microsoft's login (`login_tests::DEP`) and the sources of `login_replay_tests.rs`, as
//! The check's and the image's tests do.
//!
//! A test marked GUARD holds whether or not a session replays; it must stay green.

use super::login_replay_tests::{MAIN, OPAQUE_INSIDE, compiled, compiled_with};
use super::session_flags::SessionFlags;
use super::*;
use crate::login_replay::{Key, first_difference};
use crate::login_start::{
    self, DEFAULT_BYTES, DEFAULT_STARTS, LoginReplayLimits, MAX_START_BYTES, NotTaken, Start, starts,
};
use al_builtins::assumptions::HitBits;
use al_builtins::value::Value;

/// What a run of `sessions_with` adds to the options: a change of the scratch, an install baseline, an observer.
#[derive(Default)]
struct Setup<'a> {
    tweak: Option<&'a dyn Fn(&mut Scratch)>,
    baseline: Option<&'a al_vm::Baseline>,
    observed: bool,
}

/// What `sessions` ran: each session's state after its company open, the outcomes, and the assumption ids each test
/// carries.
struct Ran {
    states: Vec<crate::login_replay::LoginState>,
    outcomes: Vec<Vec<(String, Outcome)>>,
    marks: Vec<Vec<(String, Vec<&'static str>)>>,
}

/// Runs test codeunit `name` of `l` `n` times on `engine`, one session each, with the session flags of `opts` and the
/// scratch `tweak` leaves, under `OPENED` (each session's state after `open_company`, run or replayed).
fn sessions_with(engine: &Arc<Engine>, l: &Lowered, name: &str, n: usize, opts: &RunOptions, setup: &Setup) -> Ran {
    login_start::OPENED.with(|o| *o.borrow_mut() = Some(Vec::new()));
    let oi = l.program.objects.iter().position(|o| o.name == name).unwrap();
    let mut ran = Ran {
        states: Vec::new(),
        outcomes: Vec::new(),
        marks: Vec::new(),
    };
    for _ in 0..n {
        let mut scratch = Scratch {
            flags: SessionFlags::of(opts),
            ..Scratch::new(crate::host::RunClock::begin())
        };
        if let Some(tweak) = setup.tweak {
            tweak(&mut scratch);
        }
        let mut hits = Vec::new();
        let mut observe = |_: &str, _: &Outcome, _: &Observed| {};
        let observer: &mut Observer = &mut observe;
        let outcomes = run_codeunit_core(
            engine,
            l,
            oi,
            setup.baseline,
            TestIsolation::Codeunit,
            setup.observed.then_some(observer),
            &mut Vec::new(),
            &mut Vec::new(),
            &mut hits,
            &mut scratch,
            None,
            false,
        );
        ran.outcomes.push(outcomes);
        ran.marks.push(
            hits.into_iter()
                .map(|(t, h)| (t, h.iter().map(|a| a.info().id).collect()))
                .collect(),
        );
    }
    ran.states = login_start::OPENED.with(|o| o.borrow_mut().take()).unwrap_or_default();
    ran
}

/// The options of a run with the replay on (`RunOptions::login_replay`).
fn on() -> RunOptions {
    RunOptions {
        login_replay: true,
        ..Default::default()
    }
}

/// [`sessions_with`] with no tweak, no baseline and no observer.
fn sessions(engine: &Arc<Engine>, l: &Lowered, name: &str, n: usize, opts: &RunOptions) -> Ran {
    sessions_with(engine, l, name, n, opts, &Setup::default())
}

fn passes() -> Vec<(String, Outcome)> {
    vec![("SeesTheRow".to_string(), Outcome::Pass)]
}

/// Asserts every session of `ran` passed and opened the company in one state (nothing differs after a replay).
fn assert_one_state(ran: &Ran, what: &str) {
    for (i, o) in ran.outcomes.iter().enumerate() {
        assert_eq!(*o, passes(), "{what}: session {i}");
    }
    for (i, s) in ran.states.iter().enumerate().skip(1) {
        assert_eq!(
            first_difference(&ran.states[0], s),
            None,
            "{what}: session {i} holds another state than session 0 after the open"
        );
    }
}

// Issue 8211, the replay against a real login: the first session of a key runs the login, the later ones take its start,
// and each holds after the open what a real login leaves (rows, session state, marks, what the open noted), as the
// check's comparison of a thaw with a real login (`login_replay.rs`) requires.
#[test]
fn a_second_session_of_a_key_takes_the_start_and_holds_what_a_real_login_leaves() {
    let (engine, l) = compiled();
    let ran = sessions(&engine, &l, "RPL Tests A", 3, &on());
    assert_eq!(
        starts(&engine).replayed(),
        2,
        "sessions 2 and 3 take the start of session 1"
    );
    assert_eq!(starts(&engine).kept(), 1);
    assert_eq!(
        ran.states.len(),
        3,
        "OPENED is handed the state of a replayed session too"
    );
    assert_one_state(&ran, "replay on");
    // A real login on another engine leaves the same state (same inputs, same seed).
    let (off_engine, l) = compiled();
    let off = RunOptions::default();
    let real = sessions(&off_engine, &l, "RPL Tests A", 1, &off);
    assert_eq!(first_difference(&ran.states[1], &real.states[0]), None);
}

// A guard: with the switch off (the default; `ALRUNNER_LOGIN_REPLAY=1`, `RunOptions::login_replay`, turns it on) every session runs
// the login, nothing is kept, and the outcomes and states are those of the replay's.
#[test]
fn the_switch_off_runs_every_login() {
    let (engine, l) = compiled();
    let off = RunOptions::default();
    let ran = sessions(&engine, &l, "RPL Tests A", 3, &off);
    assert_eq!(starts(&engine).replayed(), 0);
    assert_eq!(starts(&engine).kept(), 0, "nothing is kept with the switch off");
    assert_one_state(&ran, "replay off");
    assert_eq!(starts(&engine).line(), None);
}

// The check replays nothing: every session runs the login, and each later session of a key is compared with a thaw of the
// Key's start.
#[test]
fn the_check_replays_nothing_and_compares_each_later_session_with_a_thaw_of_its_start() {
    let (engine, l) = compiled();
    let ran = sessions(
        &engine,
        &l,
        "RPL Tests A",
        3,
        &RunOptions {
            login_replay_check: true,
            ..Default::default()
        },
    );
    assert_one_state(&ran, "check");
    assert_eq!(
        starts(&engine).replayed(),
        0,
        "GUARD: under the check no session replays"
    );
    assert_eq!(
        starts(&engine).kept(),
        1,
        "the check keeps starts exactly as the replay does"
    );
    let report = crate::login_replay::report(&engine).expect("a report");
    assert!(report.contains("replay vs run: 2 checked, 0 differ"), "{report}");
    // GUARD: the first two lines keep the shape of the check line and the image line.
    assert!(
        report.starts_with(
            "login replay check: 3 sessions over 1 keys (a key seen before: 2 sessions, 66%), 2 compared, 0 differ; across test codeunits: 0 compared, 0 differ\nlogin image: 1 keys, 1 imaged, 0 not imaged; round trips 1, 0 differ"
        ),
        "{report}"
    );
}

// A planted difference in the thaw the check compares with is reported (`TWIN_PLANT`, as the round trip's is).
#[test]
fn a_planted_difference_in_the_replay_is_reported() {
    let (engine, l) = compiled();
    crate::login_replay::TWIN_PLANT.with(|p| p.set(Some(|twin| twin.set_work_date(1))));
    sessions(
        &engine,
        &l,
        "RPL Tests A",
        3,
        &RunOptions {
            login_replay_check: true,
            ..Default::default()
        },
    );
    crate::login_replay::TWIN_PLANT.with(|p| p.set(None));
    let report = crate::login_replay::report(&engine).expect("a report");
    let line = report
        .lines()
        .find(|l| l.contains("replay vs run: "))
        .unwrap_or_else(|| panic!("{report}"));
    assert!(
        line.contains("replay vs run: 2 checked, 2 differ"),
        "the plant is found by the replay-vs-run compare: {line}"
    );
}

/// [`MAIN`] with install code that sets WorkDate: a codeunit's session that starts from no install baseline starts from
/// what it left (`install_once.rs`, `Session::install_left` carries WorkDate), before the session enters the codeunit
/// and logs in.
const INSTALL_SETS_WORKDATE: &str = r#"
codeunit 77480 "RPL Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin WorkDate(DMY2Date(3, 4, 2021)); end;
}
"#;

/// The state lines of each twin of the check before its thaw (`TWIN_TARGET`), for the test below only.
static TWINS_BEFORE_THAW: std::sync::Mutex<Vec<al_vm::StateLines>> = std::sync::Mutex::new(Vec::new());

// Issue 8211: the replay thaws into the test session after the steps of `Start::open` (the install code's leftovers,
// the test codeunit entered), so the fields the thaw keeps from its target are the session's own. The check's twin is
// made by the same steps: before its thaw it holds what the session held before its login (the key's session state),
// here the WorkDate the install code left.
#[test]
fn the_checks_twin_holds_before_its_thaw_what_the_session_held_before_its_login() {
    let (engine, l) = compiled_with(&format!("{MAIN}{INSTALL_SETS_WORKDATE}"));
    TWINS_BEFORE_THAW.lock().unwrap().clear();
    crate::login_replay::TWIN_TARGET
        .with(|p| p.set(Some(|twin| TWINS_BEFORE_THAW.lock().unwrap().push(twin.state_lines()))));
    let check = RunOptions {
        login_replay_check: true,
        ..Default::default()
    };
    let ran = sessions(&engine, &l, "RPL Tests A", 2, &check);
    crate::login_replay::TWIN_TARGET.with(|p| p.set(None));
    assert_one_state(&ran, "check");
    let twins = std::mem::take(&mut *TWINS_BEFORE_THAW.lock().unwrap());
    assert_eq!(twins.len(), 2, "the round trip's twin and the replay-vs-run twin");
    let firsts = crate::login_replay::check(&engine).firsts();
    let key = &firsts[0].0;
    for (i, twin) in twins.iter().enumerate() {
        assert!(
            key.key.holds_session(twin),
            "twin {i} before its thaw is not the session before its login: {:?}",
            twin.lines
                .iter()
                .filter(|l| l.to_lowercase().contains("work"))
                .collect::<Vec<_>>()
        );
    }
}

/// [`MAIN`] with a login subscriber that commits: at UpdateNoLocks, the type the login runs at, the Commit keeps the
/// type and marks it `commit-keeps-transaction-type` (al-vm `Session::keep_type_at_commit`, `TxState::type_origin`).
const COMMITS_IN_THE_LOGIN: &str = r#"
codeunit 77491 "RPL Commit Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() begin Commit(); end;
}
"#;

/// The transaction line of `state` (al-vm `state_facts.rs`, with the type's origin).
fn transaction_line(state: &crate::login_replay::LoginState) -> String {
    let lines = &state.state_lines().lines;
    lines
        .iter()
        .find(|l| l.starts_with("transaction "))
        .cloned()
        .unwrap_or_default()
}

// Issue 8211, from the #8528 review: a login that commits at a type other than Update leaves the type's origin (the
// mark a later read of CurrentTransactionType before the first test method is flagged by). A replayed session holds the
// same origin as a real login, in the transaction line the check compares.
#[test]
fn a_replayed_session_has_the_type_origin_a_real_login_leaves() {
    let src = format!("{MAIN}{COMMITS_IN_THE_LOGIN}");
    let (engine, l) = compiled_with(&src);
    let ran = sessions(&engine, &l, "RPL Tests A", 3, &on());
    assert_eq!(
        starts(&engine).replayed(),
        2,
        "sessions 2 and 3 take the start of session 1"
    );
    let (off_engine, l) = compiled_with(&src);
    let real = sessions(&off_engine, &l, "RPL Tests A", 1, &RunOptions::default());
    for i in 0..3 {
        assert_eq!(ran.outcomes[i], real.outcomes[0], "session {i}");
        assert_eq!(first_difference(&ran.states[i], &real.states[0]), None, "session {i}");
    }
    let (plain, l) = compiled();
    let unmarked = sessions(&plain, &l, "RPL Tests A", 1, &RunOptions::default());
    assert_ne!(
        transaction_line(&ran.states[2]),
        transaction_line(&unmarked.states[0]),
        "GUARD: the login's Commit shows in the transaction line"
    );
}

/// [`MAIN`] with a login subscriber that shares element 1 of a SingleInstance codeunit's array of temporary records
/// into another SingleInstance codeunit's global (`Copy(x, true)`, which notes the copy target on the table's token),
/// and a test that clears the first codeunit: ClearAll keeps the shared table in place (al-vm
/// `clear_shared_codeunit.rs`, ClearSharedCodeunitTempArrayShare.al SIArrNever_ :736); without the note it refuses.
const SHARES_BY_COPY: &str = r#"
codeunit 77492 "RPL Arr Owner"
{
    SingleInstance = true;
    var A: array[2] of Record "Guided Experience Item" temporary;
    procedure ShareA1(var R: Record "Guided Experience Item" temporary) begin R.Copy(A[1], true); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
}
codeunit 77493 "RPL Copy Keeper"
{
    SingleInstance = true;
    var Keep: Record "Guided Experience Item" temporary;
    procedure Take() var O: Codeunit "RPL Arr Owner"; begin O.ShareA1(Keep); end;
}
codeunit 77494 "RPL Copy Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var K: Codeunit "RPL Copy Keeper"; begin K.Take(); end;
}
codeunit 77495 "RPL Clear Tests"
{
    Subtype = Test;
    [Test]
    procedure ClearsTheOwner() var O: Codeunit "RPL Arr Owner"; begin O.ClearAllGlobals(); end;
}
"#;

// Issue 8211, from the #8528 review: the copy targets a login notes on a temporary table's token (main's `TempToken`
// `copy_targets`) decide whether a test's ClearAll of a shared codeunit is the measured case. A replayed session has
// the copy targets of a real login: the test passes in both, where losing them would refuse it.
#[test]
fn a_replayed_session_has_the_copy_targets_a_real_login_notes() {
    let src = format!("{MAIN}{SHARES_BY_COPY}");
    let passes = vec![("ClearsTheOwner".to_string(), Outcome::Pass)];
    let (off_engine, l) = compiled_with(&src);
    let real = sessions(&off_engine, &l, "RPL Clear Tests", 1, &RunOptions::default());
    assert_eq!(
        real.outcomes,
        std::slice::from_ref(&passes),
        "GUARD: a real login's session passes"
    );
    let (engine, l) = compiled_with(&src);
    let ran = sessions(&engine, &l, "RPL Clear Tests", 3, &on());
    assert_eq!(
        starts(&engine).replayed(),
        2,
        "sessions 2 and 3 take the start of session 1"
    );
    assert_eq!(ran.outcomes, vec![passes; 3]);
    for (i, s) in ran.states.iter().enumerate() {
        assert_eq!(first_difference(s, &real.states[0]), None, "session {i}");
    }
}

/// A login that also makes an assumption mark when the company opens (`OnCompanyOpenCompleted`, a Variant to Integer
/// conversion): every test of the session is a flagged pass (al-runner issue 8060; `login_tests.rs`
/// `an_assumption_the_company_open_step_relied_on_flags_every_test_of_the_session`).
const MARKS_INSIDE: &str = r#"
codeunit 77470 "RPL Marks"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure Completed() var V: Variant; I: Integer; begin V := '55'; I := V; end;
}
"#;

/// Sets `DROP_HITS` for its lifetime (a test that fails must not leave it set).
struct DropHits;

impl DropHits {
    fn on() -> DropHits {
        login_start::DROP_HITS.with(|d| d.set(true));
        DropHits
    }
}

impl Drop for DropHits {
    fn drop(&mut self) {
        login_start::DROP_HITS.with(|d| d.set(false));
    }
}

// The marks the login made are raised again after a replay, so a flagged pass stays flagged. The plant
// `DROP_HITS` takes a start without its marks: the replayed session's list then differs, which shows this test bites; without
// It the lists are equal.
#[test]
fn a_replayed_session_flags_the_passes_a_real_login_flags() {
    let flagged = vec![("SeesTheRow".to_string(), vec!["text-digits-to-integer"])];
    let (engine, l) = compiled_with(&format!("{MAIN}{MARKS_INSIDE}"));
    {
        let _plant = DropHits::on();
        let dropped = sessions(&engine, &l, "RPL Tests A", 2, &on());
        assert_eq!(dropped.marks[0], flagged, "GUARD: the real login's pass is flagged");
        assert_eq!(starts(&engine).replayed(), 1);
        assert_ne!(
            dropped.marks[0], dropped.marks[1],
            "a start without its marks flags nothing"
        );
    }
    let (engine, l) = compiled_with(&format!("{MAIN}{MARKS_INSIDE}"));
    let ran = sessions(&engine, &l, "RPL Tests A", 3, &on());
    assert_eq!(starts(&engine).replayed(), 2);
    for (i, marks) in ran.marks.iter().enumerate() {
        assert_eq!(*marks, flagged, "session {i}");
    }
    assert_one_state(&ran, "marks");
}

// A key whose session the image refuses (`NotImaged`) runs the real login every time and is not frozen again: it is counted
// Once in the report, however many sessions it has.
#[test]
fn a_key_that_is_not_imaged_runs_the_login_every_time_and_is_frozen_once() {
    let (engine, l) = compiled_with(&format!("{MAIN}{OPAQUE_INSIDE}"));
    let ran = sessions(&engine, &l, "RPL Tests A", 3, &on());
    // The states of this app are "not comparable" (an HttpClient in a global): the outcomes are what is compared.
    assert!(ran.outcomes.iter().all(|o| *o == passes()), "{:?}", ran.outcomes);
    assert_eq!(
        starts(&engine).replayed(),
        0,
        "GUARD: a session the image refuses takes no start"
    );
    assert_eq!(starts(&engine).kept(), 0);
    let line = starts(&engine).line().expect("a line once a key was refused");
    assert!(line.starts_with("login start: 0 replayed, 0 kept"), "{line}");
    assert!(line.contains(", 1 not imaged"), "one key, frozen once: {line}");
}

// With the replay alone (no check), the report is the starts line: the keys not imaged, the starts too large and the
// sessions the gate or a refused thaw kept from a start are counted there and nowhere else (maintainer's review of
// al-runner PR 8665).
#[test]
fn with_the_replay_alone_the_report_is_the_starts_line() {
    let (engine, l) = compiled_with(&format!("{MAIN}{OPAQUE_INSIDE}"));
    sessions(&engine, &l, "RPL Tests A", 2, &on());
    let line = starts(&engine).line().expect("a line once a key was refused");
    assert!(line.contains(", 1 not imaged"), "{line}");
    assert_eq!(crate::login_replay::report(&engine), Some(line));
    let (_, line) = gated_line(&|s| s.perturb.library_seed = Some(3), false);
    assert!(line.is_some_and(|l| l.starts_with("login start: 0 replayed") && l.contains("refused by the gate")));
}

/// How many times `open_company` took a session before its login while `run` ran.
fn bases_taken(run: impl FnOnce()) -> u32 {
    login_start::BASES.with(|b| b.set(0));
    run();
    login_start::BASES.with(std::cell::Cell::get)
}

// The session before the login (`Session::login_base`, a clone of the store) is taken only where it is read: for a
// start that will be recorded (a key not known yet, a run the gate lets take starts) or for the check. Not for a
// session that replays, one whose key was refused, nor a run the gate keeps from starts (issue 8211).
#[test]
fn the_session_before_the_login_is_taken_only_to_record_a_start_or_for_the_check() {
    let (engine, l) = compiled();
    let recorded = bases_taken(|| drop(sessions(&engine, &l, "RPL Tests A", 3, &on())));
    assert_eq!(
        (recorded, starts(&engine).replayed()),
        (1, 2),
        "the first session records, the others replay"
    );
    let (engine, l) = compiled_with(&format!("{MAIN}{OPAQUE_INSIDE}"));
    let refused = bases_taken(|| drop(sessions(&engine, &l, "RPL Tests A", 3, &on())));
    assert_eq!(refused, 1, "a key the image refused is not frozen again");
    let (engine, l) = compiled();
    let observed = Setup {
        observed: true,
        ..Default::default()
    };
    let gated = bases_taken(|| drop(sessions_with(&engine, &l, "RPL Tests A", 2, &on(), &observed)));
    assert_eq!(gated, 0, "a run the gate keeps from starts records none");
    let (engine, l) = compiled();
    let check = RunOptions {
        login_replay_check: true,
        ..Default::default()
    };
    let checked = bases_taken(|| drop(sessions(&engine, &l, "RPL Tests A", 2, &check)));
    assert_eq!(checked, 2, "GUARD: the check compares every session");
}

// An uncaught error of the install code leaves `error_finals` on the session (`install_once`'s fallback makes the install
// session the test session), and the login's calls clear it: the session that records is not a thaw target before its
// login (`LoginBase::target_why`), so it keeps no start, as a later such session takes none (`thaw_target_why`).
#[test]
fn an_uncaught_install_error_keeps_error_finals_and_no_start_is_made_or_taken() {
    let failing = r#"
codeunit 77495 "RPL Failing Install" { Subtype = Install; trigger OnInstallAppPerCompany() begin Error('install boom'); end; }
"#;
    let (engine, l) = compiled_with(&format!("{MAIN}{failing}"));
    let ran = sessions(&engine, &l, "RPL Tests A", 3, &on());
    assert_eq!(starts(&engine).replayed(), 0, "GUARD");
    assert_eq!(starts(&engine).kept(), 0);
    assert_eq!(ran.outcomes[0], ran.outcomes[1]);
    assert_eq!(ran.outcomes[0], ran.outcomes[2]);
    let line = starts(&engine).line().expect("the refused key is reported");
    assert!(
        line.contains("3 refused by the gate (3: target: error_finals)"),
        "{line}"
    );
}

// What the starts' report line says, names only: replays, starts kept with their size, why keys were not
// Imaged, why sessions were refused.
#[test]
fn the_report_names_the_starts() {
    let (engine, l) = compiled();
    sessions(&engine, &l, "RPL Tests A", 3, &on());
    let line = starts(&engine).line().expect("a login start line");
    assert!(line.starts_with("login start: 2 replayed, 1 kept ("), "{line}");
    assert!(line.contains(" MB)"), "{line}");
}

// The image line under the check gains the kept bytes and the count per node kind, names only.
#[test]
fn the_image_line_reports_the_kept_bytes_and_the_nodes_per_kind() {
    let (engine, l) = compiled();
    super::login_replay_tests::run(&engine, &l, "RPL Tests A");
    let report = crate::login_replay::report(&engine).expect("a report");
    let image = report
        .lines()
        .find(|l| l.starts_with("login image: "))
        .expect("an image line");
    let per_key = image.split("per key: ").nth(1).unwrap_or_else(|| panic!("{image}"));
    assert!(per_key.contains(", kept "), "kept_bytes per start: {per_key}");
    assert!(per_key.contains(" (delta)"), "the rows are a delta: {per_key}");
    for kind in ["record", "temporary table"] {
        assert!(per_key.contains(kind), "the count of {kind} nodes: {per_key}");
    }
    assert!(!per_key.contains("77401"), "no value in the line: {per_key}");
}

/// The key of a new session of `engine` whose login skipped `skipped` (committed: its SQL counters are at rest, as a
/// test session's are before its login; a session whose counters are not has no comparable form and no key).
fn key_skipping(engine: &Arc<Engine>, skipped: &[al_vm::SkippedInstall]) -> Key {
    let mut session = target(engine);
    Key::of(&mut session, Default::default(), SessionFlags::default(), skipped).expect("a key")
}

// The open runs under the watch of the skipped installs, whose hit marks or refuses: another list is another key.
#[test]
fn another_skipped_install_list_is_another_key() {
    let (engine, _) = compiled();
    let skip = |tables: &[u32], why: &str| al_vm::SkippedInstall::new(tables.iter().copied().collect(), why.into());
    let (a, b) = (skip(&[1990], "reads a table"), skip(&[1996], "reads a table"));
    assert_eq!(
        key_skipping(&engine, std::slice::from_ref(&a)),
        key_skipping(&engine, std::slice::from_ref(&a)),
        "GUARD"
    );
    assert_eq!(key_skipping(&engine, &[]), key_skipping(&engine, &[]), "GUARD");
    assert_ne!(
        key_skipping(&engine, std::slice::from_ref(&a)),
        key_skipping(&engine, std::slice::from_ref(&b))
    );
    assert_ne!(
        key_skipping(&engine, &[]),
        key_skipping(&engine, std::slice::from_ref(&a))
    );
    assert_ne!(
        key_skipping(&engine, std::slice::from_ref(&a)),
        key_skipping(&engine, &[a.clone(), b.clone()])
    );
}

/// [`OPAQUE_INSIDE`]'s holder kept from a codeunit's `OnRun`: run before the login, it leaves a SingleInstance global
/// with no comparable form.
const OPAQUE_BEFORE: &str = r#"
codeunit 77490 "RPL Opaque Before"
{
    trigger OnRun() var Holder: Codeunit "RPL Holder"; begin Holder.Keep(); end;
}
"#;

// `Key::of` holds a value with no comparable form only by its path (al-vm `NotComparable`): two sessions with other
// values at the same path would get equal keys. A session that holds one before its login gets no key, and runs its
// login.
#[test]
fn a_session_with_a_value_of_no_comparable_form_before_its_login_has_no_key() {
    let (engine, _) = compiled_with(&format!("{MAIN}{OPAQUE_INSIDE}{OPAQUE_BEFORE}"));
    let mut session = target(&engine);
    assert!(
        Key::of(&mut session, Default::default(), SessionFlags::default(), &[]).is_ok(),
        "GUARD: a new session has a key"
    );
    assert!(session.run_codeunit_by_id(77490).is_ok());
    session.commit_transaction();
    assert!(
        !session.state_lines().opaque.is_empty(),
        "GUARD: the holder has no comparable form"
    );
    assert!(matches!(
        Key::of(&mut session, Default::default(), SessionFlags::default(), &[]),
        Err(crate::login_replay::NoKey::NotComparable(_))
    ));
}

/// `engine` with its emit version set to `v`, as `set_run_options` sets it for each request on a kept engine.
fn with_emit_version(mut engine: Arc<Engine>, v: al_vm::emit_version::EmitVersion) -> Arc<Engine> {
    Arc::get_mut(&mut engine)
        .expect("nothing else holds the engine")
        .emit_version = v;
    engine
}

// A warm server sets the emit version per request (`set_run_options`, `run_emit_version`: one per build), also on a
// kept engine, whose starts outlive the request; the missing-object error the login may raise names it
// (errors/error-texts/AzErrorTexts.al:125). Another build's emit version is another key; the same reader is the same
// key.
#[test]
fn another_emit_version_is_another_key() {
    use al_vm::emit_version::EmitVersion;
    let (engine, _) = compiled();
    let v28 = EmitVersion::known(28015);
    let engine = with_emit_version(engine, v28.clone());
    let a = key_skipping(&engine, &[]);
    assert_eq!(a, key_skipping(&engine, &[]), "GUARD: the same reader");
    let engine = with_emit_version(engine, v28);
    assert_eq!(a, key_skipping(&engine, &[]), "GUARD: a clone of the same reader");
    let engine = with_emit_version(engine, EmitVersion::known(29022));
    assert_ne!(a, key_skipping(&engine, &[]), "another build's emit version");
}

// The tables a read of is refused on an engine are set per run after its install baseline (`corpus.rs`: `unseeded` from
// the baseline, `demo_tables` without test data), also on a kept engine whose starts outlive the request: a login that
// reads one is refused there. Another set, or another reason, is another key.
#[test]
fn another_set_of_tables_whose_reads_are_refused_is_another_key() {
    let (engine, _) = compiled();
    let table = (engine.program.objects.iter())
        .position(|o| o.name == "Guided Experience Item")
        .unwrap();
    let set = |engine: Arc<Engine>, unseeded: &[(usize, &str)], demo: &[usize]| {
        let mut engine = engine;
        let e = Arc::get_mut(&mut engine).expect("nothing else holds the engine");
        e.unseeded = unseeded.iter().map(|(t, why)| (*t, Arc::from(*why))).collect();
        e.demo_tables = demo.iter().copied().collect();
        engine
    };
    let engine = set(engine, &[], &[]);
    let none = key_skipping(&engine, &[]);
    let engine = set(engine, &[], &[]);
    assert_eq!(none, key_skipping(&engine, &[]), "GUARD");
    let engine = set(engine, &[(table, "a root stopped")], &[]);
    let unseeded = key_skipping(&engine, &[]);
    assert_ne!(none, unseeded, "an unseeded table");
    let engine = set(engine, &[(table, "another root stopped")], &[]);
    assert_ne!(unseeded, key_skipping(&engine, &[]), "another reason");
    let engine = set(engine, &[], &[table]);
    assert_ne!(none, key_skipping(&engine, &[]), "a table of demo rows");
}

// ---- the gate: why a session does not take a start ----

/// A start taken from a new session of `engine` (nothing logged in: what the gate looks at is the target and the run).
fn plain_start(engine: &Arc<Engine>) -> Start {
    let mut session = al_vm::Session::new(engine.clone());
    session.commit_transaction();
    let base = session.login_base();
    Start {
        image: session
            .login_image(&base)
            .unwrap_or_else(|why| panic!("{}", why.reason())),
        wrote: Default::default(),
        hits: HitBits::EMPTY,
    }
}

/// A new session of `engine` that can take a start.
fn target(engine: &Arc<Engine>) -> al_vm::Session {
    let mut session = al_vm::Session::new(engine.clone());
    session.commit_transaction();
    session
}

// A session that can take the start of its key takes it.
#[test]
fn a_session_with_nothing_against_it_takes_the_start() {
    let (engine, _) = compiled();
    let start = plain_start(&engine);
    assert_eq!(login_start::why_not(&target(&engine), &start, Default::default()), None);
}

// One reason per gate condition; each is counted in the report.
#[test]
fn the_gate_names_a_run_that_notes_code_a_perturbation_and_moved_in_singles() {
    let (engine, _) = compiled();
    let start = plain_start(&engine);
    let why = |notes_code, perturbed, moved_in| {
        let run = login_start::RunFacts {
            notes_code,
            perturbed,
            moved_in,
        };
        login_start::why_not(&target(&engine), &start, run)
    };
    assert_eq!(why(true, false, false), Some(NotTaken::NotesCode));
    assert_eq!(why(false, true, false), Some(NotTaken::Perturbed));
    assert_eq!(why(false, false, true), Some(NotTaken::MovedIn));
}

/// Sets `host::UNKEYED` for its lifetime (a test that fails must not leave it set).
struct Unkeyed;

impl Unkeyed {
    fn set(input: crate::host::Unkeyed) -> Unkeyed {
        crate::host::UNKEYED.with(|u| u.set(Some(Some(input))));
        Unkeyed
    }
}

impl Drop for Unkeyed {
    fn drop(&mut self) {
        crate::host::UNKEYED.with(|u| u.set(None));
    }
}

// The real clock (`ALRUNNER_REAL_CLOCK`) and a real files directory (`--files-dir`) are settings of the process no key
// holds, for the install baseline cache as for the starts (`host::unkeyed`, the one check of both): a kept serve daemon
// under the real clock would hand a later session the time an earlier login stamped into a row, and a login that reads
// a file an earlier test wrote reads what no key holds. Through `open_company`: a run under either neither records nor
// takes a start, and the report names why (issue 8211).
#[test]
fn a_run_under_a_setting_no_key_holds_neither_records_nor_takes_a_start() {
    for input in [crate::host::Unkeyed::RealClock, crate::host::Unkeyed::RealFiles] {
        let (engine, l) = compiled();
        let ran = {
            let _set = Unkeyed::set(input);
            sessions(&engine, &l, "RPL Tests A", 2, &on())
        };
        assert_one_state(&ran, input.reason());
        assert_eq!(starts(&engine).replayed(), 0, "{input:?}");
        assert_eq!(starts(&engine).kept(), 0, "{input:?}: no start is recorded");
        let line = starts(&engine)
            .line()
            .unwrap_or_else(|| panic!("{input:?}: no report line"));
        assert!(
            line.contains(&format!("2 refused by the gate (2: {})", input.reason())),
            "{line}"
        );
    }
}

#[test]
fn the_gate_names_what_keeps_the_session_from_being_a_target() {
    let (engine, _) = compiled();
    let start = plain_start(&engine);
    let mut covering = target(&engine);
    covering.enable_coverage();
    assert_eq!(
        login_start::why_not(&covering, &start, Default::default()),
        Some(NotTaken::Target("coverage"))
    );
    let mut tracing = target(&engine);
    tracing.trace_faults(true);
    assert_eq!(
        login_start::why_not(&tracing, &start, Default::default()),
        Some(NotTaken::Target("fault_trace"))
    );
}

// A target whose store has another shape than the start's (a table entry the start's base did not have: the key's digest
// Of the rows does not see an empty entry) is refused, `base shape`.
#[test]
fn the_gate_refuses_a_target_whose_base_has_another_shape() {
    use al_data::Store;
    let (engine, _) = compiled();
    let start = plain_start(&engine);
    let mut other = target(&engine);
    let at = engine
        .program
        .objects
        .iter()
        .position(|o| o.name == "Guided Experience Item")
        .unwrap();
    let table = engine.tables[at].clone().unwrap();
    let cx = al_data::AccessContext {
        company: other.access_company().into(),
        user: other.user_id().into(),
    };
    let row: Vec<Value> = table.fields.iter().map(|f| Value::default_for(&f.ty)).collect();
    assert!(other.store_mut().insert(&cx, &table, row).is_ok());
    other.commit_transaction();
    assert_ne!(other.base_shape(), *start.image.base_shape(), "the shapes differ");
    assert_eq!(
        login_start::why_not(&other, &start, Default::default()),
        Some(NotTaken::BaseShape)
    );
}

/// Runs "RPL Tests A" twice with `tweak` on both runs (and an observer when `observed`): the second session would take
/// the first's start. Returns how many sessions replayed, and the report (the starts line, with the replay alone).
fn gated_line(tweak: &dyn Fn(&mut Scratch), observed: bool) -> (u64, Option<String>) {
    let (engine, l) = compiled();
    let ran = sessions_with(
        &engine,
        &l,
        "RPL Tests A",
        2,
        &on(),
        &Setup {
            tweak: Some(tweak),
            observed,
            ..Default::default()
        },
    );
    assert_one_state(&ran, "gated");
    (starts(&engine).replayed(), crate::login_replay::report(&engine))
}

// At run level: a run that follows the code it runs, a perturbation and a set clock start never take a start, and the
// Report counts the sessions the gate refused.
#[test]
fn a_session_the_gate_refuses_runs_its_login_and_is_counted() {
    let perturbed = |s: &mut Scratch| s.perturb.library_seed = Some(3);
    let clocked = |s: &mut Scratch| {
        s.clock = crate::host::RunClock::At(1_700_000_000_000);
        s.clock_set = true;
    };
    let cases: [(&str, (u64, Option<String>)); 3] = [
        ("--explain or an observer", gated_line(&|_| {}, true)),
        ("a stability perturbation", gated_line(&perturbed, false)),
        ("a set clock start", gated_line(&clocked, false)),
    ];
    for (what, (replayed, line)) in cases {
        assert_eq!(replayed, 0, "{what}");
        let line = line.unwrap_or_else(|| panic!("{what}: no report line"));
        assert!(line.contains("refused by the gate"), "{what}: {line}");
    }
    crate::coverage::begin();
    let (replayed, line) = gated_line(&|_| {}, false);
    crate::coverage::finish();
    assert_eq!(replayed, 0, "coverage");
    assert!(line.is_some_and(|l| l.contains("refused by the gate")), "coverage");
}

// ---- the caps: 8 starts and 64 MB unless the run sets others, a start of at most 16 MB, least recently used out ----

/// The key of a new session of `engine` whose work date is `n` (so every `n` is another key).
fn numbered_key(engine: &Arc<Engine>, n: i32) -> Key {
    let mut session = target(engine);
    session.set_work_date(1000 + n);
    Key::of(&mut session, Default::default(), SessionFlags::default(), &[]).expect("a key")
}

/// [`numbered_key`] as a start is kept under it: by a session that read no resource file.
fn numbered_start_key(engine: &Arc<Engine>, n: i32) -> crate::login_replay::StartKey {
    crate::login_replay::StartKey::of(numbered_key(engine, n), &target(engine))
}

/// A start whose image holds `bytes` of text in a SingleInstance codeunit's global: `kept_bytes` is a little more.
fn start_of_size(engine: &Arc<Engine>, l: &Lowered, bytes: usize) -> Start {
    let mut session = al_vm::Session::new(engine.clone());
    let big = l.program.objects.iter().position(|o| o.name == "RPL Big").unwrap();
    session.codeunit_instance(big).borrow_mut().globals[0] = Value::Text("x".repeat(bytes).into());
    session.commit_transaction();
    let base = session.login_base();
    Start {
        image: session
            .login_image(&base)
            .unwrap_or_else(|why| panic!("{}", why.reason())),
        wrote: Default::default(),
        hits: HitBits::EMPTY,
    }
}

const BIG: &str = r#"
codeunit 77490 "RPL Big"
{
    SingleInstance = true;
    var Payload: Text;
}
"#;

const MIB: usize = 1 << 20;

fn big_engine() -> (Arc<Engine>, Lowered) {
    compiled_with(&format!("{MAIN}{BIG}"))
}

#[test]
fn the_caps_are_eight_starts_sixty_four_mb_and_sixteen_mb_a_start() {
    assert_eq!(
        (DEFAULT_STARTS, DEFAULT_BYTES, MAX_START_BYTES),
        (8, 64 << 20, 16 << 20)
    );
}

// With nothing set, a run's options and the environment give the default caps, and a new engine's starts hold them.
#[test]
fn with_nothing_set_the_run_keeps_eight_starts_and_sixty_four_mb() {
    let defaults = LoginReplayLimits {
        max_starts: 8,
        max_bytes: 64 << 20,
    };
    assert_eq!(LoginReplayLimits::default(), defaults);
    assert_eq!(RunOptions::default().login_replay_limits, defaults);
    assert_eq!(LoginReplayLimits::read(|_| None), Ok(defaults));
    let (engine, l) = big_engine();
    let st = starts(&engine);
    for n in 0..9 {
        st.record(numbered_start_key(&engine, n), Ok(start_of_size(&engine, &l, 1000)));
    }
    assert_eq!(st.kept(), 8, "the ninth start pushes the first out");
}

/// The caps `max_starts` starts and `mib` MiB.
fn limits(max_starts: usize, mib: u64) -> LoginReplayLimits {
    LoginReplayLimits {
        max_starts,
        max_bytes: mib << 20,
    }
}

// A run that sets the count keeps exactly that many starts, above the default too, the least recently used out.
#[test]
fn a_set_count_keeps_that_many_starts_least_recently_used_out() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    st.set_limits(limits(12, 64));
    for n in 0..12 {
        st.record(numbered_start_key(&engine, n), Ok(start_of_size(&engine, &l, 1000)));
    }
    assert_eq!(st.kept(), 12, "twelve starts, beyond the default eight");
    // Key 0 is used again: key 1 is now the least recently used.
    assert!(st.take(&numbered_key(&engine, 0), &engine).is_some());
    st.record(numbered_start_key(&engine, 12), Ok(start_of_size(&engine, &l, 1000)));
    assert_eq!(st.kept(), 12, "the thirteenth start pushes one out");
    assert!(
        st.take(&numbered_key(&engine, 1), &engine).is_none(),
        "the least recently used is out"
    );
    for n in [0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
        assert!(st.take(&numbered_key(&engine, n), &engine).is_some(), "key {n}");
    }
    // A lower count (a kept engine under the next run's options) drops the least recently used at once: 10, 11, 12 stay.
    st.set_limits(limits(3, 64));
    assert_eq!(st.kept(), 3);
    for n in [10, 11, 12] {
        assert!(st.take(&numbered_key(&engine, n), &engine).is_some(), "key {n}");
    }
}

// A run that sets the megabytes binds the starts by their bytes: once the total would pass it, the least recently used
// go; one start over it is not kept (`too large`), even below the 16 MB of one start.
#[test]
fn set_megabytes_bind_the_starts_by_bytes() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    st.set_limits(limits(100, 20));
    // Two starts of 9 MiB fit in 20 MiB (18 MiB and their small parts), a third does not: the oldest goes.
    for n in 0..2 {
        st.record(numbered_start_key(&engine, n), Ok(start_of_size(&engine, &l, 9 * MIB)));
    }
    assert_eq!(st.kept(), 2);
    st.record(numbered_start_key(&engine, 2), Ok(start_of_size(&engine, &l, 9 * MIB)));
    assert_eq!(st.kept(), 2, "a third start of 9 MiB would pass 20 MiB");
    assert!(st.bytes() <= 20 << 20, "{}", st.bytes());
    assert!(st.take(&numbered_key(&engine, 0), &engine).is_none());
    assert!(st.take(&numbered_key(&engine, 2), &engine).is_some());
    // A start of 12 MiB alone passes 10 MiB: not kept, and the kept ones stay.
    st.set_limits(limits(100, 10));
    assert_eq!(st.kept(), 1, "a lower cap drops what passes it at once");
    st.record(numbered_start_key(&engine, 3), Ok(start_of_size(&engine, &l, 12 * MIB)));
    assert!(st.take(&numbered_key(&engine, 3), &engine).is_none());
    assert_eq!(st.kept(), 1);
    let line = st.line().expect("a line");
    assert!(line.contains("1 too large"), "{line}");
}

// A start is kept with its bytes counted.
#[test]
fn a_recorded_start_is_kept_and_its_bytes_are_counted() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    let start = start_of_size(&engine, &l, 2 * MIB);
    let bytes = start.image.kept_bytes();
    assert!(bytes > 2 * MIB as u64, "kept_bytes counts the text: {bytes}");
    st.record(numbered_start_key(&engine, 0), Ok(start));
    assert_eq!(st.kept(), 1);
    assert_eq!(st.bytes(), bytes);
    assert!(st.take(&numbered_key(&engine, 0), &engine).is_some());
    assert!(st.knows(&numbered_key(&engine, 0), &engine));
}

// At most eight starts, the least recently used out; a take refreshes.
#[test]
fn starts_are_bounded_by_eight_least_recently_used_out() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    for n in 0..DEFAULT_STARTS as i32 {
        st.record(numbered_start_key(&engine, n), Ok(start_of_size(&engine, &l, 1000)));
    }
    assert_eq!(st.kept(), 8);
    // Key 0 is used again: key 1 is now the least recently used.
    assert!(st.take(&numbered_key(&engine, 0), &engine).is_some());
    st.record(numbered_start_key(&engine, 8), Ok(start_of_size(&engine, &l, 1000)));
    assert_eq!(st.kept(), 8, "the ninth start pushes one out");
    assert!(
        st.take(&numbered_key(&engine, 1), &engine).is_none(),
        "the least recently used is out"
    );
    for n in [0, 2, 3, 4, 5, 6, 7, 8] {
        assert!(st.take(&numbered_key(&engine, n), &engine).is_some(), "key {n}");
    }
}

// At most 64 MB of kept bytes; an eviction gives its bytes back to the count the memory pool reads.
#[test]
fn starts_are_bounded_by_sixty_four_mb_and_an_eviction_returns_the_bytes() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    // Seven starts of 9 MiB fit (63 MiB plus their small parts), an eighth does not: the oldest goes.
    for n in 0..7 {
        st.record(numbered_start_key(&engine, n), Ok(start_of_size(&engine, &l, 9 * MIB)));
    }
    assert_eq!(st.kept(), 7);
    let seven = st.bytes();
    assert!(seven > 63 * MIB as u64 && seven <= DEFAULT_BYTES, "{seven}");
    st.record(numbered_start_key(&engine, 7), Ok(start_of_size(&engine, &l, 9 * MIB)));
    assert_eq!(
        st.kept(),
        7,
        "an eighth start of 9 MiB would pass 64 MB: the least recently used is out"
    );
    assert!(st.bytes() <= DEFAULT_BYTES, "{}", st.bytes());
    assert!(st.take(&numbered_key(&engine, 0), &engine).is_none());
    assert!(st.take(&numbered_key(&engine, 7), &engine).is_some());
    // The bytes of the one pushed out are not counted any more: 7 starts, not 8.
    assert!(st.bytes() < seven + 100 * 1024, "{} against {seven}", st.bytes());
}

// A start over 16 MB is not kept, with its own reason `too large`, so one start never empties the others.
#[test]
fn a_start_over_sixteen_mb_is_not_kept_and_says_too_large() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    st.record(numbered_start_key(&engine, 0), Ok(start_of_size(&engine, &l, 17 * MIB)));
    assert_eq!(st.kept(), 0);
    assert_eq!(st.bytes(), 0);
    let line = st.line().expect("a line");
    assert!(line.contains("too large"), "{line}");
    // The key is remembered as refused: its session is not frozen again.
    assert!(st.knows(&numbered_key(&engine, 0), &engine));
    // A start of 15 MiB is kept: the bound is on the start, not near it.
    st.record(numbered_start_key(&engine, 1), Ok(start_of_size(&engine, &l, 15 * MIB)));
    assert_eq!(st.kept(), 1);
}

// A refused key (no image) is remembered with why, so its session runs the login every time.
#[test]
fn a_refused_key_is_remembered_and_never_taken() {
    let (engine, _) = compiled();
    let st = starts(&engine);
    let key = numbered_key(&engine, 0);
    assert!(!st.knows(&key, &engine));
    st.record(numbered_start_key(&engine, 0), Err(al_vm::NotImaged::Field("tasks")));
    assert!(st.knows(&key, &engine));
    assert!(st.take(&key, &engine).is_none());
    assert_eq!(st.kept(), 0);
    assert!(st.line().is_some_and(|l| l.contains("field tasks")));
}

// Two threads that meet one new key both run the login; the first `record` wins and one start is kept.
#[test]
fn two_threads_with_one_new_key_keep_one_start() {
    let (engine, l) = compiled();
    let states: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| scope.spawn(|| sessions(&engine, &l, "RPL Tests A", 1, &on())))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for ran in &states {
        assert_one_state(ran, "a thread");
    }
    assert_eq!(starts(&engine).kept(), 1);
}

// ---- the rows of a start are the login's own ----

const BULK: &str = r#"
table 77480 "RPL Bulk" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 77481 "RPL Seed Small" { trigger OnRun() var R: Record "RPL Bulk"; i: Integer; begin for i := 1 to 50 do begin R.Id := i; R.Insert(); end; end; }
codeunit 77482 "RPL Seed Large" { trigger OnRun() var R: Record "RPL Bulk"; i: Integer; begin for i := 1 to 5000 do begin R.Id := i; R.Insert(); end; end; }
"#;

/// The bytes the one start of "RPL Tests A" keeps over the base left by codeunit `seed`'s OnRun.
fn kept_over_base(seed: &str) -> u64 {
    let (engine, l) = compiled_with(&format!("{MAIN}{BULK}"));
    let mut session = al_vm::Session::new(engine.clone());
    let s = l.program.objects.iter().position(|o| o.name == seed).unwrap();
    let on_run = l.program.objects[s]
        .routines
        .iter()
        .position(|r| r.name == "OnRun")
        .unwrap();
    let inst = session.instance(s);
    assert!(session.call(s, on_run, inst, Vec::new()).is_ok());
    let baseline = session.capture_baseline();
    let ran = sessions_with(
        &engine,
        &l,
        "RPL Tests A",
        2,
        &on(),
        &Setup {
            baseline: Some(&baseline),
            ..Default::default()
        },
    );
    assert_one_state(&ran, seed);
    assert_eq!(starts(&engine).kept(), 1, "{seed}");
    starts(&engine).bytes()
}

// An install codeunit that leaves 5,000 rows does not make the start larger than one over 50 rows: the start keeps the rows
// The login wrote, as a delta over the session's own store.
#[test]
fn a_start_from_a_large_install_base_keeps_only_the_logins_rows() {
    let (small, large) = (kept_over_base("RPL Seed Small"), kept_over_base("RPL Seed Large"));
    assert!(small > 0, "kept_bytes counts what the start holds: {small}");
    assert!(
        large.abs_diff(small) < 16 * 1024,
        "5,000 rows of base keep {large} bytes, 50 keep {small}: the base is in the start"
    );
}

// The number sequences' refusal mark the runner sets before the login (`Scratch::numbers_unmeasured`,
// al-runner issue 8152) is an input of the key, and the thaw keeps the target's own: two sessions that differ only in it
// never share a start.
#[test]
fn the_numbers_mark_the_runner_sets_is_another_key_and_the_thaw_keeps_the_targets() {
    let (engine, _) = compiled();
    let key_of = |mark: Option<&str>| {
        let mut session = target(&engine);
        if let Some(why) = mark {
            session.mark_numbers_unmeasured(std::rc::Rc::from(why));
        }
        Key::of(&mut session, Default::default(), SessionFlags::default(), &[]).expect("a key")
    };
    assert_eq!(key_of(None), key_of(None), "GUARD");
    assert_ne!(key_of(None), key_of(Some("an earlier codeunit took numbers")));
    // A start taken without the mark, thawn into a target with it: the target keeps its mark.
    let start = plain_start(&engine);
    let mut marked = target(&engine);
    marked.mark_numbers_unmeasured(std::rc::Rc::from("marked"));
    let before = marked.state_lines();
    marked
        .thaw_login_image(&start.image)
        .expect("a thaw target takes the start");
    let kept = |s: &al_vm::StateLines| {
        s.lines
            .iter()
            .find(|l| l.starts_with("field.numbers_unmeasured"))
            .cloned()
    };
    assert_eq!(kept(&marked.state_lines()), kept(&before));
    assert!(kept(&before).is_some_and(|l| l.contains("marked")));
}

// The key's digest of each committed table is kept per state of the table (its stamp), so a later
// session whose tables a write did not reach digests nothing again; a write is a new state, so another digest.
#[test]
fn a_tables_digest_is_kept_per_state_and_a_write_makes_another() {
    use al_data::Store;
    let (engine, _) = compiled();
    let at = engine
        .program
        .objects
        .iter()
        .position(|o| o.name == "Guided Experience Item")
        .unwrap();
    let table = engine.tables[at].clone().unwrap();
    let mut session = target(&engine);
    let cx = al_data::AccessContext {
        company: session.access_company().into(),
        user: session.user_id().into(),
    };
    let row = |id: i32| {
        let mut r: Vec<Value> = table.fields.iter().map(|f| Value::default_for(&f.ty)).collect();
        r[0] = Value::Integer(id);
        r
    };
    assert!(session.store_mut().insert(&cx, &table, row(1)).is_ok());
    session.commit_transaction();
    let key = |s: &mut al_vm::Session| Key::of(s, Default::default(), SessionFlags::default(), &[]).expect("a key");
    let first = key(&mut session);
    assert_eq!(
        key(&mut session),
        first,
        "the kept digest is the digest of the same rows"
    );
    // Another session over the same rows from another history (another stamp) has the same key.
    let mut other = target(&engine);
    assert!(other.store_mut().insert(&cx, &table, row(1)).is_ok());
    other.commit_transaction();
    assert_eq!(key(&mut other), first, "the digest is of the rows, not of the stamp");
    assert!(session.store_mut().insert(&cx, &table, row(2)).is_ok());
    session.commit_transaction();
    assert_ne!(key(&mut session), first, "a write is another state of the table");
}

/// Base Application's login code that sets WorkDate (`install::login_work_date`): its first run is recorded per engine
/// (`install::LoginMemo`) and a later session takes the recorded answer without calling it, so real logins of one key make
/// other numbers of calls (`Session::calls_made`).
const LOGIN_MANAGEMENT: &str = r#"
codeunit 40 LogInManagement
{
    SingleInstance = true;
    procedure GetDefaultWorkDate(): Date begin exit(Last()); end;
    local procedure Last(): Date begin exit(20260115D); end;
}
"#;

// A counter read only as a difference around one call (`calls_made` and its kind) is no
// input or output of the login: real logins of one key differ in it (the WorkDate memo), and the check must not report
// that as a difference, real against real or replay against run.
#[test]
fn a_call_count_that_differs_between_real_logins_of_one_key_is_no_difference() {
    let (engine, l) = compiled_with(&format!("{MAIN}{LOGIN_MANAGEMENT}"));
    let baseline = al_vm::Session::new(engine.clone()).capture_baseline();
    let check = RunOptions {
        login_replay_check: true,
        ..Default::default()
    };
    let setup = Setup {
        baseline: Some(&baseline),
        ..Default::default()
    };
    sessions_with(&engine, &l, "RPL Tests A", 3, &check, &setup);
    let report = crate::login_replay::report(&engine).expect("a report");
    let memo = engine.derived(crate::install::LoginMemo::default);
    assert_eq!(
        memo.replays.load(std::sync::atomic::Ordering::Relaxed),
        2,
        "GUARD: sessions 2 and 3 take the memo"
    );
    assert!(report.contains("2 compared, 0 differ"), "{report}");
    assert!(report.contains("replay vs run: 2 checked, 0 differ"), "{report}");
}

// ---- The key hashes each row's bytes; a row with no byte form gives no key ----

/// A session of `engine` holding a committed row of "Guided Experience Item" whose second field (when it has one) or first is
/// `v`, and the install baseline captured from it.
fn baseline_with_value(engine: &Arc<Engine>, id: i32, v: Value) -> al_vm::Baseline {
    use al_data::Store;
    let mut session = al_vm::Session::new(engine.clone());
    let at = (engine.program.objects.iter())
        .position(|o| o.name == "Guided Experience Item")
        .unwrap();
    let t = engine.tables[at].clone().unwrap();
    let mut row: Vec<Value> = t.fields.iter().map(|f| Value::default_for(&f.ty)).collect();
    row[0] = Value::Integer(id);
    *row.last_mut().unwrap() = v;
    let cx = al_data::AccessContext {
        company: session.access_company().into(),
        user: session.user_id().into(),
    };
    assert!(session.store_mut().insert(&cx, &t, row).is_ok());
    session.commit_transaction();
    session.capture_baseline()
}

// A committed row that holds a value with no byte form (a SQL counter, a placeholder's size) gives no key: the
// sessions run their logins (nothing replays) and pass as before. The starts line counts them in a part of its own, `no
// key`, not among the gate's refusals, and the report prints it with the replay alone (no check line).
#[test]
fn a_row_with_no_byte_form_gives_no_key_and_the_session_runs_its_login() {
    use al_builtins::value::{SqlCount, SqlForm, SqlType};
    let count = Value::SqlCount(SqlCount {
        n: 3,
        form: SqlForm::Delta,
        ty: SqlType::BigInteger,
    });
    let size = Value::PlaceholderSize("mark".into(), SqlType::Integer);
    for (what, v) in [("SqlCount", count), ("PlaceholderSize", size)] {
        let (engine, l) = compiled();
        let baseline = baseline_with_value(&engine, 99, v);
        let ran = sessions_with(
            &engine,
            &l,
            "RPL Tests A",
            2,
            &on(),
            &Setup {
                baseline: Some(&baseline),
                ..Default::default()
            },
        );
        for (i, o) in ran.outcomes.iter().enumerate() {
            assert_eq!(*o, passes(), "{what}: session {i}");
        }
        assert_eq!(starts(&engine).replayed(), 0, "{what}: no key, so no start is taken");
        let line = format!(
            "login start: 0 replayed, 0 kept (0.0 MB), 0 not imaged, 0 too large, 0 refused by the gate, 2 no key (2: \
             no byte form: {what})"
        );
        assert_eq!(starts(&engine).line().as_deref(), Some(line.as_str()), "{what}");
        assert_eq!(
            crate::login_replay::report(&engine),
            Some(line),
            "{what}: the report prints it"
        );
    }
}

// Under the check every session takes a key, also one whose login is not raised: such a session has nothing to replay,
// so a missing key is not counted for it (the apps under test's own login alone is not raised).
#[test]
fn a_session_whose_login_is_not_raised_is_not_counted_as_no_key() {
    use al_builtins::value::{SqlCount, SqlForm, SqlType};
    let count = Value::SqlCount(SqlCount {
        n: 3,
        form: SqlForm::Delta,
        ty: SqlType::BigInteger,
    });
    let (engine, l) = compiled_with(super::login_replay_tests::OWN_LOGIN);
    let baseline = baseline_with_value(&engine, 99, count);
    let check = RunOptions {
        login_replay_check: true,
        ..Default::default()
    };
    let setup = Setup {
        baseline: Some(&baseline),
        ..Default::default()
    };
    sessions_with(&engine, &l, "RPL Own Tests", 2, &check, &setup);
    assert_eq!(starts(&engine).line(), None);
    assert_eq!(crate::login_replay::report(&engine), None);
}

/// [`MAIN`] with the Company table, a table of the apps under test whose rows the database holds in company `Other`
/// ([`OtherCompanyRows`]), a login subscriber that makes company `Other` and deletes it with its rows, and a test that
/// makes the company again and reads its table there: a company deleted and made again starts empty
/// (record/TestChangeCompanyFiltersCopyDeleteContracts.al:204-212; al-vm `store/companies.rs`
/// `deleting_and_recreating_a_selected_backup_company_starts_empty`).
const DELETES_A_COMPANY: &str = r#"
table 2000000006 Company { DataPerCompany = false;
  fields { field(1; Name; Text[30]) { } field(2; "Evaluation Company"; Boolean) { } field(3; "Display Name"; Text[250]) { } field(8000; Id; Guid) { } }
  keys { key(PK; Name) { } } }
table 77496 "RPL Company Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 77497 "RPL Company Remover"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup()
    var Co: Record Company;
    begin
        Co.Name := 'Other'; Co.Insert(); Co.Delete(true);
    end;
}
codeunit 77498 "RPL Company Tests"
{
    Subtype = Test;
    [Test]
    procedure TheDeletedCompanyStartsEmpty()
    var Co: Record Company; R: Record "RPL Company Row";
    begin
        Co.Name := 'Other'; Co.Insert();
        R.ChangeCompany('Other');
        if R.FindFirst() then Error('a row of the deleted company');
    end;
}
"#;

/// A database whose company `Other` holds one row of "RPL Company Row"; the session company holds none.
struct OtherCompanyRows;

impl al_host::testdata::TableProvider for OtherCompanyRows {
    fn company(&self) -> &str {
        "CRONUS International Ltd."
    }

    fn known_companies(&self) -> Option<Vec<String>> {
        Some(vec!["CRONUS International Ltd.".into(), "Other".into()])
    }

    fn source(&self) -> String {
        "other company rows".into()
    }

    fn read(
        &self,
        table: &al_host::testdata::TableRequest,
    ) -> Result<Arc<al_host::testdata::TableRows>, al_host::testdata::TableError> {
        self.read_for_company("CRONUS International Ltd.", table)
    }

    fn read_for_company(
        &self,
        company: &str,
        table: &al_host::testdata::TableRequest,
    ) -> Result<Arc<al_host::testdata::TableRows>, al_host::testdata::TableError> {
        match (table.name.as_str(), company) {
            ("RPL Company Row", "Other") => Ok(Arc::new(al_host::testdata::TableRows {
                columns: vec!["Id".into()],
                rows: vec![vec![al_host::testdata::Cell::Int(1)]],
            })),
            _ => Err(al_host::testdata::TableError::Absent),
        }
    }
}

/// [`compiled_with`] `src`, on a database of [`OtherCompanyRows`].
fn compiled_over_other_company_rows(src: &str) -> (Arc<Engine>, Lowered) {
    let (mut engine, l) = compiled_with(src);
    Arc::get_mut(&mut engine).expect("a new engine").test_data =
        Some(al_host::testdata::TestData::new(Arc::new(OtherCompanyRows)));
    (engine, l)
}

// Maintainer's review of al-runner PR 8665: a login that deletes a company takes the rows the database holds in it
// (`MemStore::drop_company`, a mark the rows do not show). A later session of the key answers as a real login's: the
// company made again starts empty.
#[test]
fn a_login_that_deletes_a_company_leaves_it_deleted_in_every_session_of_its_key() {
    let src = format!("{MAIN}{DELETES_A_COMPANY}");
    let passes = vec![("TheDeletedCompanyStartsEmpty".to_string(), Outcome::Pass)];
    let (off_engine, l) = compiled_over_other_company_rows(&src);
    let real = sessions(&off_engine, &l, "RPL Company Tests", 1, &RunOptions::default());
    assert_eq!(
        real.outcomes,
        std::slice::from_ref(&passes),
        "GUARD: a real login's session passes"
    );
    let (engine, l) = compiled_over_other_company_rows(&src);
    let ran = sessions(&engine, &l, "RPL Company Tests", 3, &on());
    assert_eq!(ran.outcomes, vec![passes; 3]);
    // The rows do not carry the deleted company: the image refuses the session, and every session runs its login.
    assert_eq!(starts(&engine).replayed(), 0);
    let line = starts(&engine).line().unwrap_or_default();
    assert!(line.contains("1 not imaged (1: rows: "), "{line}");
    for (i, s) in ran.states.iter().enumerate() {
        assert_eq!(first_difference(s, &real.states[0]), None, "session {i}");
    }
}

/// An app under test whose subscriber of the raised login (`MAIN`'s) keeps the day it ran on (`Today`) in a SingleInstance
/// codeunit, an install codeunit that sets WorkDate (which every session then starts with, `InstallLeft`), and a test that
/// the kept day is the session's own.
const KEEPS_TODAY: &str = r#"
codeunit 77400 "RPL Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup()
    var G: Codeunit "Guided Experience"; Day: Codeunit "RPL Day";
    begin
        G.InsertAssistedSetup(77400);
        Day.Keep(Today);
    end;
}
codeunit 77405 "RPL Day"
{
    SingleInstance = true;
    var Kept: Date;
    procedure Keep(D: Date) begin Kept := D; end;
    procedure Kept(): Date begin exit(Kept); end;
}
codeunit 77407 "RPL Install Work Date"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin WorkDate(20240101D); end;
}
codeunit 77402 "RPL Tests A"
{
    Subtype = Test;
    [Test]
    procedure SeesTheRow() var Day: Codeunit "RPL Day"; begin if Day.Kept() <> Today then Error('login day %1, today %2', Day.Kept(), Today); end;
}
"#;

// The run clock is begun per run (`host::RunClock`, so per request of a daemon): a session whose run began on another
// UTC day than the session that kept a start holds that other day's clock, and the login it would replay stamped the
// first day. With no WorkDate set the session lines hold the clock's day (`work_date_day`), but install code that set
// WorkDate leaves every session that date, so only `Key::clock` holds the day apart: the session runs its own login
// and sees its own `Today`.
#[test]
fn a_session_whose_run_clock_began_on_another_day_runs_its_own_login() {
    const DAY: i64 = 86_400_000;
    let (engine, l) = compiled_with(KEEPS_TODAY);
    let nth = std::cell::Cell::new(0);
    let next_day = |s: &mut Scratch| {
        s.clock = crate::host::RunClock::At(1_700_000_000_000 + nth.get() * DAY);
        nth.set(nth.get() + 1);
    };
    let setup = Setup {
        tweak: Some(&next_day),
        ..Setup::default()
    };
    let ran = sessions_with(&engine, &l, "RPL Tests A", 2, &on(), &setup);
    for (i, o) in ran.outcomes.iter().enumerate() {
        assert_eq!(*o, passes(), "session {i} (day {i}) sees the day its own login ran on");
    }
    assert_eq!(starts(&engine).replayed(), 0, "a start of another day is never taken");
}

// ---- the memory limit: the serve budget holds the room the byte cap leaves the starts (al-runner issue 8211) ----

/// How many test threads a soft budget (serve's) of 2000 MB admits beside 500 MB resident, at a learned cost of 100 MB
/// a codeunit, on an engine that keeps one start of 4 MiB under the caps of `opts`.
fn threads_admitted(opts: &RunOptions) -> usize {
    use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
    const MB: u64 = 1 << 20;
    let (engine, l) = big_engine();
    starts(&engine).set_limits(opts.login_replay_limits);
    starts(&engine).record(numbered_start_key(&engine, 0), Ok(start_of_size(&engine, &l, 4 * MIB)));
    let rss = Arc::new(AtomicU64::new(500 * MB));
    let read = rss.clone();
    let budget = super::memory::MemoryBudget {
        limit: 2000 * MB,
        source: "maxMemory".into(),
        rss: Arc::new(move || Some(read.load(SeqCst))),
        hard: false,
    };
    let pool = super::memory::Pool::new(Some(budget), 500 * MB, 8);
    super::corpus::share_with_pool(&pool, &engine, None, opts);
    let first = pool.enter();
    rss.store(600 * MB, SeqCst);
    drop(first);
    rss.store(500 * MB, SeqCst);
    let mut members = Vec::new();
    while let Some(m) = pool.try_add(0) {
        members.push(m);
    }
    1 + members.len()
}

// A raised byte cap counts in the soft budget before a thread is admitted: 1020 MB of room the starts may still take
// leave room for two threads, where the default cap leaves room for the eight the pool may use. No start is dropped.
#[test]
fn a_raised_byte_cap_lowers_the_threads_a_soft_budget_admits() {
    let on = |mib: u64| RunOptions {
        login_replay: true,
        login_replay_limits: limits(8, mib),
        ..Default::default()
    };
    assert_eq!(threads_admitted(&on(64)), 8, "60 MB of room fit beside eight threads");
    assert_eq!(threads_admitted(&on(1024)), 2, "1020 MB of room leave two");
    // The check keeps starts too; a run that keeps none holds no room.
    let check = RunOptions {
        login_replay_check: true,
        ..on(1024)
    };
    assert_eq!(threads_admitted(&check), 2);
    let off = RunOptions {
        login_replay: false,
        ..on(1024)
    };
    assert_eq!(threads_admitted(&off), 8, "no start is kept: nothing held");
}
