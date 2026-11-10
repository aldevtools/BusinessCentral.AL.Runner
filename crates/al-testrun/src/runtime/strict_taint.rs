//! Which codeunits of a sequential strict run are flagged `strict-after-refused` (ADR-070 step 2, narrowed by the owner on
//! 2026-11-10).
//!
//! Once a test of a BC session was refused while it ran, a time limit stopped it, no test of its codeunit ran, or its codeunit
//! panicked, the state the session holds is not BC's. A later codeunit is flagged only when its static reach can read
//! what the stopped codeunit's reach could have changed: [`ReachFacts::changes_read_by`], the analysis a failure after a
//! refused test uses (`run.rs`). Tables count only when the stopped codeunit's database changes stay (a Disabled or OnRun
//! carry member); under Codeunit isolation the rollback undoes them as it does on BC. A stopped codeunit whose reach is
//! not fully known may have changed anything, so every later codeunit is flagged. A flagged codeunit may have written
//! wrong values itself, so its reach joins what the later codeunits are checked against.

use super::written::{ReachFacts, ReachIndex, union_reach_facts_skipping};
use al_sema::lower::Lowered;
use std::collections::BTreeSet;
use std::sync::Arc;

/// What the stopped (and flagged) codeunits of one walk could have changed.
#[derive(Default)]
pub(super) struct Taint {
    /// Changes the rollback of the codeunit's isolation undoes in the database (tables do not count).
    rolled_back: Option<ReachFacts>,
    /// Changes that stay in the database (tables count).
    kept: Option<ReachFacts>,
}

impl Taint {
    /// Whether a codeunit stopped or was flagged before.
    pub fn active(&self) -> bool {
        self.rolled_back.is_some() || self.kept.is_some()
    }

    /// Notes a stopped or flagged codeunit whose reach is `facts`; `database_stays`: what it writes to tables stays.
    pub fn add(&mut self, facts: &ReachFacts, database_stays: bool) {
        let slot = if database_stays {
            &mut self.kept
        } else {
            &mut self.rolled_back
        };
        slot.get_or_insert_with(ReachFacts::default).absorb(facts);
    }

    /// Whether a codeunit whose reach is `later` can read what the noted codeunits could have changed.
    pub fn reaches(&self, later: &ReachFacts, lowered: &Lowered) -> bool {
        let settled = BTreeSet::new();
        let hit = |f: &Option<ReachFacts>, tables: bool| {
            f.as_ref()
                .is_some_and(|f| f.changes_read_by(later, tables, false, &settled, lowered).is_some())
        };
        hit(&self.rolled_back, false) || hit(&self.kept, true)
    }
}

/// The reach of everything test codeunit `oi` runs: its OnRun trigger and its test methods, in one walk.
pub(super) fn unit_facts(engine: &Arc<al_vm::Engine>, lowered: &Lowered, oi: usize) -> ReachFacts {
    let index = engine.derived(|| ReachIndex::new(&engine.events, lowered));
    let o = &lowered.program.objects[oi];
    let roots: Vec<(usize, usize)> = (0..o.routines.len())
        .filter(|&r| {
            let routine = &o.routines[r];
            (routine.is_trigger && routine.name.eq_ignore_ascii_case("OnRun"))
                || routine.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test"))
        })
        .map(|r| (oi, r))
        .collect();
    union_reach_facts_skipping(&engine.events, lowered, &index, &roots, &[])
}
