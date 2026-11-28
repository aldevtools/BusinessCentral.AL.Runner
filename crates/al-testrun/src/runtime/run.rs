//! Running the test codeunits of one compiled program, each test on a session of its own.
//!
//! Entry points: `run_codeunit`, `run_codeunit_isolated`, `run_codeunit_observed`,
//! `run_codeunit_core`.
use super::al_runner::run_unit;
use super::held_login::{
    self, carried_company_open, first_login, held_reads, later_runner_marks, login_once_on_hub, login_rows_as_held,
    runs_own_login, thaw_held,
};
use super::written::{
    self, ReachFacts, ReachIndex, remainder_reach_facts, settled_flags, test_reach_facts, unfinished_reach_facts,
};
use super::*;
// What a reader of a table a skipped install holds on assumption gets (`blocked.rs` `Assumed`).
use super::Assumed::{Marked, Refused};

/// Outcomes of every `[Test]` procedure of test codeunit `object`, in order.
pub fn run_codeunit(engine: &Arc<Engine>, lowered: &Lowered, object: usize) -> Vec<(String, Outcome)> {
    run_codeunit_from(engine, lowered, object, None)
}

/// [`run_codeunit`] on a session that starts from the install baseline (ADR-013 decision 5), under
/// `TestIsolation = Codeunit`: the isolation every corpus oracle was recorded under.
pub fn run_codeunit_from(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    object: usize,
    baseline: Option<&al_vm::Baseline>,
) -> Vec<(String, Outcome)> {
    run_codeunit_isolated(engine, lowered, object, baseline, TestIsolation::Codeunit)
}

/// One isolation unit (see `isolation.rs`): a test codeunit run to its end on a session of its own.
pub fn run_codeunit_isolated(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    object: usize,
    baseline: Option<&al_vm::Baseline>,
    isolation: TestIsolation,
) -> Vec<(String, Outcome)> {
    run_codeunit_observed(engine, lowered, object, baseline, isolation, None)
}

/// What an observer sees of a test that has just ended.
pub struct Observed<'a> {
    /// The session.
    pub session: &'a Session,
    /// Where the error or the refusal of the test was raised from, outermost first (tracing is on when there is an observer).
    pub site: &'a [al_vm::FaultFrame],
    /// The error that failed the test.
    pub error: Option<&'a AlError>,
    /// The routine a refusal decided before the run belongs to (object, routine).
    pub refused_in: Option<(usize, usize)>,
}

impl<'a> Observed<'a> {
    /// A test that ran: the site is the session's.
    fn of_test(session: &'a Session, error: Option<&'a AlError>, refused_in: Option<(usize, usize)>) -> Self {
        let site = session.fault_site();
        Observed {
            session,
            site,
            error,
            refused_in,
        }
    }
}

/// Called with the test's name and outcome after each test of the codeunit that ran.
pub type Observer<'a> = dyn FnMut(&str, &Outcome, &Observed) + 'a;

/// [`run_codeunit_isolated`] that tells `observe` how each test ended and turns on the session's
/// fault tracing for it (`--explain`). The call is a run of its own: it begins its clock. A caller whose baseline was
/// made at another clock passes that one to [`run_codeunit_full`].
pub fn run_codeunit_observed(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    object: usize,
    baseline: Option<&al_vm::Baseline>,
    isolation: TestIsolation,
    observe: Option<&mut Observer>,
) -> Vec<(String, Outcome)> {
    run_codeunit_full(
        engine,
        lowered,
        object,
        baseline,
        isolation,
        observe,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        None,
        crate::host::RunClock::begin(),
    )
}

/// [`run_codeunit_isolated`] where a test that runs longer than `limit` is stopped and not run. A run of its own, as
/// [`run_codeunit_observed`].
pub fn run_codeunit_limited(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    object: usize,
    baseline: Option<&al_vm::Baseline>,
    isolation: TestIsolation,
    limit: Option<std::time::Duration>,
) -> Vec<(String, Outcome)> {
    run_codeunit_full(
        engine,
        lowered,
        object,
        baseline,
        isolation,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        limit,
        crate::host::RunClock::begin(),
    )
}

/// The frames BC's test runner has below a test, as `SessionInformation.Callstack` shows them
/// (corpus session-system/TestSessionInformationCallstackContracts.al:89-97 and the failure texts of the
/// corpus CI on 27.5, 28.5 and 29.0, where the lines are the same). The runner's own app is
/// `Test Runner by Microsoft`; its version is the loaded app's, else the run's BC version.
///
/// The frames follow the routing of the test codeunit: one BC runs on the TestRunnerHub (`hub_leg`, `corpus.rs` `on_hub`:
/// on BC 28 and later a codeunit without `[HandlerFunctions]` and with `TestPermissions = Disabled`, or one with
/// RequiredTestIsolation Function or Disabled) shows no runner frame from BC 28 on, so the text ends at the separator
/// (session-system/callstack/CallstackRunnerFrames.al:17-20, every `Plain` case, BC 28.5 and 29.0; the hub adds no AL frame).
/// Every other codeunit, and on BC 27 every codeunit, runs under Microsoft's runner 130450
/// (codeunits/run-onrun/RqIsolationModes.al:9-12) and shows the six frames (CallstackRunnerFrames.al, every `Handler` case
/// on BC 27.5, 28.5 and 29.0 and every `Plain` case on 27.5). A run without the corpus routing (`hub_leg` not set) shows
/// the six frames, as before the hub leg was told apart.
pub(crate) fn runner_frames(program: &al_ir::Program, engine: &Engine, hub_leg: bool) -> String {
    if hub_leg
        && engine
            .target
            .as_ref()
            .is_some_and(|t| t.has(al_version::Gate::CallStackRunnerFramesGoneFrom28))
    {
        return String::new();
    }
    let version = program
        .apps
        .iter()
        .find(|a| a.name == "Test Runner" && a.publisher == "Microsoft")
        .map(|a| a.version.clone())
        .or_else(|| {
            engine
                .target
                .as_ref()
                .map(|t| format!("{}.{}.0.0", t.bc.major, t.bc.minor))
        })
        .unwrap_or_else(|| "0.0.0.0".into());
    const FRAMES: [&str; 6] = [
        "\"Test Runner - Mgt\"(CodeUnit 130454).RunTests line 26",
        "\"Test Runner - Isol. Codeunit\"(CodeUnit 130450).OnRun(Trigger) line 4",
        "\"Test Suite Mgt.\"(CodeUnit 130456).RunTests line 2",
        "\"Test Suite Mgt.\"(CodeUnit 130456).RunSelectedTests line 35",
        "\"Test Suite Mgt.\"(CodeUnit 130456).RunNextTest line 20",
        "\"Command Line Test Tool\"(Page 130455).\"RunNextTest - OnAction\"(Trigger) line 7",
    ];
    FRAMES
        .iter()
        .map(|f| format!("{f} - Test Runner by Microsoft version {version}\\"))
        .collect()
}

/// [`run_codeunit_observed`] that also collects, for each test that failed with an AL error, the AL
/// call stack of that error as `(test name, stack)` (`GetLastErrorCallStack` form, innermost first). The sessions read
/// `clock`, the clock of the run `baseline` belongs to (ADR-045, amendment of 2026-11-22).
#[allow(clippy::too_many_arguments)]
pub fn run_codeunit_full(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    object: usize,
    baseline: Option<&al_vm::Baseline>,
    isolation: TestIsolation,
    observe: Option<&mut Observer>,
    stacks: &mut Vec<(String, String)>,
    times: &mut Vec<(String, std::time::Duration)>,
    hits: &mut Vec<(String, Vec<Assumption>)>,
    limit: Option<std::time::Duration>,
    clock: crate::host::RunClock,
) -> Vec<(String, Outcome)> {
    run_codeunit_core(
        engine,
        lowered,
        object,
        baseline,
        isolation,
        observe,
        stacks,
        times,
        hits,
        &mut Scratch::new(clock),
        limit,
        false,
    )
}

/// What the test session does when it opens the company, before the codeunit's OnRun and first test: the login code that
/// sets WorkDate, then the Company Triggers subscribers and the login (`install::test_session_company_open`, under
/// RequiredTestIsolation = Disabled), committed; then the `workDate.offset` of a stability run. With it, the marks of the
/// assumptions this step relied on: they count for every test of the session (`mark_test_start_assumptions`). A session
/// whose login inputs an earlier session of the engine had takes what that login left in place of running it
/// (`login_start.rs`, al-runner issue 8211), unless the gate keeps it from the start (`login_start::run_gate`,
/// `login_start::target_gate`, each reason counted). A scratch run (`execute`) is not a test session: it opens nothing.
/// `Err`: why the codeunit is not run.
fn open_company(
    start: &Start,
    session: &mut Session,
    skipped_installs: &[al_vm::SkippedInstall],
    perturb: &crate::stability::Perturb,
    scratch: &Scratch,
    after_refusal: bool,
) -> Result<(crate::install::CompanyOpenWrites, al_builtins::assumptions::HitBits), crate::install::NotOpened> {
    let (steps, isolation) = (&start.steps, start.steps.isolation);
    if steps.execute {
        return Ok(Default::default());
    }
    // The run's one login a Disabled codeunit on the hub held (`login_once_on_hub`): a later hub codeunit takes its
    // session state in place of a second login; one runner 130450 runs runs its own login on the held rows.
    let held = session.login_held();
    let own_login = held.is_some() && runs_own_login(scratch.hub_leg, isolation);
    let thawed = held
        .as_deref()
        .filter(|_| !own_login)
        .and_then(|h| thaw_held(session, h));
    let thaws = thawed.is_some();
    let (mut wrote, hits) = match thawed {
        Some(opened) => opened,
        None => {
            // On the hub BC runs no second login (LgnDisabledLoginWrites.al:471, :546): what this one writes is undone.
            let before = (held.is_some() && !own_login).then(|| session.capture_baseline());
            // The session before the run's first login, for the image a Disabled codeunit holds (`first_login`).
            let base = first_login(scratch.hub, isolation, session).then(|| session.login_base());
            let (wrote, mut hits) = run_login(start, session, skipped_installs, perturb, scratch, after_refusal)?;
            if let Some(before) = before {
                session.undo_login_writes(&before);
                hits = hits.with(al_builtins::assumptions::Assumption::HubDisabledLoginOnce);
            }
            if let Some(base) = base.filter(|_| login_once_on_hub(scratch.hub, isolation, false, &wrote)) {
                let image = session.login_image(&base).map_err(|not| not.reason());
                let opened = held_login::HeldOpened::of(session, wrote.clone(), hits);
                session.hold_login(image, Box::new(opened));
            }
            (wrote, hits)
        }
    };
    let rows_as_held = held
        .as_deref()
        .is_some_and(|h| !own_login && login_rows_as_held(h, isolation, session));
    wrote.held = held
        .as_deref()
        .and_then(|h| held_reads(h, own_login, thaws, rows_as_held));
    let hits = later_runner_marks(held.is_some(), scratch.hub_leg, isolation, hits);
    // `workDate.offset` of a stability run (ADR-058): the work date the session starts with, after the login code set it;
    // a `WorkDate(d)` of the test still wins.
    if let Some(shift) = perturb.work_date {
        let day = session.work_date();
        session.set_work_date(shift.apply_day(day));
    }
    Ok((wrote, hits))
}

/// The login of the test session (see [`open_company`]): replayed from a kept start, or run.
fn run_login(
    start: &Start,
    session: &mut Session,
    skipped_installs: &[al_vm::SkippedInstall],
    perturb: &crate::stability::Perturb,
    scratch: &Scratch,
    after_refusal: bool,
) -> Result<(crate::install::CompanyOpenWrites, al_builtins::assumptions::HitBits), crate::install::NotOpened> {
    let steps = &start.steps;
    let (lowered, isolation, observed, moved) = (steps.lowered, steps.isolation, steps.observed, steps.moved_in);
    let disabled = isolation == TestIsolation::Disabled;
    let engine = session.engine.clone();
    al_builtins::assumptions::clear_hits();
    let (flags, check) = (scratch.flags, scratch.flags.login_replay_check());
    // Al-runner issue 8211: what the login can read, before it runs (the key of a login start, or of the check). Only a
    // raised login can be replayed: a session whose login is not raised takes no key. A retry on a new session
    // (`Start::again`, `after_refusal`) takes its own key; the call that refused ends at `?` below unnoted, so only the
    // session that opens the company is checked or recorded.
    let raises = || {
        (session.test_object).is_some_and(|t| {
            crate::login::raises(
                &engine,
                lowered,
                t,
                crate::login::watched(session, disabled),
                after_refusal,
            )
        })
    };
    // No key (a value with no comparable form, a row with no byte form): the session runs its login, counted below
    // when the login is raised.
    let (key, no_key) = match (check || (flags.login_replay() && raises()))
        .then(|| crate::login_replay::Key::of(session, (isolation, scratch.hub), flags, skipped_installs))
    {
        Some(Ok(key)) => (Some(key), None),
        Some(Err(no_key)) => (None, Some(no_key)),
        None => (None, None),
    };
    let notes_code = observed || session.notes_code_run() || crate::profiling::active();
    let perturbed = *perturb != crate::stability::Perturb::default() || scratch.clock_set;
    let facts = crate::login_start::RunFacts {
        notes_code,
        perturbed,
        moved_in: moved,
    };
    let gate = crate::login_start::run_gate(facts);
    let replayed =
        (key.as_ref().filter(|_| !check)).and_then(|key| crate::login_start::replay(&engine, key, session, gate));
    let (wrote, hits) = match replayed {
        Some(opened) => opened,
        None => {
            // The session before the login, for a start that will be recorded or for the check: it clones the store.
            let records = |key: &&crate::login_replay::Key| {
                check || (gate.is_none() && !crate::login_start::starts(&engine).knows(key, &engine))
            };
            let base = key.as_ref().filter(records).map(|_| session.login_base());
            #[cfg(test)]
            crate::login_start::BASES.with(|b| b.set(b.get() + u32::from(base.is_some())));
            // Base Application's login code sets WorkDate when the session opens the company (`install::login_work_date`).
            // It runs before the workspace's own Company Triggers subscribers: the order of the two is not pinned
            // (docs/probes-needed.md, WorkDate row).
            crate::install::login_work_date(&engine, lowered, session)?;
            let wrote =
                crate::install::test_session_company_open(lowered, session, disabled, skipped_installs, after_refusal)?;
            session.commit_transaction();
            let hits = al_builtins::assumptions::take_hit_bits();
            if let (Some(no_key), true) = (&no_key, wrote.raised) {
                crate::login_start::no_key(&engine, no_key);
            }
            if let (Some(key), Some(base), true) = (key, base, wrote.raised) {
                // Without the check, a base is taken only for a start that will be recorded (`records`).
                match check {
                    true => {
                        // The check's twin: brought to its login by the steps of `Start::open`, as the replay's target
                        // is when it takes a start (`Steps::twin`). The install baseline is frozen for it only here.
                        let on = start.twin_baseline();
                        let twin = || steps.twin(scratch, on.as_ref());
                        crate::login_replay::checked(&engine, session, key, &base, &wrote, hits, gate, twin)
                    }
                    false => crate::login_start::record(&engine, key, session, &base, &wrote, hits),
                }
                #[cfg(test)]
                crate::login_replay::serve_twin(session, &base, || steps.twin(scratch, start.twin_baseline().as_ref()));
            }
            (wrote, hits)
        }
    };
    #[cfg(test)]
    crate::login_start::opened(session, &wrote, hits);
    Ok((wrote, hits))
}

