//! The plain sequential strict run (ADR-070, migration step 2; `RunOptions::strict_sequential` with `strict_order`).
//!
//! BC runs all test codeunits of a run in one session, in order. Here each BC session ([`super::single_lane::keyed_streams`]:
//! the suite and its hub leg, or each line of `--test-suites`) runs its test codeunits one after the other on one thread,
//! in BC's order, and different BC sessions run in parallel with each other, as on BC (they never share state there
//! either). The single instances, the committed rows, the number sequence values and the isolated storage move from
//! codeunit to codeunit as `chain_walk.rs` carries them (`ChainWalk::sequential`); there is no run ahead and no run
//! again. The result is exact by construction (P7: `al-language/codeunits/TestCuP7BReader.al:24-37` against
//! `TestCuP7ASetter.al:24-58`), except after a test the runtime could not run to its end: the later codeunits of that
//! session are flagged (`Assumption::StrictAfterRefused`), as their state is no longer BC's.
//!
//! A suite's hub leg and the codeunits runner 130450 runs share one database in BC, so they are one walk: the hub leg
//! first (BC runs it first, `RqIsolationModes.al:12`), then the rest, the single instances starting fresh at that
//! boundary (`chain_walk.rs` `starts_fresh`).

use super::chain_walk::{ChainWalk, Link};
use super::single_lane::{Stream, keyed_streams};
use super::*;
use al_builtins::assumptions::Assumption;

/// One member of a walk, as `ChainWalk::walk` takes it: (suite, hub leg as 0 or 1, codeunit id, what to do).
pub(super) type Item = (usize, u8, u32, Link);

/// A unit that leaves nothing for the later codeunits: its isolation rolls its tests back and its OnRun cannot write
/// (`Unit::carry` is the static analysis of that, `chain::carry_why`).
pub(super) fn leaves_nothing(u: &Unit) -> bool {
    u.isolation != TestIsolation::Disabled && u.carry.is_none()
}

/// Whether the run of a unit ends its BC session's exactness: a test was refused while it ran or stopped by the time limit,
/// or no test of it ran at all (it was refused as a whole, or panicked). BC ran what the runtime could not, so what the
/// session holds afterwards is not BC's.
pub(super) fn stops_session(r: &UnitResult) -> bool {
    match &r.result {
        Err(_) => true,
        Ok(done) => {
            done.5.refused_mid_run
                || (!done.0.is_empty() && done.0.iter().all(|(_, o)| matches!(o, Outcome::NotRun(_))))
        }
    }
}

/// Marks every test of the unit's run: it started from state that is not BC's.
pub(super) fn flag_after_refused(r: &mut UnitResult) {
    if let Ok(done) = &mut r.result {
        super::single_lane::flag_all(done, Assumption::StrictAfterRefused);
    }
}

/// Whether the walk's slot holds single instances to hand to the next unit (`run.rs` `LANE_SINGLES`).
pub(super) fn singles_held() -> bool {
    super::run::LANE_SINGLES.with(|c| matches!(*c.borrow(), Some(Some(_))))
}

/// A unit of a sequential walk ran (`r`): flags it when it can read what the stopped or flagged units before it could have
/// changed (`strict_taint.rs`), and notes it there when it stopped the session or was flagged. `dropped`: the unit was
/// handed single instances and handed none on ([`singles_held`] before and after its run). That happens when it panicked,
/// when a test of it panicked (`Scratch::hands_on_none`), and when it was refused as a whole after its session started (the
/// early returns of `run.rs` `run_codeunit_inner`; second review, findings A and B): every later unit is flagged.
pub(super) fn after_run(
    taint: &mut super::strict_taint::Taint,
    engine: &Arc<al_vm::Engine>,
    lowered: &al_sema::lower::Lowered,
    u: &Unit,
    r: &mut UnitResult,
    dropped: bool,
) {
    let stops = stops_session(r) || dropped;
    if !stops && !taint.active() {
        return;
    }
    let facts = super::strict_taint::unit_facts(engine, lowered, u.oi);
    let flagged = taint.active() && taint.reaches(&facts, lowered);
    if flagged {
        flag_after_refused(r);
    }
    if stops || flagged {
        taint.add(&facts, !leaves_nothing(u));
    }
    // After the unit's own check: a unit does not flag itself.
    if dropped {
        taint.singles_lost();
    }
}

