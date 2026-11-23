//! The tables `Record.Consistent(false)` marks in the open transaction, and what ends such a transaction.
//!
//! A mark belongs to the record variable that set it. `Consistent(true)` on that variable lifts it
//! (errors/CuCommitTests.al:393-401, :422-433). `Consistent(true)` on another variable of the table does not: a guarded
//! `Codeunit.Run` after it still raises the inconsistency error (errors/ConsistentGuardedRun.al:272-289). What a Commit
//! does after that is not pinned, nor is any other call on a mark by another variable.
//!
//! the tables `Record.Consistent(false)` marks, per record variable, and the inconsistency error a
//! guarded `Codeunit.Run` raises as it ends. Entry points: `Session::set_consistent`,
//! `guarded_run_end_marks`.

use crate::Session;
use crate::records::Rec;
use al_builtins::native::Signal;
use al_builtins::value::RecordState;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// How the variables of a table have called `Consistent` on its mark.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MarkState {
    /// Only the variable that set the mark called `Consistent` on it.
    Marked,
    /// Another variable of the table called `Consistent(true)` once, the variable that set it nothing since.
    ReleasedByAnother,
    /// Any other sequence of calls by another variable: not pinned.
    Unmeasured,
}

/// One table `Consistent(false)` marked in the open transaction.
#[derive(Clone, Debug)]
pub(crate) struct ConsistentMark {
    /// Object index of the table.
    pub(crate) table: u32,
    /// The record variable that set the mark (gone when that variable is).
    by: Weak<RefCell<RecordState>>,
    pub(crate) state: MarkState,
}

impl Session {
    /// `Record.Consistent(Value)` on `rec`, a database table: marks, lifts or changes the mark of its table for the open
    /// transaction (errors/CuCommitTests.al:382-433). A temporary record is not pinned.
    pub(crate) fn set_consistent(&mut self, rec: &Rec, consistent: bool) -> Result<(), Signal> {
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table, r.temporary)
        };
        if temporary {
            return Err(Signal::Unsupported(
                "Consistent on a temporary record (not pinned)".into(),
            ));
        }
        let Some(i) = self.tx.open.inconsistent.iter().position(|m| m.table == table) else {
            if !consistent {
                self.tx.open.inconsistent.push(ConsistentMark {
                    table,
                    by: Rc::downgrade(rec),
                    state: MarkState::Marked,
                });
            }
            return Ok(());
        };
        let mark = &mut self.tx.open.inconsistent[i];
        let same = mark.by.upgrade().is_some_and(|b| Rc::ptr_eq(&b, rec));
        match (mark.state, same, consistent) {
            (MarkState::Marked, true, true) => {
                self.tx.open.inconsistent.remove(i);
            }
            (MarkState::Marked, true, false) => {}
            (MarkState::Marked, false, true) => mark.state = MarkState::ReleasedByAnother,
            _ => mark.state = MarkState::Unmeasured,
        }
        Ok(())
    }

    /// The marks a guarded `Codeunit.Run` starts with, to compare with the marks it ends with.
    pub(crate) fn marks_at_run_start(&self) -> Vec<(u32, MarkState)> {
        self.tx.open.inconsistent.iter().map(|m| (m.table, m.state)).collect()
    }

    /// The inconsistency error a guarded `Codeunit.Run` raises as it ends, which the guard does not trap
    /// (errors/ConsistentGuardedRun.al). Pinned: one table marked before the run, by the caller (and released or not by
    /// another variable before it), and no change to the marks during the run, whether the run succeeds or fails with an
    /// `Error` (:170-236, :272-289); or no mark before the run and one the inner codeunit set and kept, when the run
    /// succeeds (:238-252). An inner codeunit that lifts its own mark leaves none (:254-270).
    /// `None` when no mark is there before or after. Any other shape is refused. The error is the one of a Commit
    /// ([`Session::inconsistent_error`]): the pin reads its text (:181); its code is the Commit one (CuCommitTests.al:416),
    /// which no test reads after a guarded run.
    pub(crate) fn guarded_run_end_marks(
        &self,
        before: &[(u32, MarkState)],
        succeeded: bool,
    ) -> Result<Option<Signal>, Signal> {
        let after = self.marks_at_run_start();
        if before.is_empty() && after.is_empty() {
            return Ok(None);
        }
        let caller_marked =
            before == after.as_slice() && matches!(before, [(_, MarkState::Marked | MarkState::ReleasedByAnother)]);
        let inner_marked = before.is_empty() && succeeded && matches!(after.as_slice(), [(_, MarkState::Marked)]);
        if caller_marked || inner_marked {
            return Ok(Some(self.inconsistent_error(after[0].0 as usize)));
        }
        self.refuse_inconsistent_end("A guarded Codeunit.Run").map(|()| None)
    }

    /// The Commit of a transaction in which another variable of a marked table called `Consistent`: not pinned.
    pub(crate) fn refuse_commit_after_other_variable(&self) -> Result<(), Signal> {
        match self.tx.open.inconsistent.first() {
            Some(m) if m.state != MarkState::Marked => Err(Signal::unsupported(format!(
                "Commit after another variable of table {} called Consistent on its Consistent(false) mark (not pinned)",
                self.engine.program.objects[m.table as usize].name
            ))),
            _ => Ok(()),
        }
    }
}
