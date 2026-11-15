//! The install code of the apps under test for a test codeunit that starts from no install baseline (no dependency app,
//! or no selected test can observe the baseline, `observe.rs`). Publishing the app runs it once, in a session of its own
//! (`blocked.rs` `run_install_codeunits`).
use super::*;

/// Runs the install code of the apps under test in `session`, as the codeunit's session does when no install baseline ran
/// it: the tables of the routines that could not run, and why.
pub(super) fn run_installs_in(lowered: &Lowered, session: &mut Session) -> Vec<(BTreeSet<u32>, String)> {
    // As the baseline does (`baseline::run_traced`, ADR-046 amendment 4): the corpus CI database's session company is an
    // evaluation company.
    if session.engine.test_data.is_none() && session.companies().len() > 1 {
        session.mark_evaluation_company();
    }
    session
        .engine
        .install_runs
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    session.set_install_scope(true);
    let skipped = run_install_codeunits(lowered, session);
    session.set_install_scope(false);
    skipped
}

/// What the install code left the last time it ran on this thread ([`start_installed`]), and for which run.
struct Installed {
    /// The engine it ran on (`Engine::uid`) and how the codeunit's session was set up.
    key: Key,
    /// The resource texts of the run when it ran (`Engine::resource_overrides`), and whether it read a resource file.
    overrides: al_vm::ResourceOverrides,
    read_resources: bool,
    /// The data it left, with the routines that could not run (`Baseline::workspace_installs`) and the install rows the
    /// session had (`Baseline::install_rows`), and the rest it left (`Session::install_left`). `None`: it left state a new
    /// session does not start from, and each codeunit's session runs the install code itself, as on a run without this
    /// path; the thread keeps that verdict for the run.
    left: Option<(al_vm::Baseline, al_vm::InstallLeft)>,
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

thread_local! {
    /// A test thread runs codeunits of one run at a time; [`forget`] clears it at the start and the end of a run.
    static INSTALLED: std::cell::RefCell<Option<Installed>> = const { std::cell::RefCell::new(None) };
}

/// Forgets what [`start_installed`] kept on this thread (at the start of a run: an earlier run's is not this run's).
pub(super) fn forget() {
    INSTALLED.with(|i| i.borrow_mut().take());
}

/// The end of a run's passes: the resource texts of its entries (`Engine::resource_overrides`) and what the install code
/// left on this thread are not the next run's. Returns how many times the install code ran for the run's codeunits
/// (`Phases::install_runs`).
pub(super) fn end_run(engine: &Engine) -> usize {
    if let Ok(mut o) = engine.resource_overrides.write() {
        o.clear();
    }
    forget();
    engine.install_runs.load(std::sync::atomic::Ordering::Relaxed)
}

/// The install code of the apps under test runs once per test thread and run, not once per test codeunit (al-runner
/// issue 8144), in a session of its own, as publishing the app runs it (record/BfInstallTests.al:142-144; the install
/// baseline does the same, `baseline.rs`). `session`, the codeunit's new session, starts from what it left: the data, the
/// language and WorkDate (`Session::carried_state`, the state besides the database a session hands on) and the clock;
/// returned are the routines that could not run, as [`run_installs_in`] returns them. Only for a codeunit whose session
/// starts from no install baseline, as the session the install code runs in here does.
///
/// It runs again for a codeunit whose resource texts (an entry's, ADR-028 decision 11) differ from those of its last run
/// when that run read a resource file. `None` (the caller runs it in `session`): a scratch run (`execute`), and a run that
/// follows the code it runs per codeunit (coverage, the impact trace, `--explain`, the profiler).
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
    INSTALLED.with(|cell| {
        let mut kept = cell.borrow_mut();
        let fits = kept
            .as_ref()
            .is_some_and(|i| i.key == key && (!i.read_resources || i.overrides == overrides));
        if !fits {
            *kept = Some(install(lowered, &engine, scratch, key, overrides));
        }
        let (data, left) = kept.as_ref().expect("kept or made just now").left.as_ref()?;
        session.restore_install_left(data, left);
        data.workspace_installs.clone()
    })
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
    let mut s = Session::with_host(engine.clone(), host);
    scratch.flags.apply(&mut s);
    let skipped = run_installs_in(lowered, &mut s);
    // Before the capture, which commits and clears part of what the check reads.
    let left = s.install_left().map(|left| {
        let mut data = s.capture_baseline();
        data.workspace_installs = Some(skipped);
        data.install_rows = Some(s.install_rows());
        (data, left)
    });
    Installed {
        key,
        overrides,
        read_resources: s.read_resources(),
        left,
    }
}