/// The session enters test codeunit `object` after the install code: the settings of its isolation, and the refusal of
/// every NumberSequence call when an earlier codeunit's numbers are not known here (`Scratch::numbers_unmeasured`).
fn enter_test_codeunit(session: &mut Session, scratch: &Scratch, object: usize, isolation: TestIsolation) {
    if let Some(why) = &scratch.numbers_unmeasured {
        session.mark_numbers_unmeasured(std::rc::Rc::from(&**why));
    }
    session.mark_install_end();
    session.in_test_codeunit = true;
    session.isolation_from_hub = scratch.hub && isolation != TestIsolation::Codeunit;
    // StartSession starts a session under Disabled isolation, from the hub (RqIsolationModes.al:573-600) and from a runner
    // (corpus PR 735, SdIsolStartSession.Codeunit.al:20-22): refused there, as no second session is modelled.
    session.hub_disabled = isolation == TestIsolation::Disabled;
    session.test_object = Some(object);
}

/// Every test of the codeunit (`all`) not run, when the code that opens the company did not end (`not`). `--explain`:
/// each test shows the stack the call that did not end left from, or the subscriber refused at lowering, as a test
/// refused while it ran or before it does (al-runner issue 8142).
fn observed_not_run(
    observe: Option<&mut Observer>,
    session: &Session,
    not: &crate::install::NotOpened,
    all: impl Fn(Outcome) -> Vec<(String, Outcome)>,
) -> Vec<(String, Outcome)> {
    let refused = all(Outcome::NotRun(not.why.clone()));
    if let Some(observe) = observe {
        let seen = Observed {
            session,
            site: &not.site,
            error: None,
            refused_in: not.refused_in,
        };
        for (name, outcome) in &refused {
            observe(name, outcome, &seen);
        }
    }
    refused
}

/// The tables `watched` and those of `skipped_installs` (`skipped_install_tables`), for the store's watch.
fn with_skipped<'a>(
    watched: impl Iterator<Item = &'a u32>,
    skipped_installs: &[al_vm::SkippedInstall],
) -> BTreeSet<u32> {
    let mut out: BTreeSet<u32> = watched.copied().collect();
    out.extend(skipped_install_tables(skipped_installs));
    out
}

/// The reason of the first skipped install that wrote a table `routine` can reach. The tables the routine reaches are
/// found once, and only when an install was skipped: the walk of its calls cost as much as the test for every skipped install.
fn skipped_install_touching<'a>(
    skipped_installs: &'a [al_vm::SkippedInstall],
    lowered: &Lowered,
    object: usize,
    routine: usize,
    assumed: Assumed,
) -> Option<&'a String> {
    if skipped_installs.is_empty() {
        return None;
    }
    let touched = tables_touched(lowered, object, routine);
    let refuses = |s: &al_vm::SkippedInstall| {
        !s.tables.is_disjoint(&touched) || (assumed == Assumed::Refused && !s.assumed.is_disjoint(&touched))
    };
    skipped_installs.iter().find(|s| refuses(s)).map(|s| &s.why)
}

/// The object id and name of the test runner codeunit `object`, for isolation::failed_test_runner_measured.
/// BC's TestRunnerHub under RequiredTestIsolation = Function: the NumberSequence log of the codeunit begins before
/// OnRun (isolation::after_test, al-vm `hub_sequences.rs`).
fn begin_hub_log(session: &mut Session, isolation: TestIsolation, hub: bool, execute: bool) {
    if isolation == TestIsolation::Function && hub && !execute {
        session.begin_hub_sequence_log();
    }
}

fn runner_key(lowered: &Lowered, object: Option<usize>) -> Option<(u32, &str)> {
    object
        .map(|i| &lowered.program.objects[i])
        .map(|o| (o.id.unwrap_or(0), &*o.name))
}

/// The runner commits a test that passes: with a table still marked by Consistent(false) what BC does there is not pinned
/// (only Commit() is, errors/CuCommitTests.al:405-419), so the test is not run. `handled`: every handler the test
/// declares ran.
fn refuse_inconsistent_end<T>(
    session: &Session,
    result: Result<T, Signal>,
    model: al_vm::TxModel,
    handled: bool,
) -> Result<T, Signal> {
    match result {
        Ok(_) if model != al_vm::TxModel::AutoRollback && handled => match session.inconsistent_table() {
            Some(t) => Err(Signal::unsupported(format!(
                "the test ends while Consistent(false) marks table {t}: the runner's commit is not pinned"
            ))),
            None => result,
        },
        r => r,
    }
}

/// The data-driven tests of a codeunit: procedures with `[TestDataSource(Codeunit, DataSet)]` and no `[Test]`.
fn data_driven_tests(lowered: &Lowered, object: usize) -> Vec<usize> {
    let obj = &lowered.program.objects[object];
    (0..obj.routines.len())
        .filter(|&r| {
            let a = &obj.routines[r].attributes;
            a.iter().any(|a| a.name.eq_ignore_ascii_case("TestDataSource"))
                && !a.iter().any(|a| a.name.eq_ignore_ascii_case("Test"))
        })
        .collect()
}

/// [`run_codeunit_full`], or with `execute` the `OnRun` trigger of the codeunit run as its one test, with the text of
/// the `Message` calls it made kept in `messages` (a scratch run). Refuses the BC 29 test framework features the runner
/// does not run yet; [`run_codeunit_inner`] runs the rest.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_codeunit_core(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    object: usize,
    baseline: Option<&al_vm::Baseline>,
    isolation: TestIsolation,
    observe: Option<&mut Observer>,
    stacks: &mut Vec<(String, String)>,
    times: &mut Vec<(String, std::time::Duration)>,
    hits: &mut Vec<(String, Vec<Assumption>)>,
    scratch: &mut Scratch,
    limit: Option<std::time::Duration>,
    execute: bool,
) -> Vec<(String, Outcome)> {
    let outcomes = run_codeunit_checked(
        engine, lowered, object, baseline, isolation, observe, stacks, times, hits, scratch, limit, execute,
    );
    // The codeunit's session has ended: the instances only a reference cycle of its AL code keeps are freed, so a warm
    // server's memory does not grow with each request (al-runner issue 7975).
    al_builtins::instance_cycles::collect();
    outcomes
}

