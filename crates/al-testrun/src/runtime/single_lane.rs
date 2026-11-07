//! SingleInstance state across the test codeunits of a run (ADR-066 amendment 2, al-runner issue 6924).
//!
//! BC runs the test codeunits of a run in one session, so the globals of a SingleInstance codeunit that one codeunit
//! left are what the next one starts with (al-language/codeunits/TestCuP7BReader.al:24-37, against
//! TestCuP7ASetter.al:24-58). al-runner runs the codeunits in parallel, each on a session of its own, and copies no
//! state from one to the next. Instead it finds the codeunits that depend on such state and runs those that need it
//! again, with the codeunits that wrote it, in a serial session in BC's order, the single instances moved from codeunit
//! to codeunit. The tests of a dependent codeunit are flagged (`Assumption::SingleInstanceOrder`).
//!
//! Three steps, each as cheap as it can be:
//!
//! 1. [`assign_watches`]: in every BC session that has two test codeunits or more, a session notes the single
//!    instances (with globals) it makes.
//! 2. The parallel run notes, for each watched codeunit, the globals read first and the end values
//!    (`al_vm::single_track`).
//! 3. [`Lane::run`] goes through each BC session in order ([`analyse`]): a codeunit depends on earlier state when a
//!    global it read first held another value than the last earlier writer left, unless it filled the same cache
//!    itself: it ended with the very values the earlier writer left. Every dependent codeunit is flagged. Amendment 4:
//!    it keeps its parallel run (fresh state, a real BC outcome: the codeunit run alone or first); only one with a
//!    failed test or a test refused while it ran (amendment 5: not one refused before it started), or every one under
//!    `--strict-order`, runs again with the writers it needs ([`Found::lane_for`]), and that run is reported.
//!    Amendment 5: each such chain is a serial session of its own ([`Found::sessions`]); the sessions run in parallel
//!    on the worker threads.
//!
//! Amendment 6: with `--test-suites` (`RunOptions::test_suites`) each listed suite is a BC session of its own, in the
//! file's order, and a codeunit in no suite is a session of its own ([`streams`]); without it the whole run is one.

use super::chain_walk::Carry as ChainCarry;
use super::*;
use al_vm::single_track::{GlobalKey, SingleOut};
use std::collections::{BTreeMap, BTreeSet};

/// The BC sessions of a run: the units of each (suite, hub leg) in BC's run order.
///
/// ADR-066 amendment 6 (al-runner issue 6924): with `--test-suites` each listed suite is a BC session of its own (per
/// suite and hub leg), its codeunits in the file's order; a codeunit listed in two suites is in both sessions, and one
/// in no suite is a session of its own (no earlier codeunit, so no link). Without it, the whole run is one session.
fn streams(
    units: &[Unit],
    place: &dyn Fn(usize, u32) -> u32,
    has: &dyn Fn(usize) -> bool,
    suites: Option<&crate::test_suites::TestSuites>,
) -> Vec<Vec<usize>> {
    let picked = (0..units.len()).filter(|&k| !units[k].execute && has(k));
    // (session key, place in the session, unit), sorted: a key is (suite or `usize::MAX`, app suite, hub leg).
    let mut order: Vec<((usize, usize, bool), u64, usize)> = match suites {
        None => picked
            .map(|k| {
                let u = &units[k];
                ((0, u.si, u.hub_leg), u64::from(place(u.si, u.id)), k)
            })
            .collect(),
        Some(s) => {
            let index = s.index();
            let mut out = Vec::new();
            for k in picked {
                let u = &units[k];
                match index.get(&u.id) {
                    Some(places) => {
                        out.extend(places.iter().map(|&(si, p)| ((si, u.si, u.hub_leg), p as u64, k)));
                    }
                    // Alone: a session of its own (no other unit gets this key).
                    None => out.push(((usize::MAX, u.si, u.hub_leg), k as u64, k)),
                }
            }
            out
        }
    };
    order.sort();
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut last = None;
    for (key, _, k) in order {
        if last == Some(key) && key.0 != usize::MAX {
            out.last_mut().expect("a stream").push(k);
        } else {
            out.push(vec![k]);
            last = Some(key);
        }
    }
    out
}

/// Step 1: sets `Unit::watch` for each codeunit of a BC session that has another. A session notes the single instances
/// it makes that have globals (SingleInstance codeunits without globals are ignored), so a codeunit that uses none pays
/// one test per global access and nothing else; a BC session of one test codeunit watches nothing. (A static reach
/// analysis that narrowed the watch to the codeunits that can read what an earlier one can write cost more than
/// watching: a walk of every test's reach through the Microsoft apps, +3.3 % instructions on the BC 29 corpus run,
/// against +0.3 % for the notes. It also read every object of a compile cache that decodes them on touch.) Returns how
/// many codeunits got a watch.
pub(super) fn assign_watches(units: &mut [Unit], place: &dyn Fn(usize, u32) -> u32, opts: &RunOptions) -> usize {
    let mut n = 0;
    for ks in streams(units, place, &|_| true, opts.test_suites.as_deref()) {
        if ks.len() < 2 {
            continue;
        }
        for k in ks {
            units[k].watch = true;
            n += 1;
        }
    }
    n
}

/// What [`analyse`] found for one BC session.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Found {
    /// Position (in the session's order) of a dependent codeunit, with the position of the earlier codeunit whose
    /// state it relies on.
    pub dependent: BTreeMap<usize, (usize, GlobalKey)>,
    /// Positions of the codeunits to run in the lane: the dependent ones and the writers they need.
    pub lane: BTreeSet<usize>,
    /// Per dependent codeunit (position), the positions of the earlier codeunits it needs in the lane: for each global
    /// that makes it dependent, the last writer (R2).
    pub writers: BTreeMap<usize, BTreeSet<usize>>,
    /// Position of a codeunit that is not dependent only because a guard then the refill of a global it read first
    /// is taken as no link although the earlier codeunit left another value (`Assumption::SingleInstanceRefill`), with
    /// that codeunit's position and the global.
    pub refill_assumed: BTreeMap<usize, (usize, GlobalKey)>,
}

