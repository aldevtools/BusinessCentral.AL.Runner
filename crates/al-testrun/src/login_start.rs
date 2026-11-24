//! Login starts (al-runner issue 8211): a test session whose login inputs ([`crate::login_replay::Key`])
//! were seen before takes what that login left (a [`Start`]) in place of running it again. The first session of a key runs
//! the login and keeps its al-vm `LoginImage` with the open's writes and assumption marks ([`record`]); a later session of
//! the key thaws the image into itself in `runtime/run.rs` `open_company` ([`replay`]), when [`why_not`] finds nothing
//! against it. A key whose session the image refuses (`NotImaged`) runs the real login every time and is not frozen again.
//! It is off unless `RunOptions::login_replay` (`ALRUNNER_LOGIN_REPLAY=1`) turns it on; under the login replay check
//! (`ALRUNNER_LOGIN_REPLAY_CHECK=1`) no session takes a start, and `login_replay.rs` compares each later session of a key
//! with a thaw of its start instead.
//!
//! The starts live on the engine ([`starts`], `Engine::derived`): at most [`MAX_STARTS`] and [`MAX_BYTES`] of
//! `LoginImage::kept_bytes`, least recently used out; one start over [`MAX_START_BYTES`] is not kept (`too large`). The
//! memory pool reads [`Starts::bytes`] through the shared-bytes meter (`runtime/corpus.rs`), so an eviction gives its
//! bytes back.

use crate::install::CompanyOpenWrites;
use crate::login_replay::Key;
use al_builtins::assumptions::HitBits;
use al_vm::{BaseShape, Engine, LoginImage, NotImaged, Session};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

mod outside;

/// The most starts an engine keeps (ADR-073 Phase 1).
pub(crate) const MAX_STARTS: usize = 8;
/// The most bytes (`LoginImage::kept_bytes`) the starts of an engine hold together.
pub(crate) const MAX_BYTES: u64 = 64 << 20;
/// The most bytes one start may hold: a larger one is not kept (`too large`), so one start never empties the others.
pub(crate) const MAX_START_BYTES: u64 = 16 << 20;
/// The most keys an engine remembers as refused (no image, or too large).
pub(crate) const MAX_REFUSED: usize = 64;

/// What the raised login left in the first session of a key: the session image, what the open noted for the tests, and
/// the assumption marks the open made (a replay raises them again, so a flagged pass stays flagged).
pub(crate) struct Start {
    pub image: LoginImage,
    pub wrote: CompanyOpenWrites,
    pub hits: HitBits,
}

// A start is kept on the engine and taken by sessions of any thread.
const _: fn() = || {
    fn both<T: Send + Sync>() {}
    both::<Start>();
};

