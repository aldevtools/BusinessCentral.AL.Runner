//! What a rollback inside a run (an asserterror, a failed guarded `Codeunit.Run`, a report or XMLport that failed, a
//! background session) leaves of the number sequences the open transaction restarted or deleted.
//!
//! Corpus PR 1573 (open; assumption `existing-sequence-asserterror`), tests/al-language-isolation-probe/src/
//! SqExistingRestart.al, measured on BC 29.0 under runners 130450 and 61301 (so the arms answer on BC 29 alone): in a test codeunit under TestIsolation =
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
    /// The NumberSequence calls since, by method and sequence ([`Session::note_sequence_call`]).
    calls_after: Vec<(&'static str, Key)>,
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
                calls_after: Vec::new(),
            },
        );
    }

    /// AL code called NumberSequence `method` on `key`: noted on every Restart and Delete of the open transaction.
    pub(crate) fn note_sequence_call(&mut self, method: &'static str, key: &Key) {
        for c in self.access_notes.tx_changes.values_mut() {
            c.calls_after.push((method, key.clone()));
        }
    }

    /// The next rollback is one inside a run, by `by` (read by [`Session::settle_tx_changes`]).
    pub(crate) fn note_rollback_by(&mut self, by: RollbackBy) {
        self.access_notes.rollback_by = Some(by);
    }

    /// The rollback [`Session::note_rollback_by`] announced did not run.
    pub(crate) fn forget_rollback_by(&mut self) {
        self.access_notes.rollback_by = None;
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
        let notes = &mut self.access_notes;
        let (Some(by), false) = (by, commit) else {
            if let (false, Some(log)) = (commit, &notes.tx_log) {
                let mut held: Vec<Key> = notes.held.drain().collect();
                held.sort();
                notes.held_at_rollback = Some((log.len(), held));
            }
            notes.held.clear();
            return;
        };
        let mut keys: Vec<&Key> = changes.keys().chain(&self.access_notes.held).collect();
        keys.sort();
        keys.dedup();
        let keys: Vec<Key> = keys.into_iter().cloned().collect();
        // Each cell restarts or deletes one sequence.
        let sole = keys.len() == 1;
        for key in keys {
            let fate = changes
                .get(&key)
                .filter(|_| sole)
                .and_then(|c| self.measured_step_fate(&key, c, by));
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
            // No cell reads the sequence in the step's test after the asserterror.
            let why = format!(
                "number sequence '{}' was restarted or deleted, then an asserterror rolled back: corpus PR 1573 measures \
                 the sequence only in a later test, not in the rest of the test that took the step",
                key.1
            );
            self.refuse_sequence_in_body(&key, why);
            self.assume_sequence(&key, Assumption::ExistingSequenceAsserterror);
        }
    }

    /// What the rollback `by` does to the sequence `key` after its change `c`, when the shape is one of corpus PR 1573
    /// (module doc), the only sequence the transaction changed; `None`: not measured. Every fact is read by equality.
    fn measured_step_fate(&self, key: &Key, c: &TxChange, by: RollbackBy) -> Option<Fate> {
        let notes = &self.access_notes;
        let log = notes.tx_log.as_deref()?;
        // RE, RE2, RE3, RE4, RE5, DE: an asserterror of `Error('inner error')`, in a test with TransactionModel AutoCommit,
        // under the runners al-testrun `failed_test_runner_measured` names, with the codeunit's tests noted, on BC 29.
        let measured_run = by == RollbackBy::AssertError { literal: true }
            && notes.measured
            && self.tx_model == TxModel::AutoCommit
            && self.sequence_cells_measured_major();
        // After the step each cell reads the sequence with `Seq.Rel` (SqExistingRestartFixture.al:69-73): Exists, then
        // Current when it exists; no other NumberSequence call until the rollback.
        let reads = match c.step {
            SequenceStep::Restart => vec![("Exists", key.clone()), ("Current", key.clone())],
            SequenceStep::Delete => vec![("Exists", key.clone())],
        };
        // The first and only step of a sequence of the session's company that existed before the transaction and was not
        // inserted while noted, changed by nothing since (no Next, no Insert), no point of the log since, no mark on it.
        let one_step = c.new_in_tx
            && !c.repeated
            && key.0 == *self.cx.company
            && !notes.inserted_at.contains_key(key)
            && self.sequences.get(key) == c.after.as_ref()
            && c.at == Some(log.len())
            && self.sequence_unmarked(key);
        if !(measured_run && one_step && c.calls_after == reads && began_after_earlier_write(log)) {
            return None;
        }
        // Nothing reached from the last point (the step's test body start) to the step, and no Page or TestPage call from
        // there to the rollback (`Interval` does not read page calls).
        let (last, now) = (log.last()?, self.data_access());
        let untouched = Interval::of(&last.access, &c.access) == Interval::Untouched;
        if !(untouched && (last.access.page_calls, c.access.page_calls) == (now.page_calls, now.page_calls)) {
            return None;
        }
        match (Interval::of(&c.access, &now), c.step) {
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

    /// The sequences whose step an asserterror kept up to the rollback outside a run (the runner's rollback of a failed
    /// test, al-testrun `isolation::failed_test`) at point `at` of [`Session::tx_log`], in the running test; empty when
    /// that point is no such rollback.
    pub fn sequences_held_at_rollback(&self, at: usize) -> &[Key] {
        match &self.access_notes.held_at_rollback {
            Some((point, keys)) if *point == at => keys,
            _ => &[],
        }
    }

    /// The noted Restarts and Deletes end without a transaction end (a baseline put back).
    pub(crate) fn clear_tx_changes(&mut self) {
        self.access_notes.deletes_undone.clear();
        self.access_notes.held_at_rollback = None;
        self.access_notes.tx_changes.clear();
        self.access_notes.held.clear();
        self.access_notes.rollback_by = None;
        self.end_body_refusals();
    }
}

/// The step's test body is the last point, and the test body before it was the one of Initialize in each cell of PR 1573
/// (SqExistingRestart.al:119-131): a Record write, then an AL Commit(), then nothing until the body ended. Between that
/// body's end and the step's body start only the runner committed: an AL Commit() of runner codeunit 130450 or a commit
/// of al-runner's own runner, which replays its steps (what the runner's steps reach between the bodies is not read).
fn began_after_earlier_write(log: &[super::TxPoint]) -> bool {
    let runner = |k: TxPointKind| {
        matches!(
            k,
            TxPointKind::Commit(CommitBy::Other) | TxPointKind::Commit(CommitBy::Statement)
        )
    };
    let Some((TxPointKind::BodyStart, before)) = log.split_last().map(|(p, b)| (p.kind, b)) else {
        return false;
    };
    let Some(end) = before.iter().rposition(|p| !runner(p.kind)) else {
        return false;
    };
    let Some(start) = end.checked_sub(2) else {
        return false;
    };
    let [s, c, e] = &log[start..=end] else {
        return false;
    };
    (s.kind, c.kind, e.kind)
        == (
            TxPointKind::BodyStart,
            TxPointKind::Commit(CommitBy::Statement),
            TxPointKind::BodyEnd,
        )
        && Interval::of(&s.access, &c.access) == Interval::RecordWrite
        && Interval::of(&c.access, &e.access) == Interval::Untouched
}

#[cfg(test)]
mod tests {
    use super::super::{CommitBy, DataAccess, TxPoint, TxPointKind};
    use super::began_after_earlier_write;

    fn access(store: u64, table_store: u64, table_writes: u64, record_calls: u64) -> DataAccess {
        DataAccess {
            store,
            table_store,
            table_writes,
            record_calls,
            ..DataAccess::default()
        }
    }

    fn at(kind: TxPointKind, access: DataAccess) -> TxPoint {
        TxPoint { kind, access }
    }

    /// The log of a PR 1573 cell up to the step's body start: test A writes a row and commits (Initialize), then the
    /// runner's points `between`.
    fn cell_log(between: &[TxPointKind], after_commit: DataAccess) -> Vec<TxPoint> {
        let (none, wrote) = (access(5, 3, 1, 2), access(9, 7, 2, 4));
        let mut log = vec![
            at(TxPointKind::Commit(CommitBy::Other), none),
            at(TxPointKind::BodyStart, none),
            at(TxPointKind::Commit(CommitBy::Statement), wrote),
            at(TxPointKind::BodyEnd, after_commit),
        ];
        log.extend(between.iter().map(|k| at(*k, after_commit)));
        log.push(at(TxPointKind::BodyStart, after_commit));
        log
    }

    const OTHER: TxPointKind = TxPointKind::Commit(CommitBy::Other);
    const STATEMENT: TxPointKind = TxPointKind::Commit(CommitBy::Statement);

    #[test]
    fn the_runners_commits_between_the_bodies_are_the_cells_shape() {
        let wrote = access(9, 7, 2, 4);
        // Al-runner's own runner.
        assert!(began_after_earlier_write(&cell_log(&[OTHER], wrote)));
        // Runner codeunit 130450 of the Test Runner app, whose AL code commits between the tests.
        assert!(began_after_earlier_write(&cell_log(&[OTHER, STATEMENT, OTHER], wrote)));
    }

    #[test]
    fn shapes_beside_the_earlier_test_are_not_the_cells_shape() {
        // A Record write after the earlier test's Commit().
        assert!(!began_after_earlier_write(&cell_log(&[OTHER], access(12, 9, 3, 5))));
        // Another test body between the committing one and the step's.
        let wrote = access(9, 7, 2, 4);
        let body = [OTHER, TxPointKind::BodyStart, TxPointKind::BodyEnd, OTHER];
        assert!(!began_after_earlier_write(&cell_log(&body, wrote)));
        // A rollback between the bodies.
        assert!(!began_after_earlier_write(&cell_log(&[TxPointKind::Rollback], wrote)));
        // The last point is not the step's body start.
        let mut log = cell_log(&[OTHER], wrote);
        log.pop();
        assert!(!began_after_earlier_write(&log));
    }
}
