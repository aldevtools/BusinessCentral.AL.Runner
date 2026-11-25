//! What a rollback inside a run (an asserterror, a failed guarded `Codeunit.Run`, a report or XMLport that failed, a
//! background session) leaves of the number sequences the open transaction restarted or deleted.
//!
//! Corpus PR 1573 (open; assumption `existing-sequence-asserterror`), tests/al-language-isolation-probe/src/
//! SqExistingRestart.al, measured on BC 29.0 under runners 130450 and 61301: in a test codeunit under TestIsolation =
//! Codeunit, a sequence that existed when the tests began is restarted or deleted by a test whose earlier test wrote a row
//! and committed. An asserterror of an `Error` with a literal text after a Record write undoes the step at once, also in a
//! test that passes (RE :892, RE2 :976, RE5 :1224, DE :2487); with nothing reached since the step the asserterror keeps it
//! (RE3 :1059, RE4 :1142). Every other shape refuses the later calls on the sequence (`refuse_unmeasured_sequence`); the
//! runner's own rollbacks only end the noted changes. Entry points: `note_tx_change` (by `sessionops.rs`),
//! `note_rollback_by` (by `trapped_error_rollback`), `rollback_failed_run`, `settle_tx_changes` (by `note_tx_end`),
//! `clear_tx_changes`.

use super::{CommitBy, Interval, Key, TxPointKind};
use crate::sessionops::Sequence;
use crate::{Session, TxModel};
use al_builtins::assumptions::Assumption;
use al_builtins::native::{AlError, ExceptionKind};

/// A Restart or a Delete of a number sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SequenceStep {
    Restart,
    Delete,
}

/// A Restart or Delete of the open transaction ([`Session::note_tx_change`]).
#[derive(Debug, Clone)]
pub(crate) struct TxChange {
    step: SequenceStep,
    /// How many points the transaction log held then; `None`: no log was kept.
    at: Option<usize>,
    /// [`Session::data_access`] then.
    access: super::DataAccess,
    /// The sequence right after it (`None`: deleted).
    after: Option<Sequence>,
    /// The session had not restarted or deleted the sequence before (no `sequences_changed` entry).
    new_in_tx: bool,
    /// The transaction restarted or deleted the sequence more than once.
    repeated: bool,
}

/// What rolls back the open transaction inside a run ([`Session::note_rollback_by`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RollbackBy {
    /// An asserterror; `literal`: its body raised an `Error` with a literal text (`ExceptionKind::Dialog`).
    AssertError { literal: bool },
    /// A failed guarded `Codeunit.Run`.
    GuardedRun,
    /// A report, an XMLport import, a background session or a deferred insert of a guarded run that failed.
    OtherInRun,
}

/// What the rollback of an asserterror does to the sequence of one measured change ([`Session::measured_step_fate`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fate {
    /// The step stays (RE3, RE4).
    Keep,
    /// The sequence is back as it was before the step (RE, RE2, RE5, DE).
    Undo,
}

impl RollbackBy {
    /// The rollback of an asserterror (`assert`) or a failed guarded run whose body raised `e`.
    pub(crate) fn trapped(e: &AlError, assert: bool) -> RollbackBy {
        match assert {
            true => RollbackBy::AssertError {
                literal: e.exception == ExceptionKind::Dialog,
            },
            false => RollbackBy::GuardedRun,
        }
    }
}

impl Session {
    /// The open transaction restarted or deleted the sequence `key` (`step`); `new_in_tx`: the session had not done so
    /// before.
    pub(crate) fn note_tx_change(&mut self, key: Key, step: SequenceStep, new_in_tx: bool) {
        let at = self.access_notes.tx_log.as_ref().map(Vec::len);
        let (access, after) = (self.data_access(), self.sequences.get(&key).cloned());
        let notes = &mut self.access_notes;
        let repeated = notes.tx_changes.contains_key(&key) || notes.held.contains(&key);
        notes.tx_changes.insert(
            key,
            TxChange {
                step,
                at,
                access,
                after,
                new_in_tx,
                repeated,
            },
        );
    }

    /// The next rollback is one inside a run, by `by` (read by [`Session::settle_tx_changes`]).
    pub(crate) fn note_rollback_by(&mut self, by: RollbackBy) {
        self.access_notes.rollback_by = Some(by);
    }

    /// Rolls back the open transaction of a report, an XMLport import, a background session or a guarded run's deferred
    /// insert that failed ([`RollbackBy::OtherInRun`]).
    pub(crate) fn rollback_failed_run(&mut self) {
        self.note_rollback_by(RollbackBy::OtherInRun);
        self.rollback_transaction();
    }

