//! ADR-058 amendment 1: order dependence through SingleInstance state, for stability mode.
//!
//! The base run already knows which test codeunits read a SingleInstance global that an earlier codeunit of their BC
//! session left with another value than a fresh instance holds (the "suspects"): the carry chain walk notes it per
//! member (`chain_walk.rs` `KeptSingles`), and the lane's analysis per codeunit outside the chain (`single_lane.rs`
//! `Found`). This module names what they read ([`CarriedCodeunit`]) and, when the run checks the dimension
//! `isolation.singleInstance` (`RunOptions::single_fresh_check`), holds each suspect's outcome with fresh
//! SingleInstance state:
//!
//! * a chain member runs once more, from the very database start the walk gave it, with fresh single instances; what
//!   that run leaves is dropped (decision 2: nothing is handed on);
//! * a dependent codeunit outside the chain costs no run: its parallel run started fresh, and the lane (strict order in
//!   the base variant, decision 3) puts its BC-order run in its place.

use al_vm::single_track::{Fp, GlobalKey};
use std::collections::BTreeMap;

/// A test codeunit that read SingleInstance state an earlier test codeunit of its BC session left.
#[derive(Debug, Clone, PartialEq)]
pub struct CarriedCodeunit {
    pub suite: usize,
    pub codeunit: u32,
    pub name: String,
    /// A member of the carry chain (its base result comes from the walk); false: a dependent codeunit outside it.
    pub in_chain: bool,
    /// Its test isolation as BC names it (`Disabled`, `Codeunit`, `Function`).
    pub isolation: String,
    /// Each global it read first with a carried value.
    pub reads: Vec<CarriedRead>,
    /// With `RunOptions::single_fresh_check`: the outcome of each of its tests with fresh SingleInstance state.
    pub fresh: Option<Vec<(String, crate::Outcome)>>,
}

/// One global a [`CarriedCodeunit`] read first with a value an earlier codeunit left.
#[derive(Debug, Clone, PartialEq)]
pub struct CarriedRead {
    /// The SingleInstance codeunit that holds the global.
    pub single_id: u32,
    pub single_name: String,
    /// The app that holds it: id, name and publisher.
    pub app_id: String,
    pub app: String,
    pub publisher: String,
    pub global: String,
    /// The earlier test codeunit that last wrote it (id, name); `None` when not known.
    pub last_writer: Option<(u32, String)>,
}

/// ADR-058 amendment 1, decision 2: the globals a chain member read first (`observed`, with what it saw) whose value an
/// earlier member of its BC session wrote last (`last`: the global and that member), where what it saw differs from
/// what a fresh instance holds (`fresh`; a value without a comparable form, or no fresh value known, counts as
/// different). Each with its last writer.
pub(super) fn carried_reads(
    observed: &BTreeMap<GlobalKey, Fp>,
    last: &BTreeMap<GlobalKey, usize>,
    fresh: &BTreeMap<GlobalKey, Fp>,
) -> Vec<(GlobalKey, usize)> {
    observed
        .iter()
        .filter_map(|(k, seen)| {
            let w = last.get(k)?;
            (fresh.get(k) != Some(seen)).then_some((*k, *w))
        })
        .collect()
}

/// Names the reads of a suspect: `reads` holds (global, last writer as a unit index).
pub(super) fn name_reads(
    lowered: &al_sema::lower::Lowered,
    units: &[super::Unit],
    reads: &[(GlobalKey, usize)],
) -> Vec<CarriedRead> {
    let objects = &lowered.program.objects;
    reads
        .iter()
        .map(|&((oi, g), w)| {
            let o = &objects[oi];
            let app = lowered.program.apps.get(o.app as usize);
            CarriedRead {
                single_id: o.id.unwrap_or(0),
                single_name: o.name.clone(),
                app_id: app.map(|a| a.id.clone()).unwrap_or_default(),
                app: app.map(|a| a.name.clone()).unwrap_or_default(),
                publisher: app.map(|a| a.publisher.clone()).unwrap_or_default(),
                global: o.globals.get(g).map(|s| s.name.clone()).unwrap_or_default(),
                last_writer: units.get(w).map(|u| (u.id, objects[u.oi].name.clone())),
            }
        })
        .collect()
}

