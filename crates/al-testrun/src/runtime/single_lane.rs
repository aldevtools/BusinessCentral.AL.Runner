//! SingleInstance state across the test codeunits of a run, in the default isolated mode (ADR-070, al-runner issue 6924).
//!
//! BC runs the test codeunits of a run in one session, so the globals of a SingleInstance codeunit that one codeunit
//! left are what the next one starts with (al-language/codeunits/TestCuP7BReader.al:24-37, against
//! TestCuP7ASetter.al:24-58). The default mode runs every test codeunit with fresh single instances, in parallel, and
//! runs nothing again; `--strict-order` runs BC's order (`strict_walk.rs`). This module only tells the user where the
//! default deviates:
//!
//! 1. [`assign_watches`]: in every BC session that has two test codeunits or more, a session notes the single
//!    instances (with globals) it makes.
//! 2. The parallel run notes, for each watched codeunit, the globals read first and the end values
//!    (`al_vm::single_track`); [`analyse`] finds the codeunits that read a global an earlier codeunit of the session
//!    would have left with another value.
//! 3. [`Lane::run`] notes each such codeunit for `--timings` and the stability report, and gives each failed test of
//!    it that read such a global an order hint (`order_hint.rs`). No outcome changes.
//!
//! ADR-066 amendment 6: with `--test-suites` (`RunOptions::test_suites`) each listed suite is a BC session of its own,
//! in the file's order, and a codeunit in no suite is a session of its own ([`streams`]); without it the whole run is
//! one.

use super::order_hint::STRICT_FLAG;
use super::*;
use al_vm::single_track::{GlobalKey, SingleOut};
use std::collections::{BTreeMap, BTreeSet};

/// The BC sessions of a run: the units of each (suite, hub leg) in BC's run order.
///
/// ADR-066 amendment 6 (al-runner issue 6924): with `--test-suites` each listed suite is a BC session of its own (per
/// suite and hub leg), its codeunits in the file's order; a codeunit listed in two suites is in both sessions, and one
/// in no suite is a session of its own (no earlier codeunit, so no link). Without it, the whole run is one session.
pub(super) fn streams(
    units: &[Unit],
    place: &dyn Fn(usize, u32) -> u32,
    has: &dyn Fn(usize) -> bool,
    suites: Option<&crate::test_suites::TestSuites>,
) -> Vec<Vec<usize>> {
    keyed_streams(units, place, has, suites)
        .into_iter()
        .map(|s| s.members.into_iter().map(|(_, k)| k).collect())
        .collect()
}

/// One BC session of [`keyed_streams`].
pub(super) struct Stream {
    /// The suite of `--test-suites` it is a line of (`usize::MAX`: a codeunit in no suite, a session of its own, or no
    /// suites file), the app suite (`Unit::si`) and whether it is the hub leg.
    pub suite: usize,
    pub si: usize,
    pub hub_leg: bool,
    /// Its units in BC's run order, each with its place in the session (the id's place, or the position in the file).
    pub members: Vec<(u64, usize)>,
}

