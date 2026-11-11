//! ADR-073 Phase 2: the suites of one compile whose CI job runs on a service tier with another
//! DisableWriteInsideTryFunctions than the run's (`RunOptions::write_in_try_suites`, the corpus suite `trywrite-allowed`)
//! run on an engine of their own, over the same program, with an install baseline of their own.
//!
//! - Lane 0, the run's own setting: the run's engine. It leaves the apps of the other lane uninstalled
//!   (`Engine::uninstalled_apps`): BC's job of those suites does not have them, so their install code does not run there.
//! - Lane 1, the other setting: a sibling engine (it shares the program, the compiled code and the platform listings)
//!   with the setting, whose install baseline runs every app's install code under that setting, as the suite's own job
//!   does.
//!
//! Lane 1 runs only when its apps cannot change a result of lane 0 (`shared_pass::check`); otherwise its tests are not
//! run, with the reason, and the corpus job fails (`Phases::shared_pass`).

use super::install_snapshot::InstallSnapshots;
use super::{RunOptions, SuiteRun};
use al_sema::lower::Lowered;
use al_vm::{Baseline, Engine};
use std::collections::BTreeSet;
use std::sync::Arc;

/// What the shared-pass check said of one suite of lane 1: its name, and why it may not share the pass.
#[derive(Debug, Clone, Default)]
pub struct SharedPassNote {
    pub suite: String,
    pub breach: Option<String>,
}

/// The engine, install baseline and snapshots one lane's tests run on.
pub(crate) struct Lane {
    pub(crate) engine: Arc<Engine>,
    pub(crate) baseline: Option<Baseline>,
    /// The engine the lane's install code ran on, when the tests run on another (several test threads).
    pub(crate) baseline_engine: Option<Arc<Engine>>,
    pub(crate) snapshots: InstallSnapshots,
    /// Why no test of the lane runs.
    pub(crate) refusal: Option<String>,
}

/// The lanes of a run.
pub(crate) struct Lanes {
    /// Per suite: 1 when it is listed in `RunOptions::write_in_try_suites` (its setting is the opposite of the run's), else 0.
    of_suite: Vec<usize>,
    /// The setting of lane 1.
    other: bool,
    /// The apps lane 0's engine leaves uninstalled: those of lane 1's suites.
    pub(crate) uninstalled: BTreeSet<u32>,
    /// Why lane 1 cannot run beside lane 0 at all (an app both lanes use).
    shared_app: Option<String>,
    second: Option<Lane>,
    pub(crate) notes: Vec<SharedPassNote>,
    /// How long lane 1's install baseline took.
    pub(crate) baseline_time: std::time::Duration,
}

impl Lanes {
    pub(crate) fn of(suites: &[SuiteRun], opts: &RunOptions) -> Lanes {
        let other = !opts.allow_write_in_try_functions;
        let of_suite: Vec<usize> = suites
            .iter()
            .map(|s| usize::from(opts.write_in_try_suites.contains(&s.name)))
            .collect();
        let apps = |lane: usize| -> Vec<Option<usize>> {
            suites
                .iter()
                .zip(&of_suite)
                .filter(|(_, l)| **l == lane)
                .map(|(s, _)| s.app)
                .collect()
        };
        let (main, second) = (apps(0), apps(1));
        let shared_app = second
            .iter()
            .find(|a| a.is_none() || main.contains(a) || main.contains(&None))
            .map(|_| "the suite's app is an app of the suites with the run's server settings too".to_string());
        let uninstalled = if shared_app.is_some() {
            BTreeSet::new()
        } else {
            second.iter().flatten().map(|&a| a as u32).collect()
        };
        debug_assert!(
            opts.entries.is_empty() || second.is_empty(),
            "entries and per-suite settings do not mix"
        );
        Lanes {
            of_suite,
            other,
            uninstalled,
            shared_app,
            second: None,
            notes: Vec::new(),
            baseline_time: std::time::Duration::ZERO,
        }
    }

    fn has_second(&self) -> bool {
        self.of_suite.contains(&1)
    }

    /// Whether suite `si` runs in lane `lane`.
    pub(crate) fn runs(&self, lane: usize, si: usize) -> bool {
        self.of_suite.get(si).copied().unwrap_or(0) == lane
    }

    /// Each pass once per lane that has suites, with its lane.
    pub(crate) fn split<P: Clone>(&self, passes: Vec<P>) -> Vec<(usize, P)> {
        let lanes: &[usize] = if self.has_second() { &[0, 1] } else { &[0] };
        passes
            .into_iter()
            .flat_map(|p| lanes.iter().map(move |&l| (l, p.clone())))
            .collect()
    }

