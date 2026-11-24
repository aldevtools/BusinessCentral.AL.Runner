//! The tables `Record.Consistent(false)` marks in the open transaction, and what ends such a transaction: a Commit, a
//! guarded `Codeunit.Run`, and the inconsistency error both raise.
//!
//! A mark belongs to the record variable that set it. `Consistent(true)` on that variable lifts it
//! (errors/CuCommitTests.al:393-401, :422-433). `Consistent(true)` on another variable of the table does not: a guarded
//! `Codeunit.Run` after it still raises the inconsistency error (errors/ConsistentGuardedRun.al:272-289).
//!
//! Beyond those pins, the shapes of corpus PR 1568 (errors/ConsistentGuardedRunEnd.al, codeunit 75788; the cells its CI
//! confirmed on BC 27.5, 28.5 and 29.0) run through the assumption `guarded-run-consistent-end`: a guarded run that fails
//! drops the marks set inside it, a temporary record marks the database table, a Commit after another variable's
//! `Consistent(true)` raises, and the error names a table by its Caption. Every other shape is refused.
//!
//! Entry points: `Session::set_consistent`, `marks_at_run_start` and `guarded_run_end_marks` (by `codeunit_run`),
//! `commit_mark_error` (by `commit_builtin`), `inconsistent_error`.

use crate::Session;
use crate::records::Rec;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, Value};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// How the variables of a table have called `Consistent` on its mark.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MarkState {
    /// Only the variable that set the mark called `Consistent` on it.
    Marked,
    /// Another database variable of the table called `Consistent(true)` once, the variable that set it nothing since.
    ReleasedByAnother,
    /// Any other sequence of calls by another variable: not measured.
    Unmeasured,
}

/// One table `Consistent(false)` marked in the open transaction.
#[derive(Clone, Debug)]
pub(crate) struct ConsistentMark {
    /// Object index of the table.
    pub(crate) table: u32,
    /// The record variable that set the mark (gone when that variable is).
    by: Weak<RefCell<RecordState>>,
    /// Whether that variable is a temporary record.
    temporary: bool,
    pub(crate) state: MarkState,
}

/// The record a guarded `Codeunit.Run` hands to OnRun, as the end-of-run check reads it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RunArg {
    /// `Codeunit.Run(Id)` or `CodeunitVar.Run()`.
    None,
    /// A Record of the table with this object index.
    Record { table: u32, temporary: bool },
    /// Anything else (a Variant that holds a RecordRef).
    Other,
}

impl RunArg {
    /// The argument of a run whose record argument is `v`.
    pub(crate) fn of(v: Option<&Value>) -> RunArg {
        match v {
            None => RunArg::None,
            Some(Value::Record(r)) => {
                let r = r.borrow();
                RunArg::Record {
                    table: r.table,
                    temporary: r.temporary,
                }
            }
            Some(_) => RunArg::Other,
        }
    }
}

/// How a guarded `Codeunit.Run` ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RunEnd<'a> {
    Succeeded,
    /// OnRun raised an error with this code, which the guard traps.
    Failed(&'a str),
}

/// The codes of the inner errors after which a guarded run that started with no mark returns false and the marks set
/// inside it are gone (ConsistentGuardedRunEnd.al, GuardedRunId_InnerStartsStateAndFails `Error('inner failed')`,
/// GuardedRunId_InnerStartsStateAndErrorsEmpty `Error('')`, GuardedRunId_InnerStartsStateAndGetFails,
/// GuardedRunId_InnerStartsStateAndTestFieldFails).
const DROPPED_BY: [&str; 3] = ["Dialog", "DB:RecordNotFound", "TestField"];

