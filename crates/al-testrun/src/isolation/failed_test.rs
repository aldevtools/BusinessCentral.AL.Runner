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
use al_vm::{DataAccess, SequenceStart, Session, TxModel, TxPoint, TxPointKind};

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
}

/// What a failed test leaves of the sequences, as corpus PRs 1465, 1468 and 1572 measured it.
#[derive(Debug, PartialEq, Eq)]
enum Leaves {
    /// Every sequence stays as it is; these are the ones the failed test's rollback could have reached.
    Kept(Vec<Key>),
    /// The sequences `gone` are gone, the others stay; `keys`: the ones the failed test's rollback could have reached.
    Gone { gone: Vec<Key>, keys: Vec<Key> },
}

/// What AL code reached between two transaction ends ([`DataAccess`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Interval {
    /// Nothing: no table, no table call, no other call counted (NumberSequence calls are not).
    Untouched,
    /// RecordRef.Open and RecordRef.Close alone, which read no table (PR 1468 OpenOnly, SqRecordRefFail.al:141).
    OpenClose,
    /// A Record call or a RecordRef table call read or wrote a table, every read and write of the store ran inside one,
    /// and no temporary Record was called.
    Table,
    /// Anything else.
    Unknown,
}

impl Interval {
    fn of(from: &DataAccess, to: &DataAccess) -> Interval {
        if from == to {
            return Interval::Untouched;
        }
        let delta = |count: fn(&DataAccess) -> u64| count(to).saturating_sub(count(from));
        let (store, table_store, refs) = (delta(|a| a.store), delta(|a| a.table_store), delta(|a| a.ref_calls));
        let other = delta(|a| a.record_calls) + delta(|a| a.other_calls) + delta(|a| a.company_name_calls);
        if store == 0 && other == 0 && refs == delta(|a| a.ref_open_close_calls) {
            return Interval::OpenClose;
        }
        if table_store > 0 && store == table_store && delta(|a| a.temporary_record_calls) == 0 {
            return Interval::Table;
        }
        Interval::Unknown
    }

    /// What was reached over this interval and then `next`.
    fn then(self, next: Interval) -> Interval {
        match (self, next) {
            (Interval::Untouched, other) | (other, Interval::Untouched) => other,
            (a, b) if a == b => a,
            _ => Interval::Unknown,
        }
    }
}

/// How a commit outside the test method bodies (the runner's, among them its commit after a passing test) acts on the
/// transaction of the tests. It ends it: after a passing test that wrote a row, a later test inserts a sequence and fails
/// with no table access, and the sequence is kept (PR 1465 D1, SqCommitFail.al:758, SqReader.al:166). Whether it keeps a
/// sequence inserted since, as an AL `Commit()` does, no cell measures, so [`fate`] reads both ways and needs one answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutsideEnd {
    /// As an AL `Commit()` inside a body.
    Keeps,
    /// It ends what was reached so far, and keeps no sequence.
    Resets,
}

/// What the failed test's rollback (`log[failure]`) leaves of the sequence inserted before `log[at]`, inside a test method
/// body, when the ends outside the bodies act as `outside`: `Some(true)` kept, `Some(false)` gone, `None` not measured.
/// Only what AL code reached inside the bodies counts: the runner's own steps between them do not keep a sequence (S1R,
/// where runner 130450's steps before the failed test commit table writes, and the sequence is gone).
fn fate(log: &[TxPoint], at: usize, failure: usize, outside: OutsideEnd) -> Option<bool> {
    let (mut reached, mut in_body, mut inserted) = (Interval::Untouched, false, false);
    for (i, point) in log[..=failure].iter().enumerate() {
        if i > 0 && in_body {
            reached = reached.then(Interval::of(&log[i - 1].access, &point.access));
        }
        if i == at {
            inserted = in_body;
            if !inserted {
                return None;
            }
        }
        let commit = match point.kind {
            TxPointKind::BodyStart | TxPointKind::BodyEnd => {
                in_body = point.kind == TxPointKind::BodyStart;
                continue;
            }
            TxPointKind::Commit { al } => Some(al),
            TxPointKind::Rollback => None,
        };
        if inserted {
            match (reached, commit) {
                (Interval::Untouched, _) => {}
                (Interval::OpenClose, None) | (Interval::Table, None) if i == failure => {
                    return Some(reached == Interval::OpenClose);
                }
                (Interval::Table, Some(al)) if (in_body && al) || (!in_body && outside == OutsideEnd::Keeps) => {
                    return Some(true);
                }
                (_, Some(_)) if !in_body && outside == OutsideEnd::Resets => {}
                _ => return None,
            }
        }
        reached = Interval::Untouched;
    }
    // The failed test reached nothing since the last end.
    inserted.then_some(true)
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
    session.note_body(true);
    let out = body(session);
    session.note_body(false);
    let autocommit = model == TxModel::AutoCommit;
    sequences.committed |= session.commits > commits;
    sequences.autocommit_inserts &= autocommit || session.sequences_inserted_since(&before).is_empty();
    sequences.body = Some(Body {
        autocommit,
        raised: matches!(out, Err(Signal::Error(_))),
        before,
        access: (access, session.data_access()),
    });
    out
}

