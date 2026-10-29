//! Running the test codeunits of one compiled program, each test on a session of its own.
use super::written::{
    self, ReachFacts, ReachIndex, reach_facts, remainder_reach_facts, settled_flags, unfinished_reach_facts,
};
use super::*;

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
    /// The session, with its fault site (tracing is on when there is an observer).
    pub session: &'a Session,
    /// The error that failed the test.
    pub error: Option<&'a AlError>,
    /// The routine a refusal decided before the run belongs to (object, routine).
    pub refused_in: Option<(usize, usize)>,
}

/// Called with the test's name and outcome after each test of the codeunit that ran.
pub type Observer<'a> = dyn FnMut(&str, &Outcome, &Observed) + 'a;

/// [`run_codeunit_isolated`] that tells `observe` how each test ended and turns on the session's
/// fault tracing for it (`--explain`).
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
    )
}

/// [`run_codeunit_isolated`] where a test that runs longer than `limit` is stopped and not run.
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
    )
}

/// The frames BC's test runner has below a test, as `SessionInformation.Callstack` shows them
/// (corpus session/TestSessionInformationCallstackContracts.al:89-97 and the failure texts of the
/// corpus CI on 27.5, 28.5 and 29.0, where the lines are the same). The runner's own app is
/// `Test Runner by Microsoft`; its version is the loaded app's, else the run's BC version.
pub(crate) fn runner_frames(program: &al_ir::Program, engine: &Engine) -> String {
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
/// call stack of that error as `(test name, stack)` (`GetLastErrorCallStack` form, innermost first).
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
        &mut Scratch::default(),
        limit,
        false,
    )
}