/// [`streams`] with each session's key and the place of each member.
pub(super) fn keyed_streams(
    units: &[Unit],
    place: &dyn Fn(usize, u32) -> u32,
    has: &dyn Fn(usize) -> bool,
    suites: Option<&crate::test_suites::TestSuites>,
) -> Vec<Stream> {
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
    let mut out: Vec<Stream> = Vec::new();
    let mut last = None;
    for (key, p, k) in order {
        if last == Some(key) && key.0 != usize::MAX {
            out.last_mut().expect("a stream").members.push((p, k));
        } else {
            out.push(Stream {
                suite: key.0,
                si: key.1,
                hub_leg: key.2,
                members: vec![(p, k)],
            });
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
    /// Per dependent codeunit (position), the globals it read first that the earlier codeunits left with another value.
    pub changed: BTreeMap<usize, BTreeSet<GlobalKey>>,
    /// Per dependent codeunit (position), each global of `changed` with the position of the codeunit that wrote it
    /// last (ADR-058 amendment 1, decision 4: the reason of a flip).
    pub last: BTreeMap<usize, Vec<(GlobalKey, usize)>>,
}

/// Step 2, the decision: from what each codeunit of a BC session (in order) noted, which read state an earlier one
/// left. `None`: no notes (the codeunit was not watched, or it did not run).
///
/// ADR-066 amendment 3: a global read first whose read was a guard directly followed by the refill (R1b) is no link
/// (the hint does not name it). What a codeunit with a test refused mid-run left is no change (R3). For each global
/// that makes a codeunit dependent, the last codeunit that wrote it is noted (R2).
pub(super) fn analyse(outs: &[Option<&SingleOut>]) -> Found {
    // Per global: what the last writer left, that writer, and whether it was refused mid-run.
    let mut state: BTreeMap<GlobalKey, (al_vm::single_track::Fp, usize, bool)> = BTreeMap::new();
    let mut found = Found::default();
    for (p, out) in outs.iter().enumerate() {
        let Some(o) = out else { continue };
        let mut diff: Vec<(GlobalKey, usize)> = Vec::new();
        // The globals read first that differ, but whose read was a guard directly followed by the refill (R1b).
        let mut guarded: Vec<(GlobalKey, usize)> = Vec::new();
        for (k, seen) in &o.observed {
            let Some((fp, w, refused)) = state.get(k) else { continue };
            if fp == seen || *refused {
                continue;
            }
            if o.guarded.contains(k) {
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
            found.changed.insert(p, diff.iter().map(|(k, _)| *k).collect());
            found.last.insert(p, diff);
        }
        for k in &o.wrote {
            if let Some(end) = o.ends.get(k) {
                state.insert(*k, (end.clone(), p, o.refused));
            }
        }
    }
    found
}

/// [`analyse`] of the BC session `ks` (in order), from what each codeunit's run in `slots` noted.
fn session_found(ks: &[usize], slots: &[Option<UnitResult>]) -> Found {
    let outs: Vec<Option<&SingleOut>> = ks
        .iter()
        .map(|&k| match slots[k].as_ref().map(|r| &r.result) {
            Some(Ok(done)) => done.5.singles_out.as_ref(),
            _ => None,
        })
        .collect();
    analyse(&outs)
}

/// What the analysis found, for `--timings` and the stability report.
#[derive(Debug, Clone, Default)]
pub struct LaneReport {
    /// Test codeunits that were watched.
    pub watched: usize,
    /// One line per dependent codeunit: what it relies on.
    pub notes: Vec<String>,
    /// Dependent codeunits found.
    pub dependent: usize,
    /// The failed tests that got an order hint.
    pub hinted: usize,
    /// ADR-070 decision 4: the passing tests that read SingleInstance state an earlier codeunit would have left
    /// differently on BC.
    pub passing_read_carried: usize,
    /// ADR-058 amendment 1: the dependent codeunits, with what they read.
    pub carried: Vec<super::CarriedCodeunit>,
}

impl LaneReport {
    /// ADR-070 decision 4: the summary line of a default run, printed with or without `--timings`, when
    /// [`passing_read_carried`](Self::passing_read_carried) is above 0.
    pub fn passing_line(&self) -> Option<String> {
        (self.passing_read_carried > 0).then(|| {
            format!(
                "{} passing tests read SingleInstance state that an earlier codeunit would have left differently on BC. \
                 {STRICT_FLAG} runs BC's order.",
                self.passing_read_carried
            )
        })
    }

    pub fn lines(&self) -> Vec<String> {
        if self.watched == 0 {
            return Vec::new();
        }
        let mut out = vec![format!(
            "SingleInstance order: each test codeunit starts with fresh SingleInstance state ({STRICT_FLAG} runs them in BC's order): {} test codeunits watched, {} read state an earlier codeunit would have left, {} failed tests got an order hint",
            self.watched, self.dependent, self.hinted
        )];
        out.extend(self.notes.iter().cloned());
        out
    }
}

/// What the analysis needs from the default run (`corpus.rs`).
pub(super) struct Lane<'a> {
    pub lowered: &'a Lowered,
    pub units: &'a [Unit],
    pub place: &'a dyn Fn(usize, u32) -> u32,
    /// `--test-suites` (amendment 6): each suite is a BC session of its own.
    pub suites: Option<&'a crate::test_suites::TestSuites>,
}

impl Lane<'_> {
    /// Step 3: finds the dependent codeunits of each BC session, notes them, and gives their failed tests the hint.
    pub(super) fn run(&self, slots: &mut [Option<UnitResult>], report: &mut LaneReport) {
        if report.watched == 0 {
            return;
        }
        let has = |k: usize| slots[k].is_some();
        // Amendment 6: a codeunit listed in two suites is in two sessions but runs and is reported once. The first
        // session (in the file's order) where it is dependent decides.
        let mut counted: BTreeSet<usize> = BTreeSet::new();
        let mut numbered: BTreeSet<usize> = BTreeSet::new();
        for ks in streams(self.units, self.place, &has, self.suites) {
            let found = session_found(&ks, slots);
            let fresh: Vec<usize> = found
                .dependent
                .keys()
                .copied()
                .filter(|&p| counted.insert(ks[p]))
                .collect();
            report.dependent += fresh.len();
            for p in fresh {
                report.notes.push(format!(
                    "test codeunit {} {}",
                    self.name(ks[p]),
                    self.dependent_why(&ks, &found, p)
                ));
                self.note_carried(&ks, &found, p, report);
                let (failed, passing) = self.hint(&ks, &found, p, slots);
                report.hinted += failed;
                report.passing_read_carried += passing;
            }
            // ADR-070 decision 3, after the SingleInstance hints: a test with both gets one hint.
            report.hinted += super::order_hint::hint_numbers(self.lowered, self.units, &ks, &mut numbered, slots);
        }
    }

    /// The hint of each failed test of dependent codeunit `p` of session `ks` that read first, in it or in a test
    /// before it, a global the earlier codeunits left otherwise; the same reads on each passing test (ADR-070 decision
    /// 4). Returns how many failed and how many passing tests got one.
    fn hint(&self, ks: &[usize], found: &Found, p: usize, slots: &mut [Option<UnitResult>]) -> (usize, usize) {
        let Some(Ok(done)) = slots[ks[p]].as_mut().map(|r| &mut r.result) else {
            return (0, 0);
        };
        let number = |t: &String, at: &[(u32, String)]| at.iter().find(|(_, name)| name == t).map(|(n, _)| *n);
        let failed: Vec<(String, Option<u32>)> = done
            .0
            .iter()
            .filter(|(_, o)| matches!(o, Outcome::Fail(_)))
            .map(|(t, _)| (t.clone(), number(t, &done.5.unfinished_at)))
            .collect();
        // A passing test without a number (not watched) counts as the last test, as a failed one does.
        let passing: Vec<(String, Option<u32>)> = done
            .0
            .iter()
            .filter(|(_, o)| matches!(o, Outcome::Pass))
            .map(|(t, _)| (t.clone(), number(t, &done.5.test_numbers)))
            .collect();
        let empty = BTreeSet::new();
        let changed = found.changed.get(&p).unwrap_or(&empty);
        let writers: Vec<(GlobalKey, usize)> = found
            .last
            .get(&p)
            .into_iter()
            .flatten()
            .map(|&(g, w)| (g, ks[w]))
            .collect();
        let read_in = done.5.singles_out.as_ref().map(|o| &o.read_in);
        let hints = super::order_hint::hints_for(self.lowered, self.units, changed, &writers, read_in, &failed);
        let reads = super::order_hint::hints_for(self.lowered, self.units, changed, &writers, read_in, &passing);
        let n = (hints.len(), reads.len());
        done.5.order_hints.extend(hints);
        done.5.order_hints.extend(reads);
        n
    }

    /// ADR-058 amendment 1, decision 4: dependent codeunit `p` of session `ks` and what it read, for the report.
    fn note_carried(&self, ks: &[usize], found: &Found, p: usize, report: &mut LaneReport) {
        let reads: Vec<(GlobalKey, usize)> = found
            .last
            .get(&p)
            .into_iter()
            .flatten()
            .map(|&(g, w)| (g, ks[w]))
            .collect();
        let c = super::single_fresh::carried_codeunit(self.lowered, self.units, ks[p], false, &reads);
        report.carried.push(c);
    }

    fn name(&self, k: usize) -> String {
        format!(
            "{} {}",
            self.units[k].id, self.lowered.program.objects[self.units[k].oi].name
        )
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

    /// ADR-066 amendment 7, option 1: the globals that make a codeunit dependent are noted per dependent codeunit.
    #[test]
    fn the_globals_read_first_that_differ_are_noted_per_dependent_codeunit() {
        let (flag, other) = ((1, 0), (2, 0));
        let a = out(&[], &[(flag, 7), (other, 1)], &[flag, other]);
        // b read both; only the flag differs from what a left.
        let b = out(&[(flag, 0), (other, 1)], &[(flag, 0), (other, 1)], &[]);
        let found = analyse(&[Some(&a), Some(&b)]);
        assert_eq!(found.changed, BTreeMap::from([(1, BTreeSet::from([flag]))]));
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
        assert!(found.dependent.is_empty(), "{found:?}");
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
    }

    #[test]
    fn a_guard_then_refill_with_another_value_is_no_link() {
        let cache = (1, 0);
        let a = out(&[], &[(cache, 3)], &[cache]);
        let mut b = out(&[(cache, 0)], &[(cache, 4)], &[cache]);
        b.guarded.insert(cache);
        b.refill.insert(cache, fp(4));
        let found = analyse(&[Some(&a), Some(&b)]);
        assert!(found.dependent.is_empty(), "{found:?}");
    }

    #[test]
    fn a_reader_of_a_value_an_earlier_codeunit_set_is_dependent() {
        let (flag, other) = ((1, 0), (2, 0));
        let a = out(&[], &[(flag, 7)], &[flag]);
        let b = out(&[(flag, 0)], &[(flag, 0)], &[]);
        let c = out(&[(other, 0)], &[(other, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        assert_eq!(found.dependent, BTreeMap::from([(1, (0, flag))]));
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
    fn equal_values_and_unobserved_codeunits_make_no_dependence() {
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
