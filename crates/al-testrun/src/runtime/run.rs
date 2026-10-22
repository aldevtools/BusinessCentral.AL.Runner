//! Running the test codeunits of one compiled program, each test on a session of its own.
use super::written::{ReachFacts, ReachIndex, reach_facts, settled_flags, unfinished_reach_facts};
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
    // Nothing pins what a runner with TestIsolation = Disabled leaves behind, within a codeunit or
    // for the next one (docs/probes-needed.md), so it is refused (hard rule 4).
    if isolation == TestIsolation::Disabled {
        return all(Outcome::NotRun(
            "TestIsolation = Disabled is not supported yet: no corpus test pins what BC keeps between tests and test \
             codeunits in this mode (corpus issue 645). Run with TestIsolation = Codeunit (al-runner: --isolation codeunit, \
             the default) or Function (--isolation test)"
                .into(),
        ));
    }
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
    session.test_object = Some(object);
    // Company Triggers when the test session opens the company, before the codeunit's OnRun and first test (ADR-046
    // amendment 1, A4 item 10). A scratch run (`execute`) is not a test session.
    let company_open = if execute {
        crate::install::CompanyOpenWrites::default()
    } else {
        match crate::install::test_session_company_open(lowered, &mut session) {
            Err(why) => return all(Outcome::NotRun(why)),
            Ok(wrote) => {
                session.commit_transaction();
                wrote
            }
        }
    };
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
    if let Some(on_run) = (0..obj.routines.len())
        .find(|&r| obj.routines[r].is_trigger && obj.routines[r].name.eq_ignore_ascii_case("OnRun"))
        .filter(|_| !execute)
    {
        if let Some(reason) = blocked_reason(lowered, object, on_run) {
            return all(Outcome::NotRun(format!("OnRun: {reason}")));
        }
        if let Some(reason) = company_open_refuses(on_run) {
            return all(Outcome::NotRun(format!("OnRun: {reason}")));
        }
        match session.call(object, on_run, instance.clone(), Vec::new()) {
            // NavTestCodeunit.DoRunAsync commits after a declared OnRun
            // (codeunit/TestTestCodeunitOnRun.al).
            Ok(_) if session.inconsistent_table().is_some() => {
                return all(Outcome::NotRun(
                    "OnRun: ends while Consistent(false) marks a table (the runner's commit is not pinned)".into(),
                ));
            }
            Ok(_) => session.commit_transaction(),
            Err(Signal::Unsupported(r)) => return all(Outcome::NotRun(format!("OnRun: {r}"))),
            Err(Signal::Error(e)) => {
                return all(Outcome::NotRun(format!(
                    "OnRun failed (runner behaviour not pinned): {}",
                    e.message
                )));
            }
            Err(_) => {}
        }
    }
    // Function isolation returns the database to its state after OnRun when each test ends
    // (corpus isolation probe T5 and T6; oracle/28/xunit-isolation-function.xml).
    let checkpoint = (isolation == TestIsolation::Function).then(|| Checkpoint::take(&mut session));
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
    let mut out = Vec::new();
    // Every earlier test of the codeunit that did not run to its end: refused before it started, refused while it ran,
    // or failed. A later failure is not trusted when such a test could have changed what it reads (`written.rs`).
    let mut unfinished: Vec<Unfinished> = Vec::new();
    // Taken at the first failure that has an unfinished test before it; built once per engine, as it depends on the
    // program only.
    let mut reach_index: Option<Arc<ReachIndex>> = None;
    let flags = settled_flags(lowered, object);
    let mut panicked = false;
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
        let mut committed = false;
        let mut failure: Option<AlError> = None;
        let mut refused_in = None;
        // Why a step of the test runner around this test was refused or failed (runner_reset.rs): what it left is not
        // known, so a later failure of the codeunit is not trusted.
        let mut runner_refused: Option<String> = None;
        session.clear_fault_site();
        al_builtins::assumptions::clear_hits();
        session.take_tdd_reached();
        let mut began = false;
        let outcome = if let Err(reason) = &handlers {
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
                    session.store_mut().watch(company_open.tables.clone());
                    session.last_error = Default::default();
                    al_builtins::assumptions::clear_hits();
                    // The runner's steps before the method (runner_reset.rs); what they rely on counts for the test.
                    // Its `LibraryRandom.SetSeed(1)` reseeds the sequence, so draws after it are not inherited.
                    session.start_random_scope();
                    let reset = runner.before(&mut session, obj, routine);
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
                    let commits_before = session.commits;
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
                    committed = session.commits != commits_before;
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
                    // table or a Commit raises it (record/TestRecordRuntimeTextsAndViews.al:180-209), but what the
                    // runner's commit or rollback at the end of the test does with it is not pinned (al-runner issue
                    // 5603: the test passed here).
                    let result = match result {
                        Ok(_) if unexecuted.is_empty() => match session.deferred_insert_table() {
                            Some(t) => Err(Signal::Unsupported(format!(
                                "the test ends while the error of a duplicate-key Insert into table {t} is deferred: whether the test runner raises it at the end of the test is not pinned"
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
        // setups the test before it left blank). A refused test's partial run counts as its whole reach. A failed test
        // counts only through the `Random` sequence: its draws are not rolled back, while its database writes are, as
        // they are in BC when the test fails there too.
        // The numbers this test drew from a sequence it did not seed itself: they depend on what earlier tests drew
        // (Tests-SCM-Planning CalcPlanReqWkshForTransferLFLItem: a random end date before a random receipt date).
        let drew = session.inherited_random_draws() > draws_before;
        // A test that read or wrote, while it ran, a table a Company Triggers subscriber wrote before the first test
        // (al-runner issue 5641): what it found there is not pinned.
        let outcome = match session.store_mut().watch_hit() {
            Some(t) if began => Outcome::NotRun(company_open.touched(t, lowered)),
            _ => outcome,
        };
        session.store_mut().watch(Default::default());
        let outcome = match outcome {
            Outcome::Fail(m) if !unfinished.is_empty() => {
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
                    let facts = u
                        .facts
                        .get_or_insert_with(|| unfinished_reach_facts(&engine.events, lowered, index, object, u.routine));
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
                    Some(why) => Outcome::NotRun(format!("{why}; this failure is not trusted: {m}")),
                    None => Outcome::Fail(m),
                }
            }
            o => o,
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
            });
        }
        // A runner step that did not run to its end may have left anything (decision 2026-10-19: a refused construct
        // in an earlier test's reach counts as "may change anything").
        if let Some(why) = runner_refused {
            unfinished.push(Unfinished {
                routine: r,
                what: format!("{}: {why}", routine.name).chars().take(120).collect(),
                ended: Ended::RefusedMidRun,
                facts: Some(ReachFacts {
                    unknown: Some("a step of the test runner around it did not run to its end".into()),
                    ..Default::default()
                }),
            });
        }
        // Whether a Commit() inside a test is kept or ignored under Function isolation is not
        // pinned (docs/probes-needed.md): a test that passed through one is not reported as run.
        let outcome = match outcome {
            Outcome::Pass if committed && checkpoint.is_some() => Outcome::NotRun(
                "Commit inside a test under TestIsolation = Function is not supported yet: no corpus test pins whether \
                 BC keeps the committed rows for the next test (corpus issue 645). Run this codeunit with TestIsolation = \
                 Codeunit (al-runner: --isolation codeunit, the default)"
                    .into(),
            ),
            o => o,
        };
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
            cp.restore(&mut session);
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
        let ran = out.len();
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
    out
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
}
