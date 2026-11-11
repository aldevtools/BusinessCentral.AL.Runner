//! The login start (ADR-073 phase 1): the two steps a test session takes when it opens the company, run once per engine
//! and replayed in a later session whose inputs are all equal.
//!
//! - The WorkDate step (`install::login_work_date`): Base Application's login code sets WorkDate.
//! - The company-open step (`install::test_session_company_open`): Company Triggers OnCompanyOpen(Completed) and the
//!   login (`login.rs`), then the commit that follows.
//!
//! Each step is its own recorded run (`al_vm::recorded`, which owns the session image, recording and replay). Its inputs
//! are the session image before it (compared on every use, `Session::replay_applies`), the rows, and here the key: the
//! baseline the session started from with no table changed since, and who offers the rows of the untouched tables;
//! for the company-open step also the login decision (`login::decide`, which reads only the program and the test
//! codeunit). A step that did not run to its end, or that left state no image holds, is not recorded and runs in every
//! session. The WorkDate step comes first and is replayed on its own when the company-open step cannot be.
//!
//! An engine keeps at most [`KEPT`] runs per step. The counts (recorded, replayed, not recorded by reason) go to
//! `--timings` and the work ratchet (`RunWork`). `Engine::recorded_runs` is the off switch and the state check.

use al_vm::{Engine, RecordedRun, RecordedRunsMode, Session};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

/// How many recorded runs an engine keeps per step (the oldest goes first): one per state a session starts from, and
/// the sessions of a run usually share one.
const KEPT: usize = 8;

/// What the recorded runs of one step did on an engine.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepCounts {
    /// Real runs recorded.
    pub recorded: u64,
    /// Sessions that took a recorded run instead of running the code.
    pub replayed: u64,
    /// Sessions that ran the code for real in the state check and left what the recorded run left.
    pub checked: u64,
    /// Sessions that ran the code for real without recording it, by reason.
    pub not_recorded: BTreeMap<String, u64>,
    /// Sessions whose key matched a kept run that they could not take, by the first difference (diagnosis only: the
    /// session then records a run of its own).
    pub misses: BTreeMap<String, u64>,
}

impl StepCounts {
    fn add(&mut self, other: &StepCounts) {
        self.recorded += other.recorded;
        self.replayed += other.replayed;
        self.checked += other.checked;
        for (why, n) in &other.not_recorded {
            *self.not_recorded.entry(why.clone()).or_default() += n;
        }
        for (why, n) in &other.misses {
            *self.misses.entry(why.clone()).or_default() += n;
        }
    }

    fn not_recorded_total(&self) -> u64 {
        self.not_recorded.values().sum()
    }
}

/// The counts of both steps of the login start.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoginCounts {
    pub work_date: StepCounts,
    pub company_open: StepCounts,
}

impl LoginCounts {
    /// Adds the counts of another engine of the same pass.
    pub fn add(&mut self, other: &LoginCounts) {
        self.work_date.add(&other.work_date);
        self.company_open.add(&other.company_open);
    }

    /// The counters of the work ratchet (`metrics/<major>/work.json`): recorded and not recorded rise when a replay
    /// stops applying.
    pub fn work(&self) -> [(&'static str, u64); 3] {
        [
            ("login_recorded", self.work_date.recorded + self.company_open.recorded),
            ("login_replayed", self.work_date.replayed + self.company_open.replayed),
            (
                "login_not_recorded",
                self.work_date.not_recorded_total() + self.company_open.not_recorded_total(),
            ),
        ]
    }

    /// The line for `--timings`, with the reasons; `None` when no session opened the company.
    pub fn line(&self) -> Option<String> {
        let step = |name: &str, c: &StepCounts| {
            let mut s = format!("{name} {} recorded, {} replayed", c.recorded, c.replayed);
            if c.checked > 0 {
                s.push_str(&format!(", {} checked", c.checked));
            }
            if !c.not_recorded.is_empty() {
                let why: Vec<String> = c.not_recorded.iter().map(|(w, n)| format!("{n} {w}")).collect();
                s.push_str(&format!(", not recorded: {}", why.join("; ")));
            }
            if !c.misses.is_empty() {
                let why: Vec<String> = c.misses.iter().map(|(w, n)| format!("{n} {w}")).collect();
                s.push_str(&format!(", missed a kept run: {}", why.join("; ")));
            }
            s
        };
        (*self != LoginCounts::default()).then(|| {
            format!(
                "login start (ADR-073): {}; {}",
                step("WorkDate", &self.work_date),
                step("company open", &self.company_open)
            )
        })
    }
}

/// The counts of the login start on the engines of a pass (the tests' engine and a spare one).
pub fn counts_of<'a>(engines: impl IntoIterator<Item = Option<&'a Arc<Engine>>>) -> LoginCounts {
    let mut all = LoginCounts::default();
    for e in engines.into_iter().flatten() {
        all.add(&counts(e));
    }
    all
}