/// Step 3, the decision: from what each codeunit of a BC session (in order) noted, which depend on state an earlier one
/// left. `None`: no notes (the codeunit was not watched, or it did not run).
///
/// ADR-066 amendment 3: a global read first whose read was a guard directly followed by the refill (R1b) is no link
/// when the refill left the very value the earlier codeunit left, and is no link under `Assumption::SingleInstanceRefill`
/// when it left another one. What a codeunit with a test refused mid-run left is no change (R3). The lane is each
/// dependent codeunit and, for each global that makes it dependent, only the last codeunit that wrote it (R2); that
/// writer's own writers join only when it is dependent itself.
pub(super) fn analyse(outs: &[Option<&SingleOut>]) -> Found {
    // Per global: what the last writer left, that writer, and whether it was refused mid-run.
    let mut state: BTreeMap<GlobalKey, (al_vm::single_track::Fp, usize, bool)> = BTreeMap::new();
    let mut found = Found::default();
    let mut lane: BTreeSet<usize> = BTreeSet::new();
    for (p, out) in outs.iter().enumerate() {
        let Some(o) = out else { continue };
        let mut diff: Vec<(GlobalKey, usize)> = Vec::new();
        // The globals read first that differ, but whose read was a guard directly followed by the refill (R1b).
        let mut guarded: Vec<(GlobalKey, usize)> = Vec::new();
        let mut assumed: Option<(usize, GlobalKey)> = None;
        for (k, seen) in &o.observed {
            let Some((fp, w, refused)) = state.get(k) else { continue };
            if fp == seen || *refused {
                continue;
            }
            if o.guarded.contains(k) {
                if o.refill.get(k) != Some(fp) {
                    assumed.get_or_insert((*w, *k));
                }
                guarded.push((*k, *w));
                continue;
            }
            diff.push((*k, *w));
        }
        // A lazy cache the codeunit filled itself with the very values the earlier codeunit left is no dependence:
        // whether it skipped the fill or did it, the state and what it reads are the same.
        let same_end = |k: &GlobalKey| match state.get(k) {
            Some((fp, _, false)) => o.ends.get(k).is_some_and(|e| e == fp || e.same_kind(fp)),
            _ => true,
        };
        let mut culprit = None;
        if !diff.is_empty() {
            // The global that gives the dependence away: the first one the codeunit did not end with as it was left. A
            // global it refilled after a guard (R1b) is not one: its end is the refill's (a time stamp never matches).
            // A cache a flag guards lives in the flag's codeunit: what the codeunit wrote in another single instance is
            // not tied to the flags it read (amendment 3).
            let objects: BTreeSet<usize> = diff.iter().map(|((o, _), _)| *o).collect();
            culprit = diff
                .iter()
                .map(|(k, _)| k)
                .chain(
                    o.wrote
                        .iter()
                        .filter(|k| !o.guarded.contains(k) && objects.contains(&k.0)),
                )
                .find(|k| !same_end(k))
                .and_then(|k| state.get(k).map(|(_, w, _)| (*w, *k)));
        } else if !guarded.is_empty() {
            // A refill is no link only when the codeunit left every other global as the earlier codeunit did: with the
            // earlier value the guard may have skipped more than the refill (a cache filled after the key).
            let objects: BTreeSet<usize> = guarded.iter().map(|((o, _), _)| *o).collect();
            culprit = o
                .wrote
                .iter()
                .filter(|k| !o.guarded.contains(k) && objects.contains(&k.0))
                .find(|k| !same_end(k))
                .and_then(|k| state.get(k).map(|(_, w, _)| (*w, *k)));
            if culprit.is_some() {
                diff = guarded;
            }
        }
        if let Some(c) = culprit {
            found.dependent.insert(p, c);
            lane.insert(p);
            lane.insert(c.0);
            lane.extend(diff.iter().map(|(_, w)| *w));
            let mine = found.writers.entry(p).or_default();
            mine.insert(c.0);
            mine.extend(diff.iter().map(|(_, w)| *w));
        } else if let Some(a) = assumed {
            found.refill_assumed.insert(p, a);
        }
        for k in &o.wrote {
            if let Some(end) = o.ends.get(k) {
                state.insert(*k, (end.clone(), p, o.refused));
            }
        }
    }
    found.lane = lane;
    found
}

impl Found {
    /// The lane for the dependent codeunits `chosen` (ADR-066 amendment 4): each one, the writers it needs, and the
    /// writers of a writer that is dependent itself (R2), to the end. With every dependent codeunit chosen, this is
    /// `lane` (amendment 3).
    pub(super) fn lane_for(&self, chosen: &BTreeSet<usize>) -> BTreeSet<usize> {
        let mut lane: BTreeSet<usize> = BTreeSet::new();
        let mut todo: Vec<usize> = chosen.iter().copied().collect();
        while let Some(p) = todo.pop() {
            if !lane.insert(p) {
                continue;
            }
            for &w in self.writers.get(&p).into_iter().flatten() {
                if self.writers.contains_key(&w) {
                    todo.push(w);
                } else {
                    lane.insert(w);
                }
            }
        }
        lane
    }

