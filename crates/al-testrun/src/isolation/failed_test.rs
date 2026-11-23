//! What a failed test of a codeunit under TestIsolation = Codeunit leaves of the number sequences for the later tests of
//! the codeunit ([`CodeunitSequences::after_test`]).
//!
//! Main's rule: a failed test takes back every NumberSequence.Insert since the codeunit's tests began
//! (tests/al-language-isolation-probe/src/IsolProbeAfterRollback.Codeunit.al:58-69 S3 inserts, :72-79 S4 fails after a
//! Get, :117-123 S8 finds the sequence gone, under runner 61301). After a Commit() in a test method body of the codeunit
//! the new sequences are refused for the later tests, and a sequence that existed when the tests began and was changed is
//! refused too. Without a Commit(), the take-back runs only after a failed test that reached no table or reached tables
//! through Record reads and writes alone ([`Reached`]); after any other access the new sequences are refused.
//!
//! Corpus PR 1465 (merged, not yet pinned; assumption `sequence-after-failed-test`) measured more under the runners of
//! [`failed_test_runner_measured`]: what a failed test reached of the tables decides ([`CodeunitSequences::measured_leaves`]).
//!
//! What a failed test under TestIsolation = Codeunit leaves of the number sequences:
//! `CodeunitSequences` (begun after OnRun, fed per test body by `noting_body` with its commits, its
//! table access, al-vm `Session::data_access`, and whether it raised), main's rule, and the cells of
//! corpus PR 1465 and the RecordRef cells of corpus PR 1468 under the runners of
//! `failed_test_runner_measured` (assumption `sequence-after-failed-test`). The bindings a failed
//! test leaves on a global of the test codeunit are kept in `runtime/run.rs` `left_bindings`
//! (assumption `binding-after-failed-test`). Entry points: `CodeunitSequences`, `noting_body`,
//! `failed_test_runner_measured`.

use al_builtins::native::Signal;
use al_vm::{DataAccess, SequenceStart, Session, TxModel};

use super::TestIsolation;

type Key = (String, String);

/// The runner codeunit of corpus PR 1465 with TestIsolation = Codeunit (tests/al-language-isolation-probe/src/
/// IsolProbeRunnerCodeunit.Codeunit.al:10), matched by object id and name.
const PROBE_CODEUNIT_RUNNER: (u32, &str) = (61_301, "Isol Probe Runner Codeunit");

/// Whether corpus PR 1465 measured what a failed test leaves under this runner: TestIsolation = Codeunit, not on the
/// TestRunnerHub (`hub`) and not in a scratch run (`execute`), under runner codeunit 130450 (by object id), the probe's
/// runner 61301 (`runner`: object id and name), or al-runner's own runner (`runner` `None`). The own runner replays the
/// steps of runner 130450 around each test (ADR-066 decision 6, `TestRunner::Builtin`), so the cells measured under 130450
/// hold for it.
pub fn failed_test_runner_measured(
    isolation: TestIsolation,
    runner: Option<(u32, &str)>,
    hub: bool,
    execute: bool,
) -> bool {
    let runner_measured = match runner {
        None => true,
        Some((id, _)) if id == crate::runtime::MICROSOFT_ISOL_CODEUNIT => true,
        Some((id, name)) => id == PROBE_CODEUNIT_RUNNER.0 && name.eq_ignore_ascii_case(PROBE_CODEUNIT_RUNNER.1),
    };
    isolation == TestIsolation::Codeunit && !hub && !execute && runner_measured
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
    /// Its call returned an AL error (an `Error()` or a runtime error it did not catch).
    raised: bool,
    /// The sequences when it began.
    before: SequenceStart,
    /// [`Session::commits`] when it began and when it ended.
    commits: (u64, u64),
    /// [`Session::data_access`] when it began and when it ended.
    access: (DataAccess, DataAccess),
}

