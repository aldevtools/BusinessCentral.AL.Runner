//! The walk of the carry chain (`corpus.rs`), and the runs ahead that let it use more than one thread (al-runner
//! issue 6924).
//!
//! The chain runs its codeunits in order, each from what the ones before it in its suite left (`Carried`). Most of
//! them leave nothing: a codeunit without an OnRun trigger whose tests took no number from a sequence of the state it
//! started from, or one whose OnRun left exactly that state (`Carried::as_started`), leaves the state it was given
//! (`numbers_taken.rs`). So while every codeunit before it in its suite left nothing, a codeunit starts from the
//! state its suite's chain started from, the install baseline.
//!
//! Helper threads run the chain's codeunits ahead of the walk, in chain order, each from its own copy of the install
//! baseline, as the codeunits outside the chain run. The walk takes such a run only for a codeunit whose suite is
//! still pristine (every codeunit before it in the chain of its suite left nothing) and that itself left nothing: its
//! start was then the very state the walk would have given it, and what it hands on, nothing, needs no baseline from
//! the helper's thread (a baseline holds `Rc`s and stays on its thread). Any other codeunit runs on the walk's thread
//! as before, and a run ahead that is not taken is dropped. The events of a run ahead (`Hooks::on_start`,
//! `Hooks::on_test`) are kept and told when the walk takes it, so a test is told once, in chain order.

use super::corpus::{not_run_link, observed_detail, reaches_session_storage, refuse_all};
use super::*;
use std::sync::{Condvar, Mutex};

/// How many new starts the walk tells the helpers for a suite beyond the runs ahead it took there: each member that
/// left something makes the runs ahead of the later ones useless and costs an encoding of its baseline on the walk's
/// thread, so a suite whose members often leave something is not run ahead any more. Only the time of the run depends
/// on it.
const RESTARTS: usize = 2;

/// How many members per helper thread the helpers run ahead of the walk at most: a new start makes their runs useless,
/// and runs far ahead take the machine from the walk. Only the time of the run depends on it.
const WINDOW_PER_HELPER: usize = 2;

/// A member of the chain: a codeunit to run (an index into the units), or one with RequiredTestIsolation = Disabled
/// that is not run (object, why).
pub(super) enum Link {
    Run(usize),
    Refused(usize, String),
}

/// What a chain member left, with what the ones before it in its suite left: (suite, id, leg, carried).
pub(super) type Carry = (usize, u32, u8, super::run::Carried);

/// The run of one chain walk.
pub(super) struct ChainWalk<'a> {
    pub engine: &'a Arc<al_vm::Engine>,
    pub lowered: &'a Lowered,
    /// The install baseline the chain of each suite starts from.
    pub baseline: Option<&'a al_vm::Baseline>,
    pub units: &'a [Unit],
    /// `RunOptions::trace` (`--explain`): note the detail of each test.
    pub trace: bool,
    pub limit: Option<std::time::Duration>,
    /// How many helper threads may run members ahead (0: none).
    pub helpers: usize,
    /// A helper thread's own copy of the install baseline.
    pub thread_baseline: &'a (dyn Fn() -> Option<al_vm::Baseline> + Sync),
}

/// An event a run ahead told its hooks, kept until the walk takes the run.
enum HookEvent {
    Start(u32, String),
    Test(Box<crate::events::TestEvent>),
}

/// A finished run ahead.
struct Ahead {
    result: UnitResult,
    /// It left nothing for the later codeunits (`chain::left_something`).
    nothing: bool,
    events: Vec<HookEvent>,
}

enum Slot {
    /// Not started; a helper may take it.
    Pending,
    /// A helper runs it from the start of this epoch of its suite.
    Running(u32),
    /// A helper ran it from the start of this epoch, or could not (`None`: the start could not be read back).
    Done(u32, Option<Box<Ahead>>),
    /// The walk took it: no helper runs it.
    Taken,
}

