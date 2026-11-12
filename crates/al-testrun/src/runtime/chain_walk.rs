//! The walk of the carry chain (`corpus.rs`).
//!
//! The chain runs its codeunits one after the other on the walk's thread, in its order, each from what the ones before
//! it in its suite left (`Carried`). Under runner 130450 a codeunit hands on the state it started from with what it
//! changed there: the numbers its tests took from the sequences of that state (`numbers_taken.rs`), what its OnRun
//! trigger wrote, and the rows the runner writes for it (`suite_rows.rs`). The walk is always serial (ADR-070,
//! migration step 1: the runs ahead on helper threads of ADR-066 amendments 8, 10 and 11 are gone).
//!
//! `ChainWalk::sequential` (ADR-070, migration step 2; `strict_walk.rs`): the walk of one BC session of the plain
//! sequential strict run. Every unit is a member, a unit that leaves nothing leaves the state it was given, every member
//! keeps the single instances of the one before it, and the members after one that stopped the session are flagged.

use super::corpus::{observed_detail, reaches_session_storage, refuse_all};
use super::not_run_link::not_run_link;
use super::*;

/// A member of the chain: a codeunit to run (an index into the units), or one with RequiredTestIsolation = Disabled
/// that is not run (object, why).
#[derive(Clone)]
pub(super) enum Link {
    Run(usize),
    Refused(usize, String),
    /// BcContainerHelper's discovery pass of the suite, before its first codeunit (ADR-074, `discovery.rs`).
    Discovery(Arc<super::discovery::Pass>),
}

/// What a chain member left, with what the ones before it in its suite left: (suite, id, leg, carried).
pub(super) type Carry = (usize, u32, u8, super::run::Carried);

/// The run of one chain walk.
#[derive(Clone, Copy)]
pub(super) struct ChainWalk<'a> {
    pub engine: &'a Arc<al_vm::Engine>,
    pub lowered: &'a Lowered,
    /// The install baseline the chain of each suite starts from.
    pub baseline: Option<&'a al_vm::Baseline>,
    pub units: &'a [Unit],
    /// `RunOptions::trace` (`--explain`): note the detail of each test.
    pub trace: bool,
    pub limit: Option<std::time::Duration>,
    /// The run's test threads and their memory (`memory.rs`): the walk's own run counts as a running codeunit.
    pub pool: &'a super::memory::Pool,
    /// ADR-066 amendment 8: the walk keeps the SingleInstance state from member to member of a BC
    /// session (suite and hub leg), as BC's one test-runner session does (P7: `TestCuP7BReader.al:24-37`), instead of
    /// starting each member with fresh single instances. The single instances move as in the lane (`run.rs`
    /// `LANE_SINGLES`).
    pub keep_singles: bool,
    /// ADR-058 amendment 1, decision 2 (`RunOptions::single_fresh_check`): a member that read a carried
    /// SingleInstance value runs once more with fresh single instances, from the same database start.
    pub fresh_check: bool,
    /// ADR-070 step 2: this is the walk of one BC session of the sequential strict run (`strict_walk.rs`); see the head of
    /// the file. The caller runs it on a thread of its own that already counts as a running codeunit.
    pub sequential: bool,
}

/// Runs `f` with the walk's kept single instances set aside (`run::LANE_SINGLES` empty): the session starts with fresh
/// ones and hands none on, and the kept ones go to the next member that keeps them (ADR-066 amendment 11, option 1).
fn without_kept_singles<R>(f: impl FnOnce() -> R) -> R {
    let kept = super::run::LANE_SINGLES.with(|c| c.borrow_mut().take());
    let r = f();
    super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = kept);
    r
}

/// The single instances the walk keeps from member to member (`ChainWalk::keep_singles`): which BC session they belong
/// to, the globals the members of that session wrote so far with the member (unit) that wrote each last, and what a
/// fresh instance holds in each global (ADR-058 amendment 1, decision 4: the reason of a flip names the last writer).
#[derive(Default)]
struct KeptSingles {
    session: Option<(usize, bool)>,
    last: std::collections::BTreeMap<al_vm::single_track::GlobalKey, usize>,
    fresh: std::collections::BTreeMap<al_vm::single_track::GlobalKey, al_vm::single_track::Fp>,
}