impl CodeunitSequences {
    /// The sequences now, as the codeunit's tests begin; `measured`: [`failed_test_runner_measured`].
    pub fn begin(session: &mut Session, measured: bool) -> CodeunitSequences {
        session.note_tx_ends();
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
            Some(Leaves::Gone { gone, keys }) => {
                session.drop_sequences(&gone);
                session.assume_sequences_after_failure(&keys);
            }
            None => self.mains_rule(session, body.map(|b| b.reached()), routine),
        }
    }

    /// What corpus PRs 1465, 1468 and 1572 measured a failed test leaves (assumption `sequence-after-failed-test`, BC
    /// 27.5, 28.5 and 29.0, runners 61301 and 130450). Each cell has one failed test, raised by its body under AutoCommit,
    /// no sequence an earlier failure left, and no sequence inserted since the tests began that was restarted or deleted
    /// after. `None`: a shape they do not measure, which main's rule answers.
    ///
    /// A commit keeps a new sequence when a table call read or wrote a table since the transaction end before it, and a
    /// failed test's rollback takes a sequence no such commit kept back when the failed test reached a table since its
    /// last transaction end (tests/al-language-isolation-probe/src/SqFailedSeq.al, corpus PR 1572): kept after a Record
    /// write before the earlier test's Commit() S1T :1546, a Record write before the insert S1U :2166, the helper's
    /// RecordRef read S1V :2232, a Record read S1X :2451 and an UpdLock read S1Y :2519, the helper's entry insert S1Z :220,
    /// S1A :85, S1J :1118 and S1Q :1344, whatever the failed test does after its Commit() (S1B :290 a temporary Record, S1C
    /// :360 a TestPage, S1D :446 a number taken, S1E :517 session builtins and a temporary RecordRef, S1L :587 RecordRef
    /// calls, S1F :662 all of them, S1G :752 without a Commit(), S1H :843 and S1P :935 the Commit() in a called codeunit,
    /// S1I :1034 and S1K :1192 after a test that inserted without a commit, S1M :1275 a trigger's IsolatedStorage call),
    /// and in the failed test itself, inserted before its own Commit() (S2A :1612, S2B :1684, S2C :1859, S2D :1949);
    /// gone after a Commit() with only NumberSequence calls since the last end and a failed test that wrote a table (S1R
    /// :1411, S1S :1478, S2F :2098, and C2, C14 and the RecordRef cells of corpus PRs 1465 and 1468), after an insert
    /// after the failed test's last Commit() (S2E :2031); kept when the failed test reached no table since its last end
    /// (S2G :2383, C13, C3, D1-D4 of PR 1465, RecordRef.Open and Close alone R14N, R2N of PR 1468).
    fn measured_leaves(&self, session: &Session, body: &Body) -> Option<Leaves> {
        if !(self.measured && self.autocommit_inserts && body.measured_failure()) {
            return None;
        }
        let new = session.sequences_inserted_since(&self.start);
        let changed = session.unkept_sequences_since(&self.start);
        if (new.is_empty() && changed.is_empty())
            || new.iter().chain(&changed).any(|k| session.sequence_left_by_failure(k))
            || !session.new_sequences_changed_since(&self.start).is_empty()
        {
            return None;
        }
        // B1-B4 of PR 1465 (SqCommitFail.al:963-1135, SqReader.al:215-236): a sequence that existed, restarted or deleted
        // by the failed test itself, which reached no table, with or without a Commit(), stays as it is.
        if !changed.is_empty() && !(body.access.0 == body.access.1 && body.before.unkept_since(&self.start).is_empty()) {
            return None;
        }
        let log = session.tx_log()?;
        // The failed test's rollback: the first end after the last body.
        let last_body = log.iter().rposition(|p| p.kind == TxPointKind::BodyEnd)?;
        let failure = last_body + log[last_body..].iter().position(|p| p.kind != TxPointKind::BodyEnd)?;
        if log[failure].kind != TxPointKind::Rollback {
            return None;
        }
        let mut gone = Vec::new();
        for key in &new {
            let at = session.sequence_inserted_at(key)?;
            match [OutsideEnd::Keeps, OutsideEnd::Resets].map(|o| fate(log, at, failure, o)) {
                [Some(kept), other] if other == Some(kept) => {
                    if !kept {
                        gone.push(key.clone());
                    }
                }
                _ => return None,
            }
        }
        let keys = [new, changed].concat();
        Some(match gone.is_empty() {
            true => Leaves::Kept(keys),
            false => Leaves::Gone { gone, keys },
        })
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