    /// The serial sessions for the dependent codeunits `chosen` (ADR-066 amendment 5, decision 1), each in BC's order,
    /// with the chosen codeunits whose run in it is reported: every chosen codeunit's chain ([`Found::lane_for`] of it
    /// alone) is a session of its own, so it sees no state from a codeunit outside its chain. A writer two chains share
    /// runs in each (decision 2). A chosen codeunit that is in a later chosen codeunit's chain, where every codeunit of
    /// that chain up to it is in its own chain, is reported from that session and gets none of its own: it starts with
    /// the very state there. Sorted by their positions.
    pub(super) fn sessions(&self, chosen: &BTreeSet<usize>) -> Vec<(Vec<usize>, BTreeSet<usize>)> {
        let chains: BTreeMap<usize, BTreeSet<usize>> = chosen
            .iter()
            .map(|&p| (p, self.lane_for(&BTreeSet::from([p]))))
            .collect();
        // Whether `p` runs in `q`'s session with what its own chain would give it.
        let inside =
            |p: usize, q: usize| p != q && chains[&q].contains(&p) && chains[&q].range(..=p).eq(chains[&p].iter());
        let absorbed: BTreeSet<usize> = chosen
            .iter()
            .copied()
            .filter(|&p| chosen.iter().any(|&q| inside(p, q)))
            .collect();
        let mut out: Vec<(Vec<usize>, BTreeSet<usize>)> = chosen
            .iter()
            .filter(|p| !absorbed.contains(p))
            .map(|&q| {
                let mut reported = BTreeSet::from([q]);
                reported.extend(
                    absorbed
                        .iter()
                        .filter(|&&p| chosen.iter().find(|&&r| !absorbed.contains(&r) && inside(p, r)) == Some(&q)),
                );
                (chains[&q].iter().copied().collect(), reported)
            })
            .collect();
        out.sort();
        out
    }
}

/// Whether a dependent codeunit runs again serially after its parallel run (ADR-066 amendment 4, decision 2, as
/// amendment 5 decision 2 narrows it): a test of it failed, or started and did not run to its end (refused while it ran,
/// a failure judged not trusted, a time limit). A test refused before it started does not count: the serial run would
/// refuse it all the same. A run that panicked has no notes and is never dependent.
fn needs_serial_run(slot: Option<&UnitResult>) -> bool {
    match slot.map(|r| &r.result) {
        Some(Ok(done)) => done.5.unfinished_after_start || done.0.iter().any(|(_, o)| matches!(o, Outcome::Fail(_))),
        _ => true,
    }
}

/// What the lane needs from the run (`corpus.rs`).
pub(super) struct Lane<'a> {
    pub engine: &'a Arc<Engine>,
    pub lowered: &'a Lowered,
    pub baseline: Option<&'a al_vm::Baseline>,
    pub units: &'a [Unit],
    pub carried: &'a [ChainCarry],
    pub sees: &'a dyn Fn(usize, u32, u8, &Unit) -> bool,
    pub place: &'a dyn Fn(usize, u32) -> u32,
    pub limit: Option<std::time::Duration>,
    /// `--strict-order` (ADR-066 amendment 4, decision 4): every dependent codeunit runs again serially.
    pub strict: bool,
    /// `--test-suites` (amendment 6): each suite is a BC session of its own.
    pub suites: Option<&'a crate::test_suites::TestSuites>,
    /// The run's worker threads: the serial sessions run on as many (amendment 5).
    pub workers: usize,
    /// A worker thread can make its own copy of the install baseline (`corpus.rs` `baseline_engine`); without it and
    /// with a baseline, the sessions run one after the other on this thread.
    pub install_copies: bool,
    /// The install baseline as bytes, written once per run (shared with the parallel run).
    pub snapshot: &'a std::cell::OnceCell<Option<Vec<u8>>>,
    /// A worker thread's own copy of the install baseline from `snapshot` (a baseline holds `Rc`s and stays on its
    /// thread).
    pub thread_baseline: &'a (dyn Fn(Option<&[u8]>) -> Option<al_vm::Baseline> + Sync),
}

/// What the lane did, for `--timings` and the stability report.
#[derive(Debug, Clone, Default)]
pub struct LaneReport {
    /// Test codeunits that were watched.
    pub watched: usize,
    /// BC sessions with at least one dependent codeunit.
    pub sessions: usize,
    /// The runs of codeunits in the lane (dependent ones and the writers they need), with the codeunit. A writer that
    /// two chains share is listed once per session it ran in (amendment 5).
    pub lane: Vec<(u32, String)>,
    /// One line per dependent codeunit: what it relies on.
    pub notes: Vec<String>,
    /// Codeunits taken as independent under `Assumption::SingleInstanceRefill` (a guard, then the refill of a global
    /// an earlier codeunit left with another value): their tests are flagged, they do not run again.
    pub refill_assumed: usize,
    /// The wall time the lane took (its serial sessions run in parallel, amendment 5).
    pub took: std::time::Duration,
    /// Dependent codeunits found (ADR-066 amendment 4): each one's tests are flagged.
    pub dependent: usize,
    /// Dependent codeunits that ran again serially because a test of theirs failed or was refused while it ran in the
    /// parallel run (amendments 4 and 5). With `strict`, every dependent codeunit runs again and this counts them all.
    pub rerun: usize,
    /// `--strict-order`: every dependent codeunit ran again serially.
    pub strict: bool,
    /// Serial sessions the lane ran (amendment 5): one per chain of a dependent codeunit that ran again (none for one
    /// whose chain starts another's).
    pub chains: usize,
    /// The time of the longest serial session.
    pub longest: std::time::Duration,
}

impl LaneReport {
    pub fn lines(&self) -> Vec<String> {
        if self.watched == 0 {
            return Vec::new();
        }
        let why = if self.strict {
            "(strict order: every dependent codeunit)"
        } else {
            "because a test failed or was refused while it ran in parallel"
        };
        let mut out = vec![format!(
            "SingleInstance order: {} test codeunits watched, {} BC sessions with a dependent codeunit, {} dependent codeunits (flagged), {} ran again serially {why}, {} codeunit runs in {} serial sessions (longest {:.2} s, lane wall time {:.2} s); {} codeunits flagged as independent under the guard-then-refill assumption",
            self.watched,
            self.sessions,
            self.dependent,
            self.rerun,
            self.lane.len(),
            self.chains,
            self.longest.as_secs_f64(),
            self.took.as_secs_f64(),
            self.refill_assumed
        )];
        out.extend(self.notes.iter().cloned());
        out
    }
}

/// One serial session of the lane (amendment 5): unit indices in BC's order, and the dependent ones whose run is
/// reported.
struct Chain {
    order: Vec<usize>,
    chosen: BTreeSet<usize>,
    /// Where each codeunit starts, for a worker thread. `None`: a carried baseline it may start from could not be
    /// written as bytes, so the session runs on this thread from the baselines themselves ([`Lane::run_bound`]).
    steps: Option<Vec<Step>>,
}