/// What a body reached of the table data. A table call is a Record method or a RecordRef, FieldRef or KeyRef method al-vm
/// `measured_ref_method` names, on a RecordRef that is not temporary and of the session's company ([`DataAccess`]).
#[derive(Debug, PartialEq, Eq)]
enum Reached {
    /// No table was read or written, no table call ran, and no other RecordRef, FieldRef, KeyRef or Query method or
    /// session builtin other than NumberSequence was called.
    Nothing,
    /// As `Nothing`, except that RecordRef.Open and RecordRef.Close ran, and no other table call.
    RefOpenOnly,
    /// Tables were read and none was written, every read inside a Record method call, and nothing else was called.
    RecordReads,
    /// As `RecordReads`, and a Record method call wrote a table.
    RecordWrites,
    /// Tables were read and none was written, every read inside a RecordRef table call, Record calls that read no table
    /// and CompanyName calls aside, and nothing else was called.
    RefReads,
    /// Every table read and write ran inside a table call, a RecordRef one among them read or wrote a table, one wrote, and
    /// nothing else was called but CompanyName.
    RefWrites,
    /// Anything else.
    Other,
}

impl Body {
    /// How much `count` of [`DataAccess`] grew while the body ran.
    fn delta(&self, count: fn(&DataAccess) -> u64) -> u64 {
        count(&self.access.1).saturating_sub(count(&self.access.0))
    }

    fn reached(&self) -> Reached {
        if self.access.0 == self.access.1 {
            return Reached::Nothing;
        }
        let delta = |count| self.delta(count);
        let store = delta(|a| a.store);
        let refs = delta(|a| a.ref_calls) > 0;
        let writes = delta(|a| a.table_writes) > 0;
        // CompanyName counts only in a body that read through a RecordRef, as GetNewLineNo does (SqRecordRefFail.al:80).
        let company_name = delta(|a| a.company_name_calls) > 0;
        if delta(|a| a.other_calls) > 0 || (company_name && !(refs && store > 0)) {
            return Reached::Other;
        }
        if store == 0 {
            let open_close = delta(|a| a.ref_open_close_calls) == delta(|a| a.ref_calls);
            return match refs && open_close && delta(|a| a.record_calls) == 0 {
                true => Reached::RefOpenOnly,
                false => Reached::Other,
            };
        }
        match (delta(|a| a.table_store) == store, refs, writes) {
            (true, false, false) => Reached::RecordReads,
            (true, false, true) => Reached::RecordWrites,
            (true, true, false) if delta(|a| a.ref_store) == store => Reached::RefReads,
            (true, true, true) if delta(|a| a.ref_store) > 0 => Reached::RefWrites,
            _ => Reached::Other,
        }
    }

    /// Every cell of corpus PR 1465 fails by an error its body raised, in a test with TransactionModel AutoCommit.
    fn measured_failure(&self) -> bool {
        self.raised && self.autocommit
    }

