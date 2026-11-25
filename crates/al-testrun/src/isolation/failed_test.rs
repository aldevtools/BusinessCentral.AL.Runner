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
use al_vm::{CommitBy, DataAccess, Interval, Reached, SequenceStart, Session, TxModel, TxPoint, TxPointKind};

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

impl Body {
    fn reached(&self) -> Reached {
        Reached::of(&self.access.0, &self.access.1)
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

/// A commit by `by` after `reached` keeps a new sequence (corpus PR 1572 SqFailedSeq.al): a Record write S1T :1546, S1U
/// :2166 (C1 SqCommitFail.al:31 of PR 1465), one FindFirst S1X :2451, S1Y :2519, the helper S1V :2232, the helper and
/// a Record write S1Z :220, S1A :85; a guarded Codeunit.Run after a Record write, C6 SqCommitFail.al:289, and an
/// isolated event's subscriber after a Record write, C7 :322.
/// `pending`: an end since the insert kept nothing; then only an AL Commit() after a Record write keeps it (S1J
/// :1118, S1I :1034, S1K :1192: inserted without a commit, the runner's commit, then a Record write and a Commit()).
/// `newest`: the target is BC 29 (`Session::sequence_cells_measured_major`); else only the PR 1465 arms answer.
fn keeps_at(reached: Interval, by: CommitBy, pending: bool, newest: bool) -> bool {
    if pending {
        return newest && by == CommitBy::Statement && reached == Interval::RecordWrite;
    }
    match by {
        CommitBy::Statement => {
            reached == Interval::RecordWrite
                || (newest && matches!(reached, Interval::FindFirst | Interval::Helper { .. }))
        }
        CommitBy::GuardedRun | CommitBy::IsolatedEvent => reached == Interval::RecordWrite,
        CommitBy::Other => false,
    }
}

/// What the failed test's rollback after `reached` leaves of a sequence no commit kept: `Some(true)` kept, `Some(false)`
/// gone. `after_own_commit`: `reached` began at a commit of the failed test's body. From the start of the failed test:
/// gone after a Record write (S1R :1411, S1S :1478, C2 SqCommitFail.al:64, C14 :570), a RecordRef table call (PR
/// 1468 R14O, R14E, R2O, R14V, R2V, R14P, R2P, R14W, R2W, R14M, R2M) or SetRange and FindLast (R14R, R2R); kept after
/// nothing (C13 :541, D1-D4 :758-912) or RecordRef.Open and Close (R14N, R2N). After the failed test's own Commit():
/// gone after a Record write S2F :2098 or the helper and a Record write S2E :2031, kept after nothing S2G :2383 (corpus
/// PR 1572, BC 29 alone: `newest`).
fn leaves_at_failure(reached: Interval, after_own_commit: bool, newest: bool) -> Option<bool> {
    match (reached, after_own_commit) {
        (Interval::Untouched | Interval::OpenClose, false) => Some(true),
        (Interval::RecordWrite | Interval::Ref | Interval::FindLast, false) => Some(false),
        // C3 of PR 1465 (SqCommitFail.al:100, BC 27.5, 28.5 and 29.0): a Commit(), then the insert, then nothing: kept.
        (Interval::Untouched, true) => Some(true),
        (Interval::RecordWrite | Interval::Helper { writes: true }, true) if newest => Some(false),
        _ => None,
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
/// where runner 130450's steps before the failed test commit table writes, and the sequence is gone). `newest`: the
/// target is BC 29; else a commit keeps the sequence only in the failed test's body (C1, C4-C7 of PR 1465), the
/// earlier tests' commits keep it on BC 29 alone (S1T, S1U, S1X-S1Z, S1A of PR 1572).
fn fate(log: &[TxPoint], at: usize, failure: usize, outside: OutsideEnd, newest: bool) -> Option<bool> {
    let (mut reached, mut in_body, mut inserted, mut end_in_body) = (Interval::Untouched, false, false, false);
    let mut pending = false;
    let failed_start = log[..failure].iter().rposition(|p| p.kind == TxPointKind::BodyStart)?;
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
        let by = match point.kind {
            TxPointKind::BodyStart => {
                (in_body, end_in_body) = (true, false);
                continue;
            }
            TxPointKind::BodyEnd => {
                in_body = false;
                continue;
            }
            TxPointKind::Commit(by) => Some(by),
            TxPointKind::Rollback => None,
        };
        if inserted {
            // An end outside the bodies read as `Keeps` acts as an AL `Commit()`.
            let by = by.map(|by| match (in_body, outside) {
                (false, OutsideEnd::Keeps) => CommitBy::Statement,
                _ => by,
            });
            match by {
                None if i == failure => {
                    // On BC 27 and 28 a sequence gone at the failure was inserted before the failed test began (C2
                    // SqCommitFail.al:64, C14 :570, the RecordRef cells of PR 1468); an insert in its own body, then a
                    // table access, is S2E of PR 1572 alone.
                    let leaves = leaves_at_failure(reached, end_in_body, newest)?;
                    return (newest || leaves || at < failed_start).then_some(leaves);
                }
                Some(by)
                    if (in_body || outside == OutsideEnd::Keeps)
                        && (newest || (at > failed_start && i > failed_start))
                        && keeps_at(reached, by, pending, newest) =>
                {
                    return Some(true);
                }
                Some(_) if reached == Interval::Untouched => pending = true,
                Some(_) if !in_body && outside == OutsideEnd::Resets => pending = true,
                _ => return None,
            }
        }
        reached = Interval::Untouched;
        end_in_body = in_body;
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
        session.note_tx_ends(measured);
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
        // DE (corpus PR 1573, SqExistingRestart.al:2487) measures the Delete an asserterror undid only in a test that fails.
        let deletes_undone = session.take_deletes_undone();
        if !failed {
            for key in deletes_undone {
                let why = format!(
                    "a test of this codeunit, {routine}, passed after an asserterror undid the Delete of number \
                     sequence '{}': what that leaves of the sequence is measured only in a test that fails after the asserterror",
                    key.1
                );
                session.refuse_unmeasured_sequence(&key, why);
            }
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

    /// What corpus PRs 1465, 1468 and 1572 measured a failed test leaves (assumption `sequence-after-failed-test`, runners
    /// 61301 and 130450; PRs 1465 and 1468 on BC 27.5, 28.5 and 29.0, PR 1572 on BC 29.0 alone, so its arms answer on BC 29
    /// alone). Each cell has one failed test, raised by its body under AutoCommit,
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
        // Without a Commit() in a test body, main's rule answers every failed test that reached a table.
        if !self.committed && body.reached() != Reached::Nothing {
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
        if !changed.is_empty() && !(body.access.0 == body.access.1 && body.before.unkept_since(&self.start).is_empty())
        {
            return None;
        }
        let log = session.tx_log()?;
        // The failed test's rollback: the first end after the last body.
        let last_body = log.iter().rposition(|p| p.kind == TxPointKind::BodyEnd)?;
        let failure = last_body + log[last_body..].iter().position(|p| p.kind != TxPointKind::BodyEnd)?;
        // The failed test's rollback is the only rollback since the tests began: no cell has an asserterror, a failed
        // guarded Codeunit.Run or an earlier failed test, but RE3 (below).
        if log[failure].kind != TxPointKind::Rollback {
            return None;
        }
        match log[..failure].iter().position(|p| p.kind == TxPointKind::Rollback) {
            None => {}
            Some(at) if new.is_empty() && kept_by_asserterror(session, log, at, failure, &changed) => {}
            Some(_) => return None,
        }
        let Some(first) = new.first() else {
            return Some(Leaves::Kept(changed));
        };
        // Every cell inserts its new sequences between the same two points of the log.
        let at = session.sequence_inserted_at(first)?;
        if new.iter().any(|k| session.sequence_inserted_at(k) != Some(at)) {
            return None;
        }
        // A Page or TestPage call since the point before the insert: under runners 61301 and 130450 the cells with a
        // TestPage (S1F, S1G, S1H, S1I, S1K, S1P, S1W, S2B, S2C, S2D, S2N) are not measured (the Windows hosts refuse it).
        let before_insert = at.checked_sub(1).map_or(body.access.0, |i| log[i].access);
        if log[failure].access.page_calls > before_insert.page_calls {
            return None;
        }
        let newest = session.sequence_cells_measured_major();
        // On BC 27 and 28 the failed test reached what PRs 1465 and 1468 measured, over its whole body (main's arms).
        let whole_body = matches!(
            body.reached(),
            Reached::Nothing
                | Reached::RefOpenOnly
                | Reached::RecordReads
                | Reached::RecordWrites
                | Reached::RefReads
                | Reached::RefWrites
        ) && body.access.0.temporary_record_calls == body.access.1.temporary_record_calls;
        if !(newest || whole_body) {
            return None;
        }
        let kept = match [OutsideEnd::Keeps, OutsideEnd::Resets].map(|o| fate(log, at, failure, o, newest)) {
            [Some(kept), other] if other == Some(kept) => kept,
            _ => return None,
        };
        let keys = [new.clone(), changed].concat();
        Some(match kept {
            true => Leaves::Kept(keys),
            false => Leaves::Gone { gone: new, keys },
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
                     TestIsolation = Codeunit is not measured for what the tests reached around their commits (only the \
                     shapes of corpus PRs 1465, 1468 and 1572 are)",
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

/// RE3 of corpus PR 1573 (tests/al-language-isolation-probe/src/SqExistingRestart.al:1059, assumption
/// `existing-sequence-asserterror`): the failed test restarted a sequence that existed, an asserterror kept the step
/// (al-vm `sequence_start/tx_changes.rs`), and its `Error` failed the test at once; the sequence stays as the step left
/// it. `at`: the asserterror's rollback, the only rollback before `failure` since the tests began, inside the failed
/// test's body, with only that body's end between them and nothing reached; `changed`: exactly the sequences the
/// asserterror kept up to the failed test's rollback. `at` is the first rollback since the tests began (the caller).
fn kept_by_asserterror(session: &Session, log: &[TxPoint], at: usize, failure: usize, changed: &[Key]) -> bool {
    let body_point = |p: &&TxPoint| matches!(p.kind, TxPointKind::BodyStart | TxPointKind::BodyEnd);
    let in_body = log[..at].iter().rev().find(body_point).map(|p| p.kind) == Some(TxPointKind::BodyStart);
    let mut changed = changed.to_vec();
    changed.sort();
    in_body
        && log[at + 1..failure].iter().map(|p| p.kind).eq([TxPointKind::BodyEnd])
        && Interval::of(&log[at].access, &log[failure].access) == Interval::Untouched
        && changed == session.sequences_held_at_rollback(failure)
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
}
