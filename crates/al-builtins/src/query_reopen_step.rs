//! Where a query variable stands in the one call sequence that queries/read-lifecycle/QueryClosedAfterLastRow.al:162-182 pins
//! for a second `Open` while rows are left. `al-vm` `query_reopen` owns the moves from step to step; this is the state
//! the variable carries.
//!
//! `ReopenStep`, the step a query variable reached in the pinned second-Open sequence (al-vm
//! `query_reopen` moves it; `Clear` refuses after the second Open).

use crate::native::{Signal, unsupported};

/// The step a query variable reached in the pinned sequence (QueryClosedAfterLastRow.al:42, :165-170).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ReopenStep {
    /// New or cleared, and nothing called on it yet (`Clear(Q)`, :42).
    #[default]
    Fresh,
    /// The calls so far are not the pinned sequence: every call takes the usual path.
    Off,
    /// `Open` as a statement (:165, :48).
    Opened(ReopenTrack),
    /// Then one `Read` whose result is used (:166, :69).
    ReadOne(ReopenTrack),
    /// Then one read of the first column (:70).
    ColumnRead(ReopenTrack),
    /// Then the second `Open` as a statement, while rows were left (:168).
    Reopened(ReopenTrack),
    /// Then one `SetRange` with one value on the second column (:169, :54).
    Ranged(ReopenTrack),
}

/// What every call of the sequence must share with the first `Open`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReopenTrack {
    /// The global variable the calls go through: the object index of the calling routine and the global slot.
    pub global: (usize, usize),
    /// The session's commit count at the first `Open`.
    pub commits: u64,
    /// The versions of the query's tables when the rows were read (set by the second `Open`).
    pub stamps: Vec<(u32, u64)>,
}

impl ReopenStep {
    /// The variable was opened again by the pinned second `Open` and the pinned end has not run yet: only the next pinned
    /// call is pinned.
    pub fn after_second_open(&self) -> bool {
        matches!(self, ReopenStep::Reopened(_) | ReopenStep::Ranged(_))
    }

    /// `Clear` of the variable: refused after the pinned second `Open` (nothing pins it there); before it, a value that
    /// still holds the old state leaves the sequence.
    pub fn cleared(&mut self) -> Result<(), Signal> {
        if self.after_second_open() {
            return unsupported(
                "Clear of a query opened again while rows were left, with no filter change (not pinned)",
            );
        }
        *self = ReopenStep::Off;
        Ok(())
    }
}
