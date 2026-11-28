//! ADR-058 amendment 1: order dependence through SingleInstance state, for stability mode and `--timings`.
//!
//! A run knows which test codeunits read a SingleInstance global that an earlier codeunit of their BC session left with
//! another value than a fresh instance holds: the strict walk notes it per member (`chain_walk.rs` `KeptSingles`,
//! [`carried_reads`]), the default run's lane per codeunit (`single_lane.rs` `Found`). This module names what they read
//! ([`CarriedCodeunit`], [`name_reads`]); `CorpusRuntime::single_carried` lists both. ADR-070 decision 7: the stability
//! dimension `isolation.singleInstance` compares two plain runs (the `fresh` variant is the default isolated run, every
//! other variant runs in BC's order, `corpus.rs` `try_run_variants`); nothing runs again. Entry points: `CarriedCodeunit`,
//! `single_carried`.

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
}
