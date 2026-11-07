//! The runs ahead of one carry chain walk (`chain_walk.rs`), shared by the helper threads and the walk.
//!
//! Each member is run ahead from a predicted start: the start of its suite's current epoch (`Start`) with what the
//! members before it changed or are predicted to change applied (`Runs::guess`, `al_vm::Delta`). A member's prediction
//! is what its own last run ahead changed ([`Kept::Left`]), or, once the walk passed it, what it really changed. The
//! walk keeps a run only when the run's start has the fingerprint of the walk's actual state (`Runs::take`,
//! `Baseline::fingerprint`), so a wrong prediction costs time, never an outcome.
//!
//! The SingleInstance state a member starts from is predicted the same way (ADR-066 amendment 1): the walk's state of
//! its BC session with what the members between wrote in their last runs applied (`Runs::guess_singles`). The walk
//! takes a run only when every global it read first held the walk's value (`single_walk::fits`).

use super::single_walk::fits;
use super::{SingleState, UnitResult};
use std::sync::{Arc, Condvar, Mutex};

/// How often a member is run ahead at most per epoch: once from whatever start is predicted when a helper reaches it,
/// and once more when the predictions of the members before it changed since. Only the time of the run depends on it.
const TRIES: u8 = 2;

/// What a run ahead left, when the walk can hand it on without the helper's baseline (a baseline holds `Rc`s and stays
/// on its thread).
#[derive(Clone, Debug)]
pub(super) enum Kept {
    /// Nothing (`chain::left_something` is false): the later members start where it started.
    Nothing,
    /// Its start with `delta` applied (`Baseline::delta_from`), and the rest of what it carried (`run::Carried`).
    Left {
        delta: Box<al_vm::Delta>,
        tables: std::collections::BTreeSet<u32>,
        unfinished: Option<String>,
        everything: bool,
        storage_why: Option<String>,
        as_started: bool,
    },
}

impl Kept {
    /// What it changed.
    pub fn delta(&self) -> al_vm::Delta {
        match self {
            Kept::Nothing => Default::default(),
            Kept::Left { delta, .. } => (**delta).clone(),
        }
    }
}

/// What [`Runs::take`] dropped when it had no run ahead from the walk's state.
#[derive(Debug)]
pub(super) enum Dropped {
    /// Nothing: no helper ran the member in the epoch.
    Nothing,
    /// A run from a prediction made while some member before it had not run yet (counted as changing nothing).
    Early,
    /// A run from a prediction that knew what every member before it changes, but some of that from a member's own run
    /// ahead, which turned out wrong (that member's change depends on its start).
    Guessed,
    /// A run from a prediction that knew what every member before it changes, and was wrong: the delta it started
    /// from, on the epoch's start.
    Wrong(Box<al_vm::Delta>),
    /// A run from the walk's rows that read a SingleInstance global that held another value than in the walk's state.
    SingleState,
}

/// An event a run ahead told its hooks, kept until the walk takes the run.
pub(super) enum HookEvent {
    Start(u32, String),
    Test(Box<crate::events::TestEvent>),
}

/// A finished run ahead.
pub(super) struct Ahead {
    pub result: UnitResult,
    /// What it left, when the walk can hand it on; `None`: the walk runs the member itself.
    pub kept: Option<Kept>,
    /// The fingerprint of the start it ran from, the Test Runner's rows apart (`Baseline::fingerprint_apart`).
    pub start: Option<Arc<al_vm::Fingerprint>>,
    /// Nothing in its run saw what the Test Runner's tables held at its start (`suite_rows::take_unseen`): it may be
    /// taken from a start whose rows of those tables differ.
    pub unseen: bool,
    pub events: Vec<HookEvent>,
}

enum Slot {
    /// Not started; a helper may take it.
    Pending,
    /// A helper runs it from the start of this epoch of its suite with this delta applied, whose fingerprint is this.
    Running(u32, al_vm::Delta, Option<Arc<al_vm::Fingerprint>>),
    /// A helper ran it from there, or could not (`None`: the start could not be read back).
    Done(u32, al_vm::Delta, Option<Box<Ahead>>),
    /// The walk took it: no helper runs it.
    Taken,
}

