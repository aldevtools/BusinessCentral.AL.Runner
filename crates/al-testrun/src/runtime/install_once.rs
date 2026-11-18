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
) -> Vec<al_vm::SkippedInstall> {
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

/// What publishing the app does first for a test codeunit's `session`: run its install codeunits, then commit. Returns the
/// routines that could not run. With an install baseline that ran them, the session started from it; a session that starts
/// from no baseline takes what they left from their run once per run ([`start_installed`]); one that starts from a
/// baseline they did not run on runs them on that baseline.
pub(super) fn installs_for(
    lowered: &Lowered,
    session: &mut Session,
    baseline: Option<&al_vm::Baseline>,
    scratch: &Scratch,
    execute: bool,
    observed: bool,
    as_made_here: bool,
) -> Vec<al_vm::SkippedInstall> {
    if let Some(skipped) = baseline.and_then(|b| b.workspace_installs.clone()) {
        return skipped;
    }
    baseline
        .is_none()
        .then(|| start_installed(lowered, session, scratch, execute, observed, as_made_here))
        .flatten()
        .unwrap_or_else(|| run_installs_in(lowered, session, &scratch.installs))
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

/// What the install code left the last time it ran for a run ([`start_installed`]).
struct Installed {
    /// The engine it ran on (`Engine::uid`) and how the codeunit's session was set up.
    key: Key,
    /// The resource files it read (`Session::resource_reads`) and their texts when it ran (`ResourceReads::texts`).
    reads: al_vm::ResourceReads,
    texts: al_vm::ReadTexts,
    /// What a codeunit's session starts from. `None`: the install code left state a new session does not start from
    /// (`Session::install_left`), or data that cannot be shared between threads (`Baseline::freeze`); each codeunit's session
    /// then runs the install code itself, as on a run without this path. The run keeps that verdict while the key and the
    /// texts of the resource files the install code read stay the same.
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

/// The start of a pass: its entry's resource texts (`Engine::resource_overrides`), of which [`start_installed`] compares
/// those of the files the install code read.
pub(super) fn begin_pass(engine: &Engine, resources: &al_vm::ResourceOverrides) {
    if let Ok(mut o) = engine.resource_overrides.write() {
        o.clone_from(resources);
    }
}

/// The end of a run's passes: the resource texts of its entries are not the next run's. Returns how many times the
/// install code ran for the run's codeunits (`Phases::install_runs`).
pub(super) fn end_run(engine: &Engine, installs: &RunInstalls) -> usize {
    if let Ok(mut o) = engine.resource_overrides.write() {
        o.clear();
    }
    installs.runs.load(std::sync::atomic::Ordering::Relaxed)
}

/// The install code of the apps under test runs once per run, not once per test codeunit (al-runner issue 8144), in a
/// session of its own, as publishing the app runs it (record/BfInstallTests.al:142-144; the install baseline does the same,
/// `baseline.rs`). `session`, the codeunit's new session, starts from what it left: the data (`Baseline`) and the rest
/// `Session::install_left` names (language, WorkDate, clock and the counters a later read compares with), which it returns
/// only when every other field of that session still holds what a new session holds. Returned are the routines that could
/// not run, as [`run_installs_in`] returns them. Only for a codeunit whose session starts from no install baseline, as the
/// session the install code runs in here does.
///
/// It runs again for a codeunit whose texts (an entry's, ADR-028 decision 11) of the resource files its last run read
/// differ from those that run read. `None` (the caller runs it in `session`): a scratch run (`execute`), a run that
/// follows the code it runs per codeunit (coverage, the impact trace, `--explain`, the profiler), and install code that left
/// more than `Session::install_left` carries, but for the codeunit whose call ran it:
///
/// for that codeunit, the session the install code ran in becomes its session, if `as_made_here`: the caller made
/// `session` as [`install`] makes its own (`Session::with_host` on the same host, the session flags, no code run), so it
/// is the session the caller would have run the install code in. A run in which no codeunit can start from what the
/// install code left thus runs it once per codeunit, as a run without this path does, unless single instances of an
/// earlier codeunit moved into the codeunit whose call ran it (`as_made_here` false, the strict-order lane): that
/// codeunit then runs it once more in its own session. Otherwise install count and CPU stay as on such a run; wall time
/// does not: this call holds `kept` while the install code runs, so with as many test threads as codeunits the others
/// wait and then each runs it, two install times in a row where a run without this path takes one.
fn start_installed(
    lowered: &Lowered,
    session: &mut Session,
    scratch: &Scratch,
    execute: bool,
    observed: bool,
    as_made_here: bool,
) -> Option<Vec<al_vm::SkippedInstall>> {
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
    // A panic in the install code leaves the slot as it was (it is set only after the install returned).
    let mut kept = scratch
        .installs
        .kept
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let fits = kept.as_ref().is_some_and(|i| {
        i.key == key
            && engine
                .resource_overrides
                .read()
                .is_ok_and(|o| i.reads.texts(&o) == i.texts)
    });
    if !fits {
        let (made, ran_in) = install(lowered, &engine, scratch, key);
        *kept = Some(made);
        if let (Some((own, skipped)), true) = (ran_in, as_made_here) {
            drop(kept);
            *session = own;
            return Some(skipped);
        }
    }
    let left = kept.as_ref().expect("kept or made just now").left.as_ref()?;
    let mut data = left.data.thaw(Some(&engine));
    data.install_rows = Some(left.install_rows.clone());
    let rest = left.rest.clone();
    drop(kept);
    session.restore_install_left(&data, &rest);
    data.workspace_installs
}

/// The session the install code ran in, with the routines that could not run, when it left more than
/// `Session::install_left` carries; nothing was done to it after the install code returned.
type RanIn = Option<(Session, Vec<al_vm::SkippedInstall>)>;

/// Runs the install code in a session of its own, made as the codeunit's session is (`run.rs` `run_codeunit_inner`).
fn install(lowered: &Lowered, engine: &Arc<Engine>, scratch: &Scratch, key: Key) -> (Installed, RanIn) {
    let mut host = crate::host::host_perturbed(scratch.clock_start, &scratch.perturb);
    host.run_seed = scratch.run_seed.seed;
    // Files on a real folder (`--files-dir`) outlive the session: a write of this run would be seen by the codeunits'
    // own install runs, so it runs only on the session's own in-memory files.
    if !host.files.in_memory() {
        let none = Installed {
            key,
            reads: Default::default(),
            texts: Vec::new(),
            left: None,
        };
        return (none, None);
    }
    let mut s = Session::with_host(engine.clone(), host);
    scratch.flags.apply(&mut s);
    let skipped = run_installs_in(lowered, &mut s, &scratch.installs);
    let reads = s.resource_reads();
    let texts = engine
        .resource_overrides
        .read()
        .map(|o| reads.texts(&o))
        .unwrap_or_default();
    // Before the capture, which commits and clears part of what the check reads.
    let Some(rest) = s.install_left() else {
        let installed = Installed {
            key,
            reads,
            texts,
            left: None,
        };
        return (installed, Some((s, skipped)));
    };
    let mut data = s.capture_baseline();
    data.workspace_installs = Some(skipped);
    let left = data.freeze().ok().map(|data| Left {
        data,
        install_rows: s.install_rows(),
        rest,
    });
    let installed = Installed {
        key,
        reads,
        texts,
        left,
    };
    (installed, None)
}
