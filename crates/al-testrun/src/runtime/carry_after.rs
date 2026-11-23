//! A codeunit that comes after a carry-chain member: what it watches, when what it found is refused, and where it
//! starts again. One owner for the walk (`chain_walk.rs`) and the re-run after the parallel part (`corpus.rs`).
//!
//! A later codeunit sees what a codeunit under Disabled isolation committed (QuCommitIsolation.al:711-760
//! QU_D11_NextCodeunitSees, RqIsolationModes.al:609-640 RQ_D11_NextCodeunitSees). What a member left when a test of it
//! did not pass or was not run, or when it changed isolated storage or a number sequence, is not known or not followed
//! (`run.rs` `carry_disabled`, [`Carried::unfinished`], [`Carried::storage_why`]): a later codeunit that reads it is
//! refused.

use super::run::Carried;
use super::*;
use std::collections::BTreeSet;

/// The tables a carry left rows in, for `Unit::leak`.
pub(super) type Leak = Option<Arc<BTreeSet<u32>>>;

/// A codeunit that comes after the chain member that left a carry (with what the members before it left): what of the
/// carry its run looks at and why that can be refused.
pub(super) struct After {
    /// The carry changed isolated storage or a number sequence, and the codeunit's reach can touch them: it counts as
    /// reading what the carry left, whatever its run read (`corpus.rs` `reaches_session_storage`).
    storage: bool,
    storage_why: Option<String>,
    unfinished: Option<String>,
    /// A codeunit outside the chain with handlers after a member on the TestRunnerHub ([`handlers_unpinned`]).
    handlers: Option<String>,
}

impl After {
    /// Codeunit `oi` after `carry`, which a member of hub leg `leg` left.
    pub(super) fn new(engine: &Arc<al_vm::Engine>, lowered: &Lowered, carry: &Carried, oi: usize, leg: u8) -> Self {
        After {
            storage: carry.everything && super::corpus::reaches_session_storage(engine, lowered, oi),
            storage_why: carry.storage_why.clone(),
            unfinished: carry.unfinished.clone(),
            handlers: handlers_unpinned(lowered, oi, leg),
        }
    }

    /// Whether the codeunit's reach can touch the storage or numbers the carry changed.
    pub(super) fn reaches_storage(&self) -> bool {
        self.storage
    }

    /// Whether run `r` read what the carry left: a table it left rows in, or the storage it changed.
    pub(super) fn read(&self, r: &UnitResult) -> bool {
        self.storage || r.result.as_ref().is_ok_and(|done| done.5.leak_hit)
    }

    /// Why what run `r` of a chain member found is refused: the storage the carry changed is not followed (when the
    /// member can touch it), or what the carry left is not known (when the run read it).
    pub(super) fn refusal(&self, r: &UnitResult) -> Option<&String> {
        let storage = if self.storage { self.storage_why.as_ref() } else { None };
        storage.or(if self.read(r) { self.unfinished.as_ref() } else { None })
    }

    /// [`After::refusal`] for a codeunit outside the chain whose run `r` read what the carry left; such a codeunit with
    /// handlers is refused after a hub member too ([`handlers_unpinned`]).
    pub(super) fn outside_refusal(&self, r: &UnitResult) -> Option<&String> {
        self.refusal(r).or(self.handlers.as_ref())
    }
}

/// The tables `carry` left rows in, for `Unit::leak`: a run notes `leak_hit` when it reads one.
pub(super) fn leak(carry: Option<&Carried>) -> Leak {
    carry.map(|c| Arc::new(c.tables.clone()))
}

/// Why a codeunit outside the chain with handlers is refused after a member on the TestRunnerHub (`leg` 0): BC's CI
/// runs a codeunit with handlers through another runner than the TestRunnerHub (QuCommitIsolation.al:11-13), so whether
/// it runs before or after the member is not pinned.
fn handlers_unpinned(lowered: &Lowered, oi: usize, leg: u8) -> Option<String> {
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