/// Where the members of a suite that runner 130450 runs (leg 1) start in an epoch: what the codeunits with Disabled
/// isolation before them (leg 0) or the last member that left more than numbers left, or the install baseline.
#[derive(Clone)]
pub(super) enum Start {
    Install,
    /// A baseline of the walk's thread, as bytes (`Baseline::to_bytes`), with what the bytes leave out.
    Bytes(Arc<StartBytes>),
}

pub(super) struct StartBytes {
    pub bytes: Vec<u8>,
    pub install_rows: Option<std::collections::BTreeSet<u32>>,
    pub origin: u64,
    /// The tables the codeunits before left rows in (`Unit::leak`).
    pub leak: Arc<std::collections::BTreeSet<u32>>,
}

/// What a helper is told to do next.
pub(super) enum Claim {
    /// Run this member from this start, of this epoch of its suite, with this delta applied, and from this
    /// SingleInstance state.
    Run(usize, u32, Start, Box<al_vm::Delta>, Arc<SingleState>),
    Done,
}

struct RunsState {
    /// Per unit index.
    slots: Vec<Slot>,
    /// Per unit index: how often it was run ahead in its suite's current epoch.
    tries: Vec<u8>,
    /// Per unit index: its last run ahead started from a prediction that knew what every member before it changes.
    complete: Vec<bool>,
    /// Per unit index: that prediction was the walk's own state: no member between the walk and it (whose change is
    /// only known from its own run ahead, which may have started elsewhere).
    sure: Vec<bool>,
    /// Per unit index: what it changes, as far as known: what its last run ahead changed, or once the walk passed it,
    /// what it changed there.
    counts: Vec<Option<al_vm::Delta>>,
    /// Per suite, once the walk knows it, with its epoch: the walk tells a new start each time a member left more than
    /// numbers.
    starts: std::collections::HashMap<usize, (u32, Start)>,
    /// Per suite: where the walk is in its epoch: the members of `order` before this place are done, and the state is the
    /// epoch's start with this delta applied.
    at: std::collections::HashMap<usize, (usize, al_vm::Delta)>,
    /// The suites whose chain lost its known start: their members are not run ahead any more.
    tainted: std::collections::HashSet<usize>,
    /// Per BC session (suite, hub leg), once the walk passed a member there: the members of `order` before this place
    /// are done, and the walk's SingleInstance state.
    singles_at: std::collections::HashMap<(usize, bool), (usize, Arc<SingleState>)>,
    /// Per unit index: the SingleInstance globals it wrote in its last run ahead, or once the walk passed it, there.
    singles_wrote: Vec<Option<Arc<SingleState>>>,
    /// The members of `order` the walk has reached.
    walked: usize,
    /// The helpers waiting in `claim` for a member to run.
    idle: usize,
    /// The suites in which a member left nothing or only numbers since their last start: only their members are run
    /// ahead, so a suite whose members leave rows costs the helpers nothing.
    open: std::collections::HashSet<usize>,
    closed: bool,
}

/// The runs ahead of one walk.
pub(super) struct Runs {
    /// The chain members a helper may run, as unit indices in chain order.
    pub order: Vec<usize>,
    /// Per unit index: its place in `order`.
    place: Vec<Option<usize>>,
    /// The suite of each unit index.
    suite: Vec<usize>,
    /// The BC session (suite, hub leg) of each unit index: the SingleInstance state goes from member to member there.
    stream: Vec<(usize, bool)>,
    /// How far ahead of the walk the helpers run (members of `order`).
    window: usize,
    state: Mutex<RunsState>,
    ready: Condvar,
}