    /// After lane 0's install baseline: checks lane 1's apps against lane 0's tests and, when they may share the pass,
    /// makes lane 1's engines and install baseline. `install` is the engine lane 0's install code ran on, `tests` the one
    /// its tests run on (the same with one test thread); `sibling` makes an engine as the run makes its tests engine.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start(
        &mut self,
        lowered: &Lowered,
        suites: &[SuiteRun],
        opts: &RunOptions,
        only: &Option<Vec<(usize, u32)>>,
        (install, tests): (&Arc<Engine>, &Arc<Engine>),
        baseline: bool,
        threads: bool,
        sibling: &dyn Fn(&Arc<Engine>) -> Arc<Engine>,
        snapshots: &dyn Fn() -> InstallSnapshots,
        halt: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) {
        if !self.has_second() {
            return;
        }
        let started = std::time::Instant::now();
        // Lane 0's selected test codeunits: what lane 1's apps must not reach.
        let main_tests: Vec<usize> = (0..lowered.program.objects.len())
            .filter(|&oi| {
                suites
                    .iter()
                    .enumerate()
                    .any(|(si, s)| self.runs(0, si) && super::is_selected_test(lowered, oi, s, only))
            })
            .collect();
        let breach = self.shared_app.clone().or_else(|| {
            self.uninstalled
                .iter()
                .find_map(|&app| crate::shared_pass::check(install, lowered, app, &main_tests).err())
                .map(|b| b.0)
        });
        let notes: Vec<SharedPassNote> = (suites.iter().enumerate())
            .filter(|(si, _)| self.runs(1, *si))
            .map(|(_, s)| SharedPassNote {
                suite: s.name.clone(),
                breach: breach.clone(),
            })
            .collect();
        self.notes = notes;
        if let Some(why) = breach {
            self.second = Some(Lane {
                engine: tests.clone(),
                baseline: None,
                baseline_engine: None,
                snapshots: snapshots(),
                refusal: Some(format!("the suite may not share the pass of the other suites: {why}")),
            });
            return;
        }
        let with_setting = |mut e: Arc<Engine>| {
            let m = Arc::get_mut(&mut e).expect("a new sibling engine is not shared");
            m.write_in_try_allowed = self.other;
            m.uninstalled_apps.clear();
            e
        };
        let own_install = with_setting(sibling(install));
        let run = baseline
            .then(|| {
                crate::baseline::run_traced_halting(
                    &own_install,
                    lowered,
                    false,
                    opts.company_data.as_ref(),
                    &opts.perturb,
                    halt,
                )
            })
            .flatten();
        let threads = threads && run.is_some();
        let mut engine = if threads {
            with_setting(sibling(&own_install))
        } else {
            own_install.clone()
        };
        // What the run set on lane 0's tests engine after its baseline, with this lane's unseeded tables.
        if let Some(e) = Arc::get_mut(&mut engine) {
            e.unseeded = run
                .as_ref()
                .map(|b| b.unseeded.iter().map(|&t| t as usize).collect())
                .unwrap_or_default();
            e.demo_tables.clone_from(&tests.demo_tables);
            e.company_refusal.clone_from(&tests.company_refusal);
        }
        self.second = Some(Lane {
            engine,
            baseline: run.map(|b| b.baseline),
            baseline_engine: threads.then_some(own_install),
            snapshots: snapshots(),
            refusal: None,
        });
        self.baseline_time = started.elapsed();
    }

    /// The engine, baseline, install engine and snapshots of lane `lane`: lane 0's are the run's own.
    pub(crate) fn pick<'a>(
        &'a self,
        lane: usize,
        main: (
            &'a Arc<Engine>,
            &'a Option<Baseline>,
            &'a Option<Arc<Engine>>,
            &'a InstallSnapshots,
        ),
    ) -> (
        &'a Arc<Engine>,
        &'a Option<Baseline>,
        &'a Option<Arc<Engine>>,
        &'a InstallSnapshots,
    ) {
        match (lane, &self.second) {
            (1, Some(l)) => (&l.engine, &l.baseline, &l.baseline_engine, &l.snapshots),
            _ => main,
        }
    }

    /// Why no test of lane `lane` runs.
    pub(crate) fn refusal(&self, lane: usize) -> Option<String> {
        match (lane, &self.second) {
            (1, Some(l)) => l.refusal.clone(),
            _ => None,
        }
    }
}
