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
use crate::login_start::{self, MAX_BYTES, MAX_START_BYTES, MAX_STARTS, NotTaken, Start, starts};
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
            ..Default::default()
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
            key.holds_session(twin),
            "twin {i} before its thaw is not the session before its login: {:?}",
            twin.lines
                .iter()
                .filter(|l| l.to_lowercase().contains("work"))
                .collect::<Vec<_>>()
        );
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

/// The key of a new session of `engine` whose login skipped `skipped`.
fn key_skipping(engine: &Arc<Engine>, skipped: &[al_vm::SkippedInstall]) -> Key {
    let mut session = al_vm::Session::new(engine.clone());
    Key::of(&mut session, Default::default(), SessionFlags::default(), skipped)
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
            ..Default::default()
        };
        login_start::why_not(&target(&engine), &start, run)
    };
    assert_eq!(why(true, false, false), Some(NotTaken::NotesCode));
    assert_eq!(why(false, true, false), Some(NotTaken::Perturbed));
    assert_eq!(why(false, false, true), Some(NotTaken::MovedIn));
}

// The wall clock is an input no key holds, as for the install baseline cache (`runtime/baseline_key.rs`): a kept serve
// daemon under `ALRUNNER_REAL_CLOCK` would otherwise hand a later session the time an earlier login stamped into a row.
// Such a run neither takes nor records a start, with its own reason (issue 8211).
#[test]
fn the_gate_names_the_real_clock() {
    let (engine, _) = compiled();
    let start = plain_start(&engine);
    let run = login_start::RunFacts {
        real_clock: true,
        ..Default::default()
    };
    assert_eq!(
        login_start::why_not(&target(&engine), &start, run),
        Some(NotTaken::RealClock)
    );
}

// A real files directory (`--files-dir`) is an input no key holds, as for the install baseline cache: a login that reads a
// file an earlier test wrote would read what no key holds. Such a run neither takes nor records a start, with its own
// reason (issue 8211).
#[test]
fn the_gate_names_a_real_files_directory() {
    let (engine, _) = compiled();
    let start = plain_start(&engine);
    let run = login_start::RunFacts {
        real_files: true,
        ..Default::default()
    };
    assert_eq!(
        login_start::why_not(&target(&engine), &start, run),
        Some(NotTaken::RealFiles)
    );
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
/// the first's start. Returns the report line of the starts.
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
    (starts(&engine).replayed(), starts(&engine).line())
}

// At run level: a run that follows the code it runs, a perturbation and a set clock start never take a start, and the
// Report counts the sessions the gate refused.
#[test]
fn a_session_the_gate_refuses_runs_its_login_and_is_counted() {
    let perturbed = |s: &mut Scratch| s.perturb.library_seed = Some(3);
    let clocked = |s: &mut Scratch| s.clock_start = Some(1_700_000_000_000);
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

// ---- the caps: 8 starts, 64 MB, a start of at most 16 MB, least recently used out ----

/// The key of a new session of `engine` whose work date is `n` (so every `n` is another key).
fn numbered_key(engine: &Arc<Engine>, n: i32) -> Key {
    let mut session = al_vm::Session::new(engine.clone());
    session.set_work_date(1000 + n);
    Key::of(&mut session, Default::default(), SessionFlags::default(), &[])
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
    assert_eq!((MAX_STARTS, MAX_BYTES, MAX_START_BYTES), (8, 64 << 20, 16 << 20));
}

// A start is kept with its bytes counted.
#[test]
fn a_recorded_start_is_kept_and_its_bytes_are_counted() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    let start = start_of_size(&engine, &l, 2 * MIB);
    let bytes = start.image.kept_bytes();
    assert!(bytes > 2 * MIB as u64, "kept_bytes counts the text: {bytes}");
    st.record(numbered_key(&engine, 0), Ok(start));
    assert_eq!(st.kept(), 1);
    assert_eq!(st.bytes(), bytes);
    assert!(st.take(&numbered_key(&engine, 0)).is_some());
    assert!(st.knows(&numbered_key(&engine, 0)));
}