#[allow(clippy::too_many_arguments)]
fn run_codeunit_checked(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    object: usize,
    baseline: Option<&al_vm::Baseline>,
    isolation: TestIsolation,
    observe: Option<&mut Observer>,
    stacks: &mut Vec<(String, String)>,
    times: &mut Vec<(String, std::time::Duration)>,
    hits: &mut Vec<(String, Vec<Assumption>)>,
    scratch: &mut Scratch,
    limit: Option<std::time::Duration>,
    execute: bool,
) -> Vec<(String, Outcome)> {
    if execute {
        return run_codeunit_inner(
            engine, lowered, object, baseline, isolation, observe, stacks, times, hits, scratch, limit, execute,
        );
    }
    let refusal = (engine.events)
        .absent_publisher_subscriber(&lowered.program.objects[object], object)
        .or_else(|| engine.events.dependency_platform_by_id())
        .or_else(|| super::test_handlers::refusal(lowered, object));
    match refusal {
        Some(why) => {
            let obj = &lowered.program.objects[object];
            obj.routines
                .iter()
                .filter(|r| {
                    r.attributes
                        .iter()
                        .any(|a| a.name.eq_ignore_ascii_case("Test") || a.name.eq_ignore_ascii_case("TestDataSource"))
                })
                .filter(|r| {
                    scratch
                        .run_only
                        .as_ref()
                        .is_none_or(|keep| keep.contains(&(obj.id.unwrap_or(0), r.name.to_lowercase())))
                })
                .map(|r| (r.name.clone(), Outcome::NotRun(why.clone())))
                .collect()
        }
        None => run_codeunit_inner(
            engine, lowered, object, baseline, isolation, observe, stacks, times, hits, scratch, limit, execute,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_codeunit_inner(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    object: usize,
    baseline: Option<&al_vm::Baseline>,
    isolation: TestIsolation,
    mut observe: Option<&mut Observer>,
    stacks: &mut Vec<(String, String)>,
    times: &mut Vec<(String, std::time::Duration)>,
    hits: &mut Vec<(String, Vec<Assumption>)>,
    scratch: &mut Scratch,
    limit: Option<std::time::Duration>,
    execute: bool,
) -> Vec<(String, Outcome)> {
    let obj = &lowered.program.objects[object];
    let perturb = scratch.perturb.clone();
    let run_seed = scratch.run_seed;
    let (mut session, moved) = new_session(engine, session_host(scratch), baseline, execute);
    set_up_session(&mut session, scratch, observe.is_some());
    let covering = crate::coverage::enabled();
    let is_on_run = |r: usize| obj.routines[r].is_trigger && obj.routines[r].name.eq_ignore_ascii_case("OnRun");
    let tests: Vec<usize> = (0..obj.routines.len())
        .filter(|&r| {
            if execute {
                return is_on_run(r);
            }
            obj.routines[r]
                .attributes
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case("Test"))
                // `--run-selected-only`: a test that is not selected does not run (issue 5611).
                && scratch.run_only.as_ref().is_none_or(|keep| {
                    keep.contains(&(obj.id.unwrap_or(0), obj.routines[r].name.to_lowercase()))
                })
        })
        .collect();
    // `order.methods` of a stability run (ADR-058): the test methods in another order.
    let tests: Vec<usize> = perturb
        .method_order
        .arrange(tests.len())
        .into_iter()
        .map(|i| tests[i])
        .collect();
    if execute {
        session.capture_messages();
    }
    // The data-driven procedures (ADR-059): they run after the plain tests, once per case (`data_driven.rs`).
    let data_driven: Vec<usize> = if execute {
        Vec::new()
    } else {
        data_driven_tests(lowered, object)
            .into_iter()
            .filter(|&r| {
                scratch
                    .run_only
                    .as_ref()
                    .is_none_or(|keep| keep.contains(&(obj.id.unwrap_or(0), obj.routines[r].name.to_lowercase())))
            })
            .collect()
    };
    let all = |o: Outcome| {
        tests
            .iter()
            .chain(&data_driven)
            .map(|&r| (obj.routines[r].name.clone(), o.clone()))
            .collect::<Vec<_>>()
    };
    // The company the tests run in is not the one asked for (`--company-data demotool` whose codeunit stopped): no test
    // runs against a part of it.
    if let Some(why) = &engine.company_refusal {
        return all(Outcome::NotRun(why.clone()));
    }
    // A test runner codeunit with TestIsolation = Disabled (al-runner --isolation disabled) runs: what it keeps between
    // tests and for the later codeunits is the probe of corpus writer SD (tests/al-language-isolation-probe/src, runner 61304,
    // real BC 27.5, 28.5 and 29.0 on Windows containers; the corpus nightly runs only 61300 and 61301). BC's TestRunnerHub under a codeunit's RequiredTestIsolation = Disabled is pinned
    // (QuCommitIsolation.al:572-718, `Scratch::hub`).
    let runner_disabled = isolation == TestIsolation::Disabled && !scratch.hub && !execute;
    // A test runner codeunit with TestIsolation = Function (al-runner --isolation test), not the hub.
    let runner_function = isolation == TestIsolation::Function && !scratch.hub && !execute;
    let models = match transaction_models(obj, tests.iter().chain(&data_driven)) {
        Ok(models) => models,
        Err(why) => return all(Outcome::NotRun(why)),
    };
    let start = Start {
        steps: Steps {
            engine,
            lowered,
            object,
            installs: super::install_once::FromBaseline::of(baseline),
            isolation,
            execute,
            observed: observe.is_some(),
            moved_in: moved.is_some(),
        },
        baseline,
    };
    let mut opened = start.open(&mut session, scratch, &perturb, false);
    // Microsoft's login code refused where only System Application's login memos made it decide (`login::decide`): the
    // codeunit starts again from a new session, with the apps under test's own login.
    if opened.as_ref().is_err_and(|not| not.retry_own_login) {
        session = start.again(scratch, moved.as_ref());
        opened = start.open(&mut session, scratch, &perturb, true);
    }
    let (skipped_installs, (company_open, company_open_marks)) = match opened {
        Ok(opened) => opened,
        Err(not) => return observed_not_run(observe.as_deref_mut(), &session, &not, all),
    };
    // SingleInstance state an earlier test codeunit left (ADR-066 amendment 2): notes from here on, and in the lane the
    // single instances of the codeunit before are moved in.
    start_singles(&mut session, execute, moved, scratch.watch);
    if scratch.discovery.is_some() {
        return discover(
            &mut session,
            lowered,
            object,
            &company_open,
            &skipped_installs,
            scratch,
            limit,
        );
    }
    // What a Disabled codeunit leaves for the later codeunits is its writes from here on ([`CARRIED`]). With
    // Company Triggers subscribers each session writes on its own when it opens the company, and how those writes
    // meet the ones carried over is not pinned, except for the run's one login on the hub (`login_once_on_hub`).
    let carries = isolation == TestIsolation::Disabled && !execute;
    let held = session.login_held().is_some();
    let company_open_marks = match carried_company_open(held, scratch.hub, isolation, execute, carries, &company_open) {
        Ok(true) => company_open_marks.with(al_builtins::assumptions::Assumption::HubDisabledLoginOnce),
        Ok(false) => company_open_marks,
        Err(why) => return all(Outcome::NotRun(why.into())),
    };
    let changes_from = session.store_mut().change_counter();
    let row_versions_from = session.row_versions_handed_out();
    let carried_from = carries.then(|| session.capture_baseline());
    // Under runner 130450 what the OnRun trigger writes reaches the later codeunits (`Scratch::onrun_carry`).
    let onrun_from = (scratch.onrun_carry && !execute).then(|| session.capture_baseline());
    let mut onrun_left: Option<Carried> = None;
    // The tables an earlier Disabled codeunit left rows in (`Scratch::leak`): a read of one is noted, and the codeunit
    // then runs again from what that codeunit left (`corpus.rs`).
    let leak: BTreeSet<u32> = scratch.leak.as_deref().cloned().unwrap_or_default();
    // The reach facts of a routine of this codeunit and the handler functions it names, for the writes of the Company
    // Triggers subscribers.
    let company_open_refuses = |r: usize| -> Option<String> {
        company_open.can_refuse().then_some(())?;
        let index = engine.derived(|| ReachIndex::new(&engine.events, lowered));
        company_open.refuses(&test_reach_facts(&engine.events, lowered, &index, object, r), lowered)
    };
    // A test codeunit with SingleInstance = true runs its tests on the session's single instance: a variable of its own type
    // and its manual subscriber share its globals, and the second test sees what the first left
    // (codeunits/QiSingleInstanceTestCodeunit.al:57 QI_SI_1_OwnTypeVariableAndManualSubscriber, :98 QI_SI_2_SecondTestSeesTheFirst).
    // Taken after the install code and the company open, which drop the session's single instances.
    // A codeunit with data-driven tests (ADR-059) stays on al-runner's own runner: their rows and the runner's answers for
    // them are not built yet.
    let al_runner_object = scratch.runner.filter(|_| data_driven.is_empty());
    // Corpus PR 1465 measured what a failed test leaves under this runner (isolation::failed_test_runner_measured).
    let runner = runner_key(lowered, al_runner_object);
    let measured_runner = crate::isolation::failed_test_runner_measured(isolation, runner, scratch.hub, execute);
    // The test codeunit run: called here, or from the OnRun of the test runner codeunit `al_runner_object` when it runs
    // the test codeunit (ADR-066 decision 1, `al_runner.rs`). An error is a result that skips what follows below.
    let ends = std::cell::Cell::new(super::Ends::default());
    let mut inner = |session: &mut Session,
                     runner: &super::al_runner::Steps|
     -> Result<super::al_runner::Outcomes, super::al_runner::Outcomes> {
        let instance = session.codeunit_instance(object);
        begin_hub_log(session, isolation, scratch.hub, execute);
        let measured_bindings = measured_runner.then_some((lowered, &instance));
        // The ITestHandler callbacks (BC 29): OnBeforeTestCodeunitRun comes before OnRun (BiTestHandlerDefault.al:15, :329).
        let th = super::test_handlers::Handlers::begin(session, lowered, object, execute)
            .map_err(|w| all(Outcome::NotRun(w)))?;
        let on_run_rows = scratch.on_run_rows;
        // A refused OnRun: whether it would raise is not known, so neither is whether the tests run.
        let on_run_refused = |why: String| all(Outcome::NotRun(why));
        // Under `OnRunRows::TestRunner`, the tests of a codeunit with a declared OnRun that did not raise: their rows rest on
        // the unprobed reporting of OnRun (ADR-055).
        let mut on_run_shape_assumed = false;
        if let Some(on_run) = (0..obj.routines.len())
            .find(|&r| obj.routines[r].is_trigger && obj.routines[r].name.eq_ignore_ascii_case("OnRun"))
            .filter(|_| !execute)
        {
            if let Some(reason) = blocked_reason(lowered, object, on_run) {
                return Err(on_run_refused(format!("OnRun: {reason}")));
            }
            if let Some(reason) = company_open_refuses(on_run) {
                return Err(on_run_refused(format!("OnRun: {reason}")));
            }
            // What the partner dependency apps' login subscribers left is watched too (`partner_login.rs`).
            let partner = &company_open.partner_login;
            let watched = with_skipped(leak.iter().chain(partner.watched()), &skipped_installs);
            session.store_mut().watch(watched);
            // Al-runner issue 5698: the time limit (CPU time, the wall backstop and the Sleep budget) applies to OnRun too,
            // with a budget of its own: its time counts toward no test method. A limit is tooling (ADR-009), not something
            // AL code observes.
            session.clear_stop();
            let watchdog = limit.map(|l| Watchdog::start_paused(session.stop_flag(), l, None, None));
            session.start_sleep_budget(limit);
            // What OnRun runs through does not count for the first test.
            al_builtins::assumptions::clear_hits();
            if let Err(why) = runner.before_on_run(session, obj) {
                return Err(on_run_refused(why));
            }
            let ran = session.call(object, on_run, instance.clone(), Vec::new());
            if let Err(why) = runner.after_on_run(session, obj, ran.is_ok(), on_run_error(&ran)) {
                return Err(on_run_refused(why));
            }
            let timed_out = watchdog.and_then(Watchdog::finish);
            let ran = on_run_time_limit(session, ran, limit, timed_out);
            let hits = session.store_mut().watch_hits();
            scratch.leak_hits.extend(leak.intersection(&hits));
            session.store_mut().watch(Default::default());
            if let Some((_, why)) = skipped_install_hit(&skipped_installs, &hits, Refused) {
                return Err(on_run_refused(format!("OnRun: {why}")));
            }
            (partner.after_on_run(&hits, lowered)).map_err(|why| on_run_refused(format!("OnRun: {why}")))?;
            match ran {
                // NavTestCodeunit.DoRunAsync commits after a declared OnRun
                // (codeunits/TestTestCodeunitOnRun.al).
                Ok(_) if session.inconsistent_table().is_some() => {
                    return Err(on_run_refused(
                        "OnRun: ends while Consistent(false) marks a table (the runner's commit is not pinned)".into(),
                    ));
                }
                Ok(_) => {
                    session.commit_transaction();
                    // OnRun ran outside the codeunit isolation of runner 130450: the later codeunits start from what it left,
                    // committed and open, whatever the tests do (RqIsolationModes.al:357-369 #else).
                    if let Some(start) = &onrun_from {
                        let left =
                            super::numbers_taken::onrun_left(session, start, baseline, &skipped_installs, changes_from);
                        onrun_left = Some(match scratch.driver.run_onrun_carries() {
                            true => left,
                            false => super::discovery::run_onrun_left(left, start, &obj.name),
                        });
                    }
                }
                Err(Signal::Unsupported(r)) if r.starts_with(TIME_LIMIT_REASON) => {
                    return Err(on_run_refused(scratch.on_run_cut(obj.id.unwrap_or(0), r.into_reason())));
                }
                Err(Signal::Unsupported(r)) => return Err(on_run_refused(format!("OnRun: {r}"))),
                Err(Signal::Error(e)) => {
                    if on_run_rows == OnRunRows::PerTest {
                        return Err(all(Outcome::NotRun(format!(
                            "OnRun failed (runner behaviour not pinned): {}",
                            e.message
                        ))));
                    }
                    // The error may come from install data this run does not have: it is not trusted.
                    if let Some(why) = skipped_install_touching(&skipped_installs, lowered, object, on_run, Refused) {
                        return Err(on_run_refused(format!(
                            "OnRun failed after install code that did not run: {why}"
                        )));
                    }
                    let marks = al_builtins::assumptions::take_hits();
                    // Microsoft's Test Runner app (ADR-055): the codeunit is Skipped and has no rows, and the run stays
                    // green (al-runner issue 5859, BC 29.0; corpus writer RL's probe c69419f6, BC 27.5). Its text is not
                    // reported, so an unpinned one does not matter; an error reached through an unprobed assumption may not
                    // happen on BC, and then the tests would run.
                    if !marks.is_empty() {
                        let ids: Vec<&str> = marks.iter().map(|a| a.info().id).collect();
                        return Err(all(Outcome::NotRun(format!(
                            "OnRun failed through unprobed assumptions ({}), so whether the tests run is not known: {}",
                            ids.join(", "),
                            e.message
                        ))));
                    }
                    // No row carries the assumption here: the skipped codeunit has none (`test-codeunit-onrun-rows`).
                    return Err(Vec::new());
                }
                Err(_) => {}
            }
            on_run_shape_assumed = on_run_rows == OnRunRows::TestRunner;
        }
        let (checkpoint, mut sequences) = start_isolation(session, isolation, (measured_runner, execute), changes_from);
        // MinimumActiveRowVersion across the tests (al-vm `min_row_version.rs`). When OnRun wrote rows, whether they belong to
        // the isolation transaction is not pinned, and no value is taken.
        if !execute && runner.none_handed_out(session, row_versions_from) {
            start_min_row_version(session, isolation, scratch.hub);
        }
        if scratch.capture.any() {
            session.enable_capture(scratch.capture);
        }
        // `al-runner debug` (ADR-048): the tests run with the debugger; the install code above did not.
        let paused = match crate::events::debugger() {
            Some((hook, paused)) => {
                session.attach_debugger(hook);
                paused
            }
            None => None,
        };
        let mut out: Vec<(String, Outcome)> = Vec::new();
        // Every earlier test of the codeunit that did not run to its end: refused before it started, refused while it ran,
        // or failed. A later failure is not trusted when such a test could have changed what it reads (`written.rs`).
        let mut unfinished: Vec<Unfinished> = Vec::new();
        // Taken at the first failure that has an unfinished test before it; built once per engine, as it depends on the
        // program only.
        let mut reach_index: Option<Arc<ReachIndex>> = None;
        let flags = settled_flags(lowered, object);
        let mut panicked = false;
        // The refusals of the keys and sequences an earlier codeunit's tests changed (isolation::after_test) end here: this
        // codeunit runs on what the earlier one left, as main ran it; whether BC leaves the same is not measured.
        session.clear_earlier_unmeasured();
        // Why the later tests of this codeunit are not run: `--skip-after-timeout` (al-runner issue 7517) after the test the
        // time limit stopped, a stop of the pass (`stop.rs`) after the first row that triggers it.
        let mut stopped: Option<String> = None;
        let mut queue = data_driven::queue(lowered, object, &tests, &data_driven);
        // The rows of `out` that are results of data-driven procedures: BC reports none of them when it ends the session.
        let mut data_driven_rows: Vec<usize> = Vec::new();
        let mut cases_ok = true;
        while let Some(item) = queue.pop_front() {
            let r = item.routine;
            let name = item.name;
            if matches!(item.step, data_driven::Step::Source) {
                match data_driven::list(session, lowered, object, r, &name) {
                    data_driven::Listed::Items(items) => {
                        for it in items.into_iter().rev() {
                            queue.push_front(it);
                        }
                    }
                    data_driven::Listed::SessionEnds => {
                        data_driven::end_session(&mut out, &data_driven_rows, &mut queue, r, &obj.routines)
                    }
                }
                continue;
            }
            let decided = match &item.step {
                data_driven::Step::Decided(o) => Some(o.clone()),
                _ => None,
            };
            let case = match item.step {
                data_driven::Step::Case { source, index, id } => Some((source, index, id)),
                _ => None,
            };
            if !matches!((&case, &decided), (None, None)) {
                data_driven_rows.push(out.len());
            }
            let last_case =
                !matches!(queue.front(), Some(n) if n.routine == r && matches!(n.step, data_driven::Step::Case { .. }));
            // A cancelled run starts no further test (the one running finishes), nor does one stopped at its memory limit.
            if crate::events::cancelled() || crate::events::halted().is_some() {
                break;
            }
            crate::events::starting(obj.id.unwrap_or(0), &obj.routines[r].name);
            // The number of this test for the SingleInstance notes (ADR-066 amendment 7; 0 without notes).
            let test_no = test_begins(scratch, session);
            let test_started = std::time::Instant::now();
            let draws_before = session.inherited_random_draws();
            let carried_before = session.carried_random_draws();
            let bound_before = session.bound_subscriptions();
            // The flags of the codeunit that already hold their only value when this test starts.
            let settled: BTreeSet<(usize, usize)> =
                {
                    let inst = instance.borrow();
                    flags
                .iter()
                .filter(|(g, v)| matches!(inst.globals.get(*g), Some(al_builtins::value::Value::Boolean(b)) if b == v))
                .map(|(g, _)| (object, *g))
                .collect()
                };
            let routine = &obj.routines[r];
            let handlers = resolve_handlers(lowered, object, r);
            let (mut failure, mut refused_in): (Option<AlError>, Option<(usize, usize)>) = (None, None);
            // Why a step of the test runner around this test was refused or failed (runner_reset.rs): what it left is not
            // known, so a later failure of the codeunit is not trusted.
            let mut runner_refused: Option<String> = None;
            // A refusal raised by a statement of the test method itself: the statement's index in the body and the tables
            // the test wrote before the refusal (al-runner issue 5961), for the trust of later failures.
            let mut refused_rest: Option<Rest> = None;
            session.clear_fault_site();
            session.report_refusal_after_denial = refusal_after_denial(engine, lowered, &mut reach_index, object, r);
            al_builtins::assumptions::clear_hits();
            session.take_tdd_reached();
            let mut began = false;
            let mut skipped = false;
            let outcome = if let Some(o) = decided {
                o
            } else if let Some(why) = &stopped {
                scratch.skip(&name, why)
            } else if let Some(b) = case
                .as_ref()
                .and_then(|(src, ..)| blocked_site(lowered, src.object, src.get))
            {
                refused_in = Some((b.object, b.routine));
                Outcome::NotRun(b.text)
            } else if let Err(reason) = &handlers {
                Outcome::NotRun(reason.clone())
            } else if let Some(b) = blocked_site(lowered, object, r) {
                refused_in = Some((b.object, b.routine));
                Outcome::NotRun(b.text)
            } else if let Some(reason) = skipped_install_touching(&skipped_installs, lowered, object, r, Marked) {
                Outcome::NotRun(reason.clone())
            } else if let Some(reason) = company_open_refuses(r) {
                Outcome::NotRun(reason)
            } else {
                let bound = handlers.unwrap_or_default();
                // A scratch run has no UI scope: its `Message` calls are kept, not handled.
                let bound_ok = if execute {
                    Ok(())
                } else {
                    bind_handlers(session, &instance, &bound)
                };
                match bound_ok {
                    Err(reason) => Outcome::NotRun(reason),
                    Ok(()) => {
                        began = true;
                        let watched = with_skipped(company_open.watched().union(&leak), &skipped_installs);
                        session.store_mut().watch(watched);
                        session.last_error = Default::default();
                        al_builtins::assumptions::clear_hits();
                        mark_test_start_assumptions(lowered, on_run_shape_assumed, company_open_marks);
                        assume_kept_bindings(&unfinished, &bound_before);
                        // The runner's steps before the method (runner_reset.rs); what they rely on counts for the test.
                        // Its `LibraryRandom.SetSeed(1)` reseeds the sequence, so draws after it are not inherited.
                        session.start_random_scope();
                        // The run seed (ADR-061): the method's `Random` and Guids derive from it and its own name, so
                        // `--seed N --test X` gives X the values of the full run. A stability variant's own `random.hostSeed`
                        // or `guid.seed` replaces the derived value for that variant.
                        let codeunit_id = obj.id.unwrap_or(0);
                        session.seed_test_method(
                            codeunit_id,
                            &routine.name,
                            perturb.host_seed.is_none(),
                            perturb.guid_seed.is_none(),
                        );
                        // What the test (and the runner's steps around it) writes counts from here.
                        let changes_from = session.store_mut().change_counter();
                        // The session language and WorkDate a refusal while the test runs puts back.
                        let carried = session.carried_state();
                        // A runner codeunit's OnBeforeTestRun that answers false skips the method: no body, no row
                        // (corpus tests/al-language-isolation-probe/src/RpReader.Codeunit.al:20-24 P1; ADR-059's Skipped
                        // outcome is not built yet).
                        let mut reset = runner.before_or_skip(session, obj, routine, &mut skipped);
                        // `random.librarySeed` of a stability run (ADR-058, owner decision 2) or `--seed-library`
                        // (ADR-061): after codeunit 130301, the runner calls Microsoft's own `LibraryRandom.SetSeed(n)` with
                        // the variant's seed, else the one derived from the run seed.
                        let library_seed = perturb.library_seed.or_else(|| {
                            run_seed
                                .library
                                .then(|| al_host::seed::derive_library(run_seed.seed, codeunit_id, &routine.name))
                        });
                        if let (Ok(()), Some(seed)) = (&reset, library_seed) {
                            reset = runner_reset::library_seed(session, seed);
                        }
                        let reset_ok = reset.is_ok();
                        if let (Err(why), false) = (&reset, skipped) {
                            runner_refused = Some(why.clone());
                        }
                        // The AL test runner enables every application area before each test method
                        // (corpus session-system/TestSessionApplicationAreaTestBoundary.al:115-119, :131-134).
                        session.start_test_method();
                        let model = models.get(&r).copied().unwrap_or_default();
                        // TransactionModel::None is handled before the method body: every active
                        // transaction is ended on the way in (codeunits/TestWriteTxTestBoundary.al:
                        // Test08). Nothing is pending here, because every earlier test ended its
                        // transaction at its own boundary, so which way it ends is not observable.
                        if model == al_vm::TxModel::None {
                            session.commit_transaction();
                        }
                        session.tx_model = model;
                        session.clear_stop();
                        // A scratch run is stopped by `cancel` too, and has a watchdog for that even without a limit.
                        let cancel = watched_stop(execute);
                        let watchdog = match (limit, &cancel) {
                            (None, None) => None,
                            (l, _) => Some(Watchdog::start_paused(
                                session.stop_flag(),
                                l.unwrap_or(std::time::Duration::from_secs(u32::MAX.into())),
                                cancel.clone(),
                                paused.clone(),
                            )),
                        };
                        // Under a runner codeunit its own frames are on the call stack.
                        if !runner.is_al() {
                            session.set_runner_frames(runner_frames(&lowered.program, engine, scratch.hub_leg));
                        }
                        session.start_sleep_budget(limit);
                        // A panic in the test is a runtime bug: it is caught here, so that it ends this codeunit and not
                        // the process (a server must outlive it). The session is not trusted after it.
                        let called = match reset {
                            // A refused or failed reset: the test does not run, and what the reset changed stays.
                            Err(why) => Ok(Err(Signal::unsupported(why))),
                            Ok(()) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                // A debugging aid: the named test panics here, to show that a panic ends one codeunit and not the
                                // process (tests of `al-runner serve`). Not documented for users.
                                if std::env::var("ALRUNNER_DEBUG_PANIC_IN").is_ok_and(|n| n == routine.name) {
                                    panic!("requested by ALRUNNER_DEBUG_PANIC_IN");
                                }
                                let at = case
                                    .as_ref()
                                    .map(|(_, index, id)| (id.as_str(), *index == 1, last_case));
                                super::test_handlers::around(
                                    session,
                                    lowered,
                                    &th,
                                    &routine.name,
                                    at,
                                    &mut cases_ok,
                                    |session| {
                                        crate::isolation::noting_body(session, sequences.as_mut(), model, |session| {
                                            match &case {
                                                None => session.call(object, r, instance.clone(), Vec::new()),
                                                Some((source, index, id)) => data_driven::run_case(
                                                    session,
                                                    lowered,
                                                    object,
                                                    r,
                                                    instance.clone(),
                                                    source,
                                                    *index,
                                                    id,
                                                ),
                                            }
                                        })
                                    },
                                )
                            })),
                        };
                        let result = match called {
                            Ok(r) => r,
                            Err(p) => {
                                panicked = true;
                                let msg = p
                                    .downcast_ref::<String>()
                                    .cloned()
                                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                                    .unwrap_or_default();
                                Err(Signal::unsupported(format!("runtime panic (bug): {msg}")))
                            }
                        };
                        let result = runner.rows_touched(session, result);
                        session.set_runner_frames(String::new());
                        let timed_out = watchdog.and_then(Watchdog::finish);
                        let slept_out = session.sleep_budget_passed();
                        // A refusal the method's call raised, whose innermost frame is the method: the statement it came
                        // from. The writes are read before the rollback below undoes them.
                        if reset_ok
                            && !panicked
                            && timed_out.is_none()
                            && !slept_out
                            && matches!(result, Err(Signal::Unsupported(_)))
                        {
                            refused_rest = session
                                .refusal_site()
                                .and_then(|site| site.last().copied())
                                .filter(|f| f.object == object && f.routine == r)
                                .and_then(|f| written::statement_at(lowered, object, r, f.pos))
                                .map(|from| Rest {
                                    from,
                                    handlers: bound.iter().map(|&(_, h)| h).collect(),
                                    written: session.store_mut().tables_changed_since(changes_from),
                                });
                        }
                        session.end_sleep_budget();
                        // The reason names the limit; the VM only knows that it was stopped. Both reasons start with the
                        // same words, which the outputs turn into the timeout row (`Test exceeded <N>s timeout.`).
                        let result = match result {
                            Err(Signal::Unsupported(_)) if slept_out => Err(Signal::unsupported(format!(
                                "the test used more than the time limit of {} s and was stopped (--test-timeout-secs): its Sleep calls asked to wait {} ms in total (Sleep does not wait here; the time it asks for counts toward the limit)",
                                limit.map_or(0, |l| l.as_secs()),
                                session.slept_in_test_ms()
                            ))),
                            Err(Signal::Unsupported(_)) if timed_out.is_some() => {
                                Err(Signal::unsupported(time_limit_reason(timed_out, limit, "")))
                            }
                            r => r,
                        };
                        // ADR-062: a test that lowered its permissions to sets that do not grant everything, and made no
                        // access since that the gate refused, is not run either: no access kind is enforced yet.
                        let result = match result {
                            Ok(_) | Err(Signal::Error(_)) => match session.permissions_lowered_at_end() {
                                Some(why) => Err(Signal::unsupported(why)),
                                None => result,
                            },
                            r => r,
                        };
                        session.tx_model = al_vm::TxModel::AutoCommit;
                        let unexecuted = session.end_ui_scope();
                        let result = refuse_inconsistent_end(session, result, model, unexecuted.is_empty());
                        // A duplicate-key Insert whose error is still deferred when the test ends: the next access to the
                        // table or a Commit raises it (record/TestRecordRuntimeTextsAndViews.al:180-209). At the end of the
                        // test the runner raises it: the test fails with the error, whose AL call stack has no frame of the
                        // test, and is rolled back (tests/al-language-isolation-probe/src/IsolProbeFixture.Codeunit.al:118-160,
                        // T8-T11, under TestIsolation = Codeunit and Function, oracle of BC 28.5). A row committed before stays
                        // under Codeunit; under Function the checkpoint restore removes it (T11). Measured for the default
                        // TransactionModel only: AutoRollback is not run. TestIsolation = Disabled is refused before the run.
                        let result = match result {
                            Ok(_) if unexecuted.is_empty() => match session.deferred_insert_table() {
                                Some(_) if model == al_vm::TxModel::AutoCommit => {
                                    let mut e = session.take_deferred_insert_error().expect("checked");
                                    e.call_stack.clear();
                                    Err(Signal::Error(Box::new(e)))
                                }
                                Some(t) => Err(Signal::unsupported(format!(
                                    "the test ends while the error of a duplicate-key Insert into table {t} is deferred: whether the test runner raises it at the end of the test is measured only for the default TransactionModel"
                                ))),
                                None => result,
                            },
                            r => r,
                        };
                        let outcome = match result {
                            // AutoRollback rolls the test's work back the moment it finishes, pass
                            // or fail (record/TestTransactionModelAutoRollback.al:Test01-Test03).
                            Ok(_) if unexecuted.is_empty() && model == al_vm::TxModel::AutoRollback => {
                                session.rollback_transaction();
                                Outcome::Pass
                            }
                            // The default TransactionModel commits at the test boundary; a failed
                            // test's transaction is rolled back (corpus codeunits/TestWriteTxTestBoundary.al:26).
                            Ok(_) if unexecuted.is_empty() => {
                                session.commit_transaction();
                                Outcome::Pass
                            }
                            // corpus ui-handlers/TestPageQueryCloseMessageHandler_Tests.al:51: a declared
                            // handler that never fires is a test failure. The text after the prefix
                            // is not pinned (docs/probes-needed.md).
                            Ok(_) => {
                                session.rollback_transaction();
                                Outcome::Fail(format!(
                                    "The following UI handlers were not executed: {}",
                                    unexecuted.join(", ")
                                ))
                            }
                            // An uncaught error whose text is not pinned would put a made-up text in the result: the test is not run.
                            // Opt-in (`RunOptions::report_unpinned_errors`, al-runner issue 6487): it fails, its message marked.
                            Err(Signal::Error(e))
                                if al_builtins::native::text_unpinned(&e) && scratch.report_unpinned_errors =>
                            {
                                session.rollback_transaction();
                                let message = al_builtins::native::unpinned_failure_text(&e);
                                failure = Some(*e);
                                Outcome::Fail(message)
                            }
                            Err(Signal::Error(e)) if al_builtins::native::text_unpinned(&e) => {
                                session.rollback_transaction();
                                let Err(Signal::Unsupported(r)) =
                                    al_builtins::native::refuse_unpinned_text(&e, "The test result")
                                else {
                                    unreachable!("an unpinned error is refused")
                                };
                                Outcome::NotRun(r.into_reason())
                            }
                            Err(Signal::Error(e)) => {
                                session.rollback_transaction();
                                let message = e.message.clone();
                                failure = Some(*e);
                                Outcome::Fail(message)
                            }
                            // A refused test did not run: what it wrote before the refusal must not
                            // reach the next test of the codeunit, nor the session language or WorkDate it set.
                            Err(Signal::Unsupported(_)) if execute && crate::events::cancelled() => {
                                crate::stopped_state::roll_back_refused_test(session, carried);
                                Outcome::NotRun("cancelled".into())
                            }
                            Err(Signal::Unsupported(r)) => {
                                crate::stopped_state::roll_back_refused_test(session, carried);
                                Outcome::NotRun(not_run_reason(r))
                            }
                            Err(other) => Outcome::Fail(format!("unexpected signal {}", other.describe())),
                        };
                        // The runner's step after the method. It only changes what later tests of the codeunit start
                        // from; if it cannot run, their failures are not trusted.
                        // A skipped method gets no OnAfterTestRun (RpReader.Codeunit.al:20-24 P1).
                        if let Err(why) = runner.after(session, obj, routine, (&outcome, failure.as_ref()), skipped) {
                            session.rollback_transaction();
                            runner_refused.get_or_insert(why);
                        }
                        outcome
                    }
                }
            };
            if skipped {
                session.store_mut().watch(Default::default());
                al_builtins::assumptions::take_hits();
                continue;
            }
            session.test_method_ended();
            // The test failed, taken before the checks below report a failure as not trusted or not run: its transaction was
            // rolled back either way, and the end of the test puts back what that rollback covers (isolation::after_test).
            let failed = matches!(outcome, Outcome::Fail(_));
            // A test that did not run to its end (refused before it started, refused while it ran, failed) left the state the
            // tests of the codeunit share other than BC did: the database (under TestIsolation = Codeunit), the globals of the
            // codeunit and of the codeunits it holds, single-instance codeunits, and session state such as the `Random`
            // sequence. A failure of a later test is not trusted when an unfinished test's static reach can change what the
            // failing test reads (Tests-ERM "Discount Setup Notifications": the refused test would have reset the posting
            // setups the test before it left blank). A refused test's partial run counts as its whole reach, except for one a
            // statement of its own body refused: what it wrote before the refusal and the reach of the statements from that one
            // on count (`remainder_reach_facts`, al-runner issue 5961). A failed test counts only through the `Random`
            // sequence: its draws are not rolled back, while its database writes are, as they are in BC when the test fails
            // there too. A failure judged not trusted may pass on BC and leave its writes and globals: it counts with its whole
            // reach, as a test refused while it ran (`Ended::Untrusted`).
            // The numbers this test drew from a sequence it did not seed itself: they depend on what earlier tests drew
            // (Tests-SCM-Planning CalcPlanReqWkshForTransferLFLItem: a random end date before a random receipt date).
            let drew = session.inherited_random_draws() > draws_before;
            // Of those, the numbers the run seed did not restart either (ADR-061): only they depend on an earlier test that
            // cannot seed the sequence with `Randomize(n)`.
            let drew_carried = session.carried_random_draws() > carried_before;
            // A test that read or wrote, while it ran, a table a Company Triggers subscriber wrote before the first test
            // (al-runner issue 5641), or one a watched login wrote (`login.rs`): what it found there is not pinned.
            // A read or write of any table an earlier Disabled codeunit left rows in is noted in `leak_hits`, whatever watched
            // table the test touched first (review of al-runner issue 7717, round five, probe p13).
            let touched = session.store_mut().watch_hits();
            let hit = company_open.watch_hit(&touched, began);
            // A test that read or wrote a table install code that could not run touches, through a path its record
            // variables do not show (a RecordRef): not run, as the static check does for a record variable (al-runner
            // issue 8115, scenario s03b).
            let skipped_hit = began
                .then(|| skipped_install_hit(&skipped_installs, &touched, Marked))
                .flatten();
            let outcome = match (hit.and_then(|t| company_open.touched(t, lowered)))
                .or_else(|| skipped_hit.map(|(_, why)| why.clone()))
            {
                Some(why) => Outcome::NotRun(why),
                None => {
                    if began {
                        scratch.leak_hits.extend(leak.intersection(&touched));
                    }
                    outcome
                }
            };
            session.store_mut().watch(Default::default());
            let hit_time_limit = began && matches!(&outcome, Outcome::NotRun(m) if is_time_limit(m));
            // A test the time limit stopped started from the same state as a failure does, so it is judged alike (al-runner
            // issue 5776): `kind` names it in the reason.
            let judged = match outcome {
                Outcome::Fail(m) if !unfinished.is_empty() => Ok(("failure", m)),
                Outcome::NotRun(m) if !unfinished.is_empty() && is_time_limit(&m) => Ok(("timeout", m)),
                o => Err(o),
            };
            // `untrusted`: this test's failure was judged not trusted just above; it holds the failure's own error.
            let (outcome, untrusted) = match judged {
                Ok((kind, m)) => {
                    let engine = session.engine.clone();
                    let index: &ReachIndex =
                        reach_index.get_or_insert_with(|| engine.derived(|| ReachIndex::new(&engine.events, lowered)));
                    // This test's reach is walked only when an unfinished test's reach is fully known: an unknown part
                    // decides alone (`changes_read_by`), and a walk costs about 150 ms with Microsoft's apps.
                    let mut mine: Option<ReachFacts> = None;
                    let found = unfinished.iter_mut().find_map(|u| {
                    // A failed test counts through a manual binding it left that this test started with, and through the
                    // `Random` sequence, which matters only when this test drew.
                    if let Some(why) = u.left_binding_reason(&bound_before) {
                        return Some(why);
                    } else if u.ended == Ended::Failed && !drew {
                        return None;
                    }
                    u.work_out_facts(&engine.events, lowered, index, object);
                    let facts = u.facts.as_ref().expect("worked out");
                    // ADR-061: each test restarts `Random` from the run seed, so an earlier test's draws matter only when
                    // the sequence goes on past that restart, after a `Randomize(n)`; an earlier test that can call
                    // `Randomize(n)` may have left that state, so every draw counts.
                    let drew = if facts.seeds_random() { drew } else { drew_carried };
                    match u.ended {
                        Ended::Failed => (drew && facts.random()).then(|| {
                            format!(
                                "a test before this one in the codeunit did not pass ({}) and could draw random numbers, and this test draws random numbers: the shared sequence is not BC's",
                                u.what
                            )
                        }),
                        Ended::RefusedBeforeStart | Ended::RefusedMidRun | Ended::Untrusted => {
                            // An unknown part of the earlier reach answers without this test's reach.
                            let later = match facts.unknown {
                                Some(_) => &ReachFacts::default(),
                                None => mine.get_or_insert_with(|| test_reach_facts(&engine.events, lowered, index, object, r)),
                            };
                            facts.changes_read_by(later, checkpoint.is_none(), drew, &settled, lowered).map(|what| {
                                let how = match u.ended {
                                    Ended::RefusedMidRun => "was refused while it ran",
                                    Ended::Untrusted => "failed and its failure is not trusted",
                                    _ => "was refused before it started",
                                };
                                format!(
                                    "an earlier test of this codeunit {how} ({}): {what}, so the state it would have left is not BC's",
                                    u.what
                                )
                            })
                        }
                    }
                });
                    match found {
                        Some(why) => {
                            let untrusted = (kind == "failure").then(|| m.clone());
                            (
                                Outcome::NotRun(format!("{why}; this {kind} is not trusted: {m}")),
                                untrusted,
                            )
                        }
                        None if kind == "timeout" => (Outcome::NotRun(m), None),
                        None => (Outcome::Fail(m), None),
                    }
                }
                Err(o) => (o, None),
            };
            let ended = ended_of(&outcome, began, untrusted.is_some());
            if stopped.is_none() {
                let row = (obj.id.unwrap_or(0), name.as_str(), routine.name.as_str());
                stopped = scratch.stopped_after(&outcome, hit_time_limit, row);
            }
            note_test_ended(scratch, session, ended, runner_refused.is_some(), test_no, &name);
            let ended_from = unfinished.len();
            if let (Some(ended), Outcome::NotRun(why) | Outcome::Fail(why)) = (ended, &outcome) {
                unfinished.push(Unfinished {
                    routine: r,
                    what: what_of(&routine.name, ended, why, untrusted.as_deref()),
                    bindings: left_bindings(session, &bound_before, ended, measured_bindings, sequences.as_ref()),
                    ended,
                    facts: None,
                    rest: if ended == Ended::RefusedMidRun {
                        refused_rest.take()
                    } else {
                        None
                    },
                });
            }
            if let Some(why) = runner_refused {
                unfinished.push(Unfinished::runner_step(r, &routine.name, &why));
            }
            type_left_unknown(
                session,
                &mut unfinished[ended_from..],
                engine,
                lowered,
                &mut reach_index,
                object,
            );
            // A Commit() inside a test under Function isolation is rolled back with the rest of the test: under BC's
            // TestRunnerHub with RequiredTestIsolation = Function (QuCommitIsolation.al:466-488 QU_F04_NextTestSees, :530-551
            // QU_F09_LaterTestSeesCommittedRows), and under a runner codeunit's TestIsolation = Function, the part before the
            // Commit() too, whether the test passes or fails (corpus PR 735, IsolProbeCommit.Codeunit.al C1-C3,
            // SdIsolCommitError.Codeunit.al:24-25 E2-E5 under runner 61300, measured on real BC by writer SD).
            // The checkpoint below restores it. A failed test after a Commit() is pinned by the nightly oracle (isolation
            // probe T10 and T11 under runner 61300, IsolProbeFixture.Codeunit.al:118-160).
            if let Some(observe) = observe.as_mut() {
                observe(
                    &name,
                    &outcome,
                    &Observed::of_test(session, failure.as_ref(), refused_in),
                );
            }
            crate::isolation::after_test(
                session,
                checkpoint.as_ref(),
                runner_function,
                sequences.as_mut(),
                failed,
                ended == Some(Ended::Failed),
                &routine.name,
            );
            if let (Outcome::Fail(_), Some(e)) = (&outcome, &failure) {
                if !e.call_stack.is_empty() {
                    stacks.push((name.clone(), e.call_stack.clone()));
                }
            }
            let marks = al_builtins::assumptions::take_hits();
            if !marks.is_empty() {
                hits.push((name.clone(), marks.clone()));
            }
            // `--tdd` (ADR-051 decision 5): the stubs that ran while the test ran, its handlers and subscribers included.
            let (reached, ui_answered) = scratch.take_marks(&name, session);
            let took = test_started.elapsed();
            // The count starts with the test (`start_sleep_budget`): a test that did not start slept nothing.
            let slept_ms = if began {
                u64::try_from(session.slept_in_test_ms()).unwrap_or(0)
            } else {
                0
            };
            if slept_ms > 0 {
                scratch.slept.push((name.clone(), slept_ms));
            }
            if execute {
                scratch.messages.push((name.clone(), session.take_messages()));
            }
            let captured_at = scratch
                .capture
                .any()
                .then(|| crate::capture::resolve(&lowered.program, &scratch.order, session.take_capture()));
            if let Some(c) = &captured_at {
                scratch.captured.push((name.clone(), c.clone()));
            }
            times.push((name.clone(), took));
            let cov = if covering { session.take_coverage() } else { Vec::new() };
            if scratch.trace {
                let touched = scratch.note_reads(session.take_impact_trace());
                scratch.entered.push((
                    routine.name.clone(),
                    crate::affected::entered_keys(&lowered.program, &touched, &lowered.dependency),
                    touched.dotnet,
                ));
                scratch.traces.push((
                    routine.name.clone(),
                    crate::impact_trace::keys(&lowered.program, &touched),
                ));
            }
            if crate::events::wants_events() {
                crate::events::report(crate::events::TestEvent {
                    codeunit: obj.id.unwrap_or(0),
                    codeunit_name: obj.name.clone(),
                    test: name.clone(),
                    outcome: outcome.clone(),
                    call_stack: last_stack(stacks, &name),
                    time: took,
                    slept_ms,
                    coverage: cov.clone(),
                    captured: captured_at.clone(),
                    stubs: reached,
                    ui_answered,
                    entry: None,
                    assumptions: marks,
                });
            }
            if covering {
                crate::coverage::push(&name, cov);
            }
            out.push((name.clone(), outcome));
            // A panic in a test ends the codeunit: the session may be inconsistent.
            if panicked {
                break;
            }
            if crate::events::cancelled() {
                break;
            }
        }
        if panicked {
            not_run_after_panic(obj, &mut queue, &mut out);
        }
        session.clear_unmeasured();
        session.stop_noting_tx_ends();
        ends.set(ends_of(&unfinished, panicked));
        th.end_codeunit(session, lowered, &mut out);
        Ok(out)
    };
    // What BC's AL test runner does around the codeunit and each test method: al-runner's own steps (runner_reset.rs), or
    // the triggers of the test runner codeunit whose OnRun runs this codeunit (al_runner.rs). A scratch run has no runner.
    let refuse = |why| all(Outcome::NotRun(why));
    let ran = run_unit(
        &mut session,
        engine,
        execute,
        object,
        al_runner_object,
        &tests,
        &refuse,
        &mut inner,
    );
    scratch.note_ended(ends.get());
    scratch.singles_out = ended(&mut session, lowered, object, times, execute, scratch);
    let out = match ran {
        Ok(out) => out,
        Err(early) => return early,
    };
    // RequiredTestIsolation = Disabled under the TestRunnerHub: everything persists into the next codeunit, the write
    // still open when a test ended included, as the boundary commits it (QuCommitIsolation.al:695-718 QU_D11_NextCodeunitSees).
    // What a test that did not pass leaves is not pinned there (QuCommitIsolation.al:22). Under a runner's TestIsolation =
    // Disabled a failed test leaves what it committed and nothing else (corpus PR 735, SdIsolCommitReader.Codeunit.al:13-15,
    // IsolProbeReader.Codeunit.al:113-120 R3): only a test that was not run leaves something not known.
    if let Some(start) = carried_from {
        carry_disabled(
            &mut session,
            start,
            runner_disabled,
            &out,
            lowered,
            object,
            tests.iter().chain(&data_driven).copied(),
            changes_from,
            &skipped_installs,
        );
    }
    super::numbers_taken::carry_left(&session, obj, onrun_left, onrun_from, &skipped_installs);
    out
}