impl Session {
    /// `Record.Consistent(Value)` on `rec`: marks, lifts or changes the mark of its table for the open transaction
    /// (errors/CuCommitTests.al:382-433). A temporary record marks the database table, and only `Consistent(true)` on
    /// the same temporary variable lifts it (ConsistentGuardedRunEnd.al Commit_CallerMarksTemporary,
    /// Commit_TemporaryMarkedThenUnmarked, Commit_TemporaryMarked_DatabaseVariableUnmarks).
    pub(crate) fn set_consistent(&mut self, rec: &Rec, consistent: bool) -> Result<(), Signal> {
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table, r.temporary)
        };
        if temporary {
            assume(Assumption::GuardedRunConsistentEnd);
        }
        let Some(i) = self.tx.open.inconsistent.iter().position(|m| m.table == table) else {
            if !consistent {
                self.tx.open.inconsistent.push(ConsistentMark {
                    table,
                    by: Rc::downgrade(rec),
                    temporary,
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
            (MarkState::Marked, false, true) if !temporary => mark.state = MarkState::ReleasedByAnother,
            // Consistent(true) on the database variable that set the mark, after another one's: lifted
            // (ConsistentGuardedRunEnd.al Commit_FalseOnOneVariable_TrueOnAnother_ThenTrueOnFirst).
            (MarkState::ReleasedByAnother, true, true) if !mark.temporary => {
                assume(Assumption::GuardedRunConsistentEnd);
                self.tx.open.inconsistent.remove(i);
            }
            _ => mark.state = MarkState::Unmeasured,
        }
        Ok(())
    }

    /// The marks a guarded `Codeunit.Run` starts with, to compare with the marks it ends with.
    pub(crate) fn marks_at_run_start(&self) -> Vec<(u32, MarkState)> {
        self.tx.open.inconsistent.iter().map(|m| (m.table, m.state)).collect()
    }

    /// What a guarded `Codeunit.Run` does with the marks as it ends: `Some` the inconsistency error, which the guard
    /// does not trap; `None` nothing (no mark, or the marks set inside a failed run, which its rollback drops). `before`
    /// are the marks at the start, `arg` the record argument, `nested` whether another guarded run is still running.
    ///
    /// Pinned (errors/ConsistentGuardedRun.al), without a record argument: one table marked before the run by the caller
    /// (and released or not by another variable before it), unchanged by the run, which succeeds or fails with an `Error`
    /// (:170-236, :272-289); or no mark before the run and one the inner codeunit set and kept, when the run succeeds
    /// (:238-252). The error is the one of a Commit: the pin reads its text (:181).
    ///
    /// Assumed (`guarded-run-consistent-end`, ConsistentGuardedRunEnd.al), outside another guarded run unless noted:
    /// - no mark before, a failed run with a code of [`DROPPED_BY`], every mark left held by the variable that set it:
    ///   nothing, also inside another guarded run (GuardedRunId_InnerMarks*AndFails, GuardedRunInstance*_*AndFails,
    ///   Preview_OuterGuardedRun_InnerStartsStateAndFails);
    /// - no mark before, a successful run that leaves marks held or released by another variable: the error naming the
    ///   table marked first (GuardedRunId_InnerMarksRowThenOther_Succeeds, _InnerMarksOtherThenRow_Succeeds,
    ///   _InnerStartsAndStopsState_Succeeds, _InnerMarksTemporary_Succeeds, GuardedRunIdRec_InnerStartsState_Succeeds);
    /// - the pinned caller shape with a record argument (GuardedRunIdRec_CallerMarks_InnerFails, _InnerSucceeds,
    ///   GuardedRunInstanceRec_CallerMarks_InnerFails).
    ///
    /// A record argument is measured only as a database record of a table no mark names. Any other shape is refused.
    pub(crate) fn guarded_run_end_marks(
        &self,
        before: &[(u32, MarkState)],
        end: RunEnd,
        arg: RunArg,
        nested: bool,
    ) -> Result<Option<Signal>, Signal> {
        let after = self.marks_at_run_start();
        if before.is_empty() && after.is_empty() {
            return Ok(None);
        }
        let caller_unchanged =
            before == after.as_slice() && matches!(before, [(_, MarkState::Marked | MarkState::ReleasedByAnother)]);
        let caller_shape = match end {
            RunEnd::Succeeded => caller_unchanged,
            RunEnd::Failed(code) => caller_unchanged && code == "Dialog",
        };
        let inner_kept = before.is_empty() && end == RunEnd::Succeeded;
        if arg == RunArg::None && (caller_shape || (inner_kept && matches!(after.as_slice(), [(_, MarkState::Marked)])))
        {
            return Ok(Some(self.inconsistent_error(after[0].0 as usize)));
        }
        let arg_measured = match arg {
            RunArg::None => true,
            RunArg::Record { table, temporary } => !temporary && !after.iter().any(|(t, _)| *t == table),
            RunArg::Other => false,
        };
        let held = |states: &[MarkState]| after.iter().all(|(_, s)| states.contains(s));
        let dropped = before.is_empty()
            && matches!(end, RunEnd::Failed(code) if DROPPED_BY.contains(&code))
            && held(&[MarkState::Marked]);
        let raised =
            !nested && (caller_shape || (inner_kept && held(&[MarkState::Marked, MarkState::ReleasedByAnother])));
        if arg_measured && (dropped || raised) {
            assume(Assumption::GuardedRunConsistentEnd);
            return Ok(raised.then(|| self.inconsistent_error(after[0].0 as usize)));
        }
        Err(Signal::unsupported(format!(
            "A guarded Codeunit.Run ends while Consistent(false) marks table {} in a shape no probe measured (marks, outcome, \
             record argument or nesting)",
            self.engine.program.objects[after.first().or(before.first()).map_or(0, |m| m.0) as usize].name
        )))
    }

    /// The error of a Commit while a mark is set, `None` without one (errors/CuCommitTests.al:405-419). After
    /// `Consistent(true)` on another variable the Commit raises it too (ConsistentGuardedRunEnd.al
    /// Commit_FalseOnOneVariable_TrueOnAnother, Commit_StateStartedAndStopped); any other call by another variable is
    /// refused.
    pub(crate) fn commit_mark_error(&self) -> Option<Signal> {
        let m = self.tx.open.inconsistent.first()?;
        match m.state {
            MarkState::Marked => {}
            MarkState::ReleasedByAnother => assume(Assumption::GuardedRunConsistentEnd),
            MarkState::Unmeasured => {
                return Some(Signal::unsupported(format!(
                    "Commit after another variable of table {} called Consistent on its Consistent(false) mark (not measured)",
                    self.engine.program.objects[m.table as usize].name
                )));
            }
        }
        Some(self.inconsistent_error(m.table as usize))
    }

    /// The inconsistency error of table `table` (errors/CuCommitTests.al:405-419, code DB:TableInConsistent). It names
    /// the table by its own Caption (ConsistentGuardedRunEnd.al Commit_CallerMarksCaptioned,
    /// GuardedRunId_InnerMarksCaptioned_Succeeds); a caption a tableextension or a translation sets is refused.
    pub(crate) fn inconsistent_error(&self, table: usize) -> Signal {
        let t = &self.engine.program.objects[table];
        if self.caption_refused(table).is_some() || t.properties.iter().any(|(k, _)| k == al_ir::EXT_CAPTION_OVER_OWN) {
            return Signal::unsupported(format!(
                "the inconsistency error of table {}, whose caption a tableextension or a translation sets (not measured)",
                t.name
            ));
        }
        let caption = self.table_caption(table);
        if caption != t.name {
            assume(Assumption::GuardedRunConsistentEnd);
        }
        crate::transactions::al_error_code(
            &format!(
                "The transaction cannot be completed because it will cause inconsistencies in the {caption} table. Check where and how \
                 the CONSISTENT function is used in the transaction to find the reason for the error.\\\\Contact your system \
                 administrator.\\\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This \
                 prevents data from being updated incorrectly."
            ),
            "DB:TableInConsistent",
        )
    }
}
