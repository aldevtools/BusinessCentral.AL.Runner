//! What a failed test of a codeunit under TestIsolation = Codeunit leaves of the number sequences for the later tests of
//! the codeunit ([`CodeunitSequences::after_test`]).
//!
//! Main's rule: a failed test takes back every NumberSequence.Insert since the codeunit's tests began
//! (tests/al-language-isolation-probe/src/IsolProbeAfterRollback.Codeunit.al:58-69 S3 inserts, :72-79 S4 fails after a
//! Get, :117-123 S8 finds the sequence gone, under runner 61301). After a Commit() in a test method body of the codeunit
//! the new sequences are refused for the later tests, and a sequence that existed when the tests began and was changed is
//! refused too.
//!
//! Corpus PR 1465 (draft, assumption `sequence-after-failed-test`) measured more under the runners of
//! [`failed_test_runner_measured`]: whether a failed test reached a table decides ([`CodeunitSequences::measured_leaves`]).

use al_vm::{DataAccess, SequenceStart, Session, TxModel};

use super::TestIsolation;

type Key = (String, String);

/// The runner codeunit of corpus PR 1465 with TestIsolation = Codeunit (tests/al-language-isolation-probe).
const PROBE_CODEUNIT_RUNNER: u32 = 61_301;

/// Whether corpus PR 1465 measured what a failed test leaves under this runner: TestIsolation = Codeunit, under
/// al-runner's own runner (`al_runner` `None`, which replays the steps of runner 130450) or runner codeunit 130450 or
/// 61301 by object id, not on the TestRunnerHub (`hub`) and not in a scratch run (`execute`).
pub fn failed_test_runner_measured(isolation: TestIsolation, al_runner: Option<u32>, hub: bool, execute: bool) -> bool {
    isolation == TestIsolation::Codeunit
        && !hub
        && !execute
        && matches!(
            al_runner,
            None | Some(crate::runtime::MICROSOFT_ISOL_CODEUNIT) | Some(PROBE_CODEUNIT_RUNNER)
        )
}

/// The number sequences of one test codeunit under TestIsolation = Codeunit when its tests began (after OnRun), and what
/// its tests did since.
pub struct CodeunitSequences {
    start: SequenceStart,
    /// The runner is one [`failed_test_runner_measured`] names.
    measured: bool,
    /// A commit ran inside a test method body of the codeunit (its AL Commit(), or one its code causes); the runner's own
    /// commits around the method do not count.
    committed: bool,
    /// Every sequence a test body inserted since `start` was inserted under TransactionModel AutoCommit.
    autocommit_inserts: bool,
    /// The last test method body ([`noting_body`]), until [`CodeunitSequences::after_test`] reads it.
    body: Option<Body>,
}

/// What one test method body did.
struct Body {
    /// It ran under TransactionModel AutoCommit.
    autocommit: bool,
    /// The sequences when it began.
    before: SequenceStart,
    /// [`Session::commits`] when it began and when it ended.
    commits: (u64, u64),
    /// [`Session::data_access`] when it began and when it ended.
    access: (DataAccess, DataAccess),
}

/// What a body reached of the table data.
#[derive(Debug, PartialEq, Eq)]
enum Reached {
    /// No table was read or written, and no Record, RecordRef, FieldRef, KeyRef, Query or IsolatedStorage method was called.
    Nothing,
    /// A table was read or written, and no RecordRef, FieldRef, KeyRef, Query or IsolatedStorage method was called.
    RecordTables,
    /// Anything else.
    Other,
}

impl Body {
    fn reached(&self) -> Reached {
        let (from, to) = self.access;
        if from == to {
            Reached::Nothing
        } else if to.store > from.store && to.other_calls == from.other_calls {
            Reached::RecordTables
        } else {
            Reached::Other
        }
    }
}

/// What a failed test leaves of the sequences, as corpus PR 1465 measured it.
#[derive(Debug, PartialEq, Eq)]
enum Leaves {
    /// Every sequence stays as it is; these are the ones the failed test's rollback could have reached.
    Kept(Vec<Key>),
    /// These sequences, the ones inserted since the codeunit's tests began, are gone.
    Gone(Vec<Key>),
}

/// Runs `body`, a test method's body under TransactionModel `model`, and notes in `sequences` what it did.
pub fn noting_body<T>(
    session: &mut Session,
    sequences: Option<&mut CodeunitSequences>,
    model: TxModel,
    body: impl FnOnce(&mut Session) -> T,
) -> T {
    let Some(sequences) = sequences else {
        return body(session);
    };
    let before = session.sequences_at_start();
    let (commits, access) = (session.commits, session.data_access());
    let out = body(session);
    let autocommit = model == TxModel::AutoCommit;
    sequences.committed |= session.commits > commits;
    sequences.autocommit_inserts &= autocommit || session.sequences_inserted_since(&before).is_empty();
    sequences.body = Some(Body {
        autocommit,
        before,
        commits: (commits, session.commits),
        access: (access, session.data_access()),
    });
    out
}

impl CodeunitSequences {
    /// The sequences now, as the codeunit's tests begin; `measured`: [`failed_test_runner_measured`].
    pub fn begin(session: &Session, measured: bool) -> CodeunitSequences {
        CodeunitSequences {
            start: session.sequences_at_start(),
            measured,
            committed: false,
            autocommit_inserts: true,
            body: None,
        }
    }

