//! What the body of a guarded `Codeunit.Run` did, for the stale-buffer rule after its rollback (al-runner issue 7795).
//!
//! The corpus measures a write through the caller's buffer after one failed guarded run only: a static
//! `Codeunit.Run(Id, Rec)` of a TableNo codeunit whose OnRun changes fields of Rec, calls `Rec.Modify()`, then `Commit()`,
//! then `Error('literal')` (record/crud/CodeunitRunTableNoWrites.al:69 CRT Runner, :205 Guarded_ModifyCommitError_ThenHandler,
//! :267 Guarded_ModifyCommitError_ThenModify; BC 27.5, 28.5, 29.0). Each guarded run that is running holds a `GuardedBody`
//! (`TxState::guarded_bodies`) that follows these steps. A record method, a Commit, a `Codeunit.Run` or an asserterror that is
//! not the next pinned step makes the body `Other`, and the rollback of such a run is one the stale-buffer rule does not
//! know (`stale_rule.rs` refuses a write through a stale buffer after it, as before the pin).

use super::*;

/// How far the body of a guarded run got along the pinned steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// No record method, Commit or run yet.
    Start,
    /// `Rec.Modify()` wrote the row of the record passed to the run.
    Modified,
    /// `Commit()` after that Modify, and nothing since.
    Committed,
    /// Anything else.
    Other,
}

/// The body of one guarded `Codeunit.Run` that is running.
#[derive(Debug)]
pub(crate) struct GuardedBody {
    /// The record a static `Codeunit.Run(Id, Rec)` passed (its address); `None` for the instance form or without a record.
    rec: Option<usize>,
    step: Step,
}

impl Session {
    /// A `Codeunit.Run` begins its OnRun: every guarded run around it leaves the pinned steps, and a guarded one gets a body of
    /// its own. `static_form` is `Codeunit.Run(Id, ...)`; `rec` is the Record value it passed.
    pub(crate) fn guarded_body_begin(&mut self, guarded: bool, static_form: bool, rec: Option<&Value>) {
        self.guarded_bodies_leave_pin();
        if guarded {
            let rec = match rec {
                Some(Value::Record(rc)) if static_form && !rc.borrow().temporary => Some(Rc::as_ptr(rc) as usize),
                _ => None,
            };
            self.tx.guarded_bodies.push(GuardedBody { rec, step: Step::Start });
        }
    }

    /// A record method of `rec` just ended with `result` (`record_op_unprofiled` calls this after every record method).
    #[inline]
    pub(crate) fn guarded_body_record_op(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &[Value],
        result: &Result<Value, Signal>,
    ) {
        if self.tx.guarded_bodies.is_empty() {
            return;
        }
        self.guarded_body_record_op_cold(method, rec, args, result);
    }

    #[cold]
    fn guarded_body_record_op_cold(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &[Value],
        result: &Result<Value, Signal>,
    ) {
        let Some(body) = self.tx.guarded_bodies.last_mut() else {
            return;
        };
        // `Rec.Modify()` (CodeunitRunTableNoWrites.al:75): no RunTrigger argument, on the record the run got, written.
        let pinned_modify = method == RecordOp::Modify
            && args.is_empty()
            && result.is_ok()
            && body.step == Step::Start
            && body.rec == Some(Rc::as_ptr(rec) as usize);
        body.step = if pinned_modify { Step::Modified } else { Step::Other };
    }

    /// `Commit` (`commit_transaction`): the pinned step after the Modify (CodeunitRunTableNoWrites.al:77).
    pub(crate) fn guarded_body_commit(&mut self) {
        if let Some(body) = self.tx.guarded_bodies.last_mut() {
            body.step = if body.step == Step::Modified {
                Step::Committed
            } else {
                Step::Other
            };
        }
    }

    /// A `Codeunit.Run` or an asserterror inside a guarded run: no pinned body has one.
    pub(crate) fn guarded_bodies_leave_pin(&mut self) {
        for body in &mut self.tx.guarded_bodies {
            body.step = Step::Other;
        }
    }

    /// The rollback of a failed guarded `Codeunit.Run` whose run raised `e` and whose body was `body` (`codeunit_run`). Only a
    /// run with the pinned body, ended by an `Error` with a literal text (:79), as the first rollback of its transaction, is a
    /// rollback the stale-buffer rule knows.
    pub(crate) fn guarded_run_rollback(
        &mut self,
        e: &al_builtins::native::AlError,
        body: Option<GuardedBody>,
    ) -> Result<(), Signal> {
        let pinned = body.is_some_and(|b| b.rec.is_some() && b.step == Step::Committed)
            && e.exception == al_builtins::native::ExceptionKind::Dialog
            && !e.message.is_empty()
            && !self.tx.rolled_back;
        self.trapped_error_rollback(e, TrappedRollback::GuardedRun)?;
        if !pinned {
            self.tx.rollback_unpinned = self.tx.rollback_unpinned.or(Some(
                "after a failed guarded Codeunit.Run other than a static Codeunit.Run(Id, Rec) whose run did Rec.Modify(), Commit and Error with a literal text",
            ));
        }
        Ok(())
    }
}
