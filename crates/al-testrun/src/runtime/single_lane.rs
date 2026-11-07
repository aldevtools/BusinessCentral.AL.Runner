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
//!    itself: it ended with the very values the earlier writer left. The lane is the dependent codeunits and, until
//!    nothing is added, the earlier codeunits that wrote a global one of them read.

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
}

/// Step 3, the decision: from what each codeunit of a BC session (in order) noted, which depend on state an earlier one
/// left. `None`: no notes (the codeunit was not watched, or it did not run).
pub(super) fn analyse(outs: &[Option<&SingleOut>]) -> Found {
    let mut state: BTreeMap<GlobalKey, (al_vm::single_track::Fp, usize)> = BTreeMap::new();
    let mut found = Found::default();
    // The writes that changed a global: the first, and each that left another value than the one before it.
    let mut effective: BTreeMap<GlobalKey, Vec<usize>> = BTreeMap::new();
    for (p, out) in outs.iter().enumerate() {
        let Some(o) = out else { continue };
        let mut diff: Vec<(GlobalKey, usize)> = Vec::new();
        for (k, seen) in &o.observed {
            if let Some((fp, w)) = state.get(k) {
                if fp != seen {
                    diff.push((*k, *w));
                }
            }
        }
        if !diff.is_empty() {
            // A lazy cache the codeunit filled itself with the very values the earlier codeunit left is no
            // dependence: whether it skipped the fill or did it, the state and what it reads are the same.
            let same_end = |k: &GlobalKey| match state.get(k) {
                Some((fp, _)) => o.ends.get(k).is_some_and(|e| e == fp || e.same_kind(fp)),
                None => true,
            };
            // The global that gives the dependence away: the first one the codeunit did not end with as it was left.
            let culprit = diff
                .iter()
                .map(|(k, _)| k)
                .chain(o.wrote.iter())
                .find(|k| !same_end(k))
                .and_then(|k| state.get(k).map(|(_, w)| (*w, *k)));
            if let Some(c) = culprit {
                found.dependent.insert(p, c);
            }
        }
        for k in &o.wrote {
            if let Some(end) = o.ends.get(k) {
                let before = state.insert(*k, (end.clone(), p));
                if before.is_none_or(|(b, _)| !(b == *end || b.same_kind(end))) {
                    effective.entry(*k).or_default().push(p);
                }
            }
        }
    }
    // The lane: the dependent codeunits and the earlier writes that changed a global one of its members read first. A
    // write that left the value as it was is no change: leaving it out gives the members the same start.
    let mut lane: BTreeSet<usize> = found.dependent.keys().copied().collect();
    let mut work: Vec<usize> = lane.iter().copied().collect();
    while let Some(m) = work.pop() {
        let Some(o) = outs[m] else { continue };
        for k in o.observed.keys() {
            for &j in effective
                .get(k)
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .filter(|&&j| j < m)
            {
                if lane.insert(j) {
                    work.push(j);
                }
            }
        }
    }
    found.lane = lane;
    found
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
}

impl LaneReport {
    pub fn lines(&self) -> Vec<String> {
        if self.watched == 0 {
            return Vec::new();
        }
        let mut out = vec![format!(
            "SingleInstance order: {} test codeunits watched, {} BC sessions with a dependent codeunit, {} codeunits ran in the lane ({} dependent)",
            self.watched,
            self.sessions,
            self.lane.len(),
            self.notes.len()
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
            if found.dependent.is_empty() {
                continue;
            }
            report.sessions += 1;
            self.run_stream(&ks, &found, slots, report);
        }
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

    fn run_stream(&self, ks: &[usize], found: &Found, slots: &mut [Option<UnitResult>], report: &mut LaneReport) {
        let dependent_why = |p: usize| {
            let (o, g) = found.dependent[&p].1;
            let obj = &self.lowered.program.objects[o];
            format!(
                "relies on SingleInstance state (variable {} of codeunit {}) that test codeunit {} left (order-dependent test; fix the test)",
                obj.globals.get(g).map_or("?", |s| s.name.as_str()),
                obj.name,
                self.name(ks[found.dependent[&p].0])
            )
        };
        super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = Some(None));
        for &p in &found.lane {
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
            if found.dependent.contains_key(&p) {
                let why = dependent_why(p);
                report.notes.push(format!("test codeunit {} {why}", self.name(k)));
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
                    for (test, _) in &done.0 {
                        match done.3.iter_mut().find(|(t, _)| t == test) {
                            Some((_, hits)) => {
                                if !hits.contains(&Assumption::SingleInstanceOrder) {
                                    hits.push(Assumption::SingleInstanceOrder);
                                    hits.sort();
                                }
                            }
                            None => done.3.push((test.clone(), vec![Assumption::SingleInstanceOrder])),
                        }
                    }
                }
                slots[k] = Some(r);
            }
            // A writer that does not depend keeps its first run: it was complete without the lane. A chain member that
            // depends hands the later codeunits the rows of its first run (what its run in the lane left is dropped).
        }
        super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = None);
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
        }
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