impl Runs {
    /// The runs of `order` (unit indices of `suite`, in chain order), with the suites that start from the install
    /// baseline.
    pub fn new(order: Vec<usize>, suite: Vec<usize>, install: Vec<usize>, window: usize) -> Runs {
        let n = suite.len();
        let mut place = vec![None; n];
        for (p, &k) in order.iter().enumerate() {
            place[k] = Some(p);
        }
        Runs {
            state: Mutex::new(RunsState {
                slots: (0..n).map(|_| Slot::Pending).collect(),
                tries: vec![0; n],
                complete: vec![false; n],
                sure: vec![false; n],
                counts: vec![None; n],
                starts: install.iter().map(|&si| (si, (0, Start::Install))).collect(),
                at: install.iter().map(|&si| (si, (0, Default::default()))).collect(),
                tainted: Default::default(),
                singles_at: Default::default(),
                singles_wrote: vec![None; n],
                walked: 0,
                idle: 0,
                open: Default::default(),
                closed: false,
            }),
            ready: Condvar::new(),
            place,
            stream: suite.iter().map(|&si| (si, false)).collect(),
            suite,
            window,
            order,
        }
    }

    /// The BC session (suite, hub leg) of each unit index (`Unit::hub_leg`); default: the suite's, no hub leg.
    pub fn with_streams(mut self, stream: Vec<(usize, bool)>) -> Runs {
        self.stream = stream;
        self
    }

    /// Where member `k` is predicted to start in its BC session's SingleInstance state: the walk's state there with what
    /// the members between wrote applied (an unknown one counts as writing nothing).
    fn guess_singles(&self, s: &RunsState, k: usize) -> Arc<SingleState> {
        let key = self.stream[k];
        let (from, at) = s.singles_at.get(&key).map_or((0, None), |(p, a)| (*p, Some(a.clone())));
        let to = self.place[k].unwrap_or(0);
        let between: Vec<&Arc<SingleState>> = self
            .order
            .get(from..to)
            .unwrap_or_default()
            .iter()
            .filter(|&&j| self.stream[j] == key)
            .filter_map(|&j| s.singles_wrote[j].as_ref())
            .collect();
        let at = at.unwrap_or_default();
        if between.is_empty() {
            return at;
        }
        let mut state = (*at).clone();
        for w in between {
            state.extend(w.iter().map(|(g, v)| (*g, v.clone())));
        }
        Arc::new(state)
    }

    /// Whether a member between the walk and member `k` runs on the walk's thread now (the walk took it, and what it
    /// changes is not known yet): no run ahead predicts it, so a run of `k` would count it as changing nothing and
    /// mostly start elsewhere; the helper waits for it instead (each such run would also leave the next member to the
    /// walk again, the walk and one helper taking turns without one run ahead kept).
    fn behind_walk(&self, s: &RunsState, k: usize) -> bool {
        let si = self.suite[k];
        let (Some((from, _)), Some(to)) = (s.at.get(&si), self.place[k]) else {
            return false;
        };
        self.order.get(*from..to).unwrap_or_default().iter().any(|&j| {
            self.suite[j] == si
                && matches!(s.slots[j], Slot::Taken)
                && (s.counts[j].is_none() || s.singles_wrote[j].is_none())
        })
    }

    /// Whether the run ahead `a` read the SingleInstance state `singles` gives it.
    fn fits_singles(a: Option<&Ahead>, singles: &SingleState) -> bool {
        a.is_none_or(|a| fits(&a.result, singles))
    }

    /// The epochs of the suites known at the start.
    pub fn epochs(&self) -> std::collections::HashMap<usize, u32> {
        self.state.lock().map_or_else(
            |_| Default::default(),
            |s| s.starts.iter().map(|(si, (e, _))| (*si, *e)).collect(),
        )
    }

    /// The place of unit `k` in `order`.
    pub fn place(&self, k: usize) -> Option<usize> {
        self.place[k]
    }

    /// Where member `k` is predicted to start: its epoch's start with the delta the walk is at and the deltas of the
    /// members between applied, and whether every one of those is known (`false`: the unknown ones count as none).
    fn guess(&self, s: &RunsState, k: usize) -> Option<(al_vm::Delta, bool)> {
        self.guess_sure(s, k).map(|(d, complete, _)| (d, complete))
    }

    /// [`Runs::guess`], and whether the prediction is the walk's own state (no member between).
    fn guess_sure(&self, s: &RunsState, k: usize) -> Option<(al_vm::Delta, bool, bool)> {
        let si = self.suite[k];
        let (from, base) = s.at.get(&si)?;
        let to = self.place[k]?;
        let mut delta = base.clone();
        let (mut complete, mut sure) = (true, true);
        for &j in self.order.get(*from..to).unwrap_or_default() {
            if self.suite[j] != si {
                continue;
            }
            sure = false;
            match &s.counts[j] {
                Some(c) => delta = delta.then(c),
                None => complete = false,
            }
        }
        Some((delta, complete, sure))
    }

