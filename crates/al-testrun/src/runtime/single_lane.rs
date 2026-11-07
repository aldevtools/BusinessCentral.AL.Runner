//! SingleInstance state across the test codeunits of a run (ADR-066 amendment 2, al-runner issue 6924).
//!
//! BC runs the test codeunits of a run in one session, so the globals of a SingleInstance codeunit that one codeunit
//! left are what the next one starts with (al-language/codeunits/TestCuP7BReader.al:24-37, against
//! TestCuP7ASetter.al:24-58). al-runner runs the codeunits in parallel, each on a session of its own, and copies no
//! state from one to the next. Instead it finds the codeunits that depend on such state and runs only those again,
//! with the codeunits that wrote it, in one serial session in BC's order, the single instances moved from codeunit to
//! codeunit. The tests of a dependent codeunit are flagged (`Assumption::SingleInstanceOrder`).
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
//!    failed or not-run test, or every one under `--strict-order`, runs again in the lane with the writers it needs
//!    ([`Found::lane_for`]), and that run is reported.

use super::chain_walk::Carry as ChainCarry;
use super::*;
use al_vm::single_track::{GlobalKey, SingleOut};
use std::collections::{BTreeMap, BTreeSet};

/// The BC sessions of a run: the units of each (suite, hub leg) in BC's run order.
fn streams(units: &[Unit], place: &dyn Fn(usize, u32) -> u32, has: &dyn Fn(usize) -> bool) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = (0..units.len()).filter(|&k| !units[k].execute && has(k)).collect();
    order.sort_by_key(|&k| {
        let u = &units[k];
        (u.si, u.hub_leg, place(u.si, u.id), k)
    });
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut last = None;
    for k in order {
        let key = (units[k].si, units[k].hub_leg);
        if last == Some(key) {
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
pub(super) fn assign_watches(units: &mut [Unit], place: &dyn Fn(usize, u32) -> u32) -> usize {
    let mut n = 0;
    for ks in streams(units, place, &|_| true) {
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
}

/// Whether a codeunit's parallel run has a test that failed or was not run (ADR-066 amendment 4, decision 2): only such
/// a dependent codeunit runs again serially, unless `--strict-order`. A run that panicked has no notes and is never
/// dependent.
fn needs_serial_run(slot: Option<&UnitResult>) -> bool {
    match slot.map(|r| &r.result) {
        Some(Ok(done)) => done.0.iter().any(|(_, o)| !matches!(o, Outcome::Pass)),
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
}

/// What the lane did, for `--timings` and the stability report.
#[derive(Debug, Clone, Default)]
pub struct LaneReport {
    /// Test codeunits that were watched.
    pub watched: usize,
    /// BC sessions with at least one dependent codeunit.
    pub sessions: usize,
    /// The codeunits that ran again in the lane (dependent ones and the writers they need), with the reason.
    pub lane: Vec<(u32, String)>,
    /// One line per dependent codeunit: what it relies on.
    pub notes: Vec<String>,
    /// Codeunits taken as independent under `Assumption::SingleInstanceRefill` (a guard, then the refill of a global
    /// an earlier codeunit left with another value): their tests are flagged, they do not run again.
    pub refill_assumed: usize,
    /// The time the lane took (all its BC sessions, one after the other on one thread).
    pub took: std::time::Duration,
    /// Dependent codeunits found (ADR-066 amendment 4): each one's tests are flagged.
    pub dependent: usize,
    /// Dependent codeunits that ran again serially because a test of theirs failed or was not run in the parallel run
    /// (amendment 4). With `strict`, every dependent codeunit runs again and this counts them all.
    pub rerun: usize,
    /// `--strict-order`: every dependent codeunit ran again serially, as amendment 3 did.
    pub strict: bool,
}

impl LaneReport {
    pub fn lines(&self) -> Vec<String> {
        if self.watched == 0 {
            return Vec::new();
        }
        let why = if self.strict {
            "(strict order: every dependent codeunit)"
        } else {
            "because a test failed or was not run in parallel"
        };
        let mut out = vec![format!(
            "SingleInstance order: {} test codeunits watched, {} BC sessions with a dependent codeunit, {} dependent codeunits (flagged), {} ran again serially {why}, {} codeunits ran in the lane ({:.2} s); {} codeunits flagged as independent under the guard-then-refill assumption",
            self.watched,
            self.sessions,
            self.dependent,
            self.rerun,
            self.lane.len(),
            self.took.as_secs_f64(),
            self.refill_assumed
        )];
        out.extend(self.notes.iter().cloned());
        out
    }
}

impl Lane<'_> {
    /// Step 3: runs the lane of each BC session that has a dependent codeunit and puts the dependent codeunits'
    /// results (flagged) in `slots`.
    pub(super) fn run(&self, slots: &mut [Option<UnitResult>], report: &mut LaneReport) {
        if report.watched == 0 {
            return;
        }
        let started = std::time::Instant::now();
        report.strict = self.strict;
        let has = |k: usize| slots[k].is_some();
        let streams = streams(self.units, self.place, &has);
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
                if let Some(Ok(done)) = slots[ks[p]].as_mut().map(|r| &mut r.result) {
                    flag_all(done, Assumption::SingleInstanceRefill);
                    report.refill_assumed += 1;
                }
            }
            if found.dependent.is_empty() {
                continue;
            }
            report.sessions += 1;
            report.dependent += found.dependent.len();
            // Amendment 4, decision 3: every dependent codeunit is flagged, whether its parallel run is kept or not.
            for &p in found.dependent.keys() {
                report.notes.push(format!(
                    "test codeunit {} {}",
                    self.name(ks[p]),
                    self.dependent_why(&ks, &found, p)
                ));
                if let Some(Ok(done)) = slots[ks[p]].as_mut().map(|r| &mut r.result) {
                    flag_all(done, Assumption::SingleInstanceOrder);
                }
            }
            // Decision 2: only a dependent codeunit with a failure or a test not run in parallel runs again serially;
            // `--strict-order` (decision 4) runs them all again.
            let chosen: BTreeSet<usize> = found
                .dependent
                .keys()
                .copied()
                .filter(|&p| self.strict || needs_serial_run(slots[ks[p]].as_ref()))
                .collect();
            if chosen.is_empty() {
                continue;
            }
            report.rerun += chosen.len();
            self.run_stream(&ks, &found, &chosen, slots, report);
        }
        report.took += started.elapsed();
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

    /// Runs the lane of the dependent codeunits `chosen` of one BC session and puts their serial runs in `slots`.
    fn run_stream(
        &self,
        ks: &[usize],
        found: &Found,
        chosen: &BTreeSet<usize>,
        slots: &mut [Option<UnitResult>],
        report: &mut LaneReport,
    ) {
        super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = Some(None));
        for &p in &found.lane_for(chosen) {
            let k = ks[p];
            let mut u = self.units[k].clone();
            u.watch = false;
            let before = if self.is_member(k) {
                self.carried
                    .iter()
                    .rev()
                    .find(|(si, id, leg, _)| (si, id) != (&u.si, &u.id) && (self.sees)(*si, *id, *leg, &u))
            } else {
                None
            };
            let base = match before {
                Some((_, _, _, c)) if c.unfinished.is_none() && c.storage_why.is_none() => {
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
            if !self.is_member(k) {
                let earlier = self
                    .carried
                    .iter()
                    .rev()
                    .find(|(si, id, leg, _)| (si, id) != (&u.si, &u.id) && (self.sees)(*si, *id, *leg, &u));
                if let Some((_, _, _, c)) = earlier {
                    let storage =
                        c.everything && super::corpus::reaches_session_storage(self.engine, self.lowered, u.oi);
                    let hit = storage || r.result.as_ref().is_ok_and(|done| done.5.leak_hit);
                    if hit {
                        match (&c.unfinished, &c.storage_why, c.baseline.as_ref()) {
                            (None, None, Some(from)) => {
                                super::run::CARRIED.with(|c| c.borrow_mut().take());
                                r = run_unit(self.engine, self.lowered, Some(from), &u, None, self.limit);
                            }
                            _ => continue,
                        }
                    }
                }
            }
            super::run::CARRIED.with(|c| c.borrow_mut().take());
            report.lane.push((u.id, self.name(k)));
            if chosen.contains(&p) {
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
            // A writer that does not depend keeps its first run: it was complete without the lane. So does a dependent
            // writer that was not chosen (amendment 4: it passed in parallel, flagged). A chain member that depends hands
            // the later codeunits the rows of its first run (what its run in the lane left is dropped).
        }
        super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = None);
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