// At most eight starts, the least recently used out; a take refreshes.
#[test]
fn starts_are_bounded_by_eight_least_recently_used_out() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    for n in 0..MAX_STARTS as i32 {
        st.record(numbered_key(&engine, n), Ok(start_of_size(&engine, &l, 1000)));
    }
    assert_eq!(st.kept(), 8);
    // Key 0 is used again: key 1 is now the least recently used.
    assert!(st.take(&numbered_key(&engine, 0)).is_some());
    st.record(numbered_key(&engine, 8), Ok(start_of_size(&engine, &l, 1000)));
    assert_eq!(st.kept(), 8, "the ninth start pushes one out");
    assert!(
        st.take(&numbered_key(&engine, 1)).is_none(),
        "the least recently used is out"
    );
    for n in [0, 2, 3, 4, 5, 6, 7, 8] {
        assert!(st.take(&numbered_key(&engine, n)).is_some(), "key {n}");
    }
}

// At most 64 MB of kept bytes; an eviction gives its bytes back to the count the memory pool reads.
#[test]
fn starts_are_bounded_by_sixty_four_mb_and_an_eviction_returns_the_bytes() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    // Seven starts of 9 MiB fit (63 MiB plus their small parts), an eighth does not: the oldest goes.
    for n in 0..7 {
        st.record(numbered_key(&engine, n), Ok(start_of_size(&engine, &l, 9 * MIB)));
    }
    assert_eq!(st.kept(), 7);
    let seven = st.bytes();
    assert!(seven > 63 * MIB as u64 && seven <= MAX_BYTES, "{seven}");
    st.record(numbered_key(&engine, 7), Ok(start_of_size(&engine, &l, 9 * MIB)));
    assert_eq!(
        st.kept(),
        7,
        "an eighth start of 9 MiB would pass 64 MB: the least recently used is out"
    );
    assert!(st.bytes() <= MAX_BYTES, "{}", st.bytes());
    assert!(st.take(&numbered_key(&engine, 0)).is_none());
    assert!(st.take(&numbered_key(&engine, 7)).is_some());
    // The bytes of the one pushed out are not counted any more: 7 starts, not 8.
    assert!(st.bytes() < seven + 100 * 1024, "{} against {seven}", st.bytes());
}

// A start over 16 MB is not kept, with its own reason `too large`, so one start never empties the others.
#[test]
fn a_start_over_sixteen_mb_is_not_kept_and_says_too_large() {
    let (engine, l) = big_engine();
    let st = starts(&engine);
    st.record(numbered_key(&engine, 0), Ok(start_of_size(&engine, &l, 17 * MIB)));
    assert_eq!(st.kept(), 0);
    assert_eq!(st.bytes(), 0);
    let line = st.line().expect("a line");
    assert!(line.contains("too large"), "{line}");
    // The key is remembered as refused: its session is not frozen again.
    assert!(st.knows(&numbered_key(&engine, 0)));
    // A start of 15 MiB is kept: the bound is on the start, not near it.
    st.record(numbered_key(&engine, 1), Ok(start_of_size(&engine, &l, 15 * MIB)));
    assert_eq!(st.kept(), 1);
}

// A refused key (no image) is remembered with why, so its session runs the login every time.
#[test]
fn a_refused_key_is_remembered_and_never_taken() {
    let (engine, _) = compiled();
    let st = starts(&engine);
    let key = numbered_key(&engine, 0);
    assert!(!st.knows(&key));
    st.record(key.clone(), Err(al_vm::NotImaged::Field("tasks")));
    assert!(st.knows(&key));
    assert!(st.take(&key).is_none());
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
        let mut session = al_vm::Session::new(engine.clone());
        if let Some(why) = mark {
            session.mark_numbers_unmeasured(std::rc::Rc::from(why));
        }
        Key::of(&mut session, Default::default(), SessionFlags::default(), &[])
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
    let key = |s: &mut al_vm::Session| Key::of(s, Default::default(), SessionFlags::default(), &[]);
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