    /// The next member a helper should run, marked as running, where it starts and the delta applied there.
    pub fn claim(&self) -> Claim {
        let Ok(mut s) = self.state.lock() else {
            return Claim::Done;
        };
        loop {
            if s.closed {
                return Claim::Done;
            }
            let rest = &self.order[s.walked.min(self.order.len())..];
            let next = rest.iter().take(self.window).copied().find_map(|k| {
                if !self.may_run(&s, k) {
                    return None;
                }
                let (epoch, start) = s.starts.get(&self.suite[k])?;
                let (delta, complete, sure) = self.guess_sure(&s, k)?;
                let singles = self.guess_singles(&s, k);
                let run = match &s.slots[k] {
                    Slot::Pending => true,
                    // Run again from a better prediction: every count before it is known now, and it differs (or the
                    // run read SingleInstance state the prediction now gives another value).
                    Slot::Done(e, from, a) => {
                        e == epoch && complete && (*from != delta || !Self::fits_singles(a.as_deref(), &singles))
                    }
                    _ => false,
                };
                run.then(|| (k, *epoch, start.clone(), delta, complete, sure, singles))
            });
            let waiting = rest
                .iter()
                .any(|k| !matches!(s.slots[*k], Slot::Taken) && !s.tainted.contains(&self.suite[*k]));
            match next {
                Some((k, epoch, start, delta, complete, sure, singles)) => {
                    s.slots[k] = Slot::Running(epoch, delta.clone(), None);
                    s.complete[k] = complete;
                    s.sure[k] = sure;
                    s.tries[k] += 1;
                    return Claim::Run(k, epoch, start, Box::new(delta), singles);
                }
                None if waiting => {
                    s.idle += 1;
                    let woke = self.ready.wait(s);
                    match woke {
                        Ok(g) => {
                            s = g;
                            s.idle -= 1;
                        }
                        Err(_) => return Claim::Done,
                    }
                }
                None => return Claim::Done,
            }
        }
    }

    /// Whether a helper may run member `k` now, as far as its suite and the walk go (`claim`).
    fn may_run(&self, s: &RunsState, k: usize) -> bool {
        let si = self.suite[k];
        !(s.tainted.contains(&si) || !s.open.contains(&si) || s.tries[k] >= TRIES || self.behind_walk(s, k))
            && s.starts.contains_key(&si)
    }

    /// Tells the helpers the new start of suite `si`, which holds for the members from place `from` of `order` on;
    /// returns its epoch. The runs of its members from an older start are run again.
    pub fn publish(&self, si: usize, start: Start, from: usize) -> u32 {
        let mut epoch = 0;
        if let Ok(mut s) = self.state.lock() {
            epoch = s.starts.get(&si).map_or(0, |(e, _)| e + 1);
            s.starts.insert(si, (epoch, start));
            s.at.insert(si, (from, Default::default()));
            s.tainted.remove(&si);
            s.open.remove(&si);
            for &k in &self.order {
                if self.suite[k] == si {
                    s.tries[k] = 0;
                    if matches!(s.slots[k], Slot::Done(..)) {
                        s.slots[k] = Slot::Pending;
                    }
                }
            }
        }
        self.ready.notify_all();
        epoch
    }

    /// A helper starts member `k` from the start whose fingerprint is `fingerprint` (the walk waits for such a run only
    /// when it is its own state's).
    pub fn starting(&self, k: usize, fingerprint: Option<Arc<al_vm::Fingerprint>>) {
        if let Ok(mut s) = self.state.lock() {
            if let Slot::Running(_, _, fp) = &mut s.slots[k] {
                *fp = fingerprint;
            }
        }
    }