impl KeptSingles {
    /// The next member belongs to BC session `key` (suite, hub leg); `None`: a member that is not run. A new session
    /// starts with fresh single instances, and so does the member after one that is not run.
    fn enter(&mut self, key: Option<(usize, bool)>) {
        if starts_fresh(self.session, key) {
            super::run::LANE_SINGLES.with(|c| {
                if let Some(slot) = c.borrow_mut().as_mut() {
                    *slot = None;
                }
            });
            self.last.clear();
            self.fresh.clear();
        }
        self.session = key;
    }

    /// Member run `r` of unit `k` ended. Counts a member that read first a global an earlier member of its session wrote, and returns the globals
    /// it read with a carried value that differs from a fresh instance's, each with its last writer (ADR-058 amendment
    /// 1, decision 2: a suspect).
    /// `past_stop`: the single instances moved on past a test refused while it ran (`Scratch::singles_past_stop`).
    fn left(
        &mut self,
        k: usize,
        r: &UnitResult,
        past_stop: bool,
        report: &mut super::ChainReport,
    ) -> Vec<(al_vm::single_track::GlobalKey, usize)> {
        report.singles_kept += 1;
        let Ok(done) = &r.result else {
            self.enter(None);
            return Vec::new();
        };
        let Some(o) = done.5.singles_out.as_ref() else {
            return Vec::new();
        };
        if o.observed.keys().any(|g| self.last.contains_key(g)) {
            report.singles_read_carried += 1;
        }
        for (g, f) in &o.fresh {
            self.fresh.entry(*g).or_insert_with(|| f.clone());
        }
        let reads = super::single_fresh::carried_reads(&o.observed, &self.last, &self.fresh);
        if o.refused && !past_stop {
            // `run.rs` `singles_out` handed nothing on (R3): the next member starts fresh.
            self.last.clear();
            self.fresh.clear();
        } else {
            self.last.extend(o.wrote.iter().map(|g| (*g, k)));
        }
        reads
    }
}