/// The walks of a run, one per BC session: each `Item` list is in BC's order. `refused` are the codeunits with
/// RequiredTestIsolation = Disabled that are not run (`Link::Refused`), at their place in the hub leg of their suite.
pub(super) fn groups(
    units: &[Unit],
    place: &dyn Fn(usize, u32) -> u32,
    suites: Option<&crate::test_suites::TestSuites>,
    refused: &[(usize, u32, usize, String)],
) -> Vec<Vec<Item>> {
    struct Group {
        suite: usize,
        si: usize,
        hub: Vec<(u64, Item)>,
        rest: Vec<(u64, Item)>,
    }
    let mut out: Vec<Group> = Vec::new();
    for Stream {
        suite,
        si,
        hub_leg,
        members,
    } in keyed_streams(units, place, &|_| true, suites)
    {
        let alone = suite == usize::MAX;
        if alone || !out.last().is_some_and(|g| (g.suite, g.si) == (suite, si)) {
            out.push(Group {
                suite,
                si,
                hub: Vec::new(),
                rest: Vec::new(),
            });
        }
        let g = out.last_mut().expect("a group");
        let items = members
            .into_iter()
            .map(|(p, k)| (p, (units[k].si, u8::from(hub_leg), units[k].id, Link::Run(k))));
        if hub_leg {
            g.hub.extend(items)
        } else {
            g.rest.extend(items)
        }
    }
    // A codeunit that is not run sits in the hub leg of each session that lists it (without a suites file: the run's).
    let index = suites.map(|s| s.index());
    for (si, id, oi, why) in refused {
        let places: Vec<(usize, u64)> = match &index {
            None => vec![(0, u64::from(place(*si, *id)))],
            Some(ix) => ix
                .get(id)
                .map(|v| v.iter().map(|&(s, p)| (s, p as u64)).collect())
                .unwrap_or_default(),
        };
        for (suite, p) in places {
            if let Some(g) = out.iter_mut().find(|g| (g.suite, g.si) == (suite, *si)) {
                g.hub.push((p, (*si, 1, *id, Link::Refused(*oi, why.clone()))));
            }
        }
    }
    out.into_iter()
        .map(|mut g| {
            g.hub.sort_by_key(|(p, _)| *p);
            g.hub.into_iter().chain(g.rest).map(|(_, item)| item).collect()
        })
        .collect()
}

/// What one walk found.
#[derive(Default)]
struct WalkOut {
    done: Vec<(usize, UnitResult)>,
    details: std::collections::HashMap<(usize, u32, String), explain::TestDetail>,
    report: ChainReport,
}

/// The sequential strict run of a pass.
pub(super) struct StrictRun<'a> {
    pub walk: ChainWalk<'a>,
    pub place: &'a dyn Fn(usize, u32) -> u32,
    pub suites: Option<&'a crate::test_suites::TestSuites>,
    pub refused: &'a [(usize, u32, usize, String)],
    /// BcContainerHelper's discovery pass of each BC session that has one (ADR-074): the first member of its walk.
    pub discovery: &'a [Item],
    /// The run's test threads (1: all walks on this thread).
    pub workers: usize,
    /// Threads other than this one can start from their own copy of the install baseline.
    pub copies: bool,
    pub snapshot: &'a super::install_snapshot::InstallSnapshots,
    pub thread_baseline:
        &'a (dyn Fn(Option<&super::install_snapshot::InstallSnapshot>) -> Option<al_vm::Baseline> + Sync),
}