    /// The run of member `k` from the start of `epoch` with `from` applied ended. A run from an older start is dropped
    /// and the member is run again; one the walk no longer waits for is dropped. What it took is its count.
    pub fn finish(&self, k: usize, epoch: u32, from: al_vm::Delta, ahead: Option<Ahead>) {
        if let Ok(mut s) = self.state.lock() {
            let current = s.starts.get(&self.suite[k]).is_some_and(|(e, _)| *e == epoch);
            let taken = matches!(s.slots[k], Slot::Taken);
            if current && !taken {
                if let Some(kept) = ahead.as_ref().and_then(|a| a.kept.as_ref()) {
                    s.counts[k] = Some(kept.delta());
                }
                if let Some(Ok(done)) = ahead.as_ref().map(|a| &a.result.result) {
                    s.singles_wrote[k] = done.5.singles_out.as_ref().map(|out| Arc::new(out.wrote.clone()));
                }
            }
            s.slots[k] = match () {
                _ if taken => Slot::Taken,
                _ if current => Slot::Done(epoch, from, ahead.map(Box::new)),
                _ => Slot::Pending,
            };
        }
        self.ready.notify_all();
    }

    /// Whether a helper runs or ran member `k` in `epoch` (the walk then needs its state's fingerprint).
    pub fn has_run(&self, k: usize, epoch: Option<u32>) -> bool {
        self.state.lock().is_ok_and(|s| match &s.slots[k] {
            Slot::Running(e, ..) | Slot::Done(e, ..) => Some(*e) == epoch,
            _ => false,
        })
    }

    /// The run ahead of member `k` from the start of `epoch` whose fingerprint is `actual`, the walk's state, waiting
    /// for it while a helper runs it; `None` when there is none (or `epoch` or `actual` is `None`: the walk runs it
    /// itself). The second part says what was dropped instead ([`Dropped`]). No helper runs the member after this.
    /// `singles` is the walk's SingleInstance state of the member's BC session: a run that read another is dropped too.
    pub fn take(
        &self,
        k: usize,
        epoch: Option<u32>,
        actual: Option<&al_vm::Fingerprint>,
        singles: &SingleState,
    ) -> (Option<Ahead>, Dropped) {
        let Ok(mut s) = self.state.lock() else {
            return (None, Dropped::Nothing);
        };
        // The same start but for the Test Runner's rows, and whether those are the same too.
        let same = |e: &u32, fp: Option<&al_vm::Fingerprint>| match (fp, actual) {
            (Some(fp), Some(actual)) if Some(*e) == epoch && fp.main == actual.main => Some(fp.rows == actual.rows),
            _ => None,
        };
        loop {
            match std::mem::replace(&mut s.slots[k], Slot::Taken) {
                Slot::Done(e, _, Some(a)) if same(&e, a.start.as_deref()).is_some_and(|rows| rows || a.unseen) => {
                    if fits(&a.result, singles) {
                        return (Some(*a), Dropped::Nothing);
                    }
                    return (None, Dropped::SingleState);
                }
                // Its fingerprint is not known yet, or it is the walk's (but perhaps for those rows): wait.
                Slot::Running(e, d, fp) if Some(e) == epoch && (fp.is_none() || same(&e, fp.as_deref()).is_some()) => {
                    s.slots[k] = Slot::Running(e, d, fp);
                    s = match self.ready.wait(s) {
                        Ok(g) => g,
                        Err(_) => return (None, Dropped::Nothing),
                    };
                }
                Slot::Done(_, from, _) | Slot::Running(_, from, _) if s.complete[k] && s.sure[k] => {
                    return (None, Dropped::Wrong(Box::new(from)));
                }
                Slot::Done(..) | Slot::Running(..) if s.complete[k] => return (None, Dropped::Guessed),
                // Not run yet, but an idle helper may run it now (the walk just ran the member before it itself): wait
                // for the helper, which runs it from the walk's own state, so the walk and the helper do not take
                // turns, the helper waiting for each member the walk runs.
                Slot::Pending
                    if s.idle > 0
                        && self.may_run(&s, k)
                        && s.starts.get(&self.suite[k]).map(|(e, _)| *e) == epoch
                        && self.place[k].is_some_and(|p| p < s.walked + self.window) =>
                {
                    s.slots[k] = Slot::Pending;
                    self.ready.notify_all();
                    s = match self.ready.wait(s) {
                        Ok(g) => g,
                        Err(_) => return (None, Dropped::Nothing),
                    };
                }
                Slot::Done(..) | Slot::Running(..) => {
                    return (None, Dropped::Early);
                }
                _ => return (None, Dropped::Nothing),
            }
        }
    }

