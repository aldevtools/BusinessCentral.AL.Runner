//! ADR-073 Phase 2: the suites of one compile whose CI job runs on a service tier with another
//! DisableWriteInsideTryFunctions than the run's (`RunOptions::write_in_try_suites`, the corpus suite `trywrite-allowed`)
//! run on an engine of their own, over the same program, with an install baseline of their own.
//!
//! - Lane 0, the run's own setting: the run's engine. It leaves the apps of the other lane uninstalled
//!   (`Engine::uninstalled_apps`): BC's job of those suites does not have them, so their install code does not run there.
//! - Lane 1, the other setting: a sibling engine (it shares the program and the compiled code) with the setting, whose
//!   install baseline runs the install code of its apps under that setting, as the suite's own job does. It leaves lane
//!   0's own apps uninstalled in turn (the OnPrem suite apps), so its tenant has the apps of the suite's own pass: the
//!   main corpus app, the try-write app and their dependencies.
//!
//! An app is not installed for a lane when only the other lane's suite apps need it: the dependency closure of the other
//! lane's suite apps less the closure of its own (`Engine::app_installed`: no install code, no subscriber, no table
//! extension trigger, no listing row, no lookup by run-time id).
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
    /// The parts of lane 0's reach the check's walk cannot follow because they are not compiled
    /// (`shared_pass::Reach::not_compiled`), printed as information.
    pub not_compiled: Vec<String>,
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
    /// The notes of the lane's install baseline and how many rows it holds.
    notes: Vec<String>,
    rows: usize,
}

/// The lanes of a run.
pub(crate) struct Lanes {
    /// Per suite: 1 when it is listed in `RunOptions::write_in_try_suites` (its setting is the opposite of the run's), else 0.
    of_suite: Vec<usize>,
    /// The setting of lane 1.
    other: bool,
    /// The apps lane 0's engine leaves uninstalled: those only lane 1's suite apps need.
    pub(crate) uninstalled: BTreeSet<u32>,
    /// The apps lane 1's engine leaves uninstalled: those only lane 0's suite apps need.
    other_uninstalled: BTreeSet<u32>,
    /// Why lane 1 cannot run beside lane 0 at all (an app both lanes use).
    shared_app: Option<String>,
    second: Option<Lane>,
    pub(crate) notes: Vec<SharedPassNote>,
    /// How long lane 1's install baseline took.
    pub(crate) baseline_time: std::time::Duration,
}