/// A [`CarriedCodeunit`] for unit `k`.
pub(super) fn carried_codeunit(
    lowered: &al_sema::lower::Lowered,
    units: &[super::Unit],
    k: usize,
    in_chain: bool,
    reads: &[(GlobalKey, usize)],
) -> CarriedCodeunit {
    let u = &units[k];
    CarriedCodeunit {
        suite: u.si,
        codeunit: u.id,
        name: lowered.program.objects[u.oi].name.clone(),
        in_chain,
        isolation: isolation_name(u.isolation).to_string(),
        reads: name_reads(lowered, units, reads),
        fresh: None,
    }
}

/// The test isolation as BC names it in `RequiredTestIsolation`.
fn isolation_name(i: crate::isolation::TestIsolation) -> &'static str {
    match i {
        crate::isolation::TestIsolation::Disabled => "Disabled",
        crate::isolation::TestIsolation::Codeunit => "Codeunit",
        crate::isolation::TestIsolation::Function => "Function",
    }
}

/// The outcome of each test of a finished unit run (none for a run that panicked).
pub(super) fn outcomes_of(r: &super::UnitResult) -> Vec<(String, crate::Outcome)> {
    match &r.result {
        Ok(done) => done.0.clone(),
        Err(_) => Vec::new(),
    }
}

/// Runs chain member `u` once more from `start` with fresh single instances (decision 2), on the walk's thread, and
/// hands the walk its own single instances back. What the run leaves (rows, single instances) is dropped.
pub(super) fn rerun_fresh(
    engine: &std::sync::Arc<al_vm::Engine>,
    lowered: &al_sema::lower::Lowered,
    start: Option<&al_vm::Baseline>,
    u: &super::Unit,
    limit: Option<std::time::Duration>,
    pool: &super::memory::Pool,
) -> super::UnitResult {
    use super::run::{CARRIED, LANE_SINGLES};
    let kept = LANE_SINGLES.with(|c| c.borrow_mut().as_mut().and_then(Option::take));
    let running = pool.enter();
    let r = super::run_unit(engine, lowered, start, u, None, limit);
    drop(running);
    CARRIED.with(|c| c.borrow_mut().take());
    LANE_SINGLES.with(|c| {
        if let Some(slot) = c.borrow_mut().as_mut() {
            *slot = kept;
        }
    });
    r
}

impl super::CorpusRuntime {
    /// ADR-058 amendment 1: every test codeunit of the run that read SingleInstance state an earlier codeunit of its
    /// BC session left (chain members first, then the dependent codeunits outside the chain).
    pub fn single_carried(&self) -> impl Iterator<Item = &CarriedCodeunit> {
        self.phases
            .chain
            .carried
            .iter()
            .chain(self.phases.chain.single_lane.carried.iter())
    }
}

impl super::CorpusRuntime {
    /// The `isolation.singleInstance = fresh` variant from its carried twin's run, which ran with
    /// `RunOptions::single_fresh_check` (decision 2): each suspect takes its fresh outcome, every other test its outcome
    /// in the twin (it read nothing a reset would change).
    pub(super) fn fresh_variant(&self) -> super::CorpusRuntime {
        let mut results = self.results.clone();
        let mut outcomes = self.outcomes.clone();
        for c in self.single_carried() {
            for (test, o) in c.fresh.iter().flatten() {
                for r in results
                    .iter_mut()
                    .filter(|r| r.suite == c.suite && r.codeunit == c.codeunit && r.test.eq_ignore_ascii_case(test))
                {
                    r.outcome = o.clone();
                }
                if let Some(slot) = outcomes.get_mut(&(c.suite, c.codeunit, test.to_lowercase())) {
                    *slot = o.clone();
                }
            }
        }
        super::CorpusRuntime {
            outcomes,
            times: self.times.clone(),
            suites: self.suites.clone(),
            panics: self.panics,
            phases: super::Phases {
                compile: std::time::Duration::ZERO,
                ..self.phases.clone()
            },
            details: self.details.clone(),
            results,
            marks: self.marks.clone(),
            coverage_statements: Vec::new(),
            impact_keys: Vec::new(),
            impact_baseline: None,
            program_apps: self.program_apps.clone(),
            tdd_stubs: self.tdd_stubs.clone(),
            compile_errors: self.compile_errors.clone(),
            entries: self.entries.clone(),
            met: self.met,
        }
    }
}