/// The counts of the login start on `engine`.
pub fn counts(engine: &Engine) -> LoginCounts {
    let starts = engine.derived(LoginStarts::default);
    LoginCounts {
        work_date: starts.work_date.counts(),
        company_open: starts.company_open.counts(),
    }
}

/// The recorded runs of both steps, per engine (`Engine::derived`).
#[derive(Default)]
struct LoginStarts {
    work_date: Runs<u64, ()>,
    company_open: Runs<(u64, crate::login::Login), crate::install::CompanyOpenWrites>,
}

/// The rows a step may run on: the baseline origin the session started from with no table changed since, and who
/// offers the rows of its untouched tables. `Err`: why no run may be recorded or replayed now.
fn rows_key(session: &mut Session) -> Result<(u64, (u64, u64)), &'static str> {
    let backing = (session.store_mut().backing_identity()).ok_or("who offers the rows is not known")?;
    let baseline = session
        .baseline_origin()
        .ok_or("no baseline, or a table changed since the baseline")?;
    Ok((baseline, backing))
}

/// The test session opens the company: the WorkDate step, then the company-open step and its commit, each replayed
/// when a recorded run applies. Base Application's login code sets WorkDate before the workspace's own Company Triggers
/// subscribers run: the order of the two is not pinned (docs/probes-needed.md, WorkDate row). The login decision reads
/// only the program and the test codeunit, so it is made first, for the key.
pub(crate) fn open_company(
    engine: &Engine,
    lowered: &al_sema::lower::Lowered,
    session: &mut Session,
    disabled: bool,
) -> Result<crate::install::CompanyOpenWrites, String> {
    let decision = crate::install::login_decision(lowered, session, disabled);
    crate::install::login_work_date(engine, lowered, session)?;
    company_open_step(engine, session, &decision.clone(), |session| {
        let wrote = crate::install::test_session_company_open(lowered, session, decision)?;
        session.commit_transaction();
        Ok(wrote)
    })
}

/// Runs the WorkDate step (`real`), or replays a recorded run of it. `kept` is told about a new recorded run (the file
/// of login records, `install::note_login_record`).
pub(crate) fn work_date_step(
    engine: &Engine,
    session: &mut Session,
    real: impl FnOnce(&mut Session) -> Result<(), String>,
    kept: impl FnOnce(&Session, &RecordedRun),
) -> Result<(), String> {
    let starts = engine.derived(LoginStarts::default);
    let key = rows_key(session);
    starts.work_date.run(engine.recorded_runs, session, key, real, kept)
}

/// Runs the company-open step (`real`, which commits at its end), or replays a recorded run of it with what it wrote.
fn company_open_step(
    engine: &Engine,
    session: &mut Session,
    decision: &crate::login::Login,
    real: impl FnOnce(&mut Session) -> Result<crate::install::CompanyOpenWrites, String>,
) -> Result<crate::install::CompanyOpenWrites, String> {
    let starts = engine.derived(LoginStarts::default);
    let key = match decision {
        crate::login::Login::Refused(_) => Err("the login decision refuses the codeunit"),
        _ => rows_key(session).map(|(baseline, backing)| ((baseline, decision.clone()), backing)),
    };
    starts
        .company_open
        .run(engine.recorded_runs, session, key, real, |_, _| {})
}

/// The recorded runs of one step, keyed by `K`, each with what the caller keeps beside it (`X`).
struct Runs<K, X> {
    state: Mutex<RunsState<K, X>>,
    /// Held while a session looks for a recorded run and records one: the threads of a run start their first sessions
    /// together, and one records while the others wait for its run instead of running the same code beside it.
    recording: Mutex<()>,
}

impl<K, X> Default for Runs<K, X> {
    fn default() -> Self {
        Runs {
            state: Mutex::new(RunsState {
                backing: None,
                kept: Vec::new(),
                failed: Vec::new(),
                counts: StepCounts::default(),
            }),
            recording: Mutex::new(()),
        }
    }
}

struct RunsState<K, X> {
    /// Who offers the rows of the kept runs: a warm server runs its next request on the same engine, maybe over other
    /// test data.
    backing: Option<(u64, u64)>,
    kept: Vec<(K, Arc<RecordedRun>, X)>,
    /// Keys whose real run could not be recorded, and why: later sessions with that key run the code without waiting.
    failed: Vec<(K, String)>,
    counts: StepCounts,
}

