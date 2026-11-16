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
use super::lane_starts::{Ask, Deltas, Held, Reply, Starts, carried_start};
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
    /// Positions of the codeunits to run in the lane: the dependent ones and the writers they need.
    pub lane: BTreeSet<usize>,
    /// Per dependent codeunit (position), the positions of the earlier codeunits it needs in the lane: for each global
    /// that makes it dependent, the last writer (R2).
    pub writers: BTreeMap<usize, BTreeSet<usize>>,
    /// Position of a codeunit that is not dependent only because a guard then the refill of a global it read first
    /// is taken as no link although the earlier codeunit left another value (`Assumption::SingleInstanceRefill`), with
    /// that codeunit's position and the global.
    pub refill_assumed: BTreeMap<usize, (usize, GlobalKey)>,
    /// Per dependent codeunit (position), the globals it read first that the earlier codeunits left with another value
    /// (ADR-066 amendment 7, option 1: only a failed test that read one, or a test before it, makes it run again).
    pub changed: BTreeMap<usize, BTreeSet<GlobalKey>>,
    /// Per dependent codeunit (position), each global of `changed` with the position of the codeunit that wrote it
    /// last (ADR-058 amendment 1, decision 4: the reason of a flip).
    pub last: BTreeMap<usize, Vec<(GlobalKey, usize)>>,
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
            found.changed.insert(p, diff.iter().map(|(k, _)| *k).collect());
            found.last.insert(p, diff.clone());
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

    /// ADR-066 amendment 7, option 2: splits `chosen` into the codeunits whose own chain ([`Found::lane_for`] of it
    /// alone) holds at most `limit` codeunits, and the others with the length of their chain.
    pub(super) fn within_limit(
        &self,
        chosen: &BTreeSet<usize>,
        limit: usize,
    ) -> (BTreeSet<usize>, BTreeMap<usize, usize>) {
        let mut kept = BTreeSet::new();
        let mut over = BTreeMap::new();
        for &p in chosen {
            let n = self.lane_for(&BTreeSet::from([p])).len();
            if n > limit {
                over.insert(p, n);
            } else {
                kept.insert(p);
            }
        }
        (kept, over)
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

/// What the run output and the JUnit file say of a codeunit whose re-run was skipped because its chain of `chain`
/// codeunits is over `--max-replay` `limit` (ADR-066 amendment 7, option 2; opt-in since the owner's change of
/// 2026-11-08: without `--max-replay` there is no cap and nothing is skipped).
pub fn rerun_skipped_text(chain: usize, limit: usize) -> String {
    format!("re-run skipped (chain of {chain} codeunits over the limit of {limit})")
}

/// ADR-066 amendment 7, option 1 (owner, 2026-11-08): whether a failed or unfinished test, the last of them numbered
/// `bound` (`Session::single_test_begins`), or a test before it, read first one of the `changed` globals. `read_in`
/// gives the test each global was first read in (0: before the first test, so it counts for every test). A changed
/// global without a note counts. Corpus P7B (al-language/codeunits/TestCuP7BReader.al:24-37): the reader test itself
/// reads what the earlier codeunit left and fails with fresh state, so it runs again.
fn failure_saw_change(read_in: &BTreeMap<GlobalKey, u32>, changed: &BTreeSet<GlobalKey>, bound: u32) -> bool {
    changed.iter().any(|k| read_in.get(k).is_none_or(|&t| t <= bound))
}

/// The number of the last test of a codeunit's run that failed or started and did not run to its end
/// (`Scratch::unfinished_at`). A failure the test loop did not number (an outcome changed after it) counts as the last
/// test of all (`u32::MAX`), as does a run without results.
fn failure_bound(slot: Option<&UnitResult>) -> u32 {
    let Some(Ok(done)) = slot.map(|r| &r.result) else {
        return u32::MAX;
    };
    let at = &done.5.unfinished_at;
    let unnumbered = done
        .0
        .iter()
        .any(|(t, o)| matches!(o, Outcome::Fail(_)) && !at.iter().any(|(_, n)| n == t));
    if unnumbered {
        return u32::MAX;
    }
    at.iter().map(|(n, _)| *n).max().unwrap_or(u32::MAX)
}

/// A dependent codeunit whose serial re-run was skipped because its chain is over `--max-replay`.
#[derive(Debug, Clone)]
pub struct RerunSkipped {
    /// The index of the run (`TestResult::suite`) and the codeunit.
    pub suite: usize,
    pub codeunit: u32,
    /// The codeunit's id and name.
    pub name: String,
    /// How many codeunits its replay chain holds.
    pub chain: usize,
    /// The `--max-replay` limit it is over.
    pub limit: usize,
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
    /// `--strict-order` (ADR-066 amendment 12, decision 4): BC's order. Flags dependent codeunits and runs them again
    /// as amendments 4, 5 and 7 say. Without it (the default, decisions 1 and 2) nothing is flagged or run again: a
    /// failed test that read a changed global gets a hint (`order_hint.rs`).
    pub bc_order: bool,
    /// `RunOptions::replay_every_dependent` (with `bc_order`; what `--strict-order` meant before amendment 12):
    /// every dependent codeunit runs again serially.
    pub strict: bool,
    /// `--max-replay N` (amendment 7, option 2, opt-in since 2026-11-08): a re-run whose chain holds more than `N`
    /// codeunits is skipped. `None`: every re-run is replayed in full. `strict` wins over it.
    pub max_replay: Option<usize>,
    /// `--test-suites` (amendment 6): each suite is a BC session of its own.
    pub suites: Option<&'a crate::test_suites::TestSuites>,
    /// The run's worker threads: the serial sessions run on as many (amendment 5).
    pub workers: usize,
    /// A worker thread can make its own copy of the install baseline (`corpus.rs` `baseline_engine`); without it and
    /// with a baseline, the sessions run one after the other on this thread.
    pub install_copies: bool,
    /// The install baseline the test threads start from, made once per run (shared with the parallel run).
    pub snapshot: &'a super::install_snapshot::InstallSnapshots,
    /// A worker thread's own baseline from `snapshot` (a baseline holds `Rc`s and stays on its thread; ADR-069: over
    /// the shared frozen one).
    pub thread_baseline:
        &'a (dyn Fn(Option<&super::install_snapshot::InstallSnapshot>) -> Option<al_vm::Baseline> + Sync),
    /// The run's test threads and their memory (`memory.rs`): a worker starts and goes on only while memory has room.
    pub pool: &'a super::memory::Pool,
    /// ADR-066 amendment 8: the chain walk kept the single instances from member to member (`ChainWalk::keep_singles`),
    /// so a chain member's result is already its run in BC's order: it is never dependent here, never runs again and is
    /// not flagged. It still counts as a writer for the codeunits that are no members.
    pub kept_chain: bool,
    /// `RunOptions::single_fresh_check` (ADR-058 amendment 1): keep the parallel (fresh) run of each dependent codeunit
    /// the lane runs again, in `LaneReport::carried`.
    pub fresh_check: bool,
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
    /// Every dependent codeunit ran again serially (`Lane::strict`).
    pub strict: bool,
    /// `--strict-order` (ADR-066 amendment 12): the lane flagged and ran again; `false`: it only gave hints.
    pub bc_order: bool,
    /// Default run (amendment 12, decision 2): the failed tests that got an order hint.
    pub hinted: usize,
    /// Dependent codeunits with a failed or unfinished test that kept their parallel run because no such test, and no
    /// test before it, read a global the chain leaves otherwise (ADR-066 amendment 7, option 1).
    pub kept: usize,
    /// Dependent codeunits that would run again but whose chain is over `--max-replay` (amendment 7, option 2; empty
    /// without the flag): they keep their parallel run, flagged. Not counted in `rerun`.
    pub skipped: Vec<RerunSkipped>,
    /// Serial sessions the lane ran (amendment 5): one per chain of a dependent codeunit that ran again (none for one
    /// whose chain starts another's).
    pub chains: usize,
    /// The time of the longest serial session.
    pub longest: std::time::Duration,
    /// The threads the serial sessions ran on (amendment 5).
    pub threads: usize,
    /// Carried starts the main thread made for the worker threads (amendment 5, decision 5): each carry's delta once
    /// (`lane_starts.rs`), and the starts written whole as bytes.
    pub starts_written: usize,
    /// Of `starts_written`, those written whole as bytes (a carry without a delta).
    pub starts_whole: usize,
    /// The most carried starts alive whole as bytes at once (al-runner issue 6924).
    pub starts_peak: usize,
    /// The most bytes of deltas the main thread kept at once (at most `lane_starts::DELTA_BUDGET`).
    pub delta_bytes_peak: usize,
    /// ADR-066 amendment 8: chain members that read first a global a codeunit outside the chain wrote before them.
    pub chain_missed: usize,
    /// ADR-058 amendment 1: the dependent codeunits outside the carry chain, with what they read and (under
    /// `Lane::fresh_check`) their parallel run's outcomes.
    pub carried: Vec<super::CarriedCodeunit>,
}

impl LaneReport {
    pub fn lines(&self) -> Vec<String> {
        if self.watched == 0 {
            return Vec::new();
        }
        if !self.bc_order {
            let mut out = vec![format!(
                "SingleInstance order: each test codeunit starts with fresh SingleInstance state ({STRICT_FLAG} runs them in BC's order): {} test codeunits watched, {} read state an earlier codeunit would have left, {} failed tests got an order hint",
                self.watched, self.dependent, self.hinted
            )];
            out.extend(self.notes.iter().cloned());
            return out;
        }
        let why = if self.strict {
            "(strict order: every dependent codeunit)"
        } else {
            "because a test failed or was refused while it ran in parallel"
        };
        let mut out = vec![format!(
            "SingleInstance order: {} test codeunits watched, {} BC sessions with a dependent codeunit, {} dependent codeunits (flagged), {} ran again serially {why}, {} kept their parallel run (no failed test read a changed global), {}{} codeunit runs in {} serial sessions (longest {:.2} s, lane wall time {:.2} s); {} codeunits flagged as independent under the guard-then-refill assumption",
            self.watched,
            self.sessions,
            self.dependent,
            self.rerun,
            self.kept,
            self.skipped_count(),
            self.lane.len(),
            self.chains,
            self.longest.as_secs_f64(),
            self.took.as_secs_f64(),
            self.refill_assumed
        )];
        if self.chain_missed > 0 {
            out[0].push_str(&format!(
                "; {} chain members read first a global a codeunit outside the chain wrote before them",
                self.chain_missed
            ));
        }
        if self.starts_written > 0 {
            out[0].push_str(&format!(
                "; {} carried starts made for {} threads ({} written whole as bytes, at most {} at once; {:.1} MB of row deltas kept at most)",
                self.starts_written,
                self.threads,
                self.starts_whole,
                self.starts_peak,
                self.delta_bytes_peak as f64 / 1e6
            ));
        }
        out.extend(self.notes.iter().cloned());
        out.extend(self.skipped_lines());
        out
    }

    /// The part of the summary line on skipped re-runs: nothing when none was skipped (always without `--max-replay`).
    fn skipped_count(&self) -> String {
        match self.skipped.first() {
            Some(s) => format!(
                "{} re-runs skipped (chain over --max-replay {}), ",
                self.skipped.len(),
                s.limit
            ),
            None => String::new(),
        }
    }

    /// One line per codeunit whose re-run was skipped (amendment 7, option 2). The run output prints them with or
    /// without `--timings`.
    pub fn skipped_lines(&self) -> Vec<String> {
        self.skipped
            .iter()
            .map(|s| {
                format!(
                    "SingleInstance order: test codeunit {}: {}; its parallel result is kept (flagged); --strict-order replays it",
                    s.name,
                    rerun_skipped_text(s.chain, s.limit)
                )
            })
            .collect()
    }

    /// The text for the tests of codeunit `codeunit` of run `suite` when its re-run was skipped.
    pub fn skipped_text(&self, suite: usize, codeunit: u32) -> Option<String> {
        self.skipped
            .iter()
            .find(|s| s.suite == suite && s.codeunit == codeunit)
            .map(|s| rerun_skipped_text(s.chain, s.limit))
    }
}

/// One serial session of the lane (amendment 5): unit indices in BC's order, the dependent ones whose run is reported,
/// and where each codeunit starts.
struct Chain {
    order: Vec<usize>,
    chosen: BTreeSet<usize>,
    steps: Vec<Step>,
    /// What the session is expected to cost: the parallel run times of its codeunits ([`dispatch_order`]).
    cost: std::time::Duration,
}

/// Where a codeunit of a serial session starts, worked out on this thread (amendment 5). A baseline a carry-chain
/// member left (RequiredTestIsolation = Disabled) holds `Rc`s and stays on the thread that made it, so a worker thread
/// asks this thread for it as bytes ([`Starts`]), as the chain walk's helpers do (`chain_walk.rs`). The same choices as
/// [`Lane::run_bound`].
struct Step {
    k: usize,
    /// A chain member starts from what the member before it left (the index of that member's carry); `None`: the
    /// install baseline.
    from: Option<usize>,
    /// A codeunit that is no member and comes after one: whether it reaches session storage that member changed (then
    /// it runs again whatever it read), and the carry it runs again from when it read a table that member wrote
    /// (`None`: it keeps its first run).
    again: Option<(bool, Option<usize>)>,
}

/// A carried start a session needs cannot be written as bytes: the session runs on this thread from the baselines
/// themselves ([`Lane::run_bound`]).
struct NoBytes;

/// What a worker thread needs to run a session's steps (all of it can be shared between threads).
struct Worker<'a> {
    engine: &'a Arc<Engine>,
    lowered: &'a Lowered,
    units: &'a [Unit],
    limit: Option<std::time::Duration>,
}

