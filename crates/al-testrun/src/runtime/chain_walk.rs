//! The walk of the carry chain (`corpus.rs`), and the runs ahead that let it use more than one thread (al-runner
//! issue 6924).
//!
//! The chain runs its codeunits in order, each from what the ones before it in its suite left (`Carried`). Under runner
//! 130450 a codeunit hands on the state it started from with what it changed there: the numbers its tests took from
//! the sequences of that state (`numbers_taken.rs`), what its OnRun trigger wrote, and the rows the runner writes for
//! it (`suite_rows.rs`). So within an epoch of its suite (from the install baseline, or from the last state the walk
//! could not express as such a change), a codeunit starts from the epoch's start with the changes of the members
//! before it applied (`al_vm::Delta`).
//!
//! Helper threads run the chain's codeunits ahead of the walk, in chain order, each from its own copy of the epoch's
//! start with the changes the members before it are predicted to make applied (`chain_runs.rs`): what each changed in
//! its own last run ahead, or once the walk passed it, what it changed there. The walk takes such a run only when its
//! start has the fingerprint of the walk's actual state (`Baseline::fingerprint`: everything a session restored from it
//! starts from). The run then started from the very state the walk would have given it, so its outcome is the serial
//! one, and the walk rebuilds what it hands on as its own state with the run's change applied (a baseline holds `Rc`s
//! and stays on its thread). A wrong prediction only costs time: that codeunit runs on the walk's thread as before,
//! and a run ahead that is not taken is dropped. The events of a run ahead (`Hooks::on_start`, `Hooks::on_test`) are
//! kept and told when the walk takes it, so a test is told once, in chain order.

use super::SerialWhy;
use super::chain_runs::{Ahead, Claim, Dropped, HookEvent, Kept, Runs, Start, StartBytes};
use super::corpus::{not_run_link, observed_detail, reaches_session_storage, refuse_all};
use super::*;
use std::sync::Mutex;

/// How many new starts the walk tells the helpers for a suite beyond the runs ahead it took there: each member that
/// left more than numbers makes the runs ahead of the later ones useless and costs an encoding of its baseline on the
/// walk's thread, so a suite whose members often leave rows is not run ahead any more. Only the time of the run
/// depends on it.
const RESTARTS: usize = 2;

/// How many runs ahead from another start than the walk's (wrong predictions) the walk drops in a suite beyond the
/// runs ahead it took there before the suite is not run ahead any more: wasted runs take the machine from the walk.
/// Only the time of the run depends on it.
const MISSES: usize = 4;

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

/// A helper thread's part of a [`ChainWalk`].
struct Helper<'a> {
    engine: &'a Arc<al_vm::Engine>,
    lowered: &'a Lowered,
    units: &'a [Unit],
    limit: Option<std::time::Duration>,
    /// The Test Runner's tables, whose rows the fingerprints hold apart (`suite_rows::tables`).
    apart: Vec<u32>,
}