impl<K: Clone + PartialEq, X: Clone + PartialEq + std::fmt::Debug> Runs<K, X> {
    fn counts(&self) -> StepCounts {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).counts.clone()
    }

    fn note(&self, why: String) {
        let mut st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        *st.counts.not_recorded.entry(why).or_default() += 1;
    }

    /// Replays a kept run whose key is `key` and whose image before equals the session's, or runs `real`, recording it
    /// when the session's state allows. `mode`: the off switch runs `real` alone; the state check runs `real` where a
    /// replay would apply and fails when it left anything else than the kept run.
    fn run(
        &self,
        mode: RecordedRunsMode,
        session: &mut Session,
        key: Result<(K, (u64, u64)), &'static str>,
        real: impl FnOnce(&mut Session) -> Result<X, String>,
        kept: impl FnOnce(&Session, &RecordedRun),
    ) -> Result<X, String> {
        if mode == RecordedRunsMode::Off {
            return real(session);
        }
        let (key, backing) = match key {
            Ok(k) => k,
            Err(why) => {
                self.note(why.to_string());
                return real(session);
            }
        };
        let guard = self.recording.lock().unwrap_or_else(PoisonError::into_inner);
        let (found, failed) = {
            let mut st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if st.backing != Some(backing) {
                (st.backing, st.kept, st.failed) = (Some(backing), Vec::new(), Vec::new());
            }
            let found = (st.kept.iter())
                .find(|(k, run, _)| *k == key && session.replay_applies(run))
                .map(|(_, run, x)| (run.clone(), x.clone()));
            if found.is_none() {
                let miss =
                    (st.kept.iter().rev().find(|(k, _, _)| *k == key)).and_then(|(_, run, _)| session.replay_miss(run));
                if let Some(miss) = miss {
                    *st.counts.misses.entry(miss).or_default() += 1;
                }
            }
            let failed = st.failed.iter().find(|(k, _)| *k == key).map(|(_, why)| why.clone());
            (found, failed)
        };
        if let Some((run, x)) = found {
            if mode == RecordedRunsMode::Check {
                drop(guard);
                return self.check(session, &run, real, x);
            }
            if session.replay(&run).is_ok() {
                drop(guard);
                self.state
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .counts
                    .replayed += 1;
                return Ok(x);
            }
        }
        if let Some(why) = failed {
            drop(guard);
            self.note(why);
            return real(session);
        }
        let recording = match session.start_recording() {
            Ok(r) => r,
            Err(why) => {
                drop(guard);
                self.note(why.to_string());
                return real(session);
            }
        };
        let out = real(session);
        let recorded = match &out {
            Ok(_) => session.finish_recording(recording).map_err(|why| why.to_string()),
            Err(_) => {
                session.abandon_recording(recording);
                Err("the run ended in an error or a refusal".to_string())
            }
        };
        let mut st = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        match (recorded, &out) {
            (Ok(run), Ok(x)) => {
                kept(session, &run);
                if st.kept.len() == KEPT {
                    st.kept.remove(0);
                }
                st.kept.push((key, Arc::new(run), x.clone()));
                st.counts.recorded += 1;
            }
            (recorded, _) => {
                let why = recorded
                    .err()
                    .unwrap_or_else(|| "the run ended in an error or a refusal".to_string());
                if st.failed.len() == KEPT {
                    st.failed.remove(0);
                }
                st.failed.push((key, why.clone()));
                *st.counts.not_recorded.entry(why).or_default() += 1;
            }
        }
        drop(st);
        drop(guard);
        out
    }

    /// The state check (ADR-073, Conformance): the session runs the code for real where it could have taken `run`, and
    /// must be left as the replay would have left it.
    fn check(
        &self,
        session: &mut Session,
        run: &RecordedRun,
        real: impl FnOnce(&mut Session) -> Result<X, String>,
        x: X,
    ) -> Result<X, String> {
        let fail = |what: String| {
            Err(format!(
                "ADR-073 state check: the login start differs from its real run: {what}"
            ))
        };
        let recording = match session.start_recording() {
            Ok(r) => r,
            Err(why) => return fail(format!("the real run cannot be recorded: {why}")),
        };
        let out = real(session);
        if out.is_err() {
            session.abandon_recording(recording);
            return fail("the real run ended in an error or a refusal".into());
        }
        match session.finish_recording(recording) {
            Err(why) => fail(format!("the real run is not recordable: {why}")),
            Ok(again) => match run.result_difference(&again) {
                Some(d) => fail(d),
                None if out.as_ref().ok() != Some(&x) => fail(format!("what the step keeps: {x:?} against {out:?}")),
                None => {
                    self.state.lock().unwrap_or_else(PoisonError::into_inner).counts.checked += 1;
                    out
                }
            },
        }
    }
}