/// The runs ahead of one walk, shared by the helper threads and the walk.
struct Runs {
    /// The chain members a helper may run, as unit indices in chain order.
    order: Vec<usize>,
    /// Per unit index: its place in `order`.
    place: Vec<Option<usize>>,
    /// How far ahead of the walk the helpers run (members of `order`).
    window: usize,
    state: Mutex<RunsState>,
    ready: Condvar,
}

/// Where the members of a suite that runner 130450 runs (leg 1) start while the suite is pristine: what the codeunits
/// with Disabled isolation before them (leg 0) left, or the install baseline when there are none.
#[derive(Clone)]
enum Start {
    Install,
    /// A baseline of the walk's thread, as bytes (`Baseline::to_bytes`), with what the bytes leave out.
    Bytes(Arc<StartBytes>),
}

struct StartBytes {
    bytes: Vec<u8>,
    install_rows: Option<std::collections::BTreeSet<u32>>,
    origin: u64,
    /// The tables the codeunits before left rows in (`Unit::leak`).
    leak: Arc<std::collections::BTreeSet<u32>>,
}

/// What a helper is told to do next.
enum Claim {
    /// Run this member from this start, of this epoch of its suite.
    Run(usize, u32, Start),
    Done,
}

struct RunsState {
    /// Per unit index.
    slots: Vec<Slot>,
    /// Per suite, once the walk knows it, with its epoch: the walk tells a new start each time a member left something.
    starts: std::collections::HashMap<usize, (u32, Start)>,
    /// The suites whose chain lost its pristine start: their members are not run ahead any more.
    tainted: std::collections::HashSet<usize>,
    /// The members of `order` the walk has reached.
    walked: usize,
    /// The suites in which a member left nothing since their last start: only their members are run ahead, so a suite
    /// whose members all hand on numbers (Microsoft's legacy buckets) costs the helpers nothing.
    open: std::collections::HashSet<usize>,
    closed: bool,
}

impl Runs {
    /// The next member a helper should run, marked as running, and where it starts.
    fn claim(&self, units: &[Unit]) -> Claim {
        let Ok(mut s) = self.state.lock() else {
            return Claim::Done;
        };
        loop {
            if s.closed {
                return Claim::Done;
            }
            let open = |k: &usize| {
                matches!(s.slots[*k], Slot::Pending)
                    && !s.tainted.contains(&units[*k].si)
                    && s.open.contains(&units[*k].si)
            };
            let next = self.order[s.walked.min(self.order.len())..]
                .iter()
                .take(self.window)
                .copied()
                .filter(open)
                .find_map(|k| Some((k, s.starts.get(&units[k].si)?.clone())));
            let waiting = self.order[s.walked.min(self.order.len())..]
                .iter()
                .any(|k| matches!(s.slots[*k], Slot::Pending) && !s.tainted.contains(&units[*k].si));
            match next {
                Some((k, (epoch, start))) => {
                    s.slots[k] = Slot::Running(epoch);
                    return Claim::Run(k, epoch, start);
                }
                None if waiting => match self.ready.wait(s) {
                    Ok(g) => s = g,
                    Err(_) => return Claim::Done,
                },
                None => return Claim::Done,
            }
        }
    }

    /// Tells the helpers the new start of suite `si` (members of other suites are in `units` too); returns its epoch.
    /// The runs of its members from an older start are run again.
    fn publish(&self, si: usize, start: Start, units: &[Unit]) -> u32 {
        let mut epoch = 0;
        if let Ok(mut s) = self.state.lock() {
            epoch = s.starts.get(&si).map_or(0, |(e, _)| e + 1);
            s.starts.insert(si, (epoch, start));
            s.tainted.remove(&si);
            s.open.remove(&si);
            for &k in &self.order {
                if units[k].si == si && matches!(s.slots[k], Slot::Done(..)) {
                    s.slots[k] = Slot::Pending;
                }
            }
        }
        self.ready.notify_all();
        epoch
    }

