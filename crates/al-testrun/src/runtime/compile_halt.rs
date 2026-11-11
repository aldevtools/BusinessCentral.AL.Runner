//! The compile under a hard memory limit (ADR-066 amendment 10, decision 6, for the whole run; owner, 2026-11-09: "if I
//! say maximum, then that has to be the maximum I can spend on this task").
//!
//! [`CorpusRuntime::compile_within_limit`] runs the compile with a watcher of the resident memory
//! ([`super::memory::CompileWatch`]) and arms the compile's threads with its flag (`al_sema::halt`): the thread that
//! calls it and the threads of a rayon pool of its own, in which the compile runs. When the resident memory passes
//! the stop line the threads stop at their next object or file, and the run ends before its install baseline: every
//! test the suites select is reported as not run with the limit as the reason, from a text scan of the apps under
//! test (`discover.rs`), since the compile that would name them did not end. The front end exits with code 2.

use std::collections::BTreeSet;

use super::*;

impl CorpusRuntime {
    /// [`compile_suites_with`](CorpusRuntime::compile_suites_with) under the run's memory limit: `Ok(Err(rt))` when a
    /// hard limit stopped the compile, `rt` being the run with every selected test not run. Without a hard limit whose
    /// resident memory can be read, the plain compile.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn compile_within_limit(
        apps: Vec<al_sema::workspace::AppSource>,
        kinds: Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        suites: &[SuiteRun],
        opts: &RunOptions,
        lean: bool,
        grow: &mut Grow,
    ) -> Result<Result<Compiled, CorpusRuntime>, String> {
        let Some(mut watch) = super::memory::CompileWatch::start(opts.memory.as_ref()) else {
            return Self::compile_suites_with(apps, kinds, manifests, symbols, target, suites, opts, lean, grow)
                .map(Compiled::for_one_run)
                .map(Ok);
        };
        let started = std::time::Instant::now();
        // The tests to report when the compile stops: read before the apps go into the compile.
        let tests = scan_tests(&apps, &kinds, suites, opts);
        let flag = watch.flag();
        let armed = flag.clone();
        // The compile ran on the front end's runner thread before it ran in this pool; a pool thread's default stack
        // (2 MB) overflowed on the 1,374 operands of one `+` chain in Base Application 28 while the compile cache was
        // built, so the pool's threads get the runner thread's stack.
        let pool = rayon::ThreadPoolBuilder::new()
            .stack_size(crate::RUNNER_STACK)
            .start_handler(move |_| al_sema::halt::arm_thread(Some(armed.clone())))
            .build()
            .map_err(|e| format!("the compile could not start its threads: {e}"))?;
        let compiled = {
            let _armed = al_sema::halt::arm(Some(flag));
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pool.install(|| {
                    Self::compile_suites_with(apps, kinds, manifests, symbols, target, suites, opts, lean, grow)
                })
            }))
        };
        drop(pool);
        let message = watch.stop();
        match (compiled, message) {
            (Ok(compiled), None) => compiled.map(Compiled::for_one_run).map(Ok),
            // Stopped at the limit, or passed it after the compile's last check: the run does not go on.
            (Ok(compiled), Some(message)) => {
                if let Ok(c) = compiled {
                    retire(c);
                }
                Ok(Err(halted(
                    tests,
                    suites,
                    opts,
                    watch.reason(),
                    message,
                    started.elapsed(),
                )))
            }
            // The stop unwinds the compile (`al_sema::halt::Halted`, or the panic of a scoped thread that carried it).
            (Err(_), Some(message)) => Ok(Err(halted(
                tests,
                suites,
                opts,
                watch.reason(),
                message,
                started.elapsed(),
            ))),
            (Err(payload), _) => std::panic::resume_unwind(payload),
        }
    }
}

/// One test a stopped compile reports: suite, app, codeunit id and name, test name.
type Scanned = (usize, usize, u32, String, String);

/// The tests the suites select, from a text scan of the apps under test (`discover.rs`), in suite and app order.
fn scan_tests(
    apps: &[al_sema::workspace::AppSource],
    kinds: &[al_sema::AppKind],
    suites: &[SuiteRun],
    opts: &RunOptions,
) -> Vec<Scanned> {
    let mut found: Vec<(usize, u32, String, Vec<String>)> = Vec::new();
    for (ai, app) in apps.iter().enumerate() {
        if kinds.get(ai) == Some(&al_sema::AppKind::Dependency) {
            continue;
        }
        for (_, text) in &app.files {
            for (id, name, tests) in crate::discover::test_codeunits_named(text) {
                found.push((ai, id, name, tests));
            }
        }
    }
    let mut out = Vec::new();
    for (si, suite) in suites.iter().enumerate() {
        for (ai, id, name, tests) in &found {
            let picked =
                suite.app.is_none_or(|a| a == *ai) && opts.only.as_ref().is_none_or(|only| only.contains(&(*ai, *id)));
            if picked {
                out.extend(tests.iter().map(|t| (si, *ai, *id, name.clone(), t.clone())));
            }
        }
    }
    out
}

/// The run of a compile stopped at the memory limit: every test not run with `reason`, the stop in the memory report.
fn halted(
    tests: Vec<Scanned>,
    suites: &[SuiteRun],
    opts: &RunOptions,
    reason: &str,
    message: String,
    compile: std::time::Duration,
) -> CorpusRuntime {
    let mut outcomes = std::collections::HashMap::new();
    let mut results = Vec::with_capacity(tests.len());
    for (si, app, codeunit, name, test) in tests {
        if let Some(f) = &opts.hooks.on_test {
            f(crate::events::TestEvent::not_run(codeunit, &name, &test, reason));
        }
        outcomes.insert((si, codeunit, test.to_lowercase()), Outcome::NotRun(reason.to_string()));
        results.push(TestResult {
            suite: si,
            app,
            codeunit,
            codeunit_name: name,
            test,
            outcome: Outcome::NotRun(reason.to_string()),
            call_stack: String::new(),
            assumptions: Vec::new(),
            time: std::time::Duration::ZERO,
            coverage: Vec::new(),
            messages: Vec::new(),
            captured: None,
            impact: Vec::new(),
            stubs: Vec::new(),
            ui_answered: Vec::new(),
            slept_ms: 0,
            entered: Vec::new(),
            dotnet: false,
            order_hint: None,
            cells: None,
        });
    }
    let memory = super::memory::MemoryReport {
        limit: opts.memory.as_ref().map(|b| (b.limit, b.source.clone())),
        asked: test_workers(),
        halted: Some(message),
        ..Default::default()
    };
    CorpusRuntime {
        outcomes,
        times: Default::default(),
        suites: suites.iter().map(|s| s.name.clone()).collect(),
        panics: 0,
        phases: Phases {
            compile,
            memory,
            ..Default::default()
        },
        details: Default::default(),
        results,
        marks: Default::default(),
        coverage_statements: Vec::new(),
        impact_keys: Vec::new(),
        impact_baseline: None,
        program_apps: Vec::new(),
        tdd_stubs: Default::default(),
        compile_errors: Vec::new(),
        entries: Vec::new(),
        met: Default::default(),
    }
}