impl Lanes {
    pub(crate) fn of(suites: &[SuiteRun], opts: &RunOptions, program_apps: &[al_ir::AppInfo]) -> Lanes {
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
        let (uninstalled, other_uninstalled) = if shared_app.is_some() || second.is_empty() {
            (BTreeSet::new(), BTreeSet::new())
        } else {
            let (main, second) = (closure(program_apps, &main), closure(program_apps, &second));
            (&second - &main, &main - &second)
        };
        debug_assert!(
            opts.entries.is_empty() || second.is_empty(),
            "entries and per-suite settings do not mix"
        );
        Lanes {
            of_suite,
            other,
            uninstalled,
            other_uninstalled,
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
        // Every object of lane 0's suite apps (its test codeunits, their helpers, its install code): what lane 1's apps
        // must not reach.
        let main_apps: BTreeSet<u32> = (suites.iter().enumerate())
            .filter(|(si, _)| self.runs(0, *si))
            .filter_map(|(_, s)| s.app.map(|a| a as u32))
            .collect();
        let heads = lowered.program.objects.heads();
        let main_roots: Vec<usize> = (0..heads.len())
            .filter(|&oi| main_apps.contains(&heads[oi].app))
            .collect();
        let checked: Result<Vec<_>, _> = (self.uninstalled.iter())
            .map(|&app| crate::shared_pass::check(install, lowered, app, &main_roots))
            .collect();
        let (breach, not_compiled) = match (&self.shared_app, checked) {
            (Some(why), _) => (Some(why.clone()), Vec::new()),
            (None, Err(b)) => (Some(b.0), Vec::new()),
            (None, Ok(r)) => {
                let mut all: Vec<String> = r.into_iter().flat_map(|r| r.not_compiled).collect();
                all.sort();
                all.dedup();
                (None, all)
            }
        };
        let notes: Vec<SharedPassNote> = (suites.iter().enumerate())
            .filter(|(si, _)| self.runs(1, *si))
            .map(|(_, s)| SharedPassNote {
                suite: s.name.clone(),
                breach: breach.clone(),
                not_compiled: not_compiled.clone(),
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
                notes: Vec::new(),
                rows: 0,
            });
            return;
        }
        let with_setting = |mut e: Arc<Engine>| {
            let m = Arc::get_mut(&mut e).expect("a new sibling engine is not shared");
            m.write_in_try_allowed = self.other;
            m.uninstalled_apps.clone_from(&self.other_uninstalled);
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
        let unseeded = (run.as_ref())
            .map(|b| b.unseeded.iter().map(|&t| t as usize).collect())
            .unwrap_or_default();
        let (engine, own_install) = tests_engine(own_install, threads, &|e| with_setting(sibling(e)), unseeded, tests);
        let (notes, rows) = run
            .as_ref()
            .map_or((Vec::new(), 0), |b| (b.notes.clone(), b.baseline.rows()));
        self.second = Some(Lane {
            engine,
            notes,
            rows,
            baseline: run.map(|b| b.baseline),
            baseline_engine: own_install,
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

    /// Lane 1's install baseline beside lane 0's: prints its notes (unless `quiet_baseline`), adds its assumptions and
    /// install orders to `facts`, and gives how many rows it holds (0 without one).
    pub(crate) fn merge_baseline(&self, facts: &mut crate::baseline::NoteFacts, opts: &RunOptions) -> usize {
        let Some(l) = &self.second else { return 0 };
        if !opts.quiet_baseline {
            for n in &l.notes {
                eprintln!("baseline (DisableWriteInsideTryFunctions = {}): {n}", self.other);
            }
        }
        facts.merge(crate::baseline::NoteFacts::of(&l.notes));
        l.rows
    }

    /// What lane 1's engines met that the compile left out (`narrowing::take_install`), with what the install routines of
    /// its baseline met when the baseline came from a file.
    pub(crate) fn take_install(&self) -> super::narrowing::Met {
        let Some(l) = &self.second else {
            return super::narrowing::Met::default();
        };
        let met = super::narrowing::take_install(std::iter::once(&*l.engine).chain(l.baseline_engine.as_deref()));
        (l.baseline.as_ref().and_then(|b| b.workspace_installs.as_deref()))
            .into_iter()
            .flatten()
            .fold(met, |m, (_, why)| m.or(super::narrowing::Met::of_reason(why)))
    }

    /// Why no test of lane `lane` runs.
    pub(crate) fn refusal(&self, lane: usize) -> Option<String> {
        match (lane, &self.second) {
            (1, Some(l)) => l.refusal.clone(),
            _ => None,
        }
    }
}

/// The apps `apps` (program app indexes) and every app they depend on, transitively.
fn closure(program_apps: &[al_ir::AppInfo], apps: &[Option<usize>]) -> BTreeSet<u32> {
    let index = |id: &str| program_apps.iter().position(|a| a.id.eq_ignore_ascii_case(id));
    let mut out = BTreeSet::new();
    let mut todo: Vec<usize> = apps.iter().flatten().copied().collect();
    while let Some(a) = todo.pop() {
        if out.insert(a as u32) {
            todo.extend(program_apps[a].dependencies.iter().filter_map(|d| index(d)));
        }
    }
    out
}

/// Lane 1's tests engine and, with several test threads, the engine its install code ran on (`install`), given what the
/// run set on lane 0's tests engine after its baseline (`main`: its demo tables and company refusal) and lane 1's own
/// unseeded tables. With one test thread, or when no install baseline ran, the tests run on `install` itself.
fn tests_engine(
    install: Arc<Engine>,
    threads: bool,
    sibling: &dyn Fn(&Arc<Engine>) -> Arc<Engine>,
    unseeded: std::collections::HashSet<usize>,
    main: &Engine,
) -> (Arc<Engine>, Option<Arc<Engine>>) {
    let (mut engine, install) = if threads {
        (sibling(&install), Some(install))
    } else {
        (install, None)
    };
    // Nothing holds the engine yet (a baseline keeps the engine's program and tables, not the engine).
    let e = Arc::get_mut(&mut engine).expect("lane 1's tests engine is not shared before its tests start");
    e.unseeded = unseeded;
    e.demo_tables.clone_from(&main.demo_tables);
    e.company_refusal.clone_from(&main.company_refusal);
    (engine, install)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lane 0's tests engine with demo tables and a company refusal, as the run sets them after its baseline.
    fn main_engine() -> Engine {
        let mut e = Arc::into_inner(Engine::new(al_ir::Program::default(), vec![])).unwrap();
        e.demo_tables = [7usize].into_iter().collect();
        e.company_refusal = Some("why".into());
        e
    }

    fn assert_set(e: &Engine) {
        assert_eq!(e.unseeded, [3usize].into_iter().collect());
        assert_eq!(e.demo_tables, [7usize].into_iter().collect());
        assert_eq!(e.company_refusal.as_deref(), Some("why"));
    }

    /// One test thread (`AL_TEST_THREADS=1`), and a run whose install baseline was skipped (no baseline: one engine):
    /// lane 1's tests run on the engine its install code ran on, which gets the settings too.
    #[test]
    fn one_engine_gets_the_settings() {
        let main = main_engine();
        let install = Engine::new(al_ir::Program::default(), vec![]);
        let (engine, own) = tests_engine(install, false, &|e| e.sibling(), [3usize].into_iter().collect(), &main);
        assert!(own.is_none());
        assert_set(&engine);
    }

    /// Several test threads: the tests engine is a sibling of the install engine, and gets the settings.
    #[test]
    fn a_sibling_tests_engine_gets_the_settings() {
        let main = main_engine();
        let install = Engine::new(al_ir::Program::default(), vec![]);
        let (engine, own) = tests_engine(install, true, &|e| e.sibling(), [3usize].into_iter().collect(), &main);
        assert!(own.is_some());
        assert_set(&engine);
    }
}