    /// The run of member `k` from the start of `epoch` ended. A run from an older start is dropped and the member is
    /// run again; one the walk no longer waits for is dropped.
    fn finish(&self, k: usize, si: usize, epoch: u32, ahead: Option<Ahead>) {
        if let Ok(mut s) = self.state.lock() {
            let current = s.starts.get(&si).is_some_and(|(e, _)| *e == epoch);
            s.slots[k] = match s.slots[k] {
                Slot::Taken => Slot::Taken,
                _ if current => Slot::Done(epoch, ahead.map(Box::new)),
                _ => Slot::Pending,
            };
        }
        self.ready.notify_all();
    }

    /// The run ahead of member `k` from the start of `epoch`, waiting for it while a helper runs it; `None` when there
    /// is none (or `epoch` is `None`: the walk runs it itself). No helper runs the member after this.
    fn take(&self, k: usize, epoch: Option<u32>) -> Option<Ahead> {
        let mut s = self.state.lock().ok()?;
        loop {
            match std::mem::replace(&mut s.slots[k], Slot::Taken) {
                Slot::Done(e, a) if Some(e) == epoch => return a.map(|a| *a),
                Slot::Running(e) if Some(e) == epoch => {
                    s.slots[k] = Slot::Running(e);
                    s = self.ready.wait(s).ok()?;
                }
                _ => return None,
            }
        }
    }

    /// The walk reached member `k`: the helpers may run up to `window` members past it.
    fn reached(&self, k: usize) {
        if let (Some(p), Ok(mut s)) = (self.place[k], self.state.lock()) {
            s.walked = s.walked.max(p + 1);
        }
        self.ready.notify_all();
    }

    /// A member of suite `si` left nothing: the helpers may run its later members.
    fn open(&self, si: usize) {
        if let Ok(mut s) = self.state.lock() {
            s.open.insert(si);
        }
        self.ready.notify_all();
    }

    fn taint(&self, si: usize) {
        if let Ok(mut s) = self.state.lock() {
            s.tainted.insert(si);
        }
        self.ready.notify_all();
    }

    fn close(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.closed = true;
        }
        self.ready.notify_all();
    }
}

/// A helper thread's part of a [`ChainWalk`].
struct Helper<'a> {
    engine: &'a Arc<al_vm::Engine>,
    lowered: &'a Lowered,
    units: &'a [Unit],
    limit: Option<std::time::Duration>,
}

impl Helper<'_> {
    /// The helper's own copy of `start`: its install baseline, or the baseline read back from the bytes (the same
    /// data, so the same origin; over the engine's test data, as the install baseline's copy is).
    fn own_start(&self, start: &Start, install: &al_vm::Baseline) -> Option<al_vm::Baseline> {
        match start {
            Start::Install => Some(install.clone()),
            Start::Bytes(b) => al_vm::Baseline::from_bytes(&b.bytes).map(|base| {
                let mut base = base.backed_by(self.engine).with_origin(b.origin);
                base.install_rows = b.install_rows.clone();
                base
            }),
        }
    }

    /// Runs member `k` from `start` (a copy of where the walk would start it), keeping the events its hooks would be
    /// told.
    fn run_ahead(
        &self,
        k: usize,
        start: &al_vm::Baseline,
        leak: Option<Arc<std::collections::BTreeSet<u32>>>,
    ) -> Ahead {
        let kept: Arc<Mutex<Vec<HookEvent>>> = Default::default();
        let mut u = self.units[k].clone();
        u.leak = leak;
        let (on_start, on_test) = (kept.clone(), kept.clone());
        u.hooks.on_start = self.units[k].hooks.on_start.is_some().then(|| {
            let f: crate::events::OnStart = Arc::new(move |id: u32, test: &str| {
                if let Ok(mut e) = on_start.lock() {
                    e.push(HookEvent::Start(id, test.to_string()));
                }
            });
            f
        });
        u.hooks.on_test = self.units[k].hooks.on_test.is_some().then(|| {
            let f: Arc<dyn Fn(crate::events::TestEvent) + Send + Sync> = Arc::new(move |e| {
                if let Ok(mut kept) = on_test.lock() {
                    kept.push(HookEvent::Test(Box::new(e)));
                }
            });
            f
        });
        let result = run_unit(self.engine, self.lowered, Some(start), &u, None, self.limit);
        let left = super::run::CARRIED.with(|c| c.borrow_mut().take());
        let events = std::mem::take(&mut *kept.lock().unwrap_or_else(|e| e.into_inner()));
        Ahead {
            result,
            nothing: !super::chain::left_something(left.as_ref()),
            events,
        }
    }
}