/// Why a session does not take a start, each counted in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotTaken {
    /// The run notes the code it runs: `--explain` or an observer, coverage, the impact trace, the profiler.
    NotesCode,
    /// A stability perturbation or a set clock start: shapes the check never measured.
    Perturbed,
    /// Single instances of the codeunit before were moved in (the strict-order lane).
    MovedIn,
    /// The wall clock (`ALRUNNER_REAL_CLOCK`): a login that stamps the time it reads would hand a later session an
    /// earlier time, and no key holds the clock.
    RealClock,
    /// A real files directory (`--files-dir`): a login that reads a file another session wrote reads what no key holds.
    RealFiles,
    /// The session is not a thaw target (`Session::thaw_target_why`).
    Target(&'static str),
    /// The store the session starts from has another shape than the start's (`LoginImage::base_shape`).
    BaseShape,
    /// The session has no key: a row holds a value with no byte form, of this kind (`login_replay::NoKey`).
    NoKey(&'static str),
    /// The thaw refused the session (`Session::thaw_login_image`).
    NotThawed(al_vm::NotThawed),
}

impl NotTaken {
    /// The reason as the report names it.
    fn reason(self) -> String {
        match self {
            NotTaken::NotesCode => "the run notes the code it runs".into(),
            NotTaken::Perturbed => "a stability perturbation or a set clock start".into(),
            NotTaken::MovedIn => "single instances moved in".into(),
            NotTaken::RealClock => "the real clock (ALRUNNER_REAL_CLOCK)".into(),
            NotTaken::RealFiles => "a real files directory (--files-dir)".into(),
            NotTaken::Target(field) => format!("target: {field}"),
            NotTaken::BaseShape => "base shape".into(),
            NotTaken::NoKey(kind) => format!("no key: {kind}"),
            NotTaken::NotThawed(why) => format!("not thawed: {}", why.reason()),
        }
    }
}

/// The starts of one engine (see the module comment).
#[derive(Default)]
pub(crate) struct Starts {
    inner: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// The kept starts, the one used last at the end.
    kept: Vec<(Key, Arc<Start>)>,
    /// The keys whose start is not kept, the newest at the end.
    refused: Vec<Key>,
    replayed: u64,
    /// Per reason (`NotImaged::reason`), the keys whose session was not imaged.
    not_imaged: BTreeMap<String, u64>,
    too_large: u64,
    /// Per reason ([`NotTaken::reason`]), the sessions the gate kept from a start.
    not_taken: BTreeMap<String, u64>,
}

impl State {
    fn bytes(&self) -> u64 {
        self.kept.iter().map(|(_, s)| s.image.kept_bytes()).sum()
    }

    fn refuse(&mut self, key: Key) {
        if self.refused.len() == MAX_REFUSED {
            self.refused.remove(0);
        }
        self.refused.push(key);
    }
}

impl Starts {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether `key` has a start or was refused: its session is then not frozen again.
    pub(crate) fn knows(&self, key: &Key) -> bool {
        let s = self.state();
        s.kept.iter().any(|(k, _)| k == key) || s.refused.contains(key)
    }

    /// The start of `key`, the one used last from now on.
    pub(crate) fn take(&self, key: &Key) -> Option<Arc<Start>> {
        let mut s = self.state();
        let at = s.kept.iter().position(|(k, _)| k == key)?;
        let entry = s.kept.remove(at);
        let start = entry.1.clone();
        s.kept.push(entry);
        Some(start)
    }

    /// Keeps the start of `key` (or why its session has none) within the caps: the first of a key wins; a start over
    /// [`MAX_START_BYTES`] is refused; the least recently used go until [`MAX_STARTS`] and [`MAX_BYTES`] hold.
    pub(crate) fn record(&self, key: Key, start: Result<Start, NotImaged>) {
        let mut s = self.state();
        if s.kept.iter().any(|(k, _)| *k == key) || s.refused.contains(&key) {
            return;
        }
        let start = match start {
            Ok(start) => start,
            Err(why) => {
                *s.not_imaged.entry(why.reason()).or_default() += 1;
                s.refuse(key);
                return;
            }
        };
        let bytes = start.image.kept_bytes();
        if bytes > MAX_START_BYTES {
            s.too_large += 1;
            s.refuse(key);
            return;
        }
        while !s.kept.is_empty() && (s.kept.len() >= MAX_STARTS || s.bytes() + bytes > MAX_BYTES) {
            s.kept.remove(0);
        }
        s.kept.push((key, Arc::new(start)));
    }

    /// Counts a session the gate kept from a start.
    pub(crate) fn not_taken(&self, why: NotTaken) {
        *self.state().not_taken.entry(why.reason()).or_default() += 1;
    }

    /// The bytes the kept starts hold (`LoginImage::kept_bytes`), for the memory pool's meter.
    pub(crate) fn bytes(&self) -> u64 {
        self.state().bytes()
    }

    /// How many sessions took a start.
    pub(crate) fn replayed(&self) -> u64 {
        self.state().replayed
    }

    /// How many starts are kept.
    pub(crate) fn kept(&self) -> usize {
        self.state().kept.len()
    }

    /// The report line, names and counts only: `None` before a start was recorded or refused, or a session gated.
    pub(crate) fn line(&self) -> Option<String> {
        let s = self.state();
        // `N what (n: reason, ...)`: the count, then the reasons when there are any.
        let by = |what: &str, m: &BTreeMap<String, u64>| {
            let n: u64 = m.values().sum();
            let why: Vec<String> = m.iter().map(|(why, n)| format!("{n}: {why}")).collect();
            match why.is_empty() {
                true => format!("{n} {what}"),
                false => format!("{n} {what} ({})", why.join(", ")),
            }
        };
        if s.kept.is_empty() && s.refused.is_empty() && s.replayed == 0 && s.not_taken.is_empty() {
            return None;
        }
        Some(format!(
            "login start: {} replayed, {} kept ({:.1} MB), {}, {} too large, {}",
            s.replayed,
            s.kept.len(),
            s.bytes() as f64 / 1_048_576.0,
            by("not imaged", &s.not_imaged),
            s.too_large,
            by("refused by the gate", &s.not_taken),
        ))
    }
}

/// The starts of `engine`.
pub(crate) fn starts(engine: &Engine) -> Arc<Starts> {
    engine.derived(Starts::default)
}

/// What the starts of `engine` did, and the check's report, for the run's phases.
pub(crate) fn phases(engine: &Engine) -> crate::runtime::LoginReplay {
    let starts = starts(engine);
    crate::runtime::LoginReplay {
        report: crate::login_replay::report(engine),
        replays: starts.replayed(),
        starts: starts.kept(),
    }
}

/// What `run_codeunit_inner` knows of a run that keeps every session of it from taking or recording a start
/// ([`run_gate`]).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RunFacts {
    /// The run notes the code it runs (`--explain` or an observer, coverage, the impact trace, the profiler).
    pub notes_code: bool,
    /// A stability perturbation or a set clock start.
    pub perturbed: bool,
    /// Single instances of the codeunit before were moved in.
    pub moved_in: bool,
    /// The system host reads the wall clock as it goes (`ALRUNNER_REAL_CLOCK`, `host::real_clock`).
    pub real_clock: bool,
    /// `File` reads and writes a real directory (`--files-dir`, `host::uses_real_files`).
    pub real_files: bool,
}