/// Where a codeunit of a serial session starts, worked out on this thread (amendment 5). A baseline a carry-chain
/// member left (RequiredTestIsolation = Disabled) holds `Rc`s and stays on the thread that made it, so a worker thread
/// gets it as bytes ([`Start`]), as the chain walk's helpers do (`chain_walk.rs`). The same choices as
/// [`Lane::run_bound`].
struct Step {
    k: usize,
    /// A chain member starts from what the member before it left (index into the starts); `None`: the install
    /// baseline.
    from: Option<usize>,
    /// A codeunit that is no member and comes after one: whether it reaches session storage that member changed (then
    /// it runs again whatever it read), and the start it runs again from when it read a table that member wrote
    /// (`None`: it keeps its first run).
    again: Option<(bool, Option<usize>)>,
}

/// A carried baseline as bytes, for a worker thread.
struct Start {
    bytes: Vec<u8>,
    install_rows: Option<BTreeSet<u32>>,
    origin: u64,
}

impl Start {
    /// The worker's own copy (the same data, so the same origin, over the engine's test data).
    fn decode(&self, engine: &Arc<Engine>) -> Option<al_vm::Baseline> {
        al_vm::Baseline::from_bytes(&self.bytes).map(|b| {
            let mut b = b.backed_by(engine).with_origin(self.origin);
            b.install_rows = self.install_rows.clone();
            b
        })
    }
}

/// The carried baselines written as bytes so far, by carry index (`None`: it has no bytes).
#[derive(Default)]
struct Starts {
    index: BTreeMap<usize, Option<usize>>,
    list: Vec<Start>,
}

impl Starts {
    fn of(&mut self, carry: usize, b: &al_vm::Baseline) -> Option<usize> {
        if let Some(i) = self.index.get(&carry) {
            return *i;
        }
        let i = b.to_bytes().map(|bytes| {
            self.list.push(Start {
                bytes,
                install_rows: b.install_rows.clone(),
                origin: b.origin(),
            });
            self.list.len() - 1
        });
        self.index.insert(carry, i);
        i
    }
}

/// What a worker thread needs to run a session's steps (all of it can be shared between threads).
struct Worker<'a> {
    engine: &'a Arc<Engine>,
    lowered: &'a Lowered,
    units: &'a [Unit],
    limit: Option<std::time::Duration>,
    starts: &'a [Start],
}

impl Worker<'_> {
    /// Runs one step from `install` (this thread's install baseline) or a carried start, decoded once per thread into
    /// `copies`. `None`: the codeunit keeps its first run.
    fn run(
        &self,
        install: Option<&al_vm::Baseline>,
        copies: &mut BTreeMap<usize, Option<al_vm::Baseline>>,
        s: &Step,
    ) -> Option<UnitResult> {
        let mut u = self.units[s.k].clone();
        u.watch = false;
        let mut copy = |i: usize| {
            copies
                .entry(i)
                .or_insert_with(|| self.starts[i].decode(self.engine))
                .clone()
        };
        let first = match s.from {
            Some(i) => Some(copy(i)?),
            None => None,
        };
        let mut r = run_unit(
            self.engine,
            self.lowered,
            first.as_ref().or(install),
            &u,
            None,
            self.limit,
        );
        if let Some((storage, again)) = s.again {
            if storage || r.result.as_ref().is_ok_and(|done| done.5.leak_hit) {
                let from = copy(again?)?;
                super::run::CARRIED.with(|c| c.borrow_mut().take());
                r = run_unit(self.engine, self.lowered, Some(&from), &u, None, self.limit);
            }
        }
        Some(r)
    }
}

/// What one serial session gave: each codeunit's run in order (`None`: kept its first run, `corpus.rs` refuses the
/// case), and the time it took.
type ChainRuns = (Vec<(usize, Option<UnitResult>)>, std::time::Duration);

/// Runs the codeunits `order` one after the other on this thread, the single instances moved from codeunit to codeunit
/// (`run.rs` `LANE_SINGLES`).
fn run_in_order(order: &[usize], one: &mut dyn FnMut(usize) -> Option<UnitResult>) -> ChainRuns {
    let started = std::time::Instant::now();
    super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = Some(None));
    let mut out = Vec::with_capacity(order.len());
    for &k in order {
        let r = one(k);
        super::run::CARRIED.with(|c| c.borrow_mut().take());
        out.push((k, r));
    }
    super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = None);
    (out, started.elapsed())
}