    /// The walk reached member `k`: the helpers may run up to `window` members past it.
    pub fn reached(&self, k: usize) {
        if let (Some(p), Ok(mut s)) = (self.place[k], self.state.lock()) {
            s.walked = s.walked.max(p + 1);
        }
        self.ready.notify_all();
    }

    /// The walk passed unit `k` (a member of `order` or not): its BC session's SingleInstance state is `state` now, and
    /// `k` wrote `wrote`. A later member whose run ahead read a global the prediction now gives another value gets one
    /// more run, as in `advance`.
    pub fn singles_walked(&self, k: usize, state: Arc<SingleState>, wrote: Option<Arc<SingleState>>) {
        let key = self.stream[k];
        if let Ok(mut s) = self.state.lock() {
            let from = match self.place[k] {
                Some(p) => p + 1,
                None => s.singles_at.get(&key).map_or(0, |(p, _)| *p),
            };
            s.singles_at.insert(key, (from, state));
            s.singles_wrote[k] = wrote;
            let stale: Vec<usize> = self.order[from.min(self.order.len())..]
                .iter()
                .copied()
                .filter(|&j| self.stream[j] == key)
                .filter(|&j| match &s.slots[j] {
                    Slot::Done(_, _, a) => !Self::fits_singles(a.as_deref(), &self.guess_singles(&s, j)),
                    _ => false,
                })
                .collect();
            for j in stale {
                s.tries[j] = s.tries[j].min(TRIES - 1);
            }
        }
        self.ready.notify_all();
    }

    /// Member `k` of suite `si` changed `took` (`None`: not known): the state after it is the epoch's start with `at`
    /// applied, and the helpers may run the suite's later members.
    pub fn advance(&self, si: usize, k: usize, at: al_vm::Delta, took: Option<al_vm::Delta>) {
        if let (Some(p), Ok(mut s)) = (self.place[k], self.state.lock()) {
            s.at.insert(si, (p + 1, at));
            s.counts[k] = took;
            s.open.insert(si);
            // The prediction of the next member is the walk's state now, and the later ones rest on fewer guesses: a
            // later member that ran from another prediction gets one more run, whatever its tries, so one member whose
            // change depends on its start does not leave all the members after it to the walk. Each pass of the walk
            // gives at most one more run per member, and only within the window (`claim`).
            let epoch = s.starts.get(&si).map(|(e, _)| *e);
            let stale: Vec<usize> = self.order[(p + 1).min(self.order.len())..]
                .iter()
                .copied()
                .filter(|&j| self.suite[j] == si)
                .filter(|&j| match &s.slots[j] {
                    Slot::Done(e, from, _) => {
                        Some(*e) == epoch && self.guess(&s, j).is_some_and(|(d, complete)| complete && d != *from)
                    }
                    _ => false,
                })
                .collect();
            for j in stale {
                s.tries[j] = s.tries[j].min(TRIES - 1);
            }
        }
        self.ready.notify_all();
    }

    /// A member of suite `si` left nothing while its suite has no known start: the helpers may run its later members
    /// once it has one.
    pub fn open(&self, si: usize) {
        if let Ok(mut s) = self.state.lock() {
            s.open.insert(si);
        }
        self.ready.notify_all();
    }

    /// Whether the helpers run suite `si`'s members (`open`; a new start closes it until one member ran from there).
    pub fn is_open(&self, si: usize) -> bool {
        self.state.lock().is_ok_and(|s| s.open.contains(&si))
    }

    /// Whether suite `si`'s members are not run ahead any more (`taint`).
    pub fn tainted(&self, si: usize) -> bool {
        self.state.lock().is_ok_and(|s| s.tainted.contains(&si))
    }

    pub fn taint(&self, si: usize) {
        if let Ok(mut s) = self.state.lock() {
            s.tainted.insert(si);
        }
        self.ready.notify_all();
    }