/// The `[TransactionModel]` of each of `routines`; one other than AutoCommit, AutoRollback or None is refused.
fn transaction_models<'a>(
    obj: &al_ir::Object,
    routines: impl Iterator<Item = &'a usize>,
) -> Result<std::collections::HashMap<usize, al_vm::TxModel>, String> {
    let mut models = std::collections::HashMap::new();
    for &r in routines {
        for a in obj.routines[r]
            .attributes
            .iter()
            .filter(|a| a.name.eq_ignore_ascii_case("TransactionModel"))
        {
            let m = al_vm::TxModel::from_attribute(&a.text)
                .ok_or_else(|| format!("{}: unknown transaction model", a.text))?;
            models.insert(r, m);
        }
    }
    Ok(models)
}

/// BcContainerHelper's discovery pass in this session instead of the codeunit's tests (`Unit::discovery`,
/// `discovery::run_pass`); the single instances it leaves go on as a codeunit's do.
fn discover(
    session: &mut Session,
    lowered: &Lowered,
    object: usize,
    company_open: &crate::install::CompanyOpenWrites,
    skipped_installs: &[al_vm::SkippedInstall],
    scratch: &mut Scratch,
    limit: Option<std::time::Duration>,
) -> Vec<(String, Outcome)> {
    let objects = scratch.discovery.clone().unwrap_or_default();
    let wrote = company_open.who.is_some() || company_open.login.is_some();
    super::discovery::run_pass(session, lowered, &objects, skipped_installs, wrote, limit);
    scratch.singles_out = ended(session, lowered, object, &[], false, scratch);
    Vec::new()
}