impl Lane<'_> {
    /// Step 3: finds the dependent codeunits of each BC session, flags them, and runs the serial sessions of those that
    /// run again (in parallel, amendment 5), putting the dependent codeunits' results (flagged) in `slots`.
    pub(super) fn run(&self, slots: &mut [Option<UnitResult>], report: &mut LaneReport) {
        if report.watched == 0 {
            return;
        }
        let started = std::time::Instant::now();
        report.strict = self.strict;
        let mut starts = Starts::default();
        let chains = self.chains(slots, report, &mut starts);
        report.chains = chains.len();
        let runs = self.run_chains(&chains, &starts.list);
        for (chain, (done, took)) in chains.iter().zip(runs) {
            report.longest = report.longest.max(took);
            for (k, r) in done {
                let Some(mut r) = r else { continue };
                report.lane.push((self.units[k].id, self.name(k)));
                if !chain.chosen.contains(&k) {
                    // A writer that does not depend keeps its first run: it was complete without the lane. So does a
                    // dependent writer that was not chosen (amendment 4: it passed in parallel, flagged), and a chosen
                    // one whose result comes from its own session. A chain member that depends hands the later
                    // codeunits the rows of its first run (its run here is dropped).
                    continue;
                }
                if let Ok(done) = &mut r.result {
                    // What the first run marked (an instance's making marks once, in the codeunit that made it) stays.
                    if let Some(Ok(first)) = slots[k].as_ref().map(|f| &f.result) {
                        for (test, hits) in &first.3 {
                            match done.3.iter_mut().find(|(t, _)| t == test) {
                                Some((_, mine)) => {
                                    mine.extend(hits.iter().copied());
                                    mine.sort();
                                    mine.dedup();
                                }
                                None => done.3.push((test.clone(), hits.clone())),
                            }
                        }
                    }
                    flag_all(done, Assumption::SingleInstanceOrder);
                }
                slots[k] = Some(r);
            }
        }
        report.took += started.elapsed();
    }

    /// Analyses each BC session, flags its dependent codeunits, and returns the serial sessions to run.
    fn chains(&self, slots: &mut [Option<UnitResult>], report: &mut LaneReport, starts: &mut Starts) -> Vec<Chain> {
        let has = |k: usize| slots[k].is_some();
        let streams = streams(self.units, self.place, &has, self.suites);
        let mut chains = Vec::new();
        // Amendment 6: a codeunit listed in two suites is in two sessions but runs and is reported once. The first
        // session (in the file's order) where it is dependent, or where the refill assumption flags it, decides.
        let mut counted: BTreeSet<usize> = BTreeSet::new();
        let mut refill_counted: BTreeSet<usize> = BTreeSet::new();
        for ks in streams {
            let found = {
                let outs: Vec<Option<&SingleOut>> = ks
                    .iter()
                    .map(|&k| match slots[k].as_ref().map(|r| &r.result) {
                        Some(Ok(done)) => done.5.singles_out.as_ref(),
                        _ => None,
                    })
                    .collect();
                analyse(&outs)
            };
            for &p in found.refill_assumed.keys() {
                if !refill_counted.insert(ks[p]) {
                    continue;
                }
                if let Some(Ok(done)) = slots[ks[p]].as_mut().map(|r| &mut r.result) {
                    flag_all(done, Assumption::SingleInstanceRefill);
                    report.refill_assumed += 1;
                }
            }
            let fresh: BTreeSet<usize> = found
                .dependent
                .keys()
                .copied()
                .filter(|&p| counted.insert(ks[p]))
                .collect();
            if fresh.is_empty() {
                continue;
            }
            report.sessions += 1;
            report.dependent += fresh.len();
            // Amendment 4, decision 3: every dependent codeunit is flagged, whether its parallel run is kept or not.
            for &p in &fresh {
                report.notes.push(format!(
                    "test codeunit {} {}",
                    self.name(ks[p]),
                    self.dependent_why(&ks, &found, p)
                ));
                if let Some(Ok(done)) = slots[ks[p]].as_mut().map(|r| &mut r.result) {
                    flag_all(done, Assumption::SingleInstanceOrder);
                }
            }
            // Decision 2: only a dependent codeunit with a failure or a test refused while it ran in parallel runs again
            // serially; `--strict-order` (decision 4) runs them all again.
            let chosen: BTreeSet<usize> = fresh
                .iter()
                .copied()
                .filter(|&p| self.strict || needs_serial_run(slots[ks[p]].as_ref()))
                .collect();
            report.rerun += chosen.len();
            for (positions, reported) in found.sessions(&chosen) {
                let order: Vec<usize> = positions.iter().map(|&p| ks[p]).collect();
                chains.push(Chain {
                    steps: self.plan(&order, starts),
                    chosen: reported.iter().map(|&p| ks[p]).collect(),
                    order,
                });
            }
        }
        chains
    }

    /// Runs the serial sessions in parallel (amendment 5): on worker threads and this one; a session without steps
    /// only on this thread. The results come back in the order of `chains`, whichever thread ran them.
    fn run_chains(&self, chains: &[Chain], starts: &[Start]) -> Vec<ChainRuns> {
        let mut out: Vec<Option<ChainRuns>> = (0..chains.len()).map(|_| None).collect();
        let free: Vec<usize> = (0..chains.len()).filter(|&i| chains[i].steps.is_some()).collect();
        let threads = if self.workers > 1 && (self.baseline.is_none() || self.install_copies) {
            self.workers
        } else {
            1
        };
        let helpers = threads.min(free.len()).saturating_sub(1);
        if helpers > 0 {
            self.snapshot
                .get_or_init(|| self.baseline.and_then(al_vm::Baseline::to_bytes));
        }
        let snapshot: Option<&[u8]> = self.snapshot.get().and_then(|s| s.as_deref());
        let next = std::sync::atomic::AtomicUsize::new(0);
        let worker = Worker {
            engine: self.engine,
            lowered: self.lowered,
            units: self.units,
            limit: self.limit,
            starts,
        };
        let pull = |install: Option<&al_vm::Baseline>| {
            let mut copies = BTreeMap::new();
            let mut done = Vec::new();
            loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(&c) = free.get(i) else { return done };
                let steps = chains[c].steps.as_deref().unwrap_or_default();
                let mut at = steps.iter();
                let runs = run_in_order(&chains[c].order, &mut |_| {
                    worker.run(install, &mut copies, at.next().expect("a step per codeunit"))
                });
                done.push((c, runs));
            }
        };
        let thread_baseline = self.thread_baseline;
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..helpers {
                let spawned = std::thread::Builder::new()
                    .stack_size(TEST_STACK)
                    .spawn_scoped(scope, || {
                        let own = thread_baseline(snapshot);
                        pull(own.as_ref())
                    });
                match spawned {
                    Ok(h) => handles.push(h),
                    // No room for another thread (an address space limit): fewer will do.
                    Err(_) => break,
                }
            }
            for (i, c) in chains.iter().enumerate().filter(|(_, c)| c.steps.is_none()) {
                out[i] = Some(run_in_order(&c.order, &mut |k| self.run_bound(k)));
            }
            for (c, runs) in pull(self.baseline) {
                out[c] = Some(runs);
            }
            for h in handles {
                for (c, runs) in h.join().expect("a lane worker panicked") {
                    out[c] = Some(runs);
                }
            }
        });
        out.into_iter().map(|r| r.expect("every serial session ran")).collect()
    }

    /// Where each codeunit of a session starts ([`Step`]); `None` when a carried baseline it needs has no bytes.
    fn plan(&self, order: &[usize], starts: &mut Starts) -> Option<Vec<Step>> {
        order
            .iter()
            .map(|&k| {
                let Some(ci) = self.earlier_carry_index(k) else {
                    return Some(Step {
                        k,
                        from: None,
                        again: None,
                    });
                };
                let c = &self.carried[ci].3;
                let start = match (&c.unfinished, &c.storage_why, &c.baseline) {
                    (None, None, Some(b)) => Some(starts.of(ci, b)?),
                    _ => None,
                };
                if self.is_member(k) {
                    return Some(Step {
                        k,
                        from: start,
                        again: None,
                    });
                }
                let storage =
                    c.everything && super::corpus::reaches_session_storage(self.engine, self.lowered, self.units[k].oi);
                Some(Step {
                    k,
                    from: None,
                    again: Some((storage, start)),
                })
            })
            .collect()
    }

    /// The index of the last chain member's carry (RequiredTestIsolation = Disabled) that codeunit `k` comes after.
    fn earlier_carry_index(&self, k: usize) -> Option<usize> {
        let u = &self.units[k];
        self.carried
            .iter()
            .rposition(|(si, id, leg, _)| (si, id) != (&u.si, &u.id) && (self.sees)(*si, *id, *leg, u))
    }

    /// The last chain member's carry that codeunit `k` comes after, if any.
    fn earlier_carry(&self, k: usize) -> Option<&ChainCarry> {
        self.earlier_carry_index(k).map(|i| &self.carried[i])
    }

    /// Runs codeunit `k` of a session bound to this thread. `None`: it keeps its first run.
    fn run_bound(&self, k: usize) -> Option<UnitResult> {
        let mut u = self.units[k].clone();
        u.watch = false;
        let earlier = self.earlier_carry(k);
        let base = match earlier {
            Some((_, _, _, c)) if self.is_member(k) && c.unfinished.is_none() && c.storage_why.is_none() => {
                c.baseline.as_ref().or(self.baseline)
            }
            _ => self.baseline,
        };
        let mut r = run_unit(self.engine, self.lowered, base, &u, None, self.limit);
        // A codeunit that is no chain member runs from the install baseline, and runs again from what an earlier
        // member left when it read a table that member wrote or reaches session storage (`corpus.rs`, "run as
        // before"). The lane does the same, or a codeunit in it loses the rows it reads (corpus Y35OnRunStorage.al:453,
        // C302_B2_TestWritesStorage: a default test handler that writes to a SingleInstance log for every test
        // codeunit puts every codeunit in the lane). A case `corpus.rs` refuses keeps its first run.
        if let (false, Some((_, _, _, c))) = (self.is_member(k), earlier) {
            let storage = c.everything && super::corpus::reaches_session_storage(self.engine, self.lowered, u.oi);
            let hit = storage || r.result.as_ref().is_ok_and(|done| done.5.leak_hit);
            if hit {
                match (&c.unfinished, &c.storage_why, c.baseline.as_ref()) {
                    (None, None, Some(from)) => {
                        super::run::CARRIED.with(|c| c.borrow_mut().take());
                        r = run_unit(self.engine, self.lowered, Some(from), &u, None, self.limit);
                    }
                    _ => return None,
                }
            }
        }
        Some(r)
    }

    fn name(&self, k: usize) -> String {
        format!(
            "{} {}",
            self.units[k].id, self.lowered.program.objects[self.units[k].oi].name
        )
    }

    fn is_member(&self, k: usize) -> bool {
        let u = &self.units[k];
        u.carry.is_some() || u.isolation == TestIsolation::Disabled
    }

    /// What the dependent codeunit at position `p` relies on, for its note.
    fn dependent_why(&self, ks: &[usize], found: &Found, p: usize) -> String {
        let (w, (o, g)) = found.dependent[&p];
        let obj = &self.lowered.program.objects[o];
        format!(
            "relies on SingleInstance state (variable {} of codeunit {}) that test codeunit {} left (order-dependent test; fix the test)",
            obj.globals.get(g).map_or("?", |s| s.name.as_str()),
            obj.name,
            self.name(ks[w])
        )
    }
}