/// Whether the member after one of BC session `before` starts with fresh single instances when it belongs to BC
/// session `next` (suite, hub leg; `None`: a member that is not run, or none before). ADR-066 amendment 8, decision 2:
/// a new BC session starts fresh, and so does a member that is not run and the member after it (R3 of amendment 3).
fn starts_fresh(before: Option<(usize, bool)>, next: Option<(usize, bool)>) -> bool {
    next.is_none() || before != next
}
impl ChainWalk<'_> {
    /// Runs the chain in its order and returns what each member left. `slots` takes the result of each member that
    /// ran, `details` the detail of each test under `trace`, `report` the members.
    pub(super) fn walk(
        &self,
        chain: Vec<(usize, u8, u32, Link)>,
        slots: &mut dyn FnMut(usize, UnitResult),
        details: &mut std::collections::HashMap<(usize, u32, String), explain::TestDetail>,
        report: &mut super::ChainReport,
    ) -> Vec<Carry> {
        let (engine, lowered, units) = (self.engine, self.lowered, self.units);
        let name_of = |oi: usize| lowered.program.objects[oi].name.clone();
        // Per suite, in id order: the codeunit id and what it left, with the tables of every one before it.
        let mut carried: Vec<Carry> = Vec::new();
        let keep = self.keep_singles;
        let mut singles = KeptSingles::default();
        // Sequential: what the members that stopped (a test refused while it ran, a time-limit stop, a codeunit not run as
        // a whole) and the ones flagged after them could have changed: a later member that can read it is flagged
        // (`strict_taint.rs`).
        let mut taint = super::strict_taint::Taint::default();
        super::run::CARRIED.with(|c| c.borrow_mut().take());
        if keep {
            super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = Some(None));
        }
        for (si, leg, id, link) in chain {
            // ADR-066 amendment 11, option 1: a member in the chain only for the numbers its tests take runs with fresh
            // single instances and hands none on (`chain::numbers_only`).
            let keep_member = keep
                && match &link {
                    Link::Run(k) => {
                        self.sequential || !super::chain::numbers_only(units[*k].isolation, units[*k].carry)
                    }
                    Link::Refused(..) | Link::Discovery(_) => true,
                };
            if keep_member {
                singles.enter(match &link {
                    Link::Run(k) => Some((si, units[*k].hub_leg)),
                    Link::Discovery(p) => Some((si, units[p.unit].hub_leg)),
                    // BC runs it and it may change the single instances: what it would leave is not known (R3).
                    Link::Refused(..) => None,
                });
            }
            let before = carried.iter().rev().find(|(s, _, _, _)| *s == si);
            let mut left = match link {
                Link::Run(k) => {
                    let start = match before {
                        Some((_, _, _, c)) => c.baseline.as_ref(),
                        None => self.baseline,
                    };
                    let mut u = units[k].clone();
                    u.leak = before.map(|(_, _, _, c)| Arc::new(c.tables.clone()));
                    u.singles_past_stop = self.sequential;
                    super::discovery::skip_methods_if_raised(&mut u, &report.discovery_raised);
                    // `--explain` names where a test of the chain failed too (al-runner issue 6924).
                    let mut observe = |name: &str, _: &Outcome, seen: &Observed| {
                        details.insert((si, id, name.to_lowercase()), observed_detail(lowered, seen));
                    };
                    let trace: Option<&mut Observer> = self.trace.then_some(&mut observe);
                    // The walk's own run counts as a running codeunit (ADR-066 amendment 10): its growth is the cost of
                    // a thread. The walk's thread never ends for memory.
                    let running = (!self.sequential).then(|| self.pool.enter());
                    let held = self.sequential && super::strict_walk::singles_held();
                    let mut r = if keep && !keep_member {
                        without_kept_singles(|| run_unit(engine, lowered, start, &u, trace, self.limit))
                    } else {
                        run_unit(engine, lowered, start, &u, trace, self.limit)
                    };
                    drop(running);
                    let mut left = super::run::CARRIED.with(|c| c.borrow_mut().take());
                    let plain = self.sequential && super::strict_walk::leaves_nothing(&units[k]);
                    if self.sequential {
                        let dropped = held && !super::strict_walk::singles_held();
                        super::strict_walk::after_run(&mut taint, engine, lowered, &units[k], &mut r, dropped);
                        // A codeunit whose tests roll back and whose OnRun cannot write leaves nothing in the database even
                        // when it was not run: the later codeunits are flagged when they can read what it changed, not refused.
                        if plain && left.is_none() {
                            left = Some(super::run::Carried::nothing());
                        }
                    }
                    let reads = if keep_member {
                        singles.left(k, &r, self.sequential && super::strict_walk::singles_held(), report)
                    } else {
                        Vec::new()
                    };
                    if !reads.is_empty() {
                        self.suspect(k, &reads, start, before.map(|(_, _, _, c)| c), &r, report);
                    }
                    report.members.push(super::ChainEntry {
                        id,
                        name: name_of(units[k].oi),
                        why: if plain {
                            super::ChainWhy::InOrder
                        } else {
                            super::chain::member_why(units[k].isolation, units[k].carry)
                        },
                        took: r.took,
                        left: super::chain::left_something(left.as_ref()),
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
                    slots(k, r);
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
                Link::Discovery(p) => self.discover(si, &p, &mut taint, report),
                Link::Refused(oi, why) => {
                    if self.sequential {
                        // `KeptSingles::enter(None)` dropped the single instances the session held (review finding 1).
                        taint.add(&super::strict_taint::unit_facts(engine, lowered, oi), true);
                        taint.singles_lost();
                    }
                    report.members.push(super::ChainEntry {
                        id,
                        name: name_of(oi),
                        why: super::ChainWhy::NotRun,
                        took: std::time::Duration::ZERO,
                        left: true,
                    });
                    not_run_link(engine, lowered, oi, false, why)
                }
            };
            // A link that leaves the state it was given (`baseline: None`) and has no link before it in its suite
            // leaves the walk's start: the first member that was not run would otherwise hand the later ones no
            // install baseline (tests/suite/refused_chain_link.rs).
            if before.is_none() && left.baseline.is_none() {
                left.baseline = self.baseline.cloned();
            }
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
            // The sequential walk needs only the last: each carries the tables and flags of the ones before it.
            if self.sequential && carried.len() > 1 {
                carried.drain(..carried.len() - 1);
            }
        }
        if keep {
            super::run::LANE_SINGLES.with(|c| *c.borrow_mut() = None);
        }
        carried
    }

    /// Runs BcContainerHelper's discovery pass `p` of suite `si` from the walk's start and returns what it left
    /// (`discovery::run_pass`); notes the codeunits whose OnRun raised in `report`.
    fn discover(
        &self,
        si: usize,
        p: &super::discovery::Pass,
        taint: &mut super::strict_taint::Taint,
        report: &mut super::ChainReport,
    ) -> super::run::Carried {
        let mut u = self.units[p.unit].clone();
        u.discovery = Some(p.objects.clone());
        u.run_only = Some(Arc::new(BTreeSet::new()));
        u.leak = None;
        u.carry = None;
        let running = (!self.sequential).then(|| self.pool.enter());
        let r = run_unit(self.engine, self.lowered, self.baseline, &u, None, self.limit);
        drop(running);
        let left = super::run::CARRIED.with(|c| c.borrow_mut().take());
        let found = super::discovery::take_found();
        report
            .discovery_raised
            .extend(found.raised.into_iter().map(|id| (si, id)));
        report.discovery_refused.extend(found.refused.map(|why| (si, why)));
        report
            .discovery_singles
            .extend(found.singles.into_iter().map(|g| (si, g)));
        report
            .discovery_wrote
            .extend(found.wrote.into_iter().map(|(id, t, st)| (si, id, t, st)));
        report.members.push(super::ChainEntry {
            id: 0,
            name: "BcContainerHelper discovery pass".into(),
            why: super::ChainWhy::Discovery,
            took: r.took,
            left: true,
        });
        match (left, &r.result) {
            (Some(left), Ok(_)) => left,
            (_, Err(panic)) => {
                taint.singles_lost();
                super::discovery::not_known(panic)
            }
            (None, Ok(_)) => super::discovery::not_known("it was refused before it ran"),
        }
    }

    /// Member `k` read carried SingleInstance values (`reads`): noted in `report.carried`, and under `fresh_check` run
    /// once more from `start` with fresh single instances (ADR-058 amendment 1, decision 2; the run hands nothing on).
    fn suspect(
        &self,
        k: usize,
        reads: &[(al_vm::single_track::GlobalKey, usize)],
        start: Option<&al_vm::Baseline>,
        before: Option<&super::run::Carried>,
        r: &UnitResult,
        report: &mut super::ChainReport,
    ) {
        let mut c = super::single_fresh::carried_codeunit(self.lowered, self.units, k, true, reads);
        if self.fresh_check && r.result.is_ok() {
            let mut u = self.units[k].clone();
            u.leak = before.map(|c| Arc::new(c.tables.clone()));
            let fresh = super::single_fresh::rerun_fresh(self.engine, self.lowered, start, &u, self.limit, self.pool);
            c.fresh = Some(super::single_fresh::outcomes_of(&fresh));
        }
        report.carried.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::starts_fresh;

    /// ADR-066 amendment 8, decision 2: which member of the walk starts with fresh single instances.
    #[test]
    fn a_member_starts_fresh_at_a_session_boundary_and_after_a_member_that_is_not_run() {
        // The first member of the walk.
        assert!(starts_fresh(None, Some((0, true))));
        // The same suite and hub leg: the state moves on.
        assert!(!starts_fresh(Some((0, true)), Some((0, true))));
        // Another hub leg or another suite: a new BC session.
        assert!(starts_fresh(Some((0, true)), Some((0, false))));
        assert!(starts_fresh(Some((0, false)), Some((1, false))));
        // A refused member (not run): what BC's run of it would leave is not known, so nothing is handed on.
        assert!(starts_fresh(Some((0, true)), None));
        // The member after a refused one, in the same session as the one before the refused member.
        assert!(starts_fresh(None, Some((0, true))));
    }
}