    pub fn close(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.closed = true;
        }
        self.ready.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ahead(start: &[u8]) -> Ahead {
        Ahead {
            result: UnitResult {
                si: 0,
                oi: 0,
                id: 0,
                result: Err(String::new()),
                took: std::time::Duration::ZERO,
                sidecar_start: std::time::Duration::ZERO,
            },
            kept: Some(Kept::Nothing),
            start: Some(Arc::new(fp(start, b""))),
            unseen: false,
            events: Vec::new(),
        }
    }

    fn fp(main: &[u8], rows: &[u8]) -> al_vm::Fingerprint {
        al_vm::Fingerprint {
            main: main.to_vec(),
            rows: rows.to_vec(),
        }
    }

    /// The walk takes a run only from the start whose fingerprint is its own state's, and drops any other.
    #[test]
    fn a_run_ahead_is_taken_only_from_the_walks_own_state() {
        let runs = Runs::new(vec![0, 1, 2], vec![0, 0, 0], vec![0], 6);
        assert!(
            runs.take(0, Some(0), Some(&fp(b"s0", b"")), &SingleState::new())
                .0
                .is_none(),
            "nothing ran yet"
        );
        runs.reached(0);
        runs.advance(0, 0, Default::default(), Some(Default::default()));
        let Claim::Run(1, 0, _, from, _) = runs.claim() else {
            panic!("member 1 first")
        };
        runs.starting(1, Some(Arc::new(fp(b"s1", b""))));
        assert!(runs.has_run(1, Some(0)));
        runs.finish(1, 0, *from, Some(ahead(b"s1")));
        let Claim::Run(2, 0, _, from, _) = runs.claim() else {
            panic!("then member 2")
        };
        runs.finish(2, 0, *from, Some(ahead(b"s2")));
        assert!(
            runs.take(1, Some(0), Some(&fp(b"s1", b"")), &SingleState::new())
                .0
                .is_some(),
            "member 1 started where the walk is"
        );
        runs.reached(1);
        let (wrong, dropped) = runs.take(2, Some(0), Some(&fp(b"other", b"")), &SingleState::new());
        assert!(
            wrong.is_none() && matches!(dropped, Dropped::Wrong(_) | Dropped::Guessed),
            "a run from another start is not taken"
        );
        assert!(
            runs.take(2, Some(0), Some(&fp(b"s2", b"")), &SingleState::new())
                .0
                .is_none(),
            "and it is gone"
        );
    }

    fn changed(n: u64) -> Option<Ahead> {
        let mut a = ahead(b"");
        a.kept = Some(Kept::Left {
            delta: Box::new(al_vm::Delta::of_guids(n)),
            tables: Default::default(),
            unfinished: None,
            everything: false,
            storage_why: None,
            as_started: false,
        });
        Some(a)
    }

    /// The next claim within a second, or `None` when the helpers would wait (the runs are then closed).
    fn claim_soon(runs: &Runs) -> Option<(usize, al_vm::Delta)> {
        std::thread::scope(|scope| {
            let (tx, rx) = std::sync::mpsc::channel();
            scope.spawn(move || {
                let _ = tx.send(match runs.claim() {
                    Claim::Run(k, _, _, from, _) => Some((k, *from)),
                    Claim::Done => None,
                });
            });
            let got = rx.recv_timeout(std::time::Duration::from_secs(1)).ok().flatten();
            runs.close();
            got
        })
    }