    /// A test ended; `failed`: it failed, `trusted`: and its failure is not judged not trusted.
    pub(super) fn after_test(&mut self, session: &mut Session, failed: bool, trusted: bool, routine: &str) {
        let body = self.body.take();
        if !failed {
            return;
        }
        let measured = body.filter(|_| trusted).and_then(|b| self.measured_leaves(session, &b));
        match measured {
            Some(Leaves::Kept(keys)) => session.assume_sequences_after_failure(&keys),
            Some(Leaves::Gone(keys)) => {
                session.drop_sequences_inserted_since(&self.start);
                session.assume_sequences_after_failure(&keys);
            }
            None => self.mains_rule(session, routine),
        }
    }

    /// What corpus PR 1465 measured a failed test leaves (assumption `sequence-after-failed-test`, cells of
    /// tests/al-language-isolation-probe/src/SqCommitFail.al asserted by SqReader.al, BC 27.5, 28.5 and 29.0, runners 61301
    /// and 130450). `None`: a shape it does not measure, or one where main's rule gives the same answer.
    fn measured_leaves(&self, session: &Session, body: &Body) -> Option<Leaves> {
        if !(self.measured && self.autocommit_inserts && body.autocommit) {
            return None;
        }
        let new = session.sequences_inserted_since(&self.start);
        let changed = session.unkept_sequences_since(&self.start);
        if new.is_empty() && changed.is_empty() {
            return None;
        }
        let (from, to) = body.commits;
        let in_body = |k: &Key| !body.before.holds(k);
        let inserted_at = |k: &Key| session.sequence_inserted_at(k);
        match body.reached() {
            // Kept: C13 :541 (SqReader.al:138), C3 :100 (:68) with a Commit() before the insert, C8-C11 :354-505, C15 :638,
            // C16 :698, D1-D4 :758-912, D6 :1225 (:201) after an earlier test's Commit(); B1-B4 :963-1135 (:215-236) for a
            // sequence that existed, restarted or deleted, with or without a Commit().
            Reached::Nothing if new.iter().filter(|k| in_body(k)).all(|k| inserted_at(k) == Some(to)) => {
                Some(Leaves::Kept([new, changed].concat()))
            }
            // Gone: C2 :64 (:61) and C14 :570 (:145), each inserted by an earlier test, after an earlier test's Commit(). Without
            // any Commit() main's rule drops them too (C12 :510, D5 :918, D7 :1260; pinned for a Get by
            // IsolProbeAfterRollback.Codeunit.al:72-79, :117-123).
            Reached::RecordTables
                if from == to && self.committed && changed.is_empty() && new.iter().all(|k| body.before.holds(k)) =>
            {
                Some(Leaves::Gone(new))
            }
            // Kept: C1 :31 (:54), C4 :130 (:75), C5 :248 (:82, a MessageHandler's Commit()), C6 :289 (:89, a guarded
            // Codeunit.Run), C7 :322 (:96, an isolated subscriber): the failed test inserted each before its last commit.
            Reached::RecordTables
                if from < to && changed.is_empty() && new.iter().all(|k| in_body(k) && inserted_at(k) < Some(to)) =>
            {
                Some(Leaves::Kept(new))
            }
            _ => None,
        }
    }

    /// Main's rule (module doc).
    fn mains_rule(&self, session: &mut Session, routine: &str) {
        let start = &self.start;
        let committed = if self.committed {
            session.sequences_inserted_since(start)
        } else {
            Vec::new()
        };
        for key in committed {
            let why = format!(
                "a test of this codeunit, {routine}, failed after number sequence '{}' was inserted and a test of the codeunit \
                 ran a Commit() since its tests began: what the rollback of the failed test leaves of it under TestIsolation = \
                 Codeunit is not measured (only a new NumberSequence.Insert without a Commit() is)",
                key.1
            );
            session.refuse_unmeasured_sequence(&key, why);
        }
        if session.drop_sequences_inserted_since(start) != Some(false) {
            return;
        }
        for key in session.unkept_sequences_since(start) {
            let why = format!(
                "a test of this codeunit, {routine}, failed after number sequence '{}', which existed when the codeunit's \
                 tests began, was restarted, deleted or changed: what the rollback of the failed test leaves of it under \
                 TestIsolation = Codeunit is not measured (only a new NumberSequence.Insert is)",
                key.1
            );
            session.refuse_unmeasured_sequence(&key, why);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_runners_of_the_probe_are_measured() {
        let m = |iso, runner, hub, execute| failed_test_runner_measured(iso, runner, hub, execute);
        assert!(m(TestIsolation::Codeunit, None, false, false));
        assert!(m(TestIsolation::Codeunit, Some(130_450), false, false));
        assert!(m(TestIsolation::Codeunit, Some(61_301), false, false));
        assert!(!m(TestIsolation::Codeunit, Some(50_999), false, false));
        assert!(!m(TestIsolation::Codeunit, None, true, false));
        assert!(!m(TestIsolation::Codeunit, None, false, true));
        assert!(!m(TestIsolation::Function, None, false, false));
        assert!(!m(TestIsolation::Disabled, Some(130_450), false, false));
    }

    fn access(store: u64, record_calls: u64, other_calls: u64) -> DataAccess {
        DataAccess {
            store,
            record_calls,
            other_calls,
        }
    }

    fn body(from: DataAccess, to: DataAccess) -> Body {
        Body {
            autocommit: true,
            before: SequenceStart::default(),
            commits: (0, 0),
            access: (from, to),
        }
    }

    #[test]
    fn a_record_call_without_a_table_or_a_recordref_call_is_no_record_table_access() {
        let from = access(5, 2, 1);
        assert_eq!(body(from, from).reached(), Reached::Nothing);
        assert_eq!(body(from, access(9, 4, 1)).reached(), Reached::RecordTables);
        assert_eq!(body(from, access(5, 3, 1)).reached(), Reached::Other);
        assert_eq!(body(from, access(9, 4, 2)).reached(), Reached::Other);
    }
}