    /// The open transaction ended (`commit`: by a commit), before its point is noted. A commit keeps every step. The
    /// rollback of an asserterror undoes or keeps a measured step; any other rollback inside a run refuses the later calls
    /// on every changed or held sequence. The runner's own rollbacks (no [`Session::note_rollback_by`]) only end the notes.
    pub(crate) fn settle_tx_changes(&mut self, commit: bool) {
        let by = self.access_notes.rollback_by.take();
        let changes = std::mem::take(&mut self.access_notes.tx_changes);
        let (Some(by), false) = (by, commit) else {
            self.access_notes.held.clear();
            return;
        };
        let mut keys: Vec<&Key> = changes.keys().chain(&self.access_notes.held).collect();
        keys.sort();
        keys.dedup();
        let keys: Vec<Key> = keys.into_iter().cloned().collect();
        for key in keys {
            let fate = changes.get(&key).and_then(|c| self.measured_step_fate(&key, c, by));
            match fate {
                Some(Fate::Keep) => {
                    self.access_notes.held.insert(key.clone());
                }
                Some(Fate::Undo) => {
                    self.access_notes.held.remove(&key);
                    if changes.get(&key).map(|c| c.step) == Some(SequenceStep::Delete) {
                        self.access_notes.deletes_undone.push(key.clone());
                    }
                    if let Some(c) = self.sequences_changed.remove(&key) {
                        self.sequences.insert(key.clone(), c.before);
                    }
                }
                None => {
                    self.access_notes.held.remove(&key);
                    let why = format!(
                        "number sequence '{}' was restarted or deleted, then a rollback inside the run ended the \
                         transaction: what it leaves of the sequence is measured only for the asserterror shapes of corpus \
                         PR 1573 (a step after the earlier test's write and Commit(), then a Record write or nothing, then \
                         an asserterror of an Error with a literal text, under TestIsolation = Codeunit and AutoCommit)",
                        key.1
                    );
                    self.refuse_unmeasured_sequence(&key, why);
                    continue;
                }
            }
            self.assume_sequence(&key, Assumption::ExistingSequenceAsserterror);
        }
    }

    /// What the rollback `by` does to the sequence `key` after its change `c`, when the shape is one of corpus PR 1573
    /// (module doc); `None`: not measured. Every fact is read by equality.
    fn measured_step_fate(&self, key: &Key, c: &TxChange, by: RollbackBy) -> Option<Fate> {
        let notes = &self.access_notes;
        let log = notes.tx_log.as_deref()?;
        // RE, RE2, RE3, RE4, RE5, DE: an asserterror of `Error('inner error')`, in a test with TransactionModel AutoCommit,
        // under the runners al-testrun `failed_test_runner_measured` names, with the codeunit's tests noted.
        let measured_run =
            by == RollbackBy::AssertError { literal: true } && notes.measured && self.tx_model == TxModel::AutoCommit;
        // The first and only step of a sequence of the session's company that existed before the transaction and was not
        // inserted while noted, changed by nothing since (no Next, no Insert), no point of the log since, no mark on it.
        let one_step = c.new_in_tx
            && !c.repeated
            && key.0 == *self.cx.company
            && notes.inserted_at.get(key).is_none()
            && self.sequences.get(key) == c.after.as_ref()
            && c.at == Some(log.len())
            && self.sequence_unmarked(key);
        if !(measured_run && one_step && began_after_earlier_write(log)) {
            return None;
        }
        // Nothing reached from the last point (the step's test body start) to the step.
        let last = log.last()?;
        if Interval::of(&last.access, &c.access) != Interval::Untouched {
            return None;
        }
        match (Interval::of(&c.access, &self.data_access()), c.step) {
            (Interval::Untouched, SequenceStep::Restart) => Some(Fate::Keep),
            (Interval::RecordWrite, SequenceStep::Restart | SequenceStep::Delete) => Some(Fate::Undo),
            _ => None,
        }
    }

    /// The Deletes an asserterror undid since the last call: DE measures them only in a test that fails after the
    /// asserterror (al-testrun `isolation::after_test` refuses them after a test that passes).
    pub fn take_deletes_undone(&mut self) -> Vec<Key> {
        std::mem::take(&mut self.access_notes.deletes_undone)
    }

    /// The noted Restarts and Deletes end without a transaction end (a baseline put back).
    pub(crate) fn clear_tx_changes(&mut self) {
        self.access_notes.deletes_undone.clear();
        self.access_notes.tx_changes.clear();
        self.access_notes.held.clear();
        self.access_notes.rollback_by = None;
    }
}

/// The open transaction began at an AL Commit() in an earlier test body after a Record write since that body began, as
/// Initialize does in each cell of PR 1573 (SqExistingRestart.al:119-131), with only body ends and starts and the
/// runner's commits since, the last point being the start of the step's test body.
fn began_after_earlier_write(log: &[super::TxPoint]) -> bool {
    let between = |k: TxPointKind| {
        matches!(
            k,
            TxPointKind::BodyStart | TxPointKind::BodyEnd | TxPointKind::Commit(CommitBy::Other)
        )
    };
    let Some(commit) = log.iter().rposition(|p| !between(p.kind)) else {
        return false;
    };
    let Some(start) = commit.checked_sub(1) else {
        return false;
    };
    log[commit].kind == TxPointKind::Commit(CommitBy::Statement)
        && log[start].kind == TxPointKind::BodyStart
        && Interval::of(&log[start].access, &log[commit].access) == Interval::RecordWrite
        && log[commit + 1..].iter().any(|p| p.kind == TxPointKind::BodyEnd)
        && log.last().map(|p| p.kind) == Some(TxPointKind::BodyStart)
}