impl StrictRun<'_> {
    /// Runs the walks, in parallel while threads and memory allow, and puts each unit's result in `slots` (a codeunit that
    /// several suites list is walked in each, reported from the first), its detail in `details`, the members in `report`.
    pub(super) fn run(
        &self,
        slots: &mut [Option<UnitResult>],
        details: &mut std::collections::HashMap<(usize, u32, String), explain::TestDetail>,
        report: &mut ChainReport,
    ) {
        let mut groups = groups(self.walk.units, self.place, self.suites, self.refused);
        for g in &mut groups {
            if let Some(d) = g.first().and_then(|f| self.discovery.iter().find(|d| d.0 == f.0)) {
                g.insert(0, (d.0, d.1, d.2, d.3.clone()));
            }
        }
        // The longest walks first: the wall time is then close to the longest session's.
        let mut order: Vec<usize> = (0..groups.len()).collect();
        order.sort_by_key(|&g| std::cmp::Reverse(groups[g].len()));
        // The closure captures only what threads may share (a baseline stays on its thread).
        let w = &self.walk;
        let (engine, lowered, units, trace, limit, pool) = (w.engine, w.lowered, w.units, w.trace, w.limit, w.pool);
        let run = |b: Option<&al_vm::Baseline>, i: usize| -> WalkOut {
            let mut out = WalkOut::default();
            let walk = ChainWalk {
                engine,
                lowered,
                baseline: b,
                units,
                trace,
                limit,
                pool,
                keep_singles: true,
                fresh_check: false,
                sequential: true,
            };
            let mut take = |k: usize, r: UnitResult| out.done.push((k, r));
            let mut local = ChainReport::default();
            let mut det = std::collections::HashMap::new();
            walk.walk(clone_items(&groups[order[i]]), &mut take, &mut det, &mut local);
            out.details = det;
            out.report = local;
            out
        };
        let first = self.walk.baseline;
        let outs: Vec<WalkOut> = if self.workers > 1 && groups.len() > 1 && self.copies {
            let snapshot = self.snapshot.get_or_make(first);
            let own = || (self.thread_baseline)(snapshot);
            super::threads::run_parallel(self.walk.pool, groups.len(), first, &own, &run)
        } else {
            (0..groups.len()).map(|i| run(first, i)).collect()
        };
        // Back in the order of the walks, so a codeunit in two suites is reported from the first.
        let mut by_group: Vec<Option<WalkOut>> = (0..groups.len()).map(|_| None).collect();
        for (i, o) in outs.into_iter().enumerate() {
            by_group[order[i]] = Some(o);
        }
        for o in by_group.into_iter().flatten() {
            for (k, r) in o.done {
                slots[k].get_or_insert(r);
            }
            for (key, d) in o.details {
                details.entry(key).or_insert(d);
            }
            report.absorb(o.report);
        }
    }
}

fn clone_items(items: &[Item]) -> Vec<Item> {
    items.iter().map(|(a, b, c, l)| (*a, *b, *c, l.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::super::{Ends, Scratch};

    fn scratch(past_stop: bool, refused_mid_run: bool, test_panicked: bool) -> Scratch {
        let mut s = Scratch {
            singles_past_stop: past_stop,
            ..Default::default()
        };
        s.note_ended(Ends {
            unfinished_after_start: refused_mid_run || test_panicked,
            refused_mid_run,
            test_panicked,
        });
        s
    }

    /// Second review of the draft (finding B): in the sequential walk the single instances move on past a refusal or a
    /// time-limit stop, but not past a panic in a test method; the lane hands nothing on after either.
    #[test]
    fn a_panic_in_a_test_drops_the_single_instances_in_the_sequential_walk() {
        assert!(!scratch(true, true, false).hands_on_none(), "a refusal: they move on");
        assert!(scratch(true, true, true).hands_on_none(), "a panic: they are dropped");
        assert!(scratch(false, true, false).hands_on_none(), "the lane, R3");
        assert!(!scratch(true, false, false).hands_on_none());
    }
}