/// The call stack recorded last for test `name` (empty when none was).
fn last_stack(stacks: &[(String, String)], name: &str) -> String {
    stacks
        .iter()
        .rev()
        .find(|(n, _)| n == name)
        .map(|(_, s)| s.clone())
        .unwrap_or_default()
}

/// The rest of a codeunit after a test panicked: every test left is not run.
fn not_run_after_panic(
    obj: &al_ir::Object,
    queue: &mut std::collections::VecDeque<data_driven::Item>,
    out: &mut Vec<(String, Outcome)>,
) {
    for name in queue.drain(..).map(|i| i.name) {
        let outcome = Outcome::NotRun(
            "an earlier test of this codeunit panicked (a runtime bug), so the rest of it was not run".into(),
        );
        if crate::events::wants_events() {
            crate::events::report(crate::events::TestEvent {
                codeunit: obj.id.unwrap_or(0),
                codeunit_name: obj.name.clone(),
                test: name.clone(),
                outcome: outcome.clone(),
                call_stack: String::new(),
                time: std::time::Duration::ZERO,
                slept_ms: 0,
                coverage: Vec::new(),
                captured: None,
                stubs: Vec::new(),
                ui_answered: Vec::new(),
                entry: None,
                assumptions: Vec::new(),
            });
        }
        out.push((name, outcome));
    }
}