impl Helper<'_> {
    /// Runs the members `runs` hands out until it has none left.
    fn serve(&self, runs: &Runs, thread_baseline: &(dyn Fn() -> Option<al_vm::Baseline> + Sync)) {
        let Some(install) = thread_baseline() else {
            return;
        };
        // The helper's copies of the starts it read back, per suite and epoch.
        let mut copies: std::collections::HashMap<(usize, u32), Option<al_vm::Baseline>> = Default::default();
        loop {
            let (k, epoch, start, from) = match runs.claim() {
                Claim::Run(k, epoch, start, from) => (k, epoch, start, *from),
                Claim::Done => return,
            };
            let si = self.units[k].si;
            // Only the newest start of a suite is used again.
            copies.retain(|(s, e), _| *s != si || *e == epoch);
            let leak = match &start {
                Start::Install => None,
                Start::Bytes(b) => Some(b.leak.clone()),
            };
            let own = copies
                .entry((si, epoch))
                .or_insert_with(|| self.own_start(&start, &install));
            // The predicted start: the epoch's start with what the members before it change applied.
            let predicted = own.as_ref().and_then(|own| own.with_delta(&from));
            let fingerprint = predicted
                .as_ref()
                .and_then(|p| p.fingerprint_apart(&self.apart))
                .map(Arc::new);
            runs.starting(k, fingerprint.clone());
            let ahead = match (predicted, fingerprint) {
                (Some(p), Some(fp)) => Some(self.run_ahead(k, &p, leak, fp)),
                _ => None,
            };
            runs.finish(k, epoch, from, ahead);
        }
    }

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

    /// Runs member `k` from `start` (a copy of where the walk is predicted to start it), keeping the events its hooks
    /// would be told.
    fn run_ahead(
        &self,
        k: usize,
        start: &al_vm::Baseline,
        leak: Option<Arc<std::collections::BTreeSet<u32>>>,
        fingerprint: Arc<al_vm::Fingerprint>,
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
        super::suite_rows::take_unseen();
        let result = run_unit(self.engine, self.lowered, Some(start), &u, None, self.limit);
        let unseen = super::suite_rows::take_unseen();
        let left = super::run::CARRIED.with(|c| c.borrow_mut().take());
        let events = std::mem::take(&mut *kept.lock().unwrap_or_else(|e| e.into_inner()));
        Ahead {
            result,
            kept: kept_of(left.as_ref(), start),
            start: Some(fingerprint),
            unseen,
            events,
        }
    }
}

/// What a run from `start` that left `left` hands on, when the walk can rebuild it from its own state: nothing, or a
/// baseline that is `start` with a delta applied (`Baseline::delta_from`); `None` for anything else (a state not known,
/// isolated storage changed, a row that is not a plain value).
fn kept_of(left: Option<&super::run::Carried>, start: &al_vm::Baseline) -> Option<Kept> {
    if !super::chain::left_something(left) {
        return Some(Kept::Nothing);
    }
    let l = left?;
    Some(Kept::Left {
        delta: Box::new(l.baseline.as_ref()?.delta_from(start)?),
        tables: l.tables.clone(),
        unfinished: l.unfinished.clone(),
        everything: l.everything,
        storage_why: l.storage_why.clone(),
        as_started: l.as_started,
    })
}

/// What a member that started from `start` (the state its run ahead started from, by fingerprint) and hands on
/// `kept` leaves: what that run left, with its baseline rebuilt on the walk's thread as `start` with the run's delta
/// applied, with the AutoIncrement counters of the tables `apart` raised as the run from `start` would leave them
/// when given (`Baseline::with_delta_apart`). `None` when the delta does not apply to `start`.
fn left_of(
    kept: &Kept,
    start: Option<&al_vm::Baseline>,
    apart: Option<&[(u32, usize)]>,
) -> Option<super::run::Carried> {
    match kept {
        Kept::Nothing => Some(super::run::Carried::nothing()),
        Kept::Left {
            delta,
            tables,
            unfinished,
            everything,
            storage_why,
            as_started,
        } => Some(super::run::Carried {
            baseline: Some(match apart {
                Some(apart) => start?.with_delta_apart(delta, apart)?,
                None => start?.with_delta(delta)?,
            }),
            tables: tables.clone(),
            unfinished: unfinished.clone(),
            everything: *everything,
            storage_why: storage_why.clone(),
            as_started: *as_started,
        }),
    }
}

/// What changed from `base` to `now` (`Baseline::delta_from`).
fn delta_from(now: Option<&al_vm::Baseline>, base: Option<&al_vm::Baseline>) -> Option<al_vm::Delta> {
    now?.delta_from(base?)
}

