//! The runs ahead of one carry chain walk (`chain_walk.rs`), shared by the helper threads and the walk.
//!
//! Each member is run ahead from a predicted start: the start of its suite's current epoch (`Start`) with what the
//! members before it changed or are predicted to change applied (`Runs::guess`, `al_vm::Delta`). A member's prediction
//! is what its own last run ahead changed ([`Kept::Left`]), or, once the walk passed it, what it really changed. The
//! walk keeps a run only when the run's start has the fingerprint of the walk's actual state (`Runs::take`,
//! `Baseline::fingerprint`), so a wrong prediction costs time, never an outcome.

use super::UnitResult;
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
    /// Run this member from this start, of this epoch of its suite, with this delta applied.
    Run(usize, u32, Start, Box<al_vm::Delta>),
    Done,
}

struct RunsState {
    /// Per unit index.
    slots: Vec<Slot>,
    /// Per unit index: how often it was run ahead in its suite's current epoch.
    tries: Vec<u8>,
    /// Per unit index: its last run ahead started from a prediction that knew what every member before it changes.
    complete: Vec<bool>,
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
    /// The members of `order` the walk has reached.
    walked: usize,
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
                counts: vec![None; n],
                starts: install.iter().map(|&si| (si, (0, Start::Install))).collect(),
                at: install.iter().map(|&si| (si, (0, Default::default()))).collect(),
                tainted: Default::default(),
                walked: 0,
                open: Default::default(),
                closed: false,
            }),
            ready: Condvar::new(),
            place,
            suite,
            window,
            order,
        }
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
        let si = self.suite[k];
        let (from, base) = s.at.get(&si)?;
        let to = self.place[k]?;
        let mut delta = base.clone();
        let mut complete = true;
        for &j in self.order.get(*from..to).unwrap_or_default() {
            if self.suite[j] != si {
                continue;
            }
            match &s.counts[j] {
                Some(c) => delta = delta.then(c),
                None => complete = false,
            }
        }
        Some((delta, complete))
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
                let si = self.suite[k];
                if s.tainted.contains(&si) || !s.open.contains(&si) || s.tries[k] >= TRIES {
                    return None;
                }
                let (epoch, start) = s.starts.get(&si)?;
                let (delta, complete) = self.guess(&s, k)?;
                let run = match &s.slots[k] {
                    Slot::Pending => true,
                    // Run again from a better prediction: every count before it is known now, and it differs.
                    Slot::Done(e, from, _) => e == epoch && complete && *from != delta,
                    _ => false,
                };
                run.then(|| (k, *epoch, start.clone(), delta, complete))
            });
            let waiting = rest
                .iter()
                .any(|k| !matches!(s.slots[*k], Slot::Taken) && !s.tainted.contains(&self.suite[*k]));
            match next {
                Some((k, epoch, start, delta, complete)) => {
                    s.slots[k] = Slot::Running(epoch, delta.clone(), None);
                    s.complete[k] = complete;
                    s.tries[k] += 1;
                    return Claim::Run(k, epoch, start, Box::new(delta));
                }
                None if waiting => match self.ready.wait(s) {
                    Ok(g) => s = g,
                    Err(_) => return Claim::Done,
                },
                None => return Claim::Done,
            }
        }
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
    /// itself). The flag tells whether a run from another start was dropped that started from a prediction which knew
    /// what every member before it changes (a wrong prediction, not one made before that was known). No helper runs the
    /// member after this.
    pub fn take(&self, k: usize, epoch: Option<u32>, actual: Option<&al_vm::Fingerprint>) -> (Option<Ahead>, bool) {
        let Ok(mut s) = self.state.lock() else {
            return (None, false);
        };
        // The same start but for the Test Runner's rows, and whether those are the same too.
        let same = |e: &u32, fp: Option<&al_vm::Fingerprint>| match (fp, actual) {
            (Some(fp), Some(actual)) if Some(*e) == epoch && fp.main == actual.main => Some(fp.rows == actual.rows),
            _ => None,
        };
        loop {
            match std::mem::replace(&mut s.slots[k], Slot::Taken) {
                Slot::Done(e, _, Some(a)) if same(&e, a.start.as_deref()).is_some_and(|rows| rows || a.unseen) => {
                    return (Some(*a), false);
                }
                // Its fingerprint is not known yet, or it is the walk's (but perhaps for those rows): wait.
                Slot::Running(e, d, fp) if Some(e) == epoch && (fp.is_none() || same(&e, fp.as_deref()).is_some()) => {
                    s.slots[k] = Slot::Running(e, d, fp);
                    s = match self.ready.wait(s) {
                        Ok(g) => g,
                        Err(_) => return (None, false),
                    };
                }
                Slot::Done(..) | Slot::Running(..) => return (None, s.complete[k]),
                _ => return (None, false),
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
            runs.take(0, Some(0), Some(&fp(b"s0", b""))).0.is_none(),
            "nothing ran yet"
        );
        runs.reached(0);
        runs.advance(0, 0, Default::default(), Some(Default::default()));
        let Claim::Run(1, 0, _, from) = runs.claim() else {
            panic!("member 1 first")
        };
        runs.starting(1, Some(Arc::new(fp(b"s1", b""))));
        assert!(runs.has_run(1, Some(0)));
        runs.finish(1, 0, *from, Some(ahead(b"s1")));
        let Claim::Run(2, 0, _, from) = runs.claim() else {
            panic!("then member 2")
        };
        runs.finish(2, 0, *from, Some(ahead(b"s2")));
        assert!(
            runs.take(1, Some(0), Some(&fp(b"s1", b""))).0.is_some(),
            "member 1 started where the walk is"
        );
        runs.reached(1);
        let (wrong, dropped) = runs.take(2, Some(0), Some(&fp(b"other", b"")));
        assert!(wrong.is_none() && dropped, "a run from another start is not taken");
        assert!(
            runs.take(2, Some(0), Some(&fp(b"s2", b""))).0.is_none(),
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
                    Claim::Run(k, _, _, from) => Some((k, *from)),
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
            let Claim::Run(got, 0, _, d) = runs.claim() else { panic!("member {k}") };
            assert_eq!((got, *d), (k, from.clone()));
            runs.finish(k, 0, from, changed(1));
        }
        // Member 0 changed 5: members 1 and 2 run again (their second run).
        runs.reached(0);
        runs.advance(0, 0, g(5), Some(g(5)));
        for (k, from) in [(1, g(5)), (2, g(6))] {
            let Claim::Run(got, 0, _, d) = runs.claim() else { panic!("member {k} again") };
            assert_eq!((got, *d), (k, from.clone()));
            runs.finish(k, 0, from, changed(1));
        }
        // Member 1 changed 3, not 1: member 2's prediction is the walk's state now, so it runs a third time.
        runs.reached(1);
        runs.advance(0, 1, g(8), Some(g(3)));
        assert_eq!(claim_soon(&runs), Some((2, g(8))));
    }

    /// Another start only in the Test Runner's rows: taken when the run did not see them, else dropped.
    #[test]
    fn a_run_that_did_not_see_the_runner_rows_is_taken_from_a_start_that_differs_only_there() {
        let runs = Runs::new(vec![0, 1, 2], vec![0, 0, 0], vec![0], 6);
        runs.reached(0);
        runs.advance(0, 0, Default::default(), Some(Default::default()));
        let Claim::Run(1, 0, _, from) = runs.claim() else {
            panic!("member 1 first")
        };
        let mut a = ahead(b"s1");
        a.start = Some(Arc::new(fp(b"s1", b"rows of 1")));
        a.unseen = true;
        runs.finish(1, 0, *from, Some(a));
        let Claim::Run(2, 0, _, from) = runs.claim() else {
            panic!("then member 2")
        };
        let mut a = ahead(b"s2");
        a.start = Some(Arc::new(fp(b"s2", b"rows of 2")));
        runs.finish(2, 0, *from, Some(a));
        assert!(runs.take(1, Some(0), Some(&fp(b"s1", b"other rows"))).0.is_some());
        runs.reached(1);
        let (seen, dropped) = runs.take(2, Some(0), Some(&fp(b"s2", b"other rows")));
        assert!(seen.is_none() && dropped, "a run that saw the rows needs the same rows");
    }
}