/// What a codeunit under Disabled isolation left for the later codeunits ([`CARRIED`]), from `start` on.
#[allow(clippy::too_many_arguments)]
fn carry_disabled(
    session: &mut Session,
    start: al_vm::Baseline,
    runner_disabled: bool,
    out: &[(String, Outcome)],
    lowered: &Lowered,
    object: usize,
    routines: impl Iterator<Item = usize> + Clone,
    changes_from: u64,
    skipped_installs: &[al_vm::SkippedInstall],
) {
    let obj = &lowered.program.objects[object];
    let mode = if runner_disabled {
        "a test runner's TestIsolation = Disabled"
    } else {
        "RequiredTestIsolation = Disabled"
    };
    let unfinished = out.iter().find(|(_, o)| match o {
        Outcome::Pass => false,
        Outcome::Fail(_) => !runner_disabled,
        _ => true,
    }).map(|(name, o)| {
        let how = match o {
            Outcome::Fail(_) => "failed",
            _ => "was not run",
        };
        format!(
            "an earlier codeunit under {mode}, {}, wrote a table this test reads, and its test {name} {how}: what it \
             left for the later codeunits is not pinned",
            obj.name
        )
    });
    let mut tables = session.store_mut().tables_changed_since(changes_from);
    // A test that did not run to its end may have left out writes it would have made: the tables it names count too.
    if unfinished.is_some() {
        for r in routines.clone() {
            tables.extend(tables_touched(lowered, object, r));
        }
    }
    // Isolated storage and number sequences carry over as well, but who reads them later is not followed.
    let everything = session.storage_or_sequences_differ(&start);
    let storage_why = everything.then(|| {
        format!(
            "an earlier codeunit under {mode}, {}, changed isolated storage or a number sequence, which every \
                 later codeunit sees: that is not followed",
            obj.name
        )
    });
    let mut baseline = session.capture_baseline();
    // The install code already ran for what it left; the later codeunits must not run it again on top.
    baseline.workspace_installs = Some(skipped_installs.to_vec());
    let carried = Carried {
        baseline: Some(baseline),
        tables,
        unfinished,
        storage_why,
        everything,
        as_started: false,
    };
    CARRIED.with(|c| *c.borrow_mut() = Some(carried));
}

/// The codeunit's tests are done: its profile (if any) is kept (`profiling::end`), and the single instances it leaves
/// are returned (`singles_out`).
fn ended(
    session: &mut Session,
    lowered: &Lowered,
    object: usize,
    times: &[(String, std::time::Duration)],
    execute: bool,
    scratch: &mut Scratch,
) -> Option<al_vm::single_track::SingleOut> {
    crate::profiling::end(session, &lowered.program, object, times);
    if scratch.driver.watches_reads() {
        scratch.note_reads(session.take_impact_trace());
    }
    let out = singles_out(session, execute, scratch.refused_mid_run, scratch.hands_on_none());
    // Notes taken only for the partner login subscribers' SingleInstance globals (`run_codeunit_inner`) are not the lane's.
    out.filter(|_| scratch.watch)
}

/// How the tests of a codeunit ended, from the tests that did not run to their end and whether one panicked.
fn ends_of(unfinished: &[Unfinished], test_panicked: bool) -> super::Ends {
    super::Ends {
        unfinished_after_start: unfinished.iter().any(|u| u.ended != Ended::RefusedBeforeStart),
        refused_mid_run: unfinished.iter().any(|u| u.ended == Ended::RefusedMidRun),
        test_panicked,
    }
}

/// A codeunit's session from the install baseline; in the lane with the single instances of the codeunit before.
fn new_session(
    engine: &Arc<Engine>,
    host: al_host::Host,
    baseline: Option<&al_vm::Baseline>,
    execute: bool,
) -> (Session, Option<al_vm::single_track::MovedSingles>) {
    // The lane: the single instances of the codeunit before, moved in later (`start_singles`).
    let moved = if execute {
        None
    } else {
        LANE_SINGLES.with(|c| c.borrow_mut().as_mut().and_then(Option::take))
    };
    (session_from(engine, host, baseline, moved.as_ref()), moved)
}

/// A session on `baseline`; the temporary tables of the single instances `moved` keep their ids.
fn session_from(
    engine: &Arc<Engine>,
    host: al_host::Host,
    baseline: Option<&al_vm::Baseline>,
    moved: Option<&al_vm::single_track::MovedSingles>,
) -> Session {
    let mut session = Session::with_host(engine.clone(), host);
    if let Some(b) = baseline {
        session.restore_baseline(b);
    }
    if let Some(m) = moved {
        session.continue_temp_ids(m);
    }
    crate::profiling::begin(&mut session);
    session
}

/// The host of a test codeunit's session: the run's clock and seed, perturbed by a stability run.
fn session_host(scratch: &Scratch) -> al_host::Host {
    let mut host = crate::host::host_perturbed(scratch.clock, &scratch.perturb);
    host.run_seed = scratch.run_seed.seed;
    host
}

/// The settings of the run on a new session: fault tracing when observed, the run's flags, coverage, impact trace.
fn set_up_session(session: &mut Session, scratch: &Scratch, observed: bool) {
    session.trace_faults(observed);
    scratch.flags.apply(session);
    if crate::coverage::enabled() {
        session.enable_coverage();
    }
    if scratch.trace || scratch.driver.watches_reads() {
        session.enable_impact_trace();
    }
}

/// What a test codeunit's session needs to open the company ([`Start::open`]), and to start once more ([`Start::again`]).
struct Start<'a> {
    steps: Steps<'a>,
    /// The install baseline the session starts from. Not shared between threads: the check's twin takes its data with
    /// the store the login started from ([`Steps::twin`]).
    baseline: Option<&'a al_vm::Baseline>,
}

/// The steps that make a test codeunit's session and bring it to its login, besides the restore of the install
/// baseline: [`Steps::session`], [`Steps::before_login`]. Any thread may take them: the check's twin is made by them
/// ([`Steps::twin`]).
#[derive(Clone, Copy)]
struct Steps<'a> {
    engine: &'a Arc<Engine>,
    lowered: &'a Lowered,
    object: usize,
    /// What the install step reads of the baseline (`install_once::installs_for`).
    installs: super::install_once::FromBaseline<'a>,
    isolation: TestIsolation,
    execute: bool,
    observed: bool,
    /// Single instances of an earlier codeunit moved in (`new_session`).
    moved_in: bool,
}

type Opened = (
    Vec<al_vm::SkippedInstall>,
    (crate::install::CompanyOpenWrites, al_builtins::assumptions::HitBits),
);

impl Start<'_> {
    /// What publishing the app does first (its install codeunits, then commit), then the session enters the codeunit and
    /// opens the company ([`open_company`]): the installs that could not run, and what the open left.
    fn open(
        &self,
        session: &mut Session,
        scratch: &Scratch,
        perturb: &crate::stability::Perturb,
        after_refusal: bool,
    ) -> Result<Opened, crate::install::NotOpened> {
        let skipped = self.steps.before_login(session, scratch);
        // Company Triggers when the test session opens the company, before the codeunit's OnRun and first test (ADR-046
        // amendment 1, A4 item 10). What the step relied on counts for every test of the session
        // (`mark_test_start_assumptions`).
        let opened = open_company(self, session, &skipped, perturb, scratch, after_refusal)?;
        Ok((skipped, opened))
    }

    /// The baseline the check's twin runs the install code on ([`Steps::twin`]): only where the session ran it on its
    /// baseline (`FromBaseline::NotInstalled`), frozen so that the twin's thread may take it. `None` too for a baseline
    /// that cannot be frozen: the twin then runs the install code on no baseline.
    fn twin_baseline(&self) -> Option<al_vm::SharedBaseline> {
        let not_installed = matches!(self.steps.installs, super::install_once::FromBaseline::NotInstalled);
        not_installed.then(|| self.baseline?.freeze().ok()).flatten()
    }

    /// A new session in place of one whose open did not end, set up as the first ([`set_up_session`]).
    fn again(&self, scratch: &Scratch, moved: Option<&al_vm::single_track::MovedSingles>) -> Session {
        self.steps.session(scratch, self.baseline, moved)
    }
}

impl Steps<'_> {
    /// A new session on `baseline` with the single instances `moved`, set up for the run ([`set_up_session`]).
    fn session(
        &self,
        scratch: &Scratch,
        baseline: Option<&al_vm::Baseline>,
        moved: Option<&al_vm::single_track::MovedSingles>,
    ) -> Session {
        let mut session = session_from(self.engine, session_host(scratch), baseline, moved);
        set_up_session(&mut session, scratch, self.observed);
        session
    }

    /// The steps before the login: the install code's leftovers (or its run), the facts of the engine, the codeunit
    /// entered. Returns the installs that could not run.
    fn before_login(&self, session: &mut Session, scratch: &Scratch) -> Vec<al_vm::SkippedInstall> {
        // No single instance of an earlier codeunit moved in: the session is as `install_once` makes one.
        let skipped = super::install_once::installs_for(
            self.lowered,
            session,
            self.installs,
            scratch,
            self.execute,
            self.observed,
            !self.moved_in,
        );
        // What install code wrote is known only in part (ADR-046): the tests read it through the facts of this engine.
        crate::install::ensure_facts(self.engine, self.lowered);
        enter_test_codeunit(session, scratch, self.object, self.isolation);
        skipped
    }

    /// The twin of the login replay check (`login_replay.rs`): a new session brought to its login by the same steps as
    /// the codeunit's session, so that, once it starts from the store the session's login started from
    /// (`Session::start_from`: the install baseline's data, which is not shared between threads, comes with it), it
    /// holds before its thaw what the session held before its login, as the replay's target does, the fields the thaw
    /// keeps from its target included. Single instances an earlier codeunit left are not moved into it (they were moved
    /// into the session, and a run with them takes no start, `NotTaken::MovedIn`). Where the session ran the install
    /// code itself, the twin runs it too, on the baseline `on` the session ran it on ([`Start::twin_baseline`]).
    fn twin(&self, scratch: &Scratch, on: Option<&al_vm::SharedBaseline>) -> Session {
        let on = on.map(|b| b.thaw(Some(self.engine)));
        let mut twin = self.session(scratch, on.as_ref(), None);
        self.before_login(&mut twin, scratch);
        twin
    }
}

/// Takes over the moved single instances and starts the notes (not for a codeunit the runner runs with
/// Codeunit.Run: `execute`). Without a watch and without moved instances nothing is noted.
fn start_singles(session: &mut Session, execute: bool, moved: Option<al_vm::single_track::MovedSingles>, watch: bool) {
    if execute {
        return;
    }
    if let Some(m) = moved {
        session.adopt_single_instances(m);
    }
    if watch {
        session.track_single_instances();
    }
}

/// A test method begins: its number for the SingleInstance notes (ADR-066 amendment 7). Without `Scratch::watch` the
/// lane uses no notes: 0. Notes the numbers OnRun and the runner steps before this test took (ADR-070 decision 3).
fn test_begins(scratch: &mut Scratch, session: &mut Session) -> u32 {
    let n = session.single_test_begins();
    super::order_hint::note_taken(scratch, session, "");
    if scratch.watch { n } else { 0 }
}

/// What the session did with the watched single instances; in the lane, hands the instances to the next codeunit.
/// `refused`: a test of the codeunit was refused while it ran, so what it left is not what BC's run to the end would
/// have left (ADR-066 amendment 3, R3): it is no change for the decision. `hands_on_none`: it hands nothing on (the next
/// codeunit starts without single instances; `Scratch::hands_on_none`).
fn singles_out(
    session: &mut Session,
    execute: bool,
    refused: bool,
    hands_on_none: bool,
) -> Option<al_vm::single_track::SingleOut> {
    let out = session
        .single_out()
        .map(|o| al_vm::single_track::SingleOut { refused, ..o });
    if execute {
        return out;
    }
    LANE_SINGLES.with(|c| {
        if let Some(slot) = c.borrow_mut().as_mut() {
            *slot = (!hands_on_none).then(|| session.take_single_instances());
        }
    });
    out
}