/// For each variant of a stability run (`perturbs`), how it is run (ADR-058 amendment 1): `Some(j)`, a `fresh`
/// variant formed from the run of variant `j` (its carried twin, an earlier variant or itself run as carried); `None`,
/// run as is. The second list says which runs check the suspects with fresh state (the twins of `fresh` variants);
/// those also run in strict order (decision 3).
pub(super) fn fresh_plan(perturbs: &[crate::stability::Perturb]) -> (Vec<Option<usize>>, Vec<bool>) {
    let twin = |p: &crate::stability::Perturb| crate::stability::Perturb {
        single_fresh: false,
        ..p.clone()
    };
    let twins: Vec<crate::stability::Perturb> = perturbs.iter().filter(|p| p.single_fresh).map(twin).collect();
    let from = perturbs
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.single_fresh
                .then(|| perturbs[..i].iter().position(|q| *q == twin(p)).unwrap_or(i))
        })
        .collect();
    let check = perturbs.iter().map(|p| p.single_fresh || twins.contains(p)).collect();
    (from, check)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(b: u8) -> Fp {
        Fp::Val(vec![b].into())
    }

    #[test]
    fn a_carried_read_is_a_suspect_only_when_it_differs_from_a_fresh_instance() {
        let (a, b, c, d) = ((1, 0), (1, 1), (2, 0), (3, 0));
        let observed = BTreeMap::from([(a, fp(5)), (b, fp(0)), (c, Fp::Opaque("x".into())), (d, fp(9))]);
        // `d` was read but no earlier member wrote it; `b` holds what a fresh instance holds.
        let last = BTreeMap::from([(a, 7), (b, 8), (c, 9)]);
        let fresh = BTreeMap::from([(a, fp(0)), (b, fp(0)), (c, Fp::Opaque("x".into()))]);
        assert_eq!(carried_reads(&observed, &last, &fresh), vec![(a, 7), (c, 9)]);
        // No fresh value known: different.
        assert_eq!(carried_reads(&observed, &last, &BTreeMap::new()).len(), 3);
    }

    #[test]
    fn a_fresh_variant_is_formed_from_its_carried_twin_or_runs_as_its_own_twin() {
        use crate::stability::{MethodOrder, Perturb};
        let base = Perturb::default();
        let fresh = Perturb {
            single_fresh: true,
            ..Default::default()
        };
        let reverse = Perturb {
            method_order: MethodOrder::Reverse,
            ..Default::default()
        };
        let reverse_fresh = Perturb {
            single_fresh: true,
            ..reverse.clone()
        };
        let (from, check) = fresh_plan(&[base, reverse, fresh, reverse_fresh.clone()]);
        assert_eq!(from, [None, None, Some(0), Some(1)]);
        assert_eq!(check, [true, true, true, true]);
        // Without its twin among the variants, a fresh variant runs as its own twin; the others run without the check.
        let other = Perturb {
            alone: true,
            ..Default::default()
        };
        let (from, check) = fresh_plan(&[Perturb::default(), other, reverse_fresh]);
        assert_eq!(from, [None, None, Some(2)]);
        assert_eq!(check, [false, false, true]);
    }
}