/// Marks every test of a codeunit's run with `a`.
fn flag_all(done: &mut UnitOutcomes, a: Assumption) {
    for (test, _) in &done.0 {
        match done.3.iter_mut().find(|(t, _)| t == test) {
            Some((_, hits)) => {
                if !hits.contains(&a) {
                    hits.push(a);
                    hits.sort();
                }
            }
            None => done.3.push((test.clone(), vec![a])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_vm::single_track::Fp;

    fn fp(b: u8) -> Fp {
        Fp::Val(vec![b].into())
    }

    fn out(observed: &[(GlobalKey, u8)], ends: &[(GlobalKey, u8)], wrote: &[GlobalKey]) -> SingleOut {
        SingleOut {
            observed: observed.iter().map(|&(k, v)| (k, fp(v))).collect(),
            ends: ends.iter().map(|&(k, v)| (k, fp(v))).collect(),
            wrote: wrote.iter().copied().collect(),
            ..Default::default()
        }
    }

    #[test]
    fn only_the_last_writer_of_a_global_joins_the_lane() {
        let flag = (1, 0);
        // Two codeunits set the flag one after the other; the reader needs only the state the last one left.
        let a = out(&[], &[(flag, 7)], &[flag]);
        let b = out(&[], &[(flag, 8)], &[flag]);
        let c = out(&[(flag, 0)], &[(flag, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        assert_eq!(found.dependent, BTreeMap::from([(2, (1, flag))]));
        assert_eq!(found.lane, BTreeSet::from([1, 2]));
    }

    #[test]
    fn a_dependent_writer_brings_its_own_writer() {
        let (k1, k2) = ((1, 0), (2, 0));
        let a = out(&[], &[(k2, 3)], &[k2]);
        // b read k2 (that a set) and set k1; c reads k1.
        let b = out(&[(k2, 0)], &[(k2, 0), (k1, 4)], &[k1]);
        let c = out(&[(k1, 0)], &[(k1, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        assert_eq!(found.dependent.keys().copied().collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(found.lane, BTreeSet::from([0, 1, 2]));
    }

    /// ADR-066 amendment 4: the lane of one chosen dependent codeunit holds what it needs and nothing of the others.
    #[test]
    fn the_lane_of_chosen_dependents_holds_only_their_chain() {
        let (k1, k2) = ((1, 0), (2, 0));
        let a = out(&[], &[(k2, 3)], &[k2]);
        let b = out(&[(k2, 0)], &[(k2, 0), (k1, 4)], &[k1]);
        let c = out(&[(k1, 0)], &[(k1, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        // Only b chosen: b and its writer a. Only c chosen: c, its writer b (dependent itself) and b's writer a.
        assert_eq!(found.lane_for(&BTreeSet::from([1])), BTreeSet::from([0, 1]));
        assert_eq!(found.lane_for(&BTreeSet::from([2])), BTreeSet::from([0, 1, 2]));
        assert_eq!(found.lane_for(&BTreeSet::from([1, 2])), found.lane);
        assert!(found.lane_for(&BTreeSet::new()).is_empty());
    }

    fn sessions(found: &Found, chosen: &[usize]) -> Vec<(Vec<usize>, Vec<usize>)> {
        found
            .sessions(&chosen.iter().copied().collect())
            .into_iter()
            .map(|(order, reported)| (order, reported.into_iter().collect()))
            .collect()
    }

    /// ADR-066 amendment 5: each chosen dependent codeunit's chain is a serial session of its own, also when two chains
    /// share a writer (the writer runs in each): a codeunit sees no state from outside its chain.
    #[test]
    fn every_chain_is_a_session_of_its_own_and_a_shared_writer_runs_in_each() {
        let (f1, f2) = ((1, 0), (2, 0));
        let a = out(&[], &[(f1, 3)], &[f1]);
        let b = out(&[], &[(f2, 4)], &[f2]);
        let c = out(&[(f1, 0)], &[(f1, 0)], &[]);
        let d = out(&[(f2, 0)], &[(f2, 0)], &[]);
        let e = out(&[(f1, 0)], &[(f1, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c), Some(&d), Some(&e)]);
        assert_eq!(found.dependent.keys().copied().collect::<Vec<_>>(), vec![2, 3, 4]);
        assert_eq!(
            sessions(&found, &[2, 3]),
            vec![(vec![0, 2], vec![2]), (vec![1, 3], vec![3])]
        );
        // c and e share their writer a: two sessions, a runs in both; e does not see what c left.
        assert_eq!(
            sessions(&found, &[2, 3, 4]),
            vec![(vec![0, 2], vec![2]), (vec![0, 4], vec![4]), (vec![1, 3], vec![3])]
        );
        assert!(sessions(&found, &[]).is_empty());
    }

    /// A dependent codeunit whose chain is the start of a later chosen codeunit's chain is reported from that session:
    /// it starts there with the very state its own session would give it.
    #[test]
    fn a_chain_that_starts_another_runs_inside_it() {
        let (k1, k2) = ((1, 0), (2, 0));
        let a = out(&[], &[(k2, 3)], &[k2]);
        let b = out(&[(k2, 0)], &[(k2, 0), (k1, 4)], &[k1]);
        let c = out(&[(k1, 0)], &[(k1, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        assert_eq!(sessions(&found, &[1, 2]), vec![(vec![0, 1, 2], vec![1, 2])]);
        // A codeunit of the later chain comes before it and is not in its chain: it keeps a session of its own.
        let (f1, f2) = ((1, 0), (2, 0));
        let w = out(&[], &[(f1, 3)], &[f1]);
        let x = out(&[], &[(f2, 4)], &[f2]);
        let y = out(&[(f1, 0)], &[(f1, 0)], &[]);
        let z = out(&[(f1, 0), (f2, 0)], &[(f1, 0), (f2, 0)], &[]);
        let found = analyse(&[Some(&w), Some(&x), Some(&y), Some(&z)]);
        assert_eq!(
            sessions(&found, &[2, 3]),
            vec![(vec![0, 1, 3], vec![3]), (vec![0, 2], vec![2])]
        );
    }

    #[test]
    fn a_writer_that_is_not_dependent_brings_no_earlier_writer() {
        let (k1, k2) = ((1, 0), (2, 0));
        let a = out(&[], &[(k2, 3)], &[k2]);
        // b wrote k2 itself before it read it, and set k1; it does not depend on a.
        let b = out(&[], &[(k2, 5), (k1, 4)], &[k1, k2]);
        let c = out(&[(k1, 0)], &[(k1, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        assert_eq!(found.lane, BTreeSet::from([1, 2]));
    }

    #[test]
    fn a_refused_writer_is_no_change() {
        let flag = (1, 0);
        // The writer set the flag, then a test of it was refused before it could reset the flag (as BC's run would).
        let mut a = out(&[], &[(flag, 7)], &[flag]);
        a.refused = true;
        let b = out(&[(flag, 0)], &[(flag, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b)]);
        assert!(found.dependent.is_empty() && found.lane.is_empty(), "{found:?}");
    }

    #[test]
    fn a_writer_after_a_refused_one_counts_again() {
        let flag = (1, 0);
        let mut a = out(&[], &[(flag, 7)], &[flag]);
        a.refused = true;
        let b = out(&[(flag, 0)], &[(flag, 5)], &[flag]);
        let c = out(&[(flag, 0)], &[(flag, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        assert_eq!(found.dependent, BTreeMap::from([(2, (1, flag))]));
        assert_eq!(found.lane, BTreeSet::from([1, 2]));
    }

    #[test]
    fn a_guard_then_refill_that_left_the_earlier_value_is_no_link() {
        let cache = (1, 0);
        // A keyed cache: a filled it with key 3; b's guard saw it empty and refilled it with key 3, then moved to key 4.
        let a = out(&[], &[(cache, 3)], &[cache]);
        let mut b = out(&[(cache, 0)], &[(cache, 4)], &[cache]);
        b.guarded.insert(cache);
        b.refill.insert(cache, fp(3));
        let found = analyse(&[Some(&a), Some(&b)]);
        assert!(found.dependent.is_empty() && found.lane.is_empty(), "{found:?}");
        assert!(
            found.refill_assumed.is_empty(),
            "the same value either way: no assumption"
        );
    }

    #[test]
    fn a_guard_then_refill_that_also_left_another_global_unlike_the_earlier_codeunit_is_a_dependence() {
        let (key, cache) = ((1, 0), (1, 1));
        // b's guard refilled the key the earlier codeunit left, then filled the cache with another value: with the
        // earlier key BC would have skipped the fill and kept a's cache.
        let a = out(&[], &[(key, 3), (cache, 30)], &[key, cache]);
        let mut b = out(&[(key, 0)], &[(key, 3), (cache, 31)], &[key, cache]);
        b.guarded.insert(key);
        b.refill.insert(key, fp(3));
        let found = analyse(&[Some(&a), Some(&b)]);
        assert_eq!(found.dependent, BTreeMap::from([(1, (0, cache))]));
        assert!(found.refill_assumed.is_empty());
    }

    #[test]
    fn a_lazy_flag_does_not_tie_in_what_the_codeunit_wrote_in_another_single_instance() {
        // The flag of codeunit 1 ends as the earlier codeunit left it; codeunit 2's dictionary (Application Area Cache,
        // filled with this codeunit's own keys) is no cache the flag guards.
        let (flag, dict) = ((1, 0), (2, 0));
        let a = out(&[], &[(flag, 1), (dict, 3)], &[flag, dict]);
        let b = out(&[(flag, 0)], &[(flag, 1), (dict, 4)], &[flag, dict]);
        assert!(analyse(&[Some(&a), Some(&b)]).dependent.is_empty());
    }

    #[test]
    fn a_refilled_time_stamp_does_not_make_a_lazy_cache_dependent() {
        // Get Shortcut Dimension Values.GetGLSetup (Base Application, BC 29): `if WhenGotGLSetup = 0DT then
        // WhenGotGLSetup := CurrentDateTime`, then the flag and the cache. The flag and the cache end as the earlier
        // codeunit left them; the time stamp never does.
        let (when, has, cache) = ((1, 0), (1, 1), (1, 2));
        let a = out(&[], &[(when, 10), (has, 1), (cache, 5)], &[when, has, cache]);
        let mut b = out(
            &[(when, 0), (has, 0)],
            &[(when, 11), (has, 1), (cache, 5)],
            &[when, has, cache],
        );
        b.guarded.insert(when);
        b.refill.insert(when, fp(11));
        let found = analyse(&[Some(&a), Some(&b)]);
        assert!(found.dependent.is_empty(), "{found:?}");
        assert_eq!(found.refill_assumed, BTreeMap::from([(1, (0, when))]));
    }

    #[test]
    fn a_guard_then_refill_with_another_value_is_no_link_under_the_assumption() {
        let cache = (1, 0);
        let a = out(&[], &[(cache, 3)], &[cache]);
        let mut b = out(&[(cache, 0)], &[(cache, 4)], &[cache]);
        b.guarded.insert(cache);
        b.refill.insert(cache, fp(4));
        let found = analyse(&[Some(&a), Some(&b)]);
        assert!(found.dependent.is_empty() && found.lane.is_empty(), "{found:?}");
        assert_eq!(found.refill_assumed, BTreeMap::from([(1, (0, cache))]));
    }

    #[test]
    fn a_reader_of_a_value_an_earlier_codeunit_set_is_dependent() {
        let (flag, other) = ((1, 0), (2, 0));
        let a = out(&[], &[(flag, 7)], &[flag]);
        let b = out(&[(flag, 0)], &[(flag, 0)], &[]);
        let c = out(&[(other, 0)], &[(other, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        assert_eq!(found.dependent, BTreeMap::from([(1, (0, flag))]));
        assert_eq!(found.lane, BTreeSet::from([0, 1]));
    }

    #[test]
    fn a_cache_the_reader_filled_with_the_same_value_is_no_dependence() {
        let (done, cache) = ((1, 0), (1, 1));
        // The first codeunit fills the cache; the second saw it empty, filled it with the same value.
        let a = out(&[(done, 0)], &[(done, 1), (cache, 9)], &[done, cache]);
        let same = out(&[(done, 0)], &[(done, 1), (cache, 9)], &[done, cache]);
        assert!(analyse(&[Some(&a), Some(&same)]).dependent.is_empty());
        let other = out(&[(done, 0)], &[(done, 1), (cache, 5)], &[done, cache]);
        assert_eq!(
            analyse(&[Some(&a), Some(&other)]).dependent,
            BTreeMap::from([(1, (0, cache))])
        );
    }

    #[test]
    fn equal_values_and_unobserved_codeunits_make_no_lane() {
        let flag = (1, 0);
        let a = out(&[], &[(flag, 0)], &[flag]);
        let b = out(&[(flag, 0)], &[(flag, 0)], &[]);
        let found = analyse(&[Some(&a), None, Some(&b)]);
        assert_eq!(found, Found::default());
    }

    #[test]
    fn a_value_that_cannot_be_compared_counts_as_different_unless_it_is_a_lazy_cache_of_the_same_kind() {
        let g = (1, 0);
        let opaque = |what: &str| Fp::Opaque(what.into());
        let one = |observed: Fp, end: Fp| SingleOut {
            observed: BTreeMap::from([(g, observed)]),
            ends: BTreeMap::from([(g, end)]),
            wrote: BTreeSet::from([g]),
            ..Default::default()
        };
        let first = SingleOut {
            ends: BTreeMap::from([(g, opaque("a DotNet object"))]),
            wrote: BTreeSet::from([g]),
            ..Default::default()
        };
        // The reader saw it empty and ended with an object of the same kind: its own fill.
        let refilled = one(opaque("nothing"), opaque("a DotNet object"));
        assert!(analyse(&[Some(&first), Some(&refilled)]).dependent.is_empty());
        // It ended with another kind of value: a dependence.
        let other = one(opaque("nothing"), opaque("a page"));
        assert_eq!(analyse(&[Some(&first), Some(&other)]).dependent.len(), 1);
    }
}