std::thread_local! {
    /// While the lane runs a BC session's codeunits on this thread (`single_lane.rs`): the single instances the last
    /// codeunit left, moved into the next codeunit's session (`al_vm::single_track::MovedSingles`). `None`: no lane.
    pub(crate) static LANE_SINGLES: std::cell::RefCell<Option<Option<al_vm::single_track::MovedSingles>>> =
        const { std::cell::RefCell::new(None) };
}

thread_local! {
    /// What the last codeunit with RequiredTestIsolation = Disabled that ran on this thread left. A baseline holds `Rc`s
    /// and stays on its thread: the codeunits that start from it run on the same one (`corpus.rs`).
    pub(crate) static CARRIED: std::cell::RefCell<Option<Carried>> = const { std::cell::RefCell::new(None) };
}

/// What a codeunit with RequiredTestIsolation = Disabled left for the codeunits after it ([`CARRIED`]).
pub(crate) struct Carried {
    /// Its committed data at its end: where a later codeunit that reads one of `tables` starts. `None`: it did not run,
    /// and the codeunit before it left nothing.
    pub baseline: Option<al_vm::Baseline>,
    /// The tables it changed.
    pub tables: BTreeSet<u32>,
    /// Why what it left is not known (a test of it did not pass): a later codeunit that reads `tables` is not run.
    pub unfinished: Option<String>,
    /// It changed isolated storage or a number sequence: a later codeunit whose reach can touch them counts as reading
    /// what it left (`corpus.rs` `reaches_session_storage`).
    pub everything: bool,
    /// Why what it left there is not followed (set with `everything`): such a later codeunit is not run.
    pub storage_why: Option<String>,
    /// What its `OnRun` left is exactly the state the codeunit started from (`Session::holds_what_it_started_from`, and
    /// the same workspace installs): a session started from `baseline` starts as one started from that state.
    pub as_started: bool,
}

impl Carried {
    /// What a codeunit that left nothing new carries: the state it was given (`baseline: None`, which the chain fills
    /// with that state, `chain_walk.rs`).
    pub fn nothing() -> Carried {
        Carried {
            baseline: None,
            tables: BTreeSet::new(),
            unfinished: None,
            everything: false,
            storage_why: None,
            as_started: false,
        }
    }
}

/// Sets a session's stop flag when a test has used more than its limit of CPU time. The time the
/// test thread computes is counted, not the wall time, so the load of the machine cannot stop a
/// test (an outcome must not depend on it). A wall-clock backstop, `WALL_BACKSTOP` times the limit (saturating: a
/// limit too large for that has no backstop),
/// still ends a test that hangs without computing. One thread per test that is run; it polls
/// until the limit or until `finish`. Where the thread's CPU clock cannot be read, the wall clock
/// counts.
pub(crate) struct Watchdog {
    pub(crate) done: std::sync::mpsc::Sender<()>,
    pub(crate) thread: std::thread::JoinHandle<Option<Stopped>>,
}

/// The clock whose reading stopped a test at its time limit (al-runner issue 8168): the outputs name it as the `clock` of the
/// timeout row (`alrunner-cli` `output.rs` `stopped_by_clock`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stopped {
    /// The test thread's CPU time passed the limit.
    Cpu,
    /// The wall clock did: the backstop of `WALL_BACKSTOP` times the limit, or the limit where no CPU clock can be read.
    Wall,
}

/// The reason of a test (or, with `place`, its codeunit's OnRun) that the clock `stopped` stopped at its time `limit`. It starts
/// as `TIME_LIMIT_REASON`, which the outputs turn into the timeout row; the clock's words decide its `clock`.
fn time_limit_reason(stopped: Option<Stopped>, limit: Option<std::time::Duration>, place: &str) -> String {
    let clock = match stopped {
        Some(Stopped::Wall) => "of wall time",
        Some(Stopped::Cpu) | None => "of CPU time",
    };
    format!(
        "{TIME_LIMIT_REASON}{} s {clock} and was stopped (--test-timeout-secs){place}; BC may have ended it differently or not at all",
        limit.map_or(0, |l| l.as_secs())
    )
}

/// Which clock stops a test now: `cpu` is the CPU time the test thread used (`None`: no CPU clock), `wall` the wall time of
/// the test. With a CPU clock the CPU time counts and the wall clock is the backstop; without one the wall clock counts.
fn stop_by(cpu: Option<std::time::Duration>, wall: std::time::Duration, limit: std::time::Duration) -> Option<Stopped> {
    match cpu {
        Some(cpu) if cpu >= limit => Some(Stopped::Cpu),
        Some(_) if wall >= limit.saturating_mul(WALL_BACKSTOP) => Some(Stopped::Wall),
        Some(_) => None,
        None if wall >= limit => Some(Stopped::Wall),
        None => None,
    }
}

/// The flag the watchdog of a test also stops it for: `cancel` for a scratch run (`execute`), else the run's stop at its
/// hard memory limit (ADR-066 amendment 10, decision 6; `Hooks::halt`).
fn watched_stop(execute: bool) -> Option<Arc<std::sync::atomic::AtomicBool>> {
    if execute {
        crate::events::cancel_flag()
    } else {
        crate::events::halt_flag()
    }
}

/// The reason of a test refused with `r`: the run's memory limit when the run stopped there and stopped this test.
fn not_run_reason(r: Refusal) -> String {
    match crate::events::halted() {
        Some(why) if r == al_vm::STOPPED => why,
        _ => r.into_reason(),
    }
}

/// How the reason of a test the time limit stopped starts (the outputs turn it into `Test exceeded <N>s timeout.`).
const TIME_LIMIT_REASON: &str = "the test used more than the time limit of ";

/// Where the reason of an OnRun the time limit stopped says it was stopped.
const ON_RUN_PLACE: &str = " in the test codeunit's OnRun trigger, before its tests";

/// Whether `reason` is that of a row the time limit stopped (`TIME_LIMIT_REASON`).
pub(super) fn is_time_limit(reason: &str) -> bool {
    reason.starts_with(TIME_LIMIT_REASON)
}

/// How many times the limit the wall clock may run before a test that does not compute is stopped.
pub(crate) const WALL_BACKSTOP: u32 = 10;
/// How often the watchdog reads the CPU clock.
pub(crate) const WATCHDOG_POLL: std::time::Duration = std::time::Duration::from_millis(50);

impl Watchdog {
    /// Watches the calling thread.
    #[cfg(test)]
    pub(crate) fn start(flag: Arc<std::sync::atomic::AtomicBool>, limit: std::time::Duration) -> Watchdog {
        Watchdog::start_cancellable(flag, limit, None)
    }

    /// [`Watchdog::start`] that also stops the routine when `cancel` is set (a scratch run: `cancel` ends it at once;
    /// a test run lets the running test finish).
    #[cfg(test)]
    pub(crate) fn start_cancellable(
        flag: Arc<std::sync::atomic::AtomicBool>,
        limit: std::time::Duration,
        cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Watchdog {
        Watchdog::start_paused(flag, limit, cancel, None)
    }

    /// [`Watchdog::start_cancellable`] whose wall clock leaves out the time `paused` counts (the time a debugger kept the
    /// test stopped, ADR-048 decision 9). A stopped thread uses no CPU time, so only the wall clock needs it.
    pub(crate) fn start_paused(
        flag: Arc<std::sync::atomic::AtomicBool>,
        limit: std::time::Duration,
        cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
        paused: Option<Arc<crate::events::PauseClock>>,
    ) -> Watchdog {
        let paused_at_start = paused.as_ref().map(|p| p.total()).unwrap_or_default();
        let clock = crate::cpuclock::ThreadClock::current();
        // The thread's CPU clock counts from the thread's birth, and a worker thread runs many tests:
        // the limit is measured from the reading at the start of this test.
        let base = clock.as_ref().and_then(|c| c.read()).unwrap_or_default();
        let (done, rx) = std::sync::mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            let started = std::time::Instant::now();
            // The wall time of the test, without the time a debugger kept it stopped.
            let elapsed = || {
                let stopped = paused
                    .as_ref()
                    .map(|p| p.total().saturating_sub(paused_at_start))
                    .unwrap_or_default();
                started.elapsed().saturating_sub(stopped)
            };
            loop {
                match rx.recv_timeout(WATCHDOG_POLL) {
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    _ => return None,
                }
                if cancel
                    .as_ref()
                    .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    return None;
                }
                let used = clock
                    .as_ref()
                    .and_then(|c| c.read())
                    .map(|cpu| cpu.saturating_sub(base));
                if let Some(stopped) = stop_by(used, elapsed(), limit) {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    return Some(stopped);
                }
            }
        });
        Watchdog { done, thread }
    }

    /// Ends the watch; the clock that stopped the test when the limit passed first.
    pub(crate) fn finish(self) -> Option<Stopped> {
        let _ = self.done.send(());
        self.thread.join().unwrap_or(None)
    }
}

/// Tests that did not run to their end here and may have on BC (refused before or while they ran, a failure not trusted, a
/// runner step around them) may have set the transaction type the next test method starts with
/// (session-system/TxTypeNextTestMethod.al): when the reach of one can set it, or is not known, the type is not known
/// (`Session::transaction_type_unknown`). A trusted failure stops on BC where it stops here, with the type it had.
fn type_left_unknown(
    session: &mut Session,
    ended: &mut [Unfinished],
    engine: &Arc<Engine>,
    lowered: &Lowered,
    reach_index: &mut Option<Arc<ReachIndex>>,
    object: usize,
) {
    for u in ended.iter_mut().filter(|u| u.ended != Ended::Failed) {
        if !session.transaction_type_known() {
            return;
        }
        let index = reach_index.get_or_insert_with(|| engine.derived(|| ReachIndex::new(&engine.events, lowered)));
        u.work_out_facts(&engine.events, lowered, index, object);
        let facts = u.facts.as_ref().expect("worked out");
        if facts.unknown.is_some() || facts.state_writes & written::state::TRANSACTION_TYPE != 0 {
            session.transaction_type_unknown();
        }
    }
}

/// Test `test_no` ended. Notes it in `Scratch::unfinished_at` when it started and did not run to its end, or a runner
/// step around it did not (ADR-066 amendment 7, option 1); notes the numbers it took (ADR-070 decision 3) and, in a
/// watched codeunit, its number (ADR-070 decision 4).
fn note_test_ended(
    scratch: &mut Scratch,
    session: &mut Session,
    ended: Option<Ended>,
    runner_refused: bool,
    test_no: u32,
    name: &str,
) {
    if ended.is_some_and(|e| e != Ended::RefusedBeforeStart) || runner_refused {
        scratch.unfinished_at.push((test_no, name.to_string()));
    }
    super::order_hint::note_taken(scratch, session, name);
    if scratch.watch {
        scratch.test_numbers.push((test_no, name.to_string()));
    }
}

/// How a test of the codeunit that did not run to its end ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ended {
    RefusedBeforeStart,
    RefusedMidRun,
    /// It failed and the failure was judged not trusted: on BC it may have passed, so it counts with its whole reach, as
    /// a test refused while it ran does. It is not one for `Ends::refused_mid_run`.
    Untrusted,
    Failed,
}

/// A test of the codeunit that did not run to its end, for the trust of later failures.
struct Unfinished {
    routine: usize,
    /// Its name and how it ended, for the reason text.
    what: String,
    ended: Ended,
    /// Its reach facts, worked out at the first later failure that asks.
    facts: Option<ReachFacts>,
    /// For a test refused by a statement of its own body: what it did not run (al-runner issue 5961).
    rest: Option<Rest>,
    /// For a failed test: the manual bindings (`BindSubscription`) it made and left alive, for example on a global of the
    /// test codeunit, because it failed before its `UnbindSubscription` (al-runner issue 6924).
    bindings: LeftBindings,
}

/// The manual bindings a failed test left alive ([`left_bindings`]).
#[derive(Default)]
struct LeftBindings {
    /// A later failure while one of them is bound is not trusted.
    untrusted: Vec<usize>,
    /// A later test starts with them as on BC: a later failure is trusted.
    kept: Vec<usize>,
}

impl Unfinished {
    /// Works out [`Self::facts`] once: for a test a statement of its own body refused, what it did not run
    /// ([`remainder_reach_facts`]), else its whole reach.
    fn work_out_facts(
        &mut self,
        events: &al_vm::events::Registry,
        lowered: &Lowered,
        index: &ReachIndex,
        object: usize,
    ) {
        let (rest, routine) = (&self.rest, self.routine);
        self.facts.get_or_insert_with(|| {
            rest.as_ref()
                .and_then(|rest| {
                    remainder_reach_facts(
                        events,
                        lowered,
                        index,
                        object,
                        routine,
                        rest.from,
                        &rest.handlers,
                        &rest.written,
                    )
                })
                .unwrap_or_else(|| unfinished_reach_facts(events, lowered, index, object, routine))
        });
    }

    /// A runner step around test `routine` (`name`) that did not run to its end may have left anything (decision
    /// 2026-10-19: a refused construct in an earlier test's reach counts as "may change anything").
    fn runner_step(routine: usize, name: &str, why: &str) -> Unfinished {
        Unfinished {
            routine,
            what: format!("{name}: {why}").chars().take(120).collect(),
            ended: Ended::RefusedMidRun,
            bindings: LeftBindings::default(),
            rest: None,
            facts: Some(ReachFacts {
                unknown: Some("a step of the test runner around it did not run to its end".into()),
                ..Default::default()
            }),
        }
    }

    /// Why a later failure is not trusted when this failed test left a manual binding that the later test started with.
    /// The binding outlives the test (codeunits/TestEventManualBinding.al:161-200, Contract 9), and a second
    /// `BindSubscription` of the instance raises "already been bound" (TestEventManualBinding.al:150-158, Contract 8). On BC
    /// the earlier test may pass and unbind, so the later test would start without it.
    fn left_binding_reason(&self, bound_at_start: &[usize]) -> Option<String> {
        self.bindings.untrusted.iter().any(|b| bound_at_start.contains(b)).then(|| {
            format!(
                "a test before this one in the codeunit did not pass ({}) and left a manual event subscription bound that it would have unbound later: the bindings this test starts with are not BC's",
                self.what
            )
        })
    }
}