impl RunFacts {
    /// The process settings of a run, besides what the caller knows of it.
    pub(crate) fn with_host(self) -> RunFacts {
        RunFacts {
            real_clock: crate::host::real_clock(),
            real_files: crate::host::uses_real_files(),
            ..self
        }
    }
}

/// Why a run keeps every session from taking or recording a start ([`RunFacts`]), in this order.
pub(crate) fn run_gate(run: RunFacts) -> Option<NotTaken> {
    if run.notes_code {
        Some(NotTaken::NotesCode)
    } else if run.perturbed {
        Some(NotTaken::Perturbed)
    } else if run.moved_in {
        Some(NotTaken::MovedIn)
    } else if run.real_clock {
        Some(NotTaken::RealClock)
    } else if run.real_files {
        Some(NotTaken::RealFiles)
    } else {
        None
    }
}

/// Counts a session of `engine` that has no key (`no_key`): it runs its login.
pub(crate) fn no_key(engine: &Engine, no_key: crate::login_replay::NoKey) {
    starts(engine).not_taken(NotTaken::NoKey(no_key.0));
}

/// Why a session that was `target` (`Session::thaw_target_why`) with a store of shape `shape` before its login cannot take
/// `start`.
pub(crate) fn target_gate(target: Option<&'static str>, shape: &BaseShape, start: &Start) -> Option<NotTaken> {
    match target {
        Some(field) => Some(NotTaken::Target(field)),
        None => (shape != start.image.base_shape()).then_some(NotTaken::BaseShape),
    }
}

