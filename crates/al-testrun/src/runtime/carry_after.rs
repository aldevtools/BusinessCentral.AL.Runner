//! A codeunit that comes after a carry-chain member: what it watches, when what it found is refused, and where it
//! starts again. One owner for the walk (`chain_walk.rs`), the re-run after the parallel part (`corpus.rs`) and the
//! SingleInstance lane (`single_lane.rs`).
//!
//! A later codeunit sees what a codeunit under Disabled isolation committed (QuCommitIsolation.al:711-760
//! QU_D11_NextCodeunitSees, RqIsolationModes.al:609-640 RQ_D11_NextCodeunitSees). What a member left when a test of it
//! did not pass or was not run, or when it changed isolated storage or a number sequence, is not known or not followed
//! (`run.rs` `carry_disabled`, [`Carried::unfinished`], [`Carried::storage_why`]): a later codeunit that reads it is
//! refused.

use super::run::Carried;
use super::*;

/// A codeunit that comes after the chain member that left `carry` (with what the members before it left).
pub(super) struct After<'a> {
    carry: &'a Carried,
    /// The carry changed isolated storage or a number sequence, and the codeunit's reach can touch them: it counts as
    /// reading what the carry left, whatever its run read (`corpus.rs` `reaches_session_storage`).
    storage: bool,
}

impl<'a> After<'a> {
    /// Codeunit `oi` after `carry`.
    pub(super) fn new(engine: &Arc<al_vm::Engine>, lowered: &Lowered, carry: &'a Carried, oi: usize) -> Self {
        let storage = carry.everything && super::corpus::reaches_session_storage(engine, lowered, oi);
        After { carry, storage }
    }

    /// Whether the codeunit's reach can touch the storage or numbers the carry changed.
    pub(super) fn reaches_storage(&self) -> bool {
        self.storage
    }

    /// Whether run `r` read what the carry left: a table it left rows in, or the storage it changed.
    pub(super) fn read(&self, r: &UnitResult) -> bool {
        read(self.storage, r)
    }

    /// Why what run `r` found is refused: the storage the carry changed is not followed (when the codeunit can touch
    /// it), or what the carry left is not known (when the run read it).
    pub(super) fn refusal(&self, r: &UnitResult) -> Option<&'a String> {
        let storage = if self.storage {
            self.carry.storage_why.as_ref()
        } else {
            None
        };
        storage.or(if self.read(r) {
            self.carry.unfinished.as_ref()
        } else {
            None
        })
    }

    /// What the carry left, as a start for a run again, when nothing of it is unknown or not followed.
    pub(super) fn clean_start(&self) -> Option<&'a al_vm::Baseline> {
        match (&self.carry.unfinished, &self.carry.storage_why, &self.carry.baseline) {
            (None, None, Some(b)) => Some(b),
            _ => None,
        }
    }

    /// What the carry left (`None`: no baseline was captured).
    pub(super) fn start(&self) -> Option<&'a al_vm::Baseline> {
        self.carry.baseline.as_ref()
    }
}

/// Whether run `r` read what a carry left: a table it left rows in, or (`storage`, [`After::reaches_storage`]) the
/// storage it changed. For a thread that holds no carry (`single_lane.rs` `Worker`).
pub(super) fn read(storage: bool, r: &UnitResult) -> bool {
    storage || r.result.as_ref().is_ok_and(|done| done.5.leak_hit)
}

/// The tables `carry` left rows in, for `Unit::leak`: a run notes `leak_hit` when it reads one.
pub(super) fn leak(carry: Option<&Carried>) -> Option<Arc<std::collections::BTreeSet<u32>>> {
    carry.map(|c| Arc::new(c.tables.clone()))
}

/// Why a codeunit outside the chain with handlers is refused after a member on the TestRunnerHub (`leg` 0): BC's CI
/// runs a codeunit with handlers through another runner than the TestRunnerHub (QuCommitIsolation.al:11-13), so whether
/// it runs before or after the member is not pinned.
pub(super) fn handlers_unpinned(lowered: &Lowered, oi: usize, leg: u8) -> Option<String> {
    let o = &lowered.program.objects[oi];
    let handlers = o.routines.iter().any(|r| {
        r.attributes
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("HandlerFunctions"))
    });
    (handlers && leg == 0).then(|| {
        "an earlier codeunit with RequiredTestIsolation = Disabled wrote a table this test reads, and whether BC runs \
         this codeunit, which has handlers, before or after it is not pinned"
            .to_string()
    })
}
