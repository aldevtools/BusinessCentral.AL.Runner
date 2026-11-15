//! The install code of the apps under test for a test codeunit that starts from no install baseline (no dependency app,
//! or no selected test can observe the baseline, `observe.rs`). Publishing the app runs it once, in a session of its own
//! (`blocked.rs` `run_install_codeunits`).
use super::*;

/// Runs the install code of the apps under test in `session`, as the codeunit's session does when no install baseline ran
/// it: the tables of the routines that could not run, and why. `installs` counts the run.
pub(super) fn run_installs_in(
    lowered: &Lowered,
    session: &mut Session,
    installs: &RunInstalls,
) -> Vec<(BTreeSet<u32>, String)> {
    // As the baseline does (`baseline::run_traced`, ADR-046 amendment 4): the corpus CI database's session company is an
    // evaluation company.
    if session.engine.test_data.is_none() && session.companies().len() > 1 {
        session.mark_evaluation_company();
    }
    installs.runs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    session.set_install_scope(true);
    let skipped = run_install_codeunits(lowered, session);
    session.set_install_scope(false);
    skipped
}

/// What the install code left for the codeunits of one run, shared by every test thread of the run ([`start_installed`]).
/// The run makes one (`corpus.rs`) and hands it to each codeunit (`Unit`, `Scratch`); it ends with the run.
#[derive(Default)]
pub(crate) struct RunInstalls {
    /// What the install code left the last time it ran for the run. Held while the install code runs, so that a second
    /// thread waits for it instead of running it too.
    kept: std::sync::Mutex<Option<Installed>>,
    /// How many times the install code ran for the run's codeunits (`Phases::install_runs`).
    runs: std::sync::atomic::AtomicUsize,
}

impl RunInstalls {
    /// How many times the install code ran for the run's codeunits (`Phases::install_runs`).
    pub(super) fn runs(&self) -> usize {
        self.runs.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// What the install code left the last time it ran for a run ([`start_installed`]).
struct Installed {
    /// The engine it ran on (`Engine::uid`) and how the codeunit's session was set up.
    key: Key,
    /// The resource texts of the run when it ran (`Engine::resource_overrides`), and whether it read a resource file.
    overrides: al_vm::ResourceOverrides,
    read_resources: bool,
    /// What a codeunit's session starts from. `None`: the install code left state a new session does not start from
    /// (`Session::install_left`), or data that cannot be shared between threads (`Baseline::freeze`); each codeunit's session
    /// then runs the install code itself, as on a run without this path, and the run keeps that verdict.
    left: Option<Left>,
}

/// What [`Installed`] hands to each codeunit's session.
struct Left {
    /// The data it left, with the routines that could not run (`Baseline::workspace_installs`), frozen for every thread
    /// (`Baseline::freeze`, ADR-069).
    data: al_vm::SharedBaseline,
    /// The install rows the session had (`Baseline::install_rows`, which a thawed baseline does not hold).
    install_rows: BTreeSet<u32>,
    /// The rest the install code left (`Session::install_left`).
    rest: al_vm::InstallLeft,
}

/// Everything a codeunit's session is made from besides the program, the same for every codeunit of a run.
#[derive(PartialEq)]
struct Key {
    engine: u64,
    clock_start: Option<i64>,
    perturb: crate::stability::Perturb,
    run_seed: crate::seed::RunSeed,
    flags: super::session_flags::SessionFlags,
}

/// The end of a run's passes: the resource texts of its entries (`Engine::resource_overrides`) are not the next run's.
pub(super) fn end_run(engine: &Engine) {
    if let Ok(mut o) = engine.resource_overrides.write() {
        o.clear();
    }
}

/// The install code of the apps under test runs once per run, not once per test codeunit (al-runner issue 8144), in a
/// session of its own, as publishing the app runs it (record/BfInstallTests.al:142-144; the install baseline does the same,
/// `baseline.rs`). `session`, the codeunit's new session, starts from what it left: the data (`Baseline`) and the rest
/// `Session::install_left` names (language, WorkDate, clock and the counters a later read compares with), which it returns
/// only when every other field of that session still holds what a new session holds. Returned are the routines that could
/// not run, as [`run_installs_in`] returns them. Only for a codeunit whose session starts from no install baseline, as the
/// session the install code runs in here does.
///
/// It runs again for a codeunit whose resource texts (an entry's, ADR-028 decision 11) differ from those of its last run
/// when that run read a resource file. `None` (the caller runs it in `session`): a scratch run (`execute`), a run that
/// follows the code it runs per codeunit (coverage, the impact trace, `--explain`, the profiler), and install code that left
/// more than `Session::install_left` carries.
pub(super) fn start_installed(
    lowered: &Lowered,
    session: &mut Session,
    scratch: &Scratch,
    execute: bool,
    observed: bool,
) -> Option<Vec<(BTreeSet<u32>, String)>> {
    let follows_code = session.notes_code_run() || observed || crate::profiling::active();
    if execute || follows_code {
        return None;
    }
    let engine = session.engine.clone();
    let key = Key {
        engine: engine.uid,
        clock_start: scratch.clock_start,
        perturb: scratch.perturb.clone(),
        run_seed: scratch.run_seed,
        flags: scratch.flags,
    };
    let overrides = engine.resource_overrides.read().map(|o| o.clone()).unwrap_or_default();
    // A panic in the install code leaves the slot as it was (it is set only after the install returned).
    let mut kept = scratch
        .installs
        .kept
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let fits = kept
        .as_ref()
        .is_some_and(|i| i.key == key && (!i.read_resources || i.overrides == overrides));
    if !fits {
        *kept = Some(install(lowered, &engine, scratch, key, overrides));
    }
    let left = kept.as_ref().expect("kept or made just now").left.as_ref()?;
    let mut data = left.data.thaw(Some(&engine));
    data.install_rows = Some(left.install_rows.clone());
    let rest = left.rest.clone();
    drop(kept);
    session.restore_install_left(&data, &rest);
    data.workspace_installs
}

/// Runs the install code in a session of its own, made as the codeunit's session is (`run.rs` `run_codeunit_inner`).
fn install(
    lowered: &Lowered,
    engine: &Arc<Engine>,
    scratch: &Scratch,
    key: Key,
    overrides: al_vm::ResourceOverrides,
) -> Installed {
    let mut host = crate::host::host_perturbed(scratch.clock_start, &scratch.perturb);
    host.run_seed = scratch.run_seed.seed;
    // Files on a real folder (`--files-dir`) outlive the session: a write of this run would be seen by the codeunits'
    // own install runs, so it runs only on the session's own in-memory files.
    if !host.files.in_memory() {
        return Installed {
            key,
            overrides,
            read_resources: false,
            left: None,
        };
    }
    let mut s = Session::with_host(engine.clone(), host);
    scratch.flags.apply(&mut s);
    let skipped = run_installs_in(lowered, &mut s, &scratch.installs);
    // Before the capture, which commits and clears part of what the check reads.
    let left = s.install_left().and_then(|rest| {
        let mut data = s.capture_baseline();
        data.workspace_installs = Some(skipped);
        let data = data.freeze().ok()?;
        Some(Left {
            data,
            install_rows: s.install_rows(),
            rest,
        })
    });
    Installed {
        key,
        overrides,
        read_resources: s.read_resources(),
        left,
    }
}