    /// The Record calls of cells R14R and R2R of corpus PR 1468 (LastLineByRecord, SqRecordRefFail.al:118): Record.SetRange
    /// and exactly one Record.FindLast, which did not fail, and every table read inside that FindLast. Any other Record call
    /// (SetAutoCalcFields, LockTable, SetLoadFields, FilterGroup, a second FindLast) or no SetRange is not measured.
    fn reads_by_find_last(&self) -> bool {
        let delta = |count| self.delta(count);
        let (find_last, set_range) = (delta(|a| a.find_last_calls), delta(|a| a.set_range_calls));
        find_last == 1
            && set_range > 0
            && delta(|a| a.record_calls) == find_last + set_range
            && delta(|a| a.find_last_failures) == 0
            && delta(|a| a.find_last_store) == delta(|a| a.store)
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
pub fn noting_body<V>(
    session: &mut Session,
    sequences: Option<&mut CodeunitSequences>,
    model: TxModel,
    body: impl FnOnce(&mut Session) -> Result<V, Signal>,
) -> Result<V, Signal> {
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
        raised: matches!(out, Err(Signal::Error(_))),
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

    /// The last test method body failed the way every cell of corpus PR 1465 does: by an error it raised, under
    /// TransactionModel AutoCommit (al-testrun `runtime/run.rs` `left_bindings` reads it too).
    pub fn body_failed_as_measured(&self) -> bool {
        self.body.as_ref().is_some_and(Body::measured_failure)
    }

    /// A test ended; `failed`: it failed, `trusted`: and its failure is not judged not trusted.
    pub(super) fn after_test(&mut self, session: &mut Session, failed: bool, trusted: bool, routine: &str) {
        let body = self.body.take();
        if !failed {
            return;
        }
        let measured = body
            .as_ref()
            .filter(|_| trusted)
            .and_then(|b| self.measured_leaves(session, b));
        match measured {
            Some(Leaves::Kept(keys)) => session.assume_sequences_after_failure(&keys),
            Some(Leaves::Gone(keys)) => {
                session.drop_sequences_inserted_since(&self.start);
                session.assume_sequences_after_failure(&keys);
            }
            None => self.mains_rule(session, body.map(|b| b.reached()), routine),
        }
    }

    /// What corpus PR 1465 measured a failed test leaves (assumption `sequence-after-failed-test`, cells of
    /// tests/al-language-isolation-probe/src/SqCommitFail.al asserted by SqReader.al, BC 27.5, 28.5 and 29.0, runners 61301
    /// and 130450). Each cell has one failed test, raised by its body under AutoCommit, and no sequence an earlier failure
    /// left, and no cell calls a temporary record in its failed test. `None`: a shape it does not measure, or one where main's
    /// rule gives the same answer.
    fn measured_leaves(&self, session: &Session, body: &Body) -> Option<Leaves> {
        let no_temporary_record = body.delta(|a| a.temporary_record_calls) == 0;
        if !(self.measured && self.autocommit_inserts && body.measured_failure() && no_temporary_record) {
            return None;
        }
        let new = session.sequences_inserted_since(&self.start);
        let changed = session.unkept_sequences_since(&self.start);
        if (new.is_empty() && changed.is_empty())
            || new.iter().chain(&changed).any(|k| session.sequence_left_by_failure(k))
        {
            return None;
        }
        let (from, to) = body.commits;
        let in_body = |k: &Key| !body.before.holds(k);
        let inserted_at = |k: &Key| session.sequence_inserted_at(k);
        // The C2 and C14 shape: the failed test ran no commit, an earlier test did, and earlier tests inserted every new
        // sequence and changed none that existed.
        let earlier_inserts_after_a_commit =
            from == to && self.committed && changed.is_empty() && new.iter().all(|k| body.before.holds(k));
        match body.reached() {
            // Kept: C13 :541 (SqReader.al:138), C3 :100 (:68) with a Commit() before the insert, C8-C11 :354-505, C15 :638,
            // C16 :698, D1-D4 :758-912, D6 :1225 (:201) after an earlier test's Commit(); B1-B4 :963-1135 (:215-236) for a
            // sequence that existed and that the failed test itself restarted or deleted, with or without a Commit().
            Reached::Nothing
                if new.iter().filter(|k| in_body(k)).all(|k| inserted_at(k) == Some(to))
                    && body.before.unkept_since(&self.start).is_empty() =>
            {
                Some(Leaves::Kept([new, changed].concat()))
            }
            // Gone: C2 :64 (:61) and C14 :570 (:145), each inserted by an earlier test, after an earlier test's Commit(); the
            // failed test wrote a row. Without any Commit() main's rule drops them too (C12 :510, D5 :918, D7 :1260; pinned
            // for a Get by IsolProbeAfterRollback.Codeunit.al:72-79, :117-123). The same with RecordRef access, corpus PR
            // 1468 (draft) SqRecordRefFail.al: a RecordRef read R14O :192, R14E :483, R2O :655 (SqRecordRefReader.al:37, :72,
            // :93), the GetNewRecNo copy R14V :249, R2V :697 (:44, :100), a Record passed to a RecordRef parameter R14P :309,
            // R2P :742 (:51, :107), RecordRef.Insert R14W :369, R2W :787 (:58, :114), GetNewRecNo and Record.Insert R14M
            // :540, R2M :871 (:79, :128).
            Reached::RecordWrites | Reached::RefReads | Reached::RefWrites if earlier_inserts_after_a_commit => {
                Some(Leaves::Gone(new))
            }
            // Gone: Record.SetRange and Record.FindLast alone (LastLineByRecord :118), PR 1468 R14R :597, a row found, R2R :913,
            // none found (SqRecordRefReader.al:86, :135).
            Reached::RecordReads if earlier_inserts_after_a_commit && body.reads_by_find_last() => {
                Some(Leaves::Gone(new))
            }
            // Kept: RecordRef.Open and Close alone, PR 1468 R14N :426, R2N :829 (SqRecordRefReader.al:65, :121).
            Reached::RefOpenOnly if earlier_inserts_after_a_commit => Some(Leaves::Kept(new)),
            // Kept: C1 :31 (:54), C4 :130 (:75), C5 :248 (:82, a MessageHandler's Commit()), C6 :289 (:89, a guarded
            // Codeunit.Run), C7 :322 (:96, an isolated subscriber): the failed test wrote a row and inserted each sequence
            // before its last commit.
            Reached::RecordWrites
                if from < to
                    && changed.is_empty()
                    && new
                        .iter()
                        .all(|k| in_body(k) && inserted_at(k).is_some_and(|at| at < to)) =>
            {
                Some(Leaves::Kept(new))
            }
            _ => None,
        }
    }

    /// Main's rule (module doc); `reached`: what the failed test's body reached, `None` when no body ran.
    fn mains_rule(&self, session: &mut Session, reached: Option<Reached>, routine: &str) {
        let start = &self.start;
        let new = session.sequences_inserted_since(start);
        for key in &new {
            let why = if self.committed {
                format!(
                    "a test of this codeunit, {routine}, failed after number sequence '{}' was inserted and a test of the \
                     codeunit ran a Commit() since its tests began: what the rollback of the failed test leaves of it under \
                     TestIsolation = Codeunit is not measured (only a new NumberSequence.Insert without a Commit() is)",
                    key.1
                )
            } else if matches!(
                reached,
                Some(Reached::Other | Reached::RefOpenOnly | Reached::RefReads | Reached::RefWrites)
            ) {
                format!(
                    "a test of this codeunit, {routine}, failed after number sequence '{}' was inserted, and it reached the \
                     database other than by Record reads and writes (a temporary record, a RecordRef, a Query, a page, \
                     IsolatedStorage or another session builtin): what the rollback of the failed test leaves of the sequence \
                     under TestIsolation = Codeunit is not measured (only no table access, or Record reads and writes, is)",
                    key.1
                )
            } else if session.sequence_left_by_failure(key) {
                format!(
                    "a test of this codeunit, {routine}, failed after an earlier failed test of the codeunit left number \
                     sequence '{}': what a second failed test leaves of it under TestIsolation = Codeunit is not measured",
                    key.1
                )
            } else {
                continue;
            };
            session.refuse_unmeasured_sequence(key, why);
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
        let probe = Some((61_301, "Isol Probe Runner Codeunit"));
        assert!(m(TestIsolation::Codeunit, None, false, false));
        assert!(m(
            TestIsolation::Codeunit,
            Some((130_450, "Test Runner - Isol. Codeunit")),
            false,
            false
        ));
        assert!(m(TestIsolation::Codeunit, probe, false, false));
        assert!(!m(
            TestIsolation::Codeunit,
            Some((61_301, "Another Runner")),
            false,
            false
        ));
        assert!(!m(
            TestIsolation::Codeunit,
            Some((50_999, "Isol Probe Runner Codeunit")),
            false,
            false
        ));
        assert!(!m(TestIsolation::Codeunit, None, true, false));
        assert!(!m(TestIsolation::Codeunit, None, false, true));
        assert!(!m(TestIsolation::Function, None, false, false));
        assert!(!m(TestIsolation::Disabled, probe, false, false));
    }

    fn access(store: u64, table_store: u64, table_writes: u64, record_calls: u64, other_calls: u64) -> DataAccess {
        DataAccess {
            store,
            table_store,
            table_writes,
            record_calls,
            other_calls,
            ..DataAccess::default()
        }
    }

    fn by_ref(a: DataAccess, ref_store: u64, ref_calls: u64) -> DataAccess {
        DataAccess {
            ref_store,
            ref_calls,
            ..a
        }
    }

    fn body(from: DataAccess, to: DataAccess) -> Body {
        Body {
            autocommit: true,
            raised: true,
            before: SequenceStart::default(),
            commits: (0, 0),
            access: (from, to),
        }
    }

    #[test]
    fn only_table_access_inside_record_calls_is_record_access() {
        let from = access(5, 3, 1, 2, 1);
        assert_eq!(body(from, from).reached(), Reached::Nothing);
        assert_eq!(body(from, access(9, 7, 1, 4, 1)).reached(), Reached::RecordReads);
        assert_eq!(body(from, access(9, 7, 2, 4, 1)).reached(), Reached::RecordWrites);
        // A Record call that reached no table (a temporary record, Init).
        assert_eq!(body(from, access(5, 3, 1, 3, 1)).reached(), Reached::Other);
        // A table read outside a Record call (a page, a native).
        assert_eq!(body(from, access(9, 6, 2, 4, 1)).reached(), Reached::Other);
        // A RecordRef method no cell of corpus PR 1468 calls, a Query or session builtin call.
        assert_eq!(body(from, access(9, 7, 2, 4, 2)).reached(), Reached::Other);
    }

    #[test]
    fn recordref_table_calls_are_told_apart_from_record_calls() {
        let from = access(5, 3, 1, 2, 1);
        let open_close = |a: DataAccess, n: u64| DataAccess {
            ref_open_close_calls: n,
            ..a
        };
        // Open and Close only.
        assert_eq!(
            body(from, open_close(by_ref(from, 0, 2), 2)).reached(),
            Reached::RefOpenOnly
        );
        // Open, Close and RecordRef calls that read no table (KeyIndex, FieldRef.SetRange or Value, GetTable and Number).
        assert_eq!(body(from, open_close(by_ref(from, 0, 4), 2)).reached(), Reached::Other);
        assert_eq!(body(from, by_ref(from, 0, 2)).reached(), Reached::Other);
        // Open and Close, and a Record write: the RecordRef calls read and wrote no table.
        assert_eq!(
            body(from, open_close(by_ref(access(9, 7, 2, 3, 1), 0, 2), 2)).reached(),
            Reached::Other
        );
        // Open and Close, and a Record call that reached no table.
        assert_eq!(
            body(from, by_ref(access(5, 3, 1, 3, 1), 0, 2)).reached(),
            Reached::Other
        );
        // Every read through a RecordRef.
        assert_eq!(
            body(from, by_ref(access(9, 7, 1, 2, 1), 4, 3)).reached(),
            Reached::RefReads
        );
        // Reads through a RecordRef and through a Record.
        assert_eq!(
            body(from, by_ref(access(9, 7, 1, 3, 1), 2, 3)).reached(),
            Reached::Other
        );
        // A RecordRef read and a Record write (GetNewRecNo, then Insert).
        assert_eq!(
            body(from, by_ref(access(9, 7, 2, 3, 1), 2, 3)).reached(),
            Reached::RefWrites
        );
        // CompanyName counts only beside a RecordRef read (GetNewLineNo).
        let named = |a: DataAccess| DataAccess {
            company_name_calls: 1,
            ..a
        };
        assert_eq!(
            body(from, named(by_ref(access(9, 7, 2, 3, 1), 2, 3))).reached(),
            Reached::RefWrites
        );
        assert_eq!(body(from, named(access(9, 7, 2, 4, 1))).reached(), Reached::Other);
        assert_eq!(
            body(from, named(open_close(by_ref(from, 0, 2), 2))).reached(),
            Reached::Other
        );
    }
}