/// [`run_codeunit_full`], or with `execute` the `OnRun` trigger of the codeunit run as its one test, with the text of
/// the `Message` calls it made kept in `messages` (a scratch run).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_codeunit_core(
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
    let mut session = Session::with_host(engine.clone(), crate::host::host_at(scratch.clock_start));
    if let Some(b) = baseline {
        session.restore_baseline(b);
    }
    session.trace_faults(observe.is_some());
    let covering = crate::coverage::enabled();
    if covering {
        session.enable_coverage();
    }
    if scratch.trace {
        session.enable_impact_trace();
    }
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
    if execute {
        session.capture_messages();
    }
    let all = |o: Outcome| {
        tests
            .iter()
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
    // A [TransactionModel] other than AutoCommit, AutoRollback or None is refused.
    let mut models = std::collections::HashMap::new();
    for &r in &tests {
        for a in obj.routines[r]
            .attributes
            .iter()
            .filter(|a| a.name.eq_ignore_ascii_case("TransactionModel"))
        {
            match al_vm::TxModel::from_attribute(&a.text) {
                Some(m) => {
                    models.insert(r, m);
                }
                None => return all(Outcome::NotRun(format!("{}: unknown transaction model", a.text))),
            }
        }
    }
    // What publishing the app does first: run its install codeunits, then commit.
    let skipped_installs = match baseline.and_then(|b| b.workspace_installs.clone()) {
        // Already run once for the whole run, with the baseline.
        Some(skipped) => skipped,
        None => {
            // As the baseline does (`baseline::run_traced`, ADR-046 amendment 4): the corpus CI database's session company
            // is an evaluation company.
            if session.engine.test_data.is_none() && session.companies().len() > 1 {
                session.mark_evaluation_company();
            }
            session.set_install_scope(true);
            let skipped = run_install_codeunits(lowered, &mut session);
            session.set_install_scope(false);
            skipped
        }
    };
    // What install code wrote is known only in part (ADR-046): the tests read it through the facts of this engine.
    crate::install::ensure_facts(engine, lowered);
    session.mark_install_end();
    session.in_test_codeunit = true;
    session.isolation_from_hub = scratch.hub && isolation != TestIsolation::Codeunit;
    // StartSession starts a session under Disabled isolation, from the hub (RqIsolationModes.al:573-600) and from a runner
    // (corpus PR 735, SdIsolStartSession.Codeunit.al:20-22): refused there, as no second session is modelled.
    session.hub_disabled = isolation == TestIsolation::Disabled;
    session.test_object = Some(object);
    // Company Triggers when the test session opens the company, before the codeunit's OnRun and first test (ADR-046
    // amendment 1, A4 item 10). A scratch run (`execute`) is not a test session.
    let company_open = if execute {
        crate::install::CompanyOpenWrites::default()
    } else {
        // Base Application's login code sets WorkDate when the session opens the company (`install::login_work_date`).
        // It runs before the workspace's own Company Triggers subscribers: the order of the two is not pinned
        // (docs/probes-needed.md, WorkDate row).
        if let Err(why) = crate::install::login_work_date(engine, lowered, &mut session) {
            return all(Outcome::NotRun(why));
        }
        match crate::install::test_session_company_open(lowered, &mut session) {
            Err(why) => return all(Outcome::NotRun(why)),
            Ok(wrote) => {
                session.commit_transaction();
                wrote
            }
        }
    };
    // What a Disabled codeunit leaves for the later codeunits is its writes from here on ([`CARRIED`]). With
    // Company Triggers subscribers each session writes on its own when it opens the company, and how those writes
    // meet the ones carried over is not pinned.
    let carries = isolation == TestIsolation::Disabled && !execute;
    if carries && company_open.who.is_some() {
        return all(Outcome::NotRun(
            "Disabled test isolation with a Company Triggers subscriber that writes when the test session opens \
             the company: how its writes meet what an earlier codeunit left is not pinned"
                .into(),
        ));
    }
    let changes_from = session.store_mut().change_counter();
    let row_versions_from = session.row_versions_handed_out();
    let carried_from = carries.then(|| session.capture_baseline());
    // Under runner 130450 what the OnRun trigger writes reaches the later codeunits (`Scratch::onrun_carry`).
    let onrun_from = (scratch.onrun_carry && !execute).then(|| session.capture_baseline());
    let mut onrun_left: Option<Carried> = None;
    // The tables an earlier Disabled codeunit left rows in (`Scratch::leak`): a read of one is noted, and the codeunit
    // then runs again from what that codeunit left (`corpus.rs`).
    let leak: BTreeSet<u32> = scratch.leak.as_deref().cloned().unwrap_or_default();
    // The reach facts of a routine of this codeunit, for the writes of the Company Triggers subscribers.
    let company_open_refuses = |r: usize| -> Option<String> {
        company_open.who.as_ref()?;
        let index = engine.derived(|| ReachIndex::new(&engine.events, lowered));
        company_open.refuses(&reach_facts(&engine.events, lowered, &index, object, r), lowered)
    };
    // A test codeunit with SingleInstance = true runs its tests on the session's single instance: a variable of its own type
    // and its manual subscriber share its globals, and the second test sees what the first left
    // (codeunit/QiSingleInstanceTestCodeunit.al:57 QI_SI_1_OwnTypeVariableAndManualSubscriber, :98 QI_SI_2_SecondTestSeesTheFirst).
    // Taken after the install code and the company open, which drop the session's single instances.
    let instance = session.codeunit_instance(object);
    // What BC's AL test runner does around each test method (runner_reset.rs). A scratch run has no runner.
    let runner = if execute {
        runner_reset::RunnerReset::default()
    } else {
        runner_reset::RunnerReset::of(engine)
    };
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
            return on_run_refused(format!("OnRun: {reason}"));
        }
        if let Some(reason) = company_open_refuses(on_run) {
            return on_run_refused(format!("OnRun: {reason}"));
        }
        session.store_mut().watch(leak.clone());
        // Al-runner issue 5698: the time limit (CPU time, the wall backstop and the Sleep budget) applies to OnRun too,
        // with a budget of its own: its time counts toward no test method. A limit is tooling (ADR-009), not something
        // AL code observes.
        session.clear_stop();
        let watchdog = limit.map(|l| Watchdog::start_paused(session.stop_flag(), l, None, None));
        session.start_sleep_budget(limit);
        // What OnRun runs through does not count for the first test.
        al_builtins::assumptions::clear_hits();
        let ran = session.call(object, on_run, instance.clone(), Vec::new());
        let timed_out = watchdog.is_some_and(Watchdog::finish);
        let slept_out = session.sleep_budget_passed();
        let slept_ms = session.slept_in_test_ms();
        session.end_sleep_budget();
        // The reasons start as a test method's do, so the outputs give the same timeout row (`Test exceeded <N>s
        // timeout.`); the rest names the trigger.
        let ran = match ran {
            Err(Signal::Unsupported(_)) if slept_out => Err(Signal::Unsupported(format!(
                "the test used more than the time limit of {} s and was stopped (--test-timeout-secs) in the test codeunit's OnRun trigger, before its tests: its Sleep calls asked to wait {slept_ms} ms in total (Sleep does not wait here; the time it asks for counts toward the limit)",
                limit.map_or(0, |l| l.as_secs())
            ))),
            Err(Signal::Unsupported(_)) if timed_out => Err(Signal::Unsupported(format!(
                "the test used more than the time limit of {} s of CPU time and was stopped (--test-timeout-secs) in the test codeunit's OnRun trigger, before its tests; BC may have ended it differently or not at all",
                limit.map_or(0, |l| l.as_secs())
            ))),
            r => r,
        };
        if session.store_mut().watch_hit().is_some() {
            scratch.leak_hit = true;
        }
        session.store_mut().watch(Default::default());
        match ran {
            // NavTestCodeunit.DoRunAsync commits after a declared OnRun
            // (codeunit/TestTestCodeunitOnRun.al).
            Ok(_) if session.inconsistent_table().is_some() => {
                return on_run_refused(
                    "OnRun: ends while Consistent(false) marks a table (the runner's commit is not pinned)".into(),
                );
            }
            Ok(_) => {
                session.commit_transaction();
                // OnRun ran outside the codeunit isolation of runner 130450: the later codeunits start from what it left,
                // committed and open, whatever the tests do (RqIsolationModes.al:357-369 #else).
                if let Some(start) = &onrun_from {
                    let everything = session.storage_or_sequences_differ(start);
                    let mut baseline = session.capture_baseline();
                    baseline.workspace_installs = Some(skipped_installs.clone());
                    // The rows OnRun wrote are in the session company only.
                    baseline.install_rows = Some(session.install_rows());
                    onrun_left = Some(Carried {
                        baseline: Some(baseline),
                        tables: session.store_mut().tables_changed_since(changes_from),
                        unfinished: None,
                        storage_why: everything.then(|| {
                            format!(
                                "an earlier test codeunit, {}, changed isolated storage or a number sequence in its OnRun \
                                 trigger, which under the test runner 130450 every later codeunit sees: that is not followed",
                                obj.name
                            )
                        }),
                        everything,
                    });
                }
            }
            Err(Signal::Unsupported(r)) if r.starts_with(TIME_LIMIT_REASON) => {
                return on_run_refused(r);
            }
            Err(Signal::Unsupported(r)) => return on_run_refused(format!("OnRun: {r}")),
            Err(Signal::Error(e)) => {
                if on_run_rows == OnRunRows::PerTest {
                    return all(Outcome::NotRun(format!(
                        "OnRun failed (runner behaviour not pinned): {}",
                        e.message
                    )));
                }
                // The error may come from install data this run does not have: it is not trusted.
                if let Some(why) = skipped_installs
                    .iter()
                    .find(|(tables, _)| !tables.is_disjoint(&tables_touched(lowered, object, on_run)))
                    .map(|(_, why)| why)
                {
                    return on_run_refused(format!("OnRun failed after install code that did not run: {why}"));
                }
                let marks = al_builtins::assumptions::take_hits();
                // Microsoft's Test Runner app (ADR-055): the codeunit is Skipped and has no rows, and the run stays
                // green (al-runner issue 5859, BC 29.0; corpus writer RL's probe c69419f6, BC 27.5). Its text is not
                // reported, so an unpinned one does not matter; an error reached through an unprobed assumption may not
                // happen on BC, and then the tests would run.
                if !marks.is_empty() {
                    let ids: Vec<&str> = marks.iter().map(|a| a.info().id).collect();
                    return all(Outcome::NotRun(format!(
                        "OnRun failed through unprobed assumptions ({}), so whether the tests run is not known: {}",
                        ids.join(", "),
                        e.message
                    )));
                }
                // No row carries the assumption here: the skipped codeunit has none (`test-codeunit-onrun-rows`).
                return Vec::new();
            }
            Err(_) => {}
        }
        on_run_shape_assumed = on_run_rows == OnRunRows::TestRunner;
    }
    // Function isolation returns the database to its state after OnRun when each test ends
    // (corpus isolation probe T5 and T6; oracle/28/xunit-isolation-function.xml).
    let checkpoint = (isolation == TestIsolation::Function).then(|| Checkpoint::take(&mut session));
    // TestIsolation = Codeunit keeps one transaction open for the tests: MinimumActiveRowVersion stays at its value now. When
    // OnRun wrote rows, whether they belong to that transaction is not pinned, and nothing is frozen.
    if isolation == TestIsolation::Codeunit && !execute && session.row_versions_handed_out() == row_versions_from {
        session.freeze_min_active_row_version();
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
    // The rows of `out` before the first test's.
    let first_test = out.len();
    // Every earlier test of the codeunit that did not run to its end: refused before it started, refused while it ran,
    // or failed. A later failure is not trusted when such a test could have changed what it reads (`written.rs`).
    let mut unfinished: Vec<Unfinished> = Vec::new();
    // Taken at the first failure that has an unfinished test before it; built once per engine, as it depends on the
    // program only.
    let mut reach_index: Option<Arc<ReachIndex>> = None;
    let flags = settled_flags(lowered, object);
    let mut panicked = false;
    // Why the tests after one are not run: under a runner's TestIsolation = Function, that test changed an isolated
    // storage key or a number sequence that was there before it, and what the rollback leaves of it is not measured.
    let mut unmeasured_restore: Option<String> = None;
    for r in tests.iter().copied() {
        // A cancelled run starts no further test (the one running finishes).
        if crate::events::cancelled() {
            break;
        }
        crate::events::starting(obj.id.unwrap_or(0), &obj.routines[r].name);
        let test_started = std::time::Instant::now();
        let draws_before = session.inherited_random_draws();
        // The flags of the codeunit that already hold their only value when this test starts.
        let settled: BTreeSet<(usize, usize)> = {
            let inst = instance.borrow();
            flags
                .iter()
                .filter(|(g, v)| matches!(inst.globals.get(*g), Some(al_builtins::value::Value::Boolean(b)) if b == v))
                .map(|(g, _)| (object, *g))
                .collect()
        };
        let routine = &obj.routines[r];
        let handlers = resolve_handlers(lowered, object, r);
        let mut failure: Option<AlError> = None;
        let mut refused_in = None;
        // Why a step of the test runner around this test was refused or failed (runner_reset.rs): what it left is not
        // known, so a later failure of the codeunit is not trusted.
        let mut runner_refused: Option<String> = None;
        // A refusal raised by a statement of the test method itself: the statement's index in the body and the tables
        // the test wrote before the refusal (al-runner issue 5961), for the trust of later failures.
        let mut refused_rest: Option<Rest> = None;
        session.clear_fault_site();
        al_builtins::assumptions::clear_hits();
        session.take_tdd_reached();
        let mut began = false;
        let outcome = if let Some(why) = &unmeasured_restore {
            Outcome::NotRun(why.clone())
        } else if let Err(reason) = &handlers {
            Outcome::NotRun(reason.clone())
        } else if let Some(b) = blocked_site(lowered, object, r) {
            refused_in = Some((b.object, b.routine));
            Outcome::NotRun(b.text)
        } else if let Some(reason) = skipped_installs
            .iter()
            .find(|(tables, _)| !tables.is_disjoint(&tables_touched(lowered, object, r)))
            .map(|(_, why)| why)
        {
            Outcome::NotRun(reason.clone())
        } else if let Some(reason) = company_open_refuses(r) {
            Outcome::NotRun(reason)
        } else {
            let bound = handlers.unwrap_or_default();
            // A scratch run has no UI scope: its `Message` calls are kept, not handled.
            let bound_ok = if execute {
                Ok(())
            } else {
                bind_handlers(&mut session, &instance, &bound)
            };
            match bound_ok {
                Err(reason) => Outcome::NotRun(reason),
                Ok(()) => {
                    began = true;
                    session
                        .store_mut()
                        .watch(company_open.tables.union(&leak).copied().collect());
                    session.last_error = Default::default();
                    al_builtins::assumptions::clear_hits();
                    if on_run_shape_assumed {
                        al_builtins::assumptions::assume(Assumption::TestCodeunitOnRunRows);
                    }
                    // The runner's steps before the method (runner_reset.rs); what they rely on counts for the test.
                    // Its `LibraryRandom.SetSeed(1)` reseeds the sequence, so draws after it are not inherited.
                    session.start_random_scope();
                    // What the test (and the runner's steps around it) writes counts from here.
                    let changes_from = session.store_mut().change_counter();
                    let reset = runner.before(&mut session, obj, routine);
                    let reset_ok = reset.is_ok();
                    if let Err(why) = &reset {
                        runner_refused = Some(why.clone());
                    }
                    // The AL test runner enables every application area before each test method
                    // (corpus session/TestSessionApplicationAreaTestBoundary.al:67552).
                    session.start_test_method();
                    let model = models.get(&r).copied().unwrap_or_default();
                    // TransactionModel::None is handled before the method body: every active
                    // transaction is ended on the way in (codeunit/TestWriteTxTestBoundary.al:
                    // Test08). Nothing is pending here, because every earlier test ended its
                    // transaction at its own boundary, so which way it ends is not observable.
                    if model == al_vm::TxModel::None {
                        session.commit_transaction();
                    }
                    session.tx_model = model;
                    session.clear_stop();
                    // A scratch run is stopped by `cancel` too, and has a watchdog for that even without a limit.
                    let cancel = if execute { crate::events::cancel_flag() } else { None };
                    let watchdog = match (limit, &cancel) {
                        (None, None) => None,
                        (l, _) => Some(Watchdog::start_paused(
                            session.stop_flag(),
                            l.unwrap_or(std::time::Duration::from_secs(u32::MAX.into())),
                            cancel.clone(),
                            paused.clone(),
                        )),
                    };
                    session.set_runner_frames(runner_frames(&lowered.program, engine));
                    session.start_sleep_budget(limit);
                    // A panic in the test is a runtime bug: it is caught here, so that it ends this codeunit and not
                    // the process (a server must outlive it). The session is not trusted after it.
                    let called = match reset {
                        // A refused or failed reset: the test does not run, and what the reset changed stays.
                        Err(why) => Ok(Err(Signal::Unsupported(why))),
                        Ok(()) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            // A debugging aid: the named test panics here, to show that a panic ends one codeunit and not the
                            // process (tests of `al-runner serve`). Not documented for users.
                            if std::env::var("ALRUNNER_DEBUG_PANIC_IN").is_ok_and(|n| n == routine.name) {
                                panic!("requested by ALRUNNER_DEBUG_PANIC_IN");
                            }
                            session.call(object, r, instance.clone(), Vec::new())
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
                            Err(Signal::Unsupported(format!("runtime panic (bug): {msg}")))
                        }
                    };
                    session.set_runner_frames(String::new());
                    let timed_out = watchdog.is_some_and(Watchdog::finish);
                    let slept_out = session.sleep_budget_passed();
                    // A refusal the method's call raised, whose innermost frame is the method: the statement it came
                    // from. The writes are read before the rollback below undoes them.
                    if reset_ok
                        && !panicked
                        && !timed_out
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
                        Err(Signal::Unsupported(_)) if slept_out => Err(Signal::Unsupported(format!(
                            "the test used more than the time limit of {} s and was stopped (--test-timeout-secs): its Sleep calls asked to wait {} ms in total (Sleep does not wait here; the time it asks for counts toward the limit)",
                            limit.map_or(0, |l| l.as_secs()),
                            session.slept_in_test_ms()
                        ))),
                        Err(Signal::Unsupported(_)) if timed_out => Err(Signal::Unsupported(format!(
                            "the test used more than the time limit of {} s of CPU time and was stopped (--test-timeout-secs); BC may have ended it differently or not at all",
                            limit.map_or(0, |l| l.as_secs())
                        ))),
                        r => r,
                    };
                    session.tx_model = al_vm::TxModel::AutoCommit;
                    let unexecuted = session.end_ui_scope();
                    // The runner commits a test that passes: with a table still marked by Consistent(false) what BC does there
                    // is not pinned (only Commit() is, misc/lt-cu-odd-leads/CuCommitTests.al:405-419), so the test is not run.
                    let result = match result {
                        Ok(_) if model != al_vm::TxModel::AutoRollback && unexecuted.is_empty() => {
                            match session.inconsistent_table() {
                                Some(t) => Err(Signal::Unsupported(format!(
                                    "the test ends while Consistent(false) marks table {t}: the runner's commit is not pinned"
                                ))),
                                None => result,
                            }
                        }
                        r => r,
                    };
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
                            Some(t) => Err(Signal::Unsupported(format!(
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
                        // test's transaction is rolled back (corpus codeunit/TestWriteTxTestBoundary.al:26).
                        Ok(_) if unexecuted.is_empty() => {
                            session.commit_transaction();
                            Outcome::Pass
                        }
                        // corpus handlers/TestPageQueryCloseMessageHandler_Tests.al:51: a declared
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
                        Err(Signal::Error(e)) if al_builtins::native::text_unpinned(&e) => {
                            session.rollback_transaction();
                            let Err(Signal::Unsupported(r)) =
                                al_builtins::native::refuse_unpinned_text(&e, "The test result")
                            else {
                                unreachable!("an unpinned error is refused")
                            };
                            Outcome::NotRun(r)
                        }
                        Err(Signal::Error(e)) => {
                            session.rollback_transaction();
                            let message = e.message.clone();
                            failure = Some(*e);
                            Outcome::Fail(message)
                        }
                        // A refused test did not run: what it wrote before the refusal must not
                        // reach the next test of the codeunit.
                        Err(Signal::Unsupported(_)) if execute && crate::events::cancelled() => {
                            session.rollback_transaction();
                            Outcome::NotRun("cancelled".into())
                        }
                        Err(Signal::Unsupported(r)) => {
                            session.rollback_transaction();
                            Outcome::NotRun(r)
                        }
                        Err(other) => Outcome::Fail(format!("unexpected signal {}", other.describe())),
                    };
                    // The runner's step after the method. It only changes what later tests of the codeunit start
                    // from; if it cannot run, their failures are not trusted.
                    if let Err(why) = runner.after(&mut session) {
                        session.rollback_transaction();
                        runner_refused.get_or_insert(why);
                    }
                    outcome
                }
            }
        };
        // A test that did not run to its end (refused before it started, refused while it ran, failed) left the state the
        // tests of the codeunit share other than BC did: the database (under TestIsolation = Codeunit), the globals of the
        // codeunit and of the codeunits it holds, single-instance codeunits, and session state such as the `Random`
        // sequence. A failure of a later test is not trusted when an unfinished test's static reach can change what the
        // failing test reads (Tests-ERM "Discount Setup Notifications": the refused test would have reset the posting
        // setups the test before it left blank). A refused test's partial run counts as its whole reach, except for one a
        // statement of its own body refused: what it wrote before the refusal and the reach of the statements from that one
        // on count (`remainder_reach_facts`, al-runner issue 5961). A failed test counts only through the `Random`
        // sequence: its draws are not rolled back, while its database writes are, as they are in BC when the test fails
        // there too.
        // The numbers this test drew from a sequence it did not seed itself: they depend on what earlier tests drew
        // (Tests-SCM-Planning CalcPlanReqWkshForTransferLFLItem: a random end date before a random receipt date).
        let drew = session.inherited_random_draws() > draws_before;
        // A test that read or wrote, while it ran, a table a Company Triggers subscriber wrote before the first test
        // (al-runner issue 5641): what it found there is not pinned.
        let outcome = match session.store_mut().watch_hit() {
            Some(t) if began && company_open.tables.contains(&t) => Outcome::NotRun(company_open.touched(t, lowered)),
            Some(t) if began && leak.contains(&t) => {
                scratch.leak_hit = true;
                outcome
            }
            _ => outcome,
        };
        session.store_mut().watch(Default::default());
        // A test the time limit stopped started from the same state as a failure does, so it is judged alike (al-runner
        // issue 5776): `kind` names it in the reason.
        let judged = match outcome {
            Outcome::Fail(m) if !unfinished.is_empty() => Ok(("failure", m)),
            Outcome::NotRun(m) if !unfinished.is_empty() && m.starts_with(TIME_LIMIT_REASON) => Ok(("timeout", m)),
            o => Err(o),
        };
        let outcome = match judged {
            Ok((kind, m)) => {
                let engine = session.engine.clone();
                let index: &ReachIndex =
                    reach_index.get_or_insert_with(|| engine.derived(|| ReachIndex::new(&engine.events, lowered)));
                // This test's reach is walked only when an unfinished test's reach is fully known: an unknown part
                // decides alone (`changes_read_by`), and a walk costs about 150 ms with Microsoft's apps.
                let mut mine: Option<ReachFacts> = None;
                let found = unfinished.iter_mut().find_map(|u| {
                    // A failed test counts only through the `Random` sequence, which matters only when this test drew.
                    if u.ended == Ended::Failed && !drew {
                        return None;
                    }
                    let facts = u.facts.get_or_insert_with(|| {
                        u.rest
                            .as_ref()
                            .and_then(|rest| {
                                remainder_reach_facts(
                                    &engine.events,
                                    lowered,
                                    index,
                                    object,
                                    u.routine,
                                    rest.from,
                                    &rest.handlers,
                                    &rest.written,
                                )
                            })
                            .unwrap_or_else(|| unfinished_reach_facts(&engine.events, lowered, index, object, u.routine))
                    });
                    match u.ended {
                        Ended::Failed => facts.random().then(|| {
                            format!(
                                "a test before this one in the codeunit did not pass ({}) and could draw random numbers, and this test draws random numbers: the shared sequence is not BC's",
                                u.what
                            )
                        }),
                        Ended::RefusedBeforeStart | Ended::RefusedMidRun => {
                            // An unknown part of the earlier reach answers without this test's reach.
                            let later = match facts.unknown {
                                Some(_) => &ReachFacts::default(),
                                None => mine.get_or_insert_with(|| reach_facts(&engine.events, lowered, index, object, r)),
                            };
                            facts.changes_read_by(later, checkpoint.is_none(), drew, &settled, lowered).map(|what| {
                                let when = if u.ended == Ended::RefusedMidRun {
                                    "refused while it ran"
                                } else {
                                    "refused before it started"
                                };
                                format!(
                                    "an earlier test of this codeunit was {when} ({}): {what}, so the state it would have left is not BC's",
                                    u.what
                                )
                            })
                        }
                    }
                });
                match found {
                    Some(why) => Outcome::NotRun(format!("{why}; this {kind} is not trusted: {m}")),
                    None if kind == "timeout" => Outcome::NotRun(m),
                    None => Outcome::Fail(m),
                }
            }
            Err(o) => o,
        };
        let ended = match &outcome {
            Outcome::NotRun(_) if !began => Some(Ended::RefusedBeforeStart),
            Outcome::NotRun(why) if why.contains("this failure is not trusted") => Some(Ended::Failed),
            Outcome::NotRun(_) => Some(Ended::RefusedMidRun),
            Outcome::Fail(_) => Some(Ended::Failed),
            _ => None,
        };
        if let (Some(ended), Outcome::NotRun(why) | Outcome::Fail(why)) = (ended, &outcome) {
            let how = match ended {
                Ended::RefusedBeforeStart => "not run",
                Ended::RefusedMidRun => "did not run",
                Ended::Failed => "failed",
            };
            unfinished.push(Unfinished {
                routine: r,
                what: format!("{}: {how} ({why})", routine.name).chars().take(120).collect(),
                ended,
                facts: None,
                rest: if ended == Ended::RefusedMidRun {
                    refused_rest.take()
                } else {
                    None
                },
            });
        }
        // A runner step that did not run to its end may have left anything (decision 2026-10-19: a refused construct
        // in an earlier test's reach counts as "may change anything").
        if let Some(why) = runner_refused {
            unfinished.push(Unfinished {
                routine: r,
                what: format!("{}: {why}", routine.name).chars().take(120).collect(),
                ended: Ended::RefusedMidRun,
                rest: None,
                facts: Some(ReachFacts {
                    unknown: Some("a step of the test runner around it did not run to its end".into()),
                    ..Default::default()
                }),
            });
        }
        // A Commit() inside a test under Function isolation is rolled back with the rest of the test: under BC's
        // TestRunnerHub with RequiredTestIsolation = Function (QuCommitIsolation.al:466-488 QU_F04_NextTestSees, :530-551
        // QU_F09_LaterTestSeesCommittedRows), and under a runner codeunit's TestIsolation = Function, the part before the
        // Commit() too, whether the test passes or fails (corpus PR 735, IsolProbeCommit.Codeunit.al C1-C3,
        // SdIsolCommitError.Codeunit.al:24-25 E2-E5 under runner 61300, measured on real BC by writer SD).
        // The checkpoint below restores it. A failed test after a Commit() is pinned by the nightly oracle (isolation
        // probe T10 and T11 under runner 61300, IsolProbeFixture.Codeunit.al:118-160).
        if let Some(observe) = observe.as_mut() {
            observe(
                &routine.name,
                &outcome,
                &Observed {
                    session: &session,
                    error: failure.as_ref(),
                    refused_in,
                },
            );
        }
        if let Some(cp) = &checkpoint {
            if runner_function {
                match cp.restore_for_runner(&mut session) {
                    // A new key's IsolatedStorage.Set and a new NumberSequence.Insert are rolled back
                    // (IsolProbeAfterRollback.Codeunit.al S1-S9 under runner 61300).
                    None | Some(true) => {}
                    Some(false) => {
                        unmeasured_restore.get_or_insert_with(|| {
                            format!(
                                "an earlier test of this codeunit, {}, changed an isolated storage key or a number sequence \
                                 that existed before it: what the rollback of TestIsolation = Function leaves of it is not \
                                 measured (only a new key's IsolatedStorage.Set and a new NumberSequence.Insert are)",
                                routine.name
                            )
                        });
                    }
                }
            } else {
                cp.restore(&mut session);
            }
        }
        if let (Outcome::Fail(_), Some(e)) = (&outcome, &failure) {
            if !e.call_stack.is_empty() {
                stacks.push((routine.name.clone(), e.call_stack.clone()));
            }
        }
        let marks = al_builtins::assumptions::take_hits();
        if !marks.is_empty() {
            hits.push((routine.name.clone(), marks));
        }
        // `--tdd` (ADR-051 decision 5): the stubs that ran while the test ran, its handlers and subscribers included.
        let reached = session.take_tdd_reached();
        if !reached.is_empty() {
            scratch.stubbed.push((routine.name.clone(), reached.clone()));
        }
        let took = test_started.elapsed();
        // The count starts with the test (`start_sleep_budget`): a test that did not start slept nothing.
        let slept_ms = if began {
            u64::try_from(session.slept_in_test_ms()).unwrap_or(0)
        } else {
            0
        };
        if slept_ms > 0 {
            scratch.slept.push((routine.name.clone(), slept_ms));
        }
        if execute {
            scratch.messages.push((routine.name.clone(), session.take_messages()));
        }
        let captured_at = scratch
            .capture
            .any()
            .then(|| crate::capture::resolve(&lowered.program, &scratch.order, session.take_capture()));
        if let Some(c) = &captured_at {
            scratch.captured.push((routine.name.clone(), c.clone()));
        }
        times.push((routine.name.clone(), took));
        let cov = if covering { session.take_coverage() } else { Vec::new() };
        if scratch.trace {
            let touched = session.take_impact_trace();
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
                test: routine.name.clone(),
                outcome: outcome.clone(),
                call_stack: stacks
                    .iter()
                    .rev()
                    .find(|(n, _)| *n == routine.name)
                    .map(|(_, s)| s.clone())
                    .unwrap_or_default(),
                time: took,
                slept_ms,
                coverage: cov.clone(),
                captured: captured_at.clone(),
                stubs: reached,
                entry: None,
            });
        }
        if covering {
            crate::coverage::push(&routine.name, cov);
        }
        out.push((routine.name.clone(), outcome));
        // A panic in a test ends the codeunit: the session may be inconsistent.
        if panicked {
            break;
        }
        if crate::events::cancelled() {
            break;
        }
    }
    if panicked {
        let ran = out.len() - first_test;
        for &r in &tests[ran..] {
            let outcome = Outcome::NotRun(
                "an earlier test of this codeunit panicked (a runtime bug), so the rest of it was not run".into(),
            );
            if crate::events::wants_events() {
                crate::events::report(crate::events::TestEvent {
                    codeunit: obj.id.unwrap_or(0),
                    codeunit_name: obj.name.clone(),
                    test: obj.routines[r].name.clone(),
                    outcome: outcome.clone(),
                    call_stack: String::new(),
                    time: std::time::Duration::ZERO,
                    slept_ms: 0,
                    coverage: Vec::new(),
                    captured: None,
                    stubs: Vec::new(),
                    entry: None,
                });
            }
            out.push((obj.routines[r].name.clone(), outcome));
        }
    }
    // RequiredTestIsolation = Disabled under the TestRunnerHub: everything persists into the next codeunit, the write
    // still open when a test ended included, as the boundary commits it (QuCommitIsolation.al:695-718 QU_D11_NextCodeunitSees).
    // What a test that did not pass leaves is not pinned there (QuCommitIsolation.al:22). Under a runner's TestIsolation =
    // Disabled a failed test leaves what it committed and nothing else (corpus PR 735, SdIsolCommitReader.Codeunit.al:13-15,
    // IsolProbeReader.Codeunit.al:113-120 R3): only a test that was not run leaves something not known.
    if let Some(start) = carried_from {
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
            for &r in &tests {
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
        baseline.workspace_installs = Some(skipped_installs.clone());
        let carried = Carried {
            baseline: Some(baseline),
            tables,
            unfinished,
            storage_why,
            everything,
        };
        CARRIED.with(|c| *c.borrow_mut() = Some(carried));
    }
    if let Some(left) = onrun_left {
        CARRIED.with(|c| *c.borrow_mut() = Some(left));
    }
    out
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
}