/// The manual bindings alive after a failed test that were not alive when it started, split in two: those a later failure
/// does not trust, and those a later test starts with as on BC. Kept, under a runner corpus PR 1465 measured (`measured`:
/// the test codeunit's instance, isolation::failed_test_runner_measured), after a trusted failure raised by the body of a
/// TransactionModel AutoCommit test (`CodeunitSequences::body_failed_as_measured`, as every P5 cell fails): the binding of a
/// manual subscriber held in a global of the test codeunit, neither of them SingleInstance (assumption
/// `binding-after-failed-test`, tests/al-language-isolation-probe/src/SqBindFailed.al P5a :36, SqReader.al:257; a local
/// binding ends with its test, P5b :85, and is not alive here).
fn left_bindings(
    session: &Session,
    before: &[usize],
    ended: Ended,
    measured: Option<(&Lowered, &std::rc::Rc<std::cell::RefCell<al_builtins::value::Instance>>)>,
    sequences: Option<&crate::isolation::CodeunitSequences>,
) -> LeftBindings {
    use al_builtins::value::Value;
    if !matches!(ended, Ended::Failed | Ended::Untrusted) {
        return LeftBindings::default();
    }
    let left = session
        .bound_subscriptions()
        .into_iter()
        .filter(|b| !before.contains(b));
    let body_measured = sequences.is_some_and(|s| s.body_failed_as_measured());
    let Some((lowered, instance)) = measured.filter(|_| ended == Ended::Failed && body_measured) else {
        return LeftBindings {
            untrusted: left.collect(),
            kept: Vec::new(),
        };
    };
    let single = |object: u32| crate::login::single_instance(lowered, object as usize);
    let globals: Vec<usize> = match instance {
        instance if !single(instance.borrow().object) => instance
            .borrow()
            .globals
            .iter()
            .filter_map(|g| match g {
                Value::Codeunit(rc) if !single(rc.borrow().object) => Some(std::rc::Rc::as_ptr(rc) as usize),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let (untrusted, kept) = left.partition(|b| !globals.contains(b));
    LeftBindings { untrusted, kept }
}

/// A test starts with a binding an earlier failed test of the codeunit left (`LeftBindings::kept`): it runs on
/// assumption `binding-after-failed-test`.
fn assume_kept_bindings(unfinished: &[Unfinished], bound: &[usize]) {
    if unfinished
        .iter()
        .any(|u| u.bindings.kept.iter().any(|b| bound.contains(b)))
    {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BindingAfterFailedTest);
    }
}

/// What a test refused by a statement of its own body left undone, for [`remainder_reach_facts`].
struct Rest {
    /// The index in the body of the statement it was refused in.
    from: usize,
    /// Its handler functions (routines of the test codeunit).
    handlers: Vec<usize>,
    /// The tables it wrote before the refusal, which the rollback undid.
    written: BTreeSet<u32>,
}

/// The name of test `name` and how it ended, for a later reason text, cut at 120 characters. `why` is its outcome's
/// text; `untrusted` the failure's own error when it was judged not trusted. An untrusted failure names that error, not
/// the reason it is not trusted, so a chain of them does not nest.
fn what_of(name: &str, ended: Ended, why: &str, untrusted: Option<&str>) -> String {
    let (how, why) = match (ended, untrusted) {
        (Ended::RefusedBeforeStart, _) => ("not run", why),
        (Ended::RefusedMidRun, _) => ("did not run", why),
        (Ended::Untrusted, Some(error)) => ("failed, not trusted", error),
        (Ended::Failed | Ended::Untrusted, _) => ("failed", why),
    };
    format!("{name}: {how} ({why})").chars().take(120).collect()
}

/// How a test that did not pass ended (`None`: it passed or was skipped). `untrusted`: its failure was judged not trusted.
fn ended_of(outcome: &Outcome, began: bool, untrusted: bool) -> Option<Ended> {
    match outcome {
        Outcome::NotRun(_) if untrusted => Some(Ended::Untrusted),
        Outcome::NotRun(_) if !began => Some(Ended::RefusedBeforeStart),
        Outcome::NotRun(_) => Some(Ended::RefusedMidRun),
        Outcome::Fail(_) => Some(Ended::Failed),
        _ => None,
    }
}

/// The AL error a call ended with, if any.
fn on_run_error<T>(ran: &Result<T, Signal>) -> Option<&AlError> {
    match ran {
        Err(Signal::Error(e)) => Some(e),
        _ => None,
    }
}

/// The OnRun trigger's result `ran`, with a stop by the time `limit` (`timed_out`: the clock that stopped it) named. The
/// reasons start as a test method's do, so the outputs give the same timeout row (`Test exceeded <N>s timeout.`); the rest
/// names the trigger. Ends the trigger's Sleep budget.
fn on_run_time_limit<T>(
    session: &mut Session,
    ran: Result<T, Signal>,
    limit: Option<std::time::Duration>,
    timed_out: Option<Stopped>,
) -> Result<T, Signal> {
    let slept_out = session.sleep_budget_passed();
    let slept_ms = session.slept_in_test_ms();
    session.end_sleep_budget();
    match ran {
        Err(Signal::Unsupported(_)) if slept_out => Err(Signal::unsupported(format!(
            "{TIME_LIMIT_REASON}{} s and was stopped (--test-timeout-secs){ON_RUN_PLACE}: its Sleep calls asked to wait {slept_ms} ms in total (Sleep does not wait here; the time it asks for counts toward the limit)",
            limit.map_or(0, |l| l.as_secs())
        ))),
        Err(Signal::Unsupported(_)) if timed_out.is_some() => {
            Err(Signal::unsupported(time_limit_reason(timed_out, limit, ON_RUN_PLACE)))
        }
        r => r,
    }
}

/// What the codeunit's isolation starts after OnRun, before its first test: Function isolation's checkpoint, which returns
/// the database to its state after OnRun when each test ends (corpus isolation probe T5 and T6;
/// oracle/28/xunit-isolation-function.xml); under Codeunit isolation the number sequences, for what a failed test leaves of
/// them (isolation::after_test); and the rows the tests write, for a page background task's locking read (al-vm
/// bg_readisolation_lock.rs; `onrun_from`: the store's change counter before OnRun). `(measured_runner, execute)`:
/// [`crate::isolation::failed_test_runner_measured`], a scratch run.
fn start_isolation(
    session: &mut Session,
    isolation: TestIsolation,
    (measured_runner, execute): (bool, bool),
    onrun_from: u64,
) -> (Option<Checkpoint>, Option<crate::isolation::CodeunitSequences>) {
    let checkpoint = (isolation == TestIsolation::Function).then(|| Checkpoint::take(session));
    // The transaction ends an earlier codeunit noted for its sequences (CodeunitSequences::begin) end here.
    session.stop_noting_tx_ends();
    let sequences = (isolation == TestIsolation::Codeunit)
        .then(|| crate::isolation::CodeunitSequences::begin(session, measured_runner));
    session.begin_codeunit_row_locks(isolation == TestIsolation::Codeunit && !execute, onrun_from);
    (checkpoint, sequences)
}

/// MinimumActiveRowVersion takes its value when the codeunit's tests begin, under TestIsolation = Codeunit and under
/// RequiredTestIsolation = Function on the hub (corpus metadata/platform-tables/MinimumActiveRowVersionStages.al:4-8, codeunits
/// 97988 and 97989 at lines 152-206). Other isolations are not probed.
fn start_min_row_version(session: &mut Session, isolation: TestIsolation, hub: bool) {
    match isolation {
        TestIsolation::Codeunit => session.freeze_min_active_row_version(),
        TestIsolation::Function if hub => session.freeze_min_active_row_version(),
        _ => {}
    }
}

/// The marks every test of the codeunit starts with: the program's own (`ir::Program::assumes`: it compiled only through
/// them), when the codeunit's OnRun shape is assumed `TestCodeunitOnRunRows`, and `company_open`, the marks of the login
/// and Company Triggers step that opened the company for this session before its first test (orchestrator decision
/// 2026-11-15, al-runner issue 8060: every test of the session runs on what that step left).
fn mark_test_start_assumptions(
    lowered: &Lowered,
    on_run_shape_assumed: bool,
    company_open: al_builtins::assumptions::HitBits,
) {
    al_builtins::assumptions::add_hit_bits(company_open);
    if on_run_shape_assumed {
        al_builtins::assumptions::assume(Assumption::TestCodeunitOnRunRows);
    }
    for &a in &lowered.program.assumes {
        al_builtins::assumptions::assume(a);
    }
}

#[cfg(test)]
mod tests {
    use super::{Ended, LeftBindings, Stopped, Unfinished, WATCHDOG_POLL, Watchdog, ends_of, stop_by};
    use std::sync::Arc;

    /// The limit counts the CPU time of the test thread: a thread that waits is not stopped at the
    /// limit (the machine's load must not decide an outcome), one that computes is.
    #[cfg(unix)]
    #[test]
    fn the_time_limit_counts_cpu_time_not_wall_time() {
        let limit = std::time::Duration::from_millis(200);
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let w = Watchdog::start(flag.clone(), limit);
        std::thread::sleep(limit * 3);
        assert_eq!(w.finish(), None, "a thread that waited was stopped");
        assert!(!flag.load(std::sync::atomic::Ordering::Relaxed));

        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let w = Watchdog::start(flag.clone(), limit);
        let started = std::time::Instant::now();
        let mut x = 0u64;
        while !flag.load(std::sync::atomic::Ordering::Relaxed) && started.elapsed() < std::time::Duration::from_secs(10)
        {
            x = x.wrapping_add(std::hint::black_box(1));
        }
        assert!(x > 0);
        assert_eq!(
            w.finish(),
            Some(Stopped::Cpu),
            "a thread that computed was not stopped by its CPU time"
        );
    }

    /// The wall backstop stops a test that waits for `WALL_BACKSTOP` times the limit, on the wall clock (al-runner issue 8168:
    /// its timeout row names the clock `wall`, not `cpu`).
    #[cfg(unix)]
    #[test]
    fn the_wall_backstop_stops_a_waiting_test_on_the_wall_clock() {
        let limit = std::time::Duration::from_millis(20);
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let w = Watchdog::start(flag.clone(), limit);
        let started = std::time::Instant::now();
        while !flag.load(std::sync::atomic::Ordering::Relaxed) && started.elapsed() < std::time::Duration::from_secs(10)
        {
            std::thread::sleep(WATCHDOG_POLL);
        }
        assert!(
            started.elapsed() >= limit * super::WALL_BACKSTOP,
            "stopped before the backstop"
        );
        assert_eq!(w.finish(), Some(Stopped::Wall));
    }

    /// Which clock stops a test (al-runner issue 8168): the CPU time at the limit; the wall time at the backstop while a CPU
    /// clock is read; the wall time at the limit where no CPU clock can be read.
    #[test]
    fn the_stop_names_the_clock_that_passed_the_limit() {
        let ms = std::time::Duration::from_millis;
        let limit = ms(100);
        assert_eq!(stop_by(Some(ms(100)), ms(100), limit), Some(Stopped::Cpu));
        assert_eq!(stop_by(Some(ms(100)), ms(1000), limit), Some(Stopped::Cpu));
        assert_eq!(stop_by(Some(ms(99)), ms(999), limit), None);
        assert_eq!(stop_by(Some(ms(10)), ms(1000), limit), Some(Stopped::Wall));
        assert_eq!(stop_by(None, ms(99), limit), None);
        assert_eq!(stop_by(None, ms(100), limit), Some(Stopped::Wall));
    }

    /// A limit so large that ten times it does not fit a `Duration` (a client may send any whole number of seconds,
    /// al-runner issue 8125) is no limit in practice: the watchdog keeps watching and does not stop the test.
    #[test]
    fn a_huge_time_limit_does_not_stop_the_watchdog_or_the_test() {
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let w = Watchdog::start(flag.clone(), std::time::Duration::from_secs(u64::MAX));
        std::thread::sleep(WATCHDOG_POLL * 4);
        assert!(!w.thread.is_finished(), "the watchdog thread ended (it panicked)");
        assert_eq!(w.finish(), None, "a huge limit stopped the test");
        assert!(!flag.load(std::sync::atomic::Ordering::Relaxed));
    }

    /// The limit is per test: the CPU time a thread used before the test started does not count.
    /// Three tests on one thread, each using 60 % of the limit, all stay under it.
    #[cfg(any(unix, windows))]
    #[test]
    fn the_cpu_limit_is_measured_from_the_start_of_each_test() {
        let limit = std::time::Duration::from_millis(500);
        for n in 0..3 {
            let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let w = Watchdog::start(flag.clone(), limit);
            let clock = crate::cpuclock::ThreadClock::current().expect("a CPU clock");
            let from = clock.read().expect("a reading");
            let mut x = 0u64;
            while clock.read().expect("a reading") - from < limit * 6 / 10 {
                x = x.wrapping_add(std::hint::black_box(1));
            }
            // Let the watchdog poll once more after the work.
            std::thread::sleep(WATCHDOG_POLL * 3);
            assert!(x > 0);
            assert_eq!(w.finish(), None, "test {n} was stopped under 60 % of the limit");
        }
    }

    fn unfinished(ended: Ended) -> Unfinished {
        Unfinished {
            routine: 0,
            what: String::new(),
            ended,
            facts: None,
            rest: None,
            bindings: LeftBindings::default(),
        }
    }

    /// An untrusted failure counts as a test that started and did not run to its end, as a trusted failure does, but it
    /// is not a test refused while it ran: the strict-order taint and the hand-on of single instances read only that.
    #[test]
    fn an_untrusted_failure_is_unfinished_after_start_but_not_refused_mid_run() {
        let e = ends_of(&[unfinished(Ended::Untrusted)], false);
        assert!(e.unfinished_after_start && !e.refused_mid_run);
        let e = ends_of(&[unfinished(Ended::Untrusted), unfinished(Ended::RefusedMidRun)], false);
        assert!(e.refused_mid_run);
    }
}