impl Worker<'_> {
    /// Runs one step from `install` (this thread's install baseline) or a carried start that `start` builds from what
    /// the thread that holds the baselines hands over (`lane_starts.rs`). `Ok(None)`: the codeunit keeps its first run.
    fn run(
        &self,
        install: Option<&al_vm::Baseline>,
        start: &mut dyn FnMut(usize) -> Result<Option<al_vm::Baseline>, NoBytes>,
        s: &Step,
    ) -> Result<Option<UnitResult>, NoBytes> {
        let mut u = self.units[s.k].clone();
        u.watch = false;
        let first = match s.from {
            Some(carry) => match start(carry)? {
                Some(b) => Some(b),
                None => return Ok(None),
            },
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
        drop(first);
        if let Some((storage, again)) = s.again {
            if storage || r.result.as_ref().is_ok_and(|done| done.5.leak_hit) {
                let Some(carry) = again else { return Ok(None) };
                let Some(from) = start(carry)? else { return Ok(None) };
                super::run::CARRIED.with(|c| c.borrow_mut().take());
                r = run_unit(self.engine, self.lowered, Some(&from), &u, None, self.limit);
            }
        }
        Ok(Some(r))
    }
}

/// What one serial session gave: each codeunit's run in order (`None`: kept its first run, `corpus.rs` refuses the
/// case), and the time it took.
type ChainRuns = (Vec<(usize, Option<UnitResult>)>, std::time::Duration);