/// Sets a session's stop flag when a test has used more than its limit of CPU time. The time the
/// test thread computes is counted, not the wall time, so the load of the machine cannot stop a
/// test (an outcome must not depend on it). A wall-clock backstop, `WALL_BACKSTOP` times the limit,
/// still ends a test that hangs without computing. One thread per test that is run; it polls
/// until the limit or until `finish`. Where the thread's CPU clock cannot be read, the wall clock
/// counts.
pub(crate) struct Watchdog {
    pub(crate) done: std::sync::mpsc::Sender<()>,
    pub(crate) thread: std::thread::JoinHandle<bool>,
}

/// How the reason of a test the time limit stopped starts (the outputs turn it into `Test exceeded <N>s timeout.`).
const TIME_LIMIT_REASON: &str = "the test used more than the time limit of ";

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
            let stop = |flag: &std::sync::atomic::AtomicBool| {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
                true
            };
            loop {
                match rx.recv_timeout(WATCHDOG_POLL) {
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    _ => return false,
                }
                if cancel
                    .as_ref()
                    .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    return false;
                }
                let used = clock
                    .as_ref()
                    .and_then(|c| c.read())
                    .map(|cpu| cpu.saturating_sub(base));
                match used {
                    Some(cpu) => {
                        if cpu >= limit || elapsed() >= limit * WALL_BACKSTOP {
                            return stop(&flag);
                        }
                    }
                    // No CPU clock: the wall clock counts.
                    None => {
                        if elapsed() >= limit {
                            return stop(&flag);
                        }
                    }
                }
            }
        });
        Watchdog { done, thread }
    }

    /// Ends the watch; true when the limit passed first.
    pub(crate) fn finish(self) -> bool {
        let _ = self.done.send(());
        self.thread.join().unwrap_or(false)
    }
}

/// How a test of the codeunit that did not run to its end ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ended {
    RefusedBeforeStart,
    RefusedMidRun,
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