/// Why `session` does not take the start `start` of its key; `None`: it takes it. `run`: what `run_codeunit_inner` knows
/// of the run ([`run_gate`]). The gate as one answer, for tests: [`replay`] asks the same two in this order, the run first
/// (it counts a gated session whether its key has a start or not).
#[cfg(test)]
pub(crate) fn why_not(session: &Session, start: &Start, run: RunFacts) -> Option<NotTaken> {
    run_gate(run).or_else(|| target_gate(session.thaw_target_why(), &session.base_shape(), start))
}

/// The login of `session`, whose key is `key`, replayed from the key's start in place of running it: the open's writes and
/// the assumption marks of the start. `None`: the session runs its login (no start, or `gate` or the target keeps it from
/// one; counted).
pub(crate) fn replay(
    engine: &Engine,
    key: &Key,
    session: &mut Session,
    gate: Option<NotTaken>,
) -> Option<(CompanyOpenWrites, HitBits)> {
    let starts = starts(engine);
    if let Some(why) = gate {
        starts.not_taken(why);
        return None;
    }
    let start = starts.take(key)?;
    if let Some(why) = target_gate(session.thaw_target_why(), &session.base_shape(), &start) {
        starts.not_taken(why);
        return None;
    }
    // The gate asked what the thaw asks; a thaw that still refuses leaves the session as it was, and it runs its login.
    if let Err(why) = session.thaw_login_image(&start.image) {
        starts.not_taken(NotTaken::NotThawed(why));
        return None;
    }
    starts.state().replayed += 1;
    #[cfg(test)]
    if DROP_HITS.with(std::cell::Cell::get) {
        return Some((start.wrote.clone(), HitBits::EMPTY));
    }
    Some((start.wrote.clone(), start.hits))
}

/// After the real login of the first session of `key` (`base`: the session before it), its start, unless the key is known
/// or `gate` keeps the run from starts.
pub(crate) fn record(
    engine: &Engine,
    key: Key,
    session: &Session,
    base: &al_vm::LoginBase,
    wrote: &CompanyOpenWrites,
    hits: HitBits,
) {
    if starts(engine).knows(&key) {
        return;
    }
    let start = session.login_image(base).map(|image| Start {
        image,
        wrote: wrote.clone(),
        hits,
    });
    keep(engine, key, base, start);
}

/// Keeps `start` as the start of `key` ([`Starts::record`]) when the session it was taken from was a thaw target before its
/// login (`base`, `LoginBase::target_why`), as a later session must be to take it ([`target_gate`]); else the session is
/// counted as refused by the gate.
pub(crate) fn keep(engine: &Engine, key: Key, base: &al_vm::LoginBase, start: Result<Start, NotImaged>) {
    let starts = starts(engine);
    match base.target_why() {
        Some(field) => starts.not_taken(NotTaken::Target(field)),
        None => starts.record(key, start),
    }
}

#[cfg(test)]
thread_local! {
    /// While `Some`, `open_company` adds each test session's state after it opened the company, run or replayed
    /// ([`opened`]): a test compares a replayed session with a real one.
    pub(crate) static OPENED: std::cell::RefCell<Option<Vec<crate::login_replay::LoginState>>> =
        const { std::cell::RefCell::new(None) };
    /// While `true`, a start is taken without its assumption marks: a test shows the marks' test bites.
    pub(crate) static DROP_HITS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Notes `session`'s state after it opened the company in [`OPENED`], when a test asked for it.
#[cfg(test)]
pub(crate) fn opened(session: &mut Session, wrote: &CompanyOpenWrites, hits: HitBits) {
    if OPENED.with(|o| o.borrow().is_some()) {
        let state = crate::login_replay::LoginState::of(session, wrote, hits);
        OPENED.with(|o| o.borrow_mut().as_mut().map(|v| v.push(state)));
    }
}