/// Runs the codeunits `order` one after the other on this thread, the single instances moved from codeunit to codeunit
/// (`run.rs` `LANE_SINGLES`). Stops at the first codeunit `one` gives up on.
fn run_in_order<E>(
    order: &[usize],
    one: &mut dyn FnMut(usize) -> Result<Option<UnitResult>, E>,
) -> Result<ChainRuns, E> {
    let started = std::time::Instant::now();
    super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = Some(None));
    let mut out = Vec::with_capacity(order.len());
    let mut ended = Ok(());
    for &k in order {
        let r = one(k);
        super::run::CARRIED.with(|c| c.borrow_mut().take());
        match r {
            Ok(r) => out.push((k, r)),
            Err(e) => {
                ended = Err(e);
                break;
            }
        }
    }
    super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = None);
    ended.map(|()| (out, started.elapsed()))
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
        report.bc_order = self.bc_order;
        let chains = self.chains(slots, report);
        report.chains = chains.len();
        // A run stopped at its hard memory limit runs nothing again: the codeunits whose result is their serial run
        // here were not reached, so their tests are not run, for the limit (ADR-066 amendment 10, decision 6).
        if self.pool.halted() {
            let halt = self.pool.halt();
            for k in chains.iter().flat_map(|c| c.chosen.iter()) {
                if let Some(Ok(done)) = slots[*k].as_mut().map(|r| &mut r.result) {
                    super::corpus::refuse_all(&mut done.0, halt.reason());
                }
            }
            return;
        }
        let runs = self.run_chains(&chains, report);
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
                if let Some(first) = slots[k].as_ref().filter(|_| self.fresh_check) {
                    let u = &self.units[k];
                    if let Some(c) = report
                        .carried
                        .iter_mut()
                        .find(|c| c.suite == u.si && c.codeunit == u.id)
                    {
                        c.fresh = Some(super::single_fresh::outcomes_of(first));
                    }
                }
                slots[k] = Some(r);
            }
        }
        report.took += started.elapsed();
    }

    /// Analyses each BC session, flags its dependent codeunits, and returns the serial sessions to run.
    fn chains(&self, slots: &mut [Option<UnitResult>], report: &mut LaneReport) -> Vec<Chain> {
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
            // Amendment 12: the default claims no BC order, so nothing is flagged.
            for &p in found.refill_assumed.keys().filter(|_| self.bc_order) {
                if !refill_counted.insert(ks[p]) {
                    continue;
                }
                if let Some(Ok(done)) = slots[ks[p]].as_mut().map(|r| &mut r.result) {
                    flag_all(done, Assumption::SingleInstanceRefill);
                    report.refill_assumed += 1;
                }
            }
            if self.kept_chain {
                report.chain_missed += self.missed_by_walk(&ks, slots);
            }
            let fresh: BTreeSet<usize> = found
                .dependent
                .keys()
                .copied()
                .filter(|&p| !(self.kept_chain && self.keeps_singles(ks[p])))
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
                if let Some(Ok(done)) = slots[ks[p]].as_mut().map(|r| &mut r.result).filter(|_| self.bc_order) {
                    flag_all(done, Assumption::SingleInstanceOrder);
                }
                self.note_carried(&ks, &found, p, report);
            }
            // Amendment 12, decisions 1 and 2: by default nothing runs again; a failed test that read a changed global
            // gets the hint.
            if !self.bc_order {
                for &p in &fresh {
                    report.hinted += self.hint(&ks, &found, p, slots);
                }
                continue;
            }
            // Decision 2: only a dependent codeunit with a failure or a test refused while it ran in parallel runs again
            // serially; `--strict-order` (decision 4) runs them all again. Amendment 7, option 1: and only when such a
            // test, or one before it, read a global the chain leaves otherwise.
            let mut chosen: BTreeSet<usize> = BTreeSet::new();
            for &p in &fresh {
                let slot = slots[ks[p]].as_ref();
                if self.strict {
                    chosen.insert(p);
                } else if needs_serial_run(slot) {
                    let read_in = match slot.map(|r| &r.result) {
                        Some(Ok(done)) => done.5.singles_out.as_ref().map(|o| &o.read_in),
                        _ => None,
                    };
                    let saw = match (read_in, found.changed.get(&p)) {
                        (Some(read_in), Some(changed)) => failure_saw_change(read_in, changed, failure_bound(slot)),
                        _ => true,
                    };
                    if saw {
                        chosen.insert(p);
                    } else {
                        report.kept += 1;
                    }
                }
            }
            // Amendment 7, option 2, opt-in since the owner's change of 2026-11-08: with `--max-replay N`, a chain over
            // `N` is not replayed (not under `--strict-order`). Without it every chosen re-run is replayed in full.
            if let Some(limit) = self.max_replay.filter(|_| !self.strict) {
                let (kept, over) = found.within_limit(&chosen, limit);
                chosen = kept;
                for (p, chain) in over {
                    let k = ks[p];
                    if let Some(Ok(done)) = slots[k].as_mut().map(|r| &mut r.result) {
                        flag_all(done, Assumption::SingleInstanceRerunSkipped);
                    }
                    report.skipped.push(RerunSkipped {
                        suite: self.units[k].si,
                        codeunit: self.units[k].id,
                        name: self.name(k),
                        chain,
                        limit,
                    });
                }
            }
            report.rerun += chosen.len();
            for (positions, reported) in found.sessions(&chosen) {
                let order: Vec<usize> = positions.iter().map(|&p| ks[p]).collect();
                let cost = order.iter().filter_map(|&k| slots[k].as_ref()).map(|r| r.took).sum();
                chains.push(Chain {
                    cost,
                    steps: self.plan(&order),
                    chosen: reported.iter().map(|&p| ks[p]).collect(),
                    order,
                });
            }
        }
        chains
    }

    /// Runs the serial sessions in parallel (amendment 5) on worker threads, as many as the parallel run uses, while this
    /// thread writes the carried starts they ask for ([`Starts`]). With one thread or one session, or for a session whose
    /// carried start cannot be written as bytes, this thread runs it from the baselines themselves. The results come
    /// back in the order of `chains`, whichever thread ran them.
    fn run_chains(&self, chains: &[Chain], report: &mut LaneReport) -> Vec<ChainRuns> {
        let mut out: Vec<Option<ChainRuns>> = (0..chains.len()).map(|_| None).collect();
        let threads = if self.workers > 1 && (self.baseline.is_none() || self.install_copies) {
            self.workers
        } else {
            1
        };
        let helpers = if threads > 1 && chains.len() > 1 {
            threads.min(chains.len())
        } else {
            0
        };
        let mut starts = Starts::default();
        // Each carried start made once, as the rows it holds apart from the one before (`lane_starts.rs`).
        // Without an install baseline the first delta is taken from a baseline that holds nothing.
        let empty = al_vm::Baseline::empty();
        let mut deltas = Deltas::new(self.carried, self.baseline.unwrap_or(&empty));
        let mut ran_on = 1;
        if helpers > 0 {
            let snapshot = self.snapshot.get_or_make(self.baseline);
            let next = std::sync::atomic::AtomicUsize::new(0);
            // The costliest sessions first: the lane's wall time is then close to the longest session's.
            let taken = dispatch_order(&chains.iter().map(|c| c.cost).collect::<Vec<_>>());
            let worker = Worker {
                engine: self.engine,
                lowered: self.lowered,
                units: self.units,
                limit: self.limit,
            };
            let (thread_baseline, pool) = (self.thread_baseline, self.pool);
            let (ask, asked) = std::sync::mpsc::channel::<Ask>();
            // Worker threads alive, and the most at once.
            let live = std::sync::atomic::AtomicUsize::new(0);
            std::thread::scope(|scope| {
                let mut handles = Vec::new();
                // ADR-066 amendment 10, decisions 1 and 3: a worker starts while a session waits and memory has room,
                // and ends after a session when there is none; the sessions no worker ran run on this thread below.
                let mut grow = |handles: &mut Vec<_>| {
                    let mut added = 0;
                    while live.load(std::sync::atomic::Ordering::SeqCst) < helpers
                        && next.load(std::sync::atomic::Ordering::Relaxed) + added < taken.len()
                    {
                        let Some(member) = pool.try_add(added) else { break };
                        let ask = ask.clone();
                        let (next, worker, taken, live) = (&next, &worker, &taken, &live);
                        live.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let spawned =
                            std::thread::Builder::new()
                                .stack_size(TEST_STACK)
                                .spawn_scoped(scope, move || {
                                    // Declared first, so it drops last: what this worker's AL code made is released at
                                    // its end.
                                    let _run_end = al_builtins::instance_cycles::RunEnd::new();
                                    let _member = member;
                                    let own = thread_baseline(snapshot);
                                    let (reply, replies) = std::sync::mpsc::channel();
                                    let mut fetch = |carry: usize, whole: bool| {
                                        ask.send((carry, whole, reply.clone())).ok()?;
                                        replies.recv().ok()
                                    };
                                    // The one carried start this thread holds (issue 6924).
                                    let mut held: Held = None;
                                    let mut start = |carry: usize| {
                                        carried_start(carry, &mut held, own.as_ref(), worker.engine, &mut fetch)
                                            .map_err(|()| NoBytes)
                                    };
                                    let mut done = Vec::new();
                                    loop {
                                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                        let Some(&c) = taken.get(i) else { break };
                                        let chain = &chains[c];
                                        let running = pool.enter();
                                        let mut at = chain.steps.iter();
                                        let runs = run_in_order(&chain.order, &mut |_| {
                                            worker.run(
                                                own.as_ref(),
                                                &mut start,
                                                at.next().expect("a step per codeunit"),
                                            )
                                        });
                                        drop(running);
                                        done.push((c, runs.ok()));
                                        if !pool.keep() {
                                            break;
                                        }
                                    }
                                    live.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                                    done
                                });
                        match spawned {
                            Ok(h) => handles.push(h),
                            // No room for another thread (an address space limit): fewer will do.
                            Err(_) => {
                                live.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                                break;
                            }
                        }
                        added += 1;
                        ran_on = ran_on.max(live.load(std::sync::atomic::Ordering::SeqCst));
                    }
                };
                grow(&mut handles);
                // Serves the workers' asks until none is alive and no other can start.
                loop {
                    match asked.recv_timeout(std::time::Duration::from_millis(20)) {
                        Ok((carry, whole, reply)) => {
                            let delta = if whole { None } else { deltas.get(carry) };
                            let answer = match delta {
                                Some((d, base)) => Reply::Delta(d, base),
                                None => Reply::Whole(
                                    self.carried[carry]
                                        .3
                                        .baseline
                                        .as_ref()
                                        .and_then(|b| starts.get(carry, b)),
                                ),
                            };
                            let _ = reply.send(answer);
                        }
                        Err(_) => {
                            grow(&mut handles);
                            if live.load(std::sync::atomic::Ordering::SeqCst) == 0 {
                                break;
                            }
                        }
                    }
                }
                for h in handles {
                    for (c, runs) in h.join().expect("a lane worker panicked") {
                        out[c] = runs;
                    }
                }
            });
        }
        for (i, c) in chains.iter().enumerate() {
            if out[i].is_none() {
                let Ok(runs) = run_in_order::<std::convert::Infallible>(&c.order, &mut |k| Ok(self.run_bound(k)));
                out[i] = Some(runs);
            }
        }
        report.threads = ran_on;
        let (made, kept) = (deltas.count, deltas.peak);
        report.starts_written = starts.written + made;
        report.starts_whole = starts.written;
        report.starts_peak = starts.peak;
        report.delta_bytes_peak = kept;
        out.into_iter().map(|r| r.expect("every serial session ran")).collect()
    }

    /// Where each codeunit of a session starts ([`Step`]).
    fn plan(&self, order: &[usize]) -> Vec<Step> {
        order
            .iter()
            .map(|&k| {
                let Some(ci) = self.earlier_carry_index(k) else {
                    return Step {
                        k,
                        from: None,
                        again: None,
                    };
                };
                let c = &self.carried[ci].3;
                let start = match (&c.unfinished, &c.storage_why, &c.baseline) {
                    (None, None, Some(_)) => Some(ci),
                    _ => None,
                };
                if self.is_member(k) {
                    return Step {
                        k,
                        from: start,
                        again: None,
                    };
                }
                let storage =
                    c.everything && super::corpus::reaches_session_storage(self.engine, self.lowered, self.units[k].oi);
                Step {
                    k,
                    from: None,
                    again: Some((storage, start)),
                }
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

    /// ADR-066 amendment 8: chain members of session `ks` that read first a global a codeunit that is no member, earlier
    /// in the session, wrote. The walk did not see that codeunit's state, so for them it is not BC's session.
    fn missed_by_walk(&self, ks: &[usize], slots: &[Option<UnitResult>]) -> usize {
        let mut wrote: BTreeSet<GlobalKey> = BTreeSet::new();
        let mut n = 0;
        for &k in ks {
            let Some(Ok(done)) = slots[k].as_ref().map(|r| &r.result) else {
                continue;
            };
            let Some(o) = done.5.singles_out.as_ref() else { continue };
            if self.keeps_singles(k) {
                if o.observed.keys().any(|g| wrote.contains(g)) {
                    n += 1;
                }
            } else if !o.refused {
                wrote.extend(o.wrote.iter().copied());
            }
        }
        n
    }

    /// ADR-066 amendment 12, decision 2: the hint of each failed test of dependent codeunit `p` of session `ks` that
    /// read first, in it or in a test before it, a global the earlier codeunits left otherwise. Returns how many tests
    /// got one.
    fn hint(&self, ks: &[usize], found: &Found, p: usize, slots: &mut [Option<UnitResult>]) -> usize {
        let Some(Ok(done)) = slots[ks[p]].as_mut().map(|r| &mut r.result) else {
            return 0;
        };
        let failed: Vec<(String, Option<u32>)> = done
            .0
            .iter()
            .filter(|(_, o)| matches!(o, Outcome::Fail(_)))
            .map(|(t, _)| {
                let n = done.5.unfinished_at.iter().find(|(_, name)| name == t).map(|(n, _)| *n);
                (t.clone(), n)
            })
            .collect();
        if failed.is_empty() {
            return 0;
        }
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
        let n = hints.len();
        done.5.order_hints.extend(hints);
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

    fn is_member(&self, k: usize) -> bool {
        let u = &self.units[k];
        u.carry.is_some() || u.isolation == TestIsolation::Disabled
    }

    /// Whether codeunit `k` is a chain member whose walk run kept the single instances of the members before it
    /// (ADR-066 amendment 8). A member in the chain only for the numbers its tests take ran with fresh ones (amendment
    /// 11, option 1, `chain::numbers_only`): it is flagged and run again here like a codeunit outside the chain.
    fn keeps_singles(&self, k: usize) -> bool {
        let u = &self.units[k];
        self.is_member(k) && !super::chain::numbers_only(u.isolation, u.carry)
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

/// The order the worker threads take the serial sessions in: the costliest first, so the longest sessions do not start
/// last while the other threads have nothing left to do. `costs[i]` is what session `i` is expected to cost.
fn dispatch_order(costs: &[std::time::Duration]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..costs.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(costs[i]));
    order
}

/// Marks every test of a codeunit's run with `a`.
pub(super) fn flag_all(done: &mut UnitOutcomes, a: Assumption) {
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

    /// ADR-066 amendment 7, option 1 (owner, 2026-11-08): a re-run is needed only when a failed test, or a test before
    /// it, read first a global that the chain leaves otherwise. A read before the first test (OnRun) counts for all.
    #[test]
    fn a_failure_sees_a_changed_global_only_when_it_or_an_earlier_test_read_it() {
        let (flag, other) = ((1, 0), (2, 0));
        let changed = BTreeSet::from([flag]);
        let read_in = BTreeMap::from([(flag, 2), (other, 1)]);
        assert!(
            !failure_saw_change(&read_in, &changed, 1),
            "the failed test 1 read only the other global"
        );
        assert!(failure_saw_change(&read_in, &changed, 2), "test 2 read it and failed");
        assert!(
            failure_saw_change(&read_in, &changed, 3),
            "test 2 read it before the failed test 3"
        );
        assert!(
            failure_saw_change(&BTreeMap::from([(flag, 0)]), &changed, 1),
            "OnRun read it"
        );
        assert!(
            failure_saw_change(&BTreeMap::new(), &changed, 1),
            "a read not noted counts"
        );
        assert!(!failure_saw_change(&read_in, &BTreeSet::new(), u32::MAX));
    }

    /// ADR-066 amendment 7, option 2: a chosen codeunit whose own chain is longer than the limit is not replayed.
    #[test]
    fn a_chain_over_the_limit_is_left_out_with_its_length() {
        let (k1, k2) = ((1, 0), (2, 0));
        let a = out(&[], &[(k2, 3)], &[k2]);
        let b = out(&[(k2, 0)], &[(k2, 0), (k1, 4)], &[k1]);
        let c = out(&[(k1, 0)], &[(k1, 0)], &[]);
        let found = analyse(&[Some(&a), Some(&b), Some(&c)]);
        // b's chain holds 2 codeunits, c's 3.
        let (kept, over) = found.within_limit(&BTreeSet::from([1, 2]), 2);
        assert_eq!(kept, BTreeSet::from([1]));
        assert_eq!(over, BTreeMap::from([(2, 3)]));
        let (kept, over) = found.within_limit(&BTreeSet::from([1, 2]), 3);
        assert_eq!((kept, over), (BTreeSet::from([1, 2]), BTreeMap::new()));
    }

    #[test]
    fn the_costliest_serial_session_is_taken_first() {
        let ms = std::time::Duration::from_millis;
        assert_eq!(
            dispatch_order(&[ms(5), ms(900), ms(40), ms(900), ms(0)]),
            vec![1, 3, 2, 0, 4]
        );
        assert!(dispatch_order(&[]).is_empty());
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