    /// Each time the walk passes a member, the prediction of a later member that ran from another start is exact up to
    /// there: it is run again from the new prediction even when it already ran `TRIES` times, so one start-dependent
    /// member does not leave the members after it to the walk.
    #[test]
    fn a_member_is_run_again_when_the_walk_passing_an_earlier_one_changes_its_prediction() {
        let g = al_vm::Delta::of_guids;
        let runs = Runs::new(vec![0, 1, 2], vec![0, 0, 0], vec![0], 6);
        runs.open(0);
        for (k, from) in [(0, g(0)), (1, g(1)), (2, g(2))] {
            let Claim::Run(got, 0, _, d, _) = runs.claim() else {
                panic!("member {k}")
            };
            assert_eq!((got, *d), (k, from.clone()));
            runs.finish(k, 0, from, changed(1));
        }
        // Member 0 changed 5: members 1 and 2 run again (their second run).
        runs.reached(0);
        runs.advance(0, 0, g(5), Some(g(5)));
        for (k, from) in [(1, g(5)), (2, g(6))] {
            let Claim::Run(got, 0, _, d, _) = runs.claim() else {
                panic!("member {k} again")
            };
            assert_eq!((got, *d), (k, from.clone()));
            runs.finish(k, 0, from, changed(1));
        }
        // Member 1 changed 3, not 1: member 2's prediction is the walk's state now, so it runs a third time.
        runs.reached(1);
        runs.advance(0, 1, g(8), Some(g(3)));
        assert_eq!(claim_soon(&runs), Some((2, g(8))));
    }

    /// A prediction that rests on what an earlier member changed in its own run ahead is not counted as a wrong
    /// prediction when that run ahead turns out wrong (the earlier member's change depends on its start): only a
    /// prediction made from what the walk saw is (`Dropped::Wrong`, which `MISSES` counts). One start-dependent member
    /// must not use up the misses of every member after it.
    #[test]
    fn a_prediction_resting_on_another_run_ahead_is_not_a_wrong_prediction() {
        let g = al_vm::Delta::of_guids;
        let runs = Runs::new(vec![0, 1, 2], vec![0, 0, 0], vec![0], 6);
        runs.reached(0);
        runs.advance(0, 0, g(0), Some(g(0)));
        // Member 1 runs from the walk's state and changes 1; member 2 then runs from that prediction (complete).
        let Claim::Run(1, 0, _, from, _) = runs.claim() else {
            panic!("member 1 first")
        };
        runs.finish(1, 0, *from, changed(1));
        let Claim::Run(2, 0, _, from, _) = runs.claim() else {
            panic!("then member 2")
        };
        assert_eq!(*from, g(1));
        let mut a = ahead(b"s2");
        a.start = Some(Arc::new(fp(b"after 1", b"")));
        runs.finish(2, 0, *from, Some(a));
        // On the walk member 1 changed 3: member 2's run ahead started elsewhere, but its prediction rested on member
        // 1's run ahead.
        runs.reached(1);
        runs.advance(0, 1, g(3), Some(g(3)));
        let (got, dropped) = runs.take(2, Some(0), Some(&fp(b"after 3", b"")), &SingleState::new());
        assert!(got.is_none());
        assert!(matches!(dropped, Dropped::Guessed), "{dropped:?}");
    }

    /// Another start only in the Test Runner's rows: taken when the run did not see them, else dropped.
    #[test]
    fn a_run_that_did_not_see_the_runner_rows_is_taken_from_a_start_that_differs_only_there() {
        let runs = Runs::new(vec![0, 1, 2], vec![0, 0, 0], vec![0], 6);
        runs.reached(0);
        runs.advance(0, 0, Default::default(), Some(Default::default()));
        let Claim::Run(1, 0, _, from, _) = runs.claim() else {
            panic!("member 1 first")
        };
        let mut a = ahead(b"s1");
        a.start = Some(Arc::new(fp(b"s1", b"rows of 1")));
        a.unseen = true;
        runs.finish(1, 0, *from, Some(a));
        let Claim::Run(2, 0, _, from, _) = runs.claim() else {
            panic!("then member 2")
        };
        let mut a = ahead(b"s2");
        a.start = Some(Arc::new(fp(b"s2", b"rows of 2")));
        runs.finish(2, 0, *from, Some(a));
        assert!(
            runs.take(1, Some(0), Some(&fp(b"s1", b"other rows")), &SingleState::new())
                .0
                .is_some()
        );
        runs.reached(1);
        let (seen, dropped) = runs.take(2, Some(0), Some(&fp(b"s2", b"other rows")), &SingleState::new());
        assert!(
            seen.is_none() && matches!(dropped, Dropped::Wrong(_) | Dropped::Guessed),
            "a run that saw the rows needs the same rows"
        );
    }
}