impl ChainWalk<'_> {
    /// Runs the chain in its order and returns what each member left. `slots` gets the result of each member that
    /// ran, `details` the detail of each test under `trace`, `report` the members.
    pub(super) fn walk(
        &self,
        chain: Vec<(usize, u8, u32, Link)>,
        slots: &mut [Option<UnitResult>],
        details: &mut std::collections::HashMap<(usize, u32, String), explain::TestDetail>,
        report: &mut super::ChainReport,
    ) -> Vec<Carry> {
        // Only a codeunit runner 130450 runs can be run ahead: one with Disabled isolation always leaves its state.
        let order: Vec<usize> = chain
            .iter()
            .filter_map(|c| match c.3 {
                Link::Run(k) if self.units[k].carry.is_some() && !self.units[k].without_baseline => Some(k),
                _ => None,
            })
            .collect();
        if self.helpers == 0 || self.trace || order.len() < 2 || self.baseline.is_none() {
            return self.walk_on(chain, slots, details, report, None);
        }
        let runs = Runs {
            state: Mutex::new(RunsState {
                slots: (0..self.units.len()).map(|_| Slot::Pending).collect(),
                // A suite whose chain has no codeunit with Disabled isolation: its members start from the install baseline.
                starts: chain
                    .iter()
                    .map(|c| c.0)
                    .filter(|si| !chain.iter().any(|c| c.0 == *si && c.1 == 0))
                    .map(|si| (si, (0, Start::Install)))
                    .collect(),
                tainted: Default::default(),
                walked: 0,
                open: Default::default(),
                closed: false,
            }),
            ready: Condvar::new(),
            place: {
                let mut place = vec![None; self.units.len()];
                for (p, &k) in order.iter().enumerate() {
                    place[k] = Some(p);
                }
                place
            },
            window: self.helpers * WINDOW_PER_HELPER,
            order,
        };
        // What a helper thread needs (a baseline stays on its thread, so not the walk's).
        let helper = Helper {
            engine: self.engine,
            lowered: self.lowered,
            units: self.units,
            limit: self.limit,
        };
        let thread_baseline = self.thread_baseline;
        std::thread::scope(|scope| {
            for _ in 0..self.helpers.min(runs.order.len() - 1) {
                let (helper, runs) = (&helper, &runs);
                let spawned = std::thread::Builder::new()
                    .stack_size(TEST_STACK)
                    .spawn_scoped(scope, move || {
                        let Some(install) = thread_baseline() else {
                            return;
                        };
                        // The helper's copies of the starts it read back, per suite and epoch.
                        let mut copies: std::collections::HashMap<(usize, u32), Option<al_vm::Baseline>> =
                            Default::default();
                        loop {
                            let (k, epoch, start) = match runs.claim(helper.units) {
                                Claim::Run(k, epoch, start) => (k, epoch, start),
                                Claim::Done => return,
                            };
                            let si = helper.units[k].si;
                            // Only the newest start of a suite is used again.
                            copies.retain(|(s, e), _| *s != si || *e == epoch);
                            let leak = match &start {
                                Start::Install => None,
                                Start::Bytes(b) => Some(b.leak.clone()),
                            };
                            let own = copies
                                .entry((si, epoch))
                                .or_insert_with(|| helper.own_start(&start, &install));
                            runs.finish(k, si, epoch, own.as_ref().map(|own| helper.run_ahead(k, own, leak)));
                        }
                    });
                if spawned.is_err() {
                    // No room for another thread: fewer will do.
                    break;
                }
            }
            let out = self.walk_on(chain, slots, details, report, Some(&runs));
            runs.close();
            out
        })
    }

    /// Tells member `k`'s hooks the events of its run ahead, in the order they happened.
    fn replay(&self, k: usize, events: Vec<HookEvent>) {
        let hooks = &self.units[k].hooks;
        for e in events {
            match e {
                HookEvent::Start(id, test) => {
                    if let Some(f) = &hooks.on_start {
                        f(id, &test);
                    }
                }
                HookEvent::Test(e) => {
                    if let Some(f) = &hooks.on_test {
                        f(*e);
                    }
                }
            }
        }
    }

    fn walk_on(
        &self,
        chain: Vec<(usize, u8, u32, Link)>,
        slots: &mut [Option<UnitResult>],
        details: &mut std::collections::HashMap<(usize, u32, String), explain::TestDetail>,
        report: &mut super::ChainReport,
        runs: Option<&Runs>,
    ) -> Vec<Carry> {
        let (engine, lowered, units) = (self.engine, self.lowered, self.units);
        let name_of = |oi: usize| lowered.program.objects[oi].name.clone();
        // Per suite, in id order: the codeunit id and what it left, with the tables of every one before it.
        let mut carried: Vec<Carry> = Vec::new();
        // Per suite: the epoch of the start the helpers were told last (`Start`); a suite without one is not run ahead.
        let mut epochs: std::collections::HashMap<usize, u32> = runs.map_or_else(Default::default, |r| {
            r.state.lock().map_or_else(
                |_| Default::default(),
                |s| s.starts.iter().map(|(si, (e, _))| (*si, *e)).collect(),
            )
        });
        // Per suite: the new starts told after the first, and the runs ahead taken (see `RESTARTS`).
        let mut restarts: std::collections::HashMap<usize, usize> = Default::default();
        let mut taken: std::collections::HashMap<usize, usize> = Default::default();
        for (si, leg, id, link) in chain {
            let before = carried.iter().rev().find(|(s, _, _, _)| *s == si);
            // Whether this member, one runner 130450 runs, left something.
            let mut leg1_left = None;
            let mut left = match link {
                Link::Run(k) => {
                    let start = match before {
                        Some((_, _, _, c)) => c.baseline.as_ref(),
                        None => self.baseline,
                    };
                    // The first member runner 130450 runs after codeunits with Disabled isolation: the members of its leg
                    // start where the walk is now.
                    if let (Some(r), Some((_, _, _, c)), 1, false) = (runs, before, leg, epochs.contains_key(&si)) {
                        if let Some(e) = self.publish(r, si, c) {
                            epochs.insert(si, e);
                        }
                    }
                    let ahead = runs.and_then(|r| r.take(k, epochs.get(&si).copied()));
                    if let Some(r) = runs {
                        r.reached(k);
                    }
                    let took_ahead = ahead.as_ref().is_some_and(|a| a.nothing);
                    if took_ahead {
                        *taken.entry(si).or_default() += 1;
                    }
                    let (mut r, mut left) = match ahead {
                        // Every member before it in its suite left nothing, so it started from `start` too
                        // (`self.baseline`, or a clone of it), and it hands on the state it was given.
                        Some(a) if a.nothing => {
                            self.replay(k, a.events);
                            (a.result, Some(super::run::Carried::nothing()))
                        }
                        _ => {
                            let mut u = units[k].clone();
                            u.leak = before.map(|(_, _, _, c)| Arc::new(c.tables.clone()));
                            // `--explain` names where a test of the chain failed too (al-runner issue 6924).
                            let mut observe = |name: &str, _: &Outcome, seen: &Observed| {
                                details.insert((si, id, name.to_lowercase()), observed_detail(lowered, seen));
                            };
                            let trace: Option<&mut Observer> = self.trace.then_some(&mut observe);
                            let r = run_unit(engine, lowered, start, &u, trace, self.limit);
                            (r, super::run::CARRIED.with(|c| c.borrow_mut().take()))
                        }
                    };
                    let something = super::chain::left_something(left.as_ref());
                    if leg == 1 {
                        leg1_left = Some(something);
                    }
                    report.members.push(super::ChainEntry {
                        id,
                        name: name_of(units[k].oi),
                        why: super::chain::member_why(units[k].isolation, units[k].carry),
                        took: r.took,
                        left: something,
                        ahead: took_ahead,
                    });
                    // A codeunit that left nothing new (`numbers_taken.rs`) leaves the state it was given, not the
                    // session's: under the baseline rule check that session started on an empty store.
                    if let Some(l) = left.as_mut().filter(|l| l.baseline.is_none()) {
                        l.baseline = start.cloned();
                    }
                    if let (Some((_, _, _, b)), Ok(done)) = (before, &mut r.result) {
                        let storage = b.everything && reaches_session_storage(engine, lowered, units[k].oi);
                        let why =
                            if storage { b.storage_why.as_ref() } else { None }.or(if done.5.leak_hit || storage {
                                b.unfinished.as_ref()
                            } else {
                                None
                            });
                        if let Some(why) = why {
                            refuse_all(&mut done.0, why);
                        }
                    }
                    slots[k] = Some(r);
                    left.unwrap_or_else(|| {
                        not_run_link(
                            engine,
                            lowered,
                            units[k].oi,
                            units[k].isolation != TestIsolation::Disabled,
                            "it was not run as a whole".into(),
                        )
                    })
                }
                Link::Refused(oi, why) => {
                    report.members.push(super::ChainEntry {
                        id,
                        name: name_of(oi),
                        why: super::ChainWhy::NotRun,
                        took: std::time::Duration::ZERO,
                        left: true,
                        ahead: false,
                    });
                    not_run_link(engine, lowered, oi, false, why)
                }
            };
            if let Some((_, _, _, b)) = before {
                left.tables.extend(b.tables.iter().copied());
                left.unfinished = left.unfinished.or_else(|| b.unfinished.clone());
                left.storage_why = left.storage_why.or_else(|| b.storage_why.clone());
                left.everything |= b.everything;
                if left.baseline.is_none() {
                    left.baseline = b.baseline.clone();
                }
            }
            carried.push((si, id, leg, left));
            // A member that left something: the later members start from what it left. The helpers are told that start,
            // while the runs ahead the walk took in the suite make up for the new starts (`RESTARTS`).
            if let (Some(false), Some(r)) = (leg1_left, runs) {
                r.open(si);
            }
            if let (Some(true), Some(r)) = (leg1_left, runs) {
                epochs.remove(&si);
                let n = restarts.entry(si).or_default();
                *n += 1;
                match carried
                    .last()
                    .filter(|_| *n <= RESTARTS + taken.get(&si).copied().unwrap_or(0))
                {
                    Some((_, _, _, c)) => {
                        let started = std::time::Instant::now();
                        if let Some(e) = self.publish(r, si, c) {
                            epochs.insert(si, e);
                        }
                        report.starts.push(started.elapsed());
                    }
                    None => r.taint(si),
                }
            }
        }
        carried
    }

    /// Tells the helpers that the members of suite `si` start from `c` now; its epoch, or `None` (and no member of the
    /// suite is run ahead any more) when `c`'s baseline cannot be written as bytes.
    fn publish(&self, r: &Runs, si: usize, c: &super::run::Carried) -> Option<u32> {
        match c.baseline.as_ref().and_then(|b| Some((b, b.to_bytes()?))) {
            Some((b, bytes)) => Some(r.publish(
                si,
                Start::Bytes(Arc::new(StartBytes {
                    bytes,
                    install_rows: b.install_rows.clone(),
                    origin: b.origin(),
                    leak: Arc::new(c.tables.clone()),
                })),
                self.units,
            )),
            None => {
                r.taint(si);
                None
            }
        }
    }
}