/// What the walk keeps per suite for the runs ahead.
#[derive(Default)]
struct Track {
    /// Per suite: the epoch of the start the helpers were told last (`Start`); a suite without one is not run ahead.
    epochs: std::collections::HashMap<usize, u32>,
    /// Per suite: that start, as the walk's own baseline.
    bases: std::collections::HashMap<usize, al_vm::Baseline>,
    /// Per suite: the new starts told after the first, the runs ahead taken and the runs ahead dropped because they
    /// started elsewhere (see `RESTARTS`, `MISSES`).
    restarts: std::collections::HashMap<usize, usize>,
    taken: std::collections::HashMap<usize, usize>,
    misses: std::collections::HashMap<usize, usize>,
    /// The suites whose leg 1 started: their first leg-1 member told the helpers its start.
    started: std::collections::HashSet<usize>,
}

/// One member of leg 1 (runner 130450) the walk just ran: its unit, and the state it started from.
struct Leg1 {
    k: usize,
    start: Option<al_vm::Baseline>,
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
        // A suite whose chain has no codeunit with Disabled isolation: its members start from the install baseline.
        let install: std::collections::BTreeSet<usize> = chain
            .iter()
            .map(|c| c.0)
            .filter(|si| !chain.iter().any(|c| c.0 == *si && c.1 == 0))
            .collect();
        let runs = Runs::new(
            order,
            self.units.iter().map(|u| u.si).collect(),
            install.into_iter().collect(),
            self.helpers * WINDOW_PER_HELPER,
        );
        // What a helper thread needs (a baseline stays on its thread, so not the walk's).
        let helper = Helper {
            engine: self.engine,
            lowered: self.lowered,
            units: self.units,
            limit: self.limit,
            apart: self.apart(),
        };
        let thread_baseline = self.thread_baseline;
        std::thread::scope(|scope| {
            for _ in 0..self.helpers.min(runs.order.len() - 1) {
                let (helper, runs) = (&helper, &runs);
                let spawned = std::thread::Builder::new()
                    .stack_size(TEST_STACK)
                    .spawn_scoped(scope, move || helper.serve(runs, thread_baseline));
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

    /// The tables whose rows a run's start may differ in when its run did not see them: the Test Runner's "Test Method
    /// Line" and "AL Test Suite", which `suite_rows::write` empties before the unit's tests (`suite_rows::take_unseen`
    /// says whether anything saw what they held).
    fn apart(&self) -> Vec<u32> {
        super::suite_rows::tables(&self.engine.program).into_iter().collect()
    }

    /// The tables of `apart` with an AutoIncrement field, with its position (`Baseline::with_delta_apart`).
    fn counters(&self, apart: &[u32]) -> Vec<(u32, usize)> {
        apart
            .iter()
            .filter_map(|&o| {
                let fields = &self.engine.program.objects[o as usize].table.as_ref()?.fields;
                Some((o, fields.iter().position(|f| f.auto_increment)?))
            })
            .collect()
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
        let apart = self.apart();
        let counters = self.counters(&apart);
        let name_of = |oi: usize| lowered.program.objects[oi].name.clone();
        // Per suite, in id order: the codeunit id and what it left, with the tables of every one before it.
        let mut carried: Vec<Carry> = Vec::new();
        let mut track = Track {
            epochs: runs.map(Runs::epochs).unwrap_or_default(),
            ..Default::default()
        };
        if let Some(b) = self.baseline {
            let install: Vec<usize> = track.epochs.keys().copied().collect();
            track.bases = install.into_iter().map(|si| (si, b.clone())).collect();
        }
        for (si, leg, id, link) in chain {
            let before = carried.iter().rev().find(|(s, _, _, _)| *s == si);
            let mut leg1 = None;
            let mut left = match link {
                Link::Run(k) => {
                    let start = match before {
                        Some((_, _, _, c)) => c.baseline.as_ref(),
                        None => self.baseline,
                    };
                    // The first member runner 130450 runs after codeunits with Disabled isolation: the members of its leg
                    // start where the walk is now.
                    if let (Some(r), Some((_, _, _, c)), 1) = (runs, before, leg) {
                        if track.started.insert(si) && !track.epochs.contains_key(&si) {
                            // It has not run yet: the start holds for it too.
                            self.publish(r, si, r.place(k), c, &mut track);
                        }
                    }
                    let epoch = track.epochs.get(&si).copied();
                    // The fingerprint of where it starts, when a helper runs or ran it in this epoch.
                    let actual = runs
                        .filter(|r| r.has_run(k, epoch))
                        .and(start)
                        .and_then(|b| b.fingerprint_apart(&apart));
                    // Whether the helpers ran the suite when the walk reached the member (`SerialWhy::Closed`).
                    let open = runs.is_some_and(|r| r.is_open(si));
                    let (ahead, dropped) = runs.map_or((None, Dropped::Nothing), |r| r.take(k, epoch, actual.as_ref()));
                    if let Dropped::Wrong(predicted) = &dropped {
                        *track.misses.entry(si).or_default() += 1;
                        self.note_miss(predicted, start, track.bases.get(&si), report);
                    }
                    if let Some(r) = runs {
                        r.reached(k);
                    }
                    if leg == 1 {
                        leg1 = Some(Leg1 {
                            k,
                            start: start.cloned(),
                        });
                    }
                    // A run ahead from the walk's own state that left nothing or only numbers: its outcome is the serial one.
                    // Taken from a start whose Test Runner rows differ (its run did not see them): their AutoIncrement
                    // counters are set as the run from the walk's start leaves them (`Baseline::with_delta_apart`).
                    let kept = ahead.as_ref().and_then(|a| {
                        let differ = a.start.as_deref().map(|f| &f.rows) != actual.as_ref().map(|f| &f.rows);
                        left_of(a.kept.as_ref()?, start, differ.then_some(counters.as_slice()))
                    });
                    let took_ahead = kept.is_some();
                    if took_ahead {
                        *track.taken.entry(si).or_default() += 1;
                    }
                    let serial = match runs {
                        _ if took_ahead => None,
                        _ if units[k].carry.is_none() || units[k].without_baseline => Some(SerialWhy::NeverAhead),
                        None => Some(SerialWhy::NoHelpers),
                        Some(_) if ahead.is_some() => Some(SerialWhy::NotRebuilt),
                        Some(_) if matches!(dropped, Dropped::Wrong(_)) => Some(SerialWhy::Missed),
                        Some(_) if matches!(dropped, Dropped::Early) => Some(SerialWhy::Early),
                        Some(_) if matches!(dropped, Dropped::Guessed) => Some(SerialWhy::Guessed),
                        Some(r) if r.tainted(si) => Some(SerialWhy::GaveUp),
                        Some(_) if epoch.is_none() => Some(SerialWhy::NoStart),
                        Some(_) if !open => Some(SerialWhy::Closed),
                        Some(_) => Some(SerialWhy::NotRun),
                    };
                    let (mut r, mut left) = match (ahead, kept) {
                        (Some(a), Some(kept)) => {
                            self.replay(k, a.events);
                            (a.result, Some(kept))
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
                    report.members.push(super::ChainEntry {
                        id,
                        name: name_of(units[k].oi),
                        why: super::chain::member_why(units[k].isolation, units[k].carry),
                        took: r.took,
                        left: super::chain::left_something(left.as_ref()),
                        ahead: took_ahead,
                    });
                    if let Some(why) = serial {
                        report.not_ahead.push(super::ChainEntry {
                            id,
                            name: name_of(units[k].oi),
                            why,
                            took: r.took,
                            left: false,
                            ahead: false,
                        });
                    }
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
            if let (Some(leg1), Some(r), Some((_, _, _, c))) = (leg1, runs, carried.last()) {
                self.after(r, si, leg1, c, &mut track, report);
            }
        }
        carried
    }

    /// A run ahead from the prediction `predicted` (a delta on the suite's epoch start `base`) was dropped because the
    /// walk's state `start` differs from it: `report` notes what differed (`--timings`; only the time of the run depends
    /// on predictions).
    fn note_miss(
        &self,
        predicted: &al_vm::Delta,
        start: Option<&al_vm::Baseline>,
        base: Option<&al_vm::Baseline>,
        report: &mut super::ChainReport,
    ) {
        let Some(actual) = delta_from(start, base) else {
            report.missed_by("the walk's state is not its start with a change");
            return;
        };
        let parts = predicted.differences(&actual);
        if parts.is_empty() {
            report.missed_by("no part of the change (the Test Runner's rows the run saw, or the start)");
        }
        let table = |o: u32| {
            let name = self
                .engine
                .program
                .objects
                .get(o as usize)
                .map_or("", |x| x.name.as_str());
            format!("{o} {name}")
        };
        for part in parts {
            report.missed_by(&match part {
                al_vm::DeltaPart::Table(o) => format!("table {}", table(o)),
                al_vm::DeltaPart::AutoIncrement(o) => format!("AutoIncrement of table {}", table(o)),
                al_vm::DeltaPart::Sequence(name) => format!("number sequence {name}"),
                al_vm::DeltaPart::GuidCounter => "GUID counter".into(),
                al_vm::DeltaPart::RowVersion => "row version counter".into(),
                al_vm::DeltaPart::Marks => "baseline marks".into(),
            });
        }
    }

    /// What the helpers are told after leg-1 member `leg1` of suite `si` ran and the suite's chain is at `c`. While
    /// the state is its epoch's start with only its sequences moved on, the later members' predictions start there.
    /// Otherwise the later members start from `c`: a new start (epoch) for the helpers, while the runs ahead the walk
    /// took in the suite make up for the new starts (`RESTARTS`); else the suite is not run ahead any more.
    fn after(
        &self,
        r: &Runs,
        si: usize,
        leg1: Leg1,
        c: &super::run::Carried,
        track: &mut Track,
        report: &mut super::ChainReport,
    ) {
        let taken = track.taken.get(&si).copied().unwrap_or(0);
        if track.misses.get(&si).copied().unwrap_or(0) > MISSES + taken {
            // Its predictions are wrong too often.
            track.epochs.remove(&si);
            r.taint(si);
            return;
        }
        let now = match track.epochs.get(&si) {
            Some(_) => delta_from(c.baseline.as_ref(), track.bases.get(&si)),
            None => None,
        };
        match now {
            Some(now) => {
                let took = delta_from(c.baseline.as_ref(), leg1.start.as_ref());
                r.advance(si, leg1.k, now, took);
            }
            // A suite without a known start whose member left nothing: it may run ahead once it has one.
            None if !super::chain::left_something(Some(c)) => r.open(si),
            None => {
                track.epochs.remove(&si);
                let n = track.restarts.entry(si).or_default();
                *n += 1;
                if *n <= RESTARTS + taken {
                    let started = std::time::Instant::now();
                    self.publish(r, si, r.place(leg1.k).map(|p| p + 1), c, track);
                    report.starts.push(started.elapsed());
                } else {
                    r.taint(si);
                }
            }
        }
    }

    /// Tells the helpers that the members of suite `si` from place `from` of their order on start from `c`, and keeps
    /// `c` as the suite's start; no member of the suite is run ahead any more when `c`'s baseline cannot be written as
    /// bytes (or `from` is not known).
    fn publish(&self, r: &Runs, si: usize, from: Option<usize>, c: &super::run::Carried, track: &mut Track) {
        let Some(from) = from else {
            track.epochs.remove(&si);
            r.taint(si);
            return;
        };
        match c.baseline.as_ref().and_then(|b| Some((b, b.to_bytes()?))) {
            Some((b, bytes)) => {
                let e = r.publish(
                    si,
                    Start::Bytes(Arc::new(StartBytes {
                        bytes,
                        install_rows: b.install_rows.clone(),
                        origin: b.origin(),
                        leak: Arc::new(c.tables.clone()),
                    })),
                    from,
                );
                track.epochs.insert(si, e);
                track.bases.insert(si, b.clone());
            }
            None => {
                track.epochs.remove(&si);
                r.taint(si);
            }
        }
    }
}
