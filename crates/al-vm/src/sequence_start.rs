//! The number sequences as a test codeunit under TestIsolation = Codeunit found them, and what a failed test of it
//! takes back (al-testrun `isolation::after_test`).
//!
//! the number sequences as a test codeunit's tests found them (`SequenceStart`), what a failed test
//! takes back, and what a test reached of the table data (`DataAccess`: the store's reads and writes,
//! al-data `MemStore::accesses` and `writes`, the part of them inside table calls from AL code,
//! `al_table_call` for Record calls in `ctx.rs` and `counted_ref_op` for the RecordRef, FieldRef and
//! KeyRef methods of `measured_ref_method`, CompanyName calls, and other RecordRef, Query and session
//! builtin calls other than NumberSequence, `note_session_call`) with the commit count at each
//! `NumberSequence.Insert` (`sequence_inserted_at`), for al-testrun `isolation/failed_test.rs`. Entry
//! points: `SequenceStart`, `DataAccess`, `data_access`, `sequence_inserted_at`.

use crate::Session;
use crate::sessionops::Sequence;
use al_builtins::native::Signal;
use al_builtins::value::{LibraryValue, Value};
use std::collections::{HashMap, HashSet};

mod interval;
mod tx_changes;
pub use interval::{Interval, Reached};
pub(crate) use tx_changes::{RollbackBy, SequenceStep};

type Key = (String, String);

/// What a test reached of the table data, counted from the session's start ([`Session::data_access`]): two values compared
/// say what a test between them read or wrote (al-testrun `isolation/failed_test.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DataAccess {
    /// Reads and writes of the database's tables, by any caller.
    pub store: u64,
    /// The part of `store` that ran inside a table call from AL code: a method of a Record variable, or a RecordRef, FieldRef or
    /// KeyRef method [`measured_ref_method`] names (the outermost such call, so a trigger, subscriber or handler it runs
    /// counts with it).
    pub table_store: u64,
    /// The writes of `table_store`.
    pub table_writes: u64,
    /// The part of `table_store` whose outermost call was a RecordRef, FieldRef or KeyRef method.
    pub ref_store: u64,
    /// The part of `table_store` whose outermost call was Record.FindLast.
    pub find_last_store: u64,
    /// Outermost Record.FindLast calls from AL code.
    pub find_last_calls: u64,
    /// The calls of `find_last_calls` that failed.
    pub find_last_failures: u64,
    /// Outermost Record.SetRange calls from AL code.
    pub set_range_calls: u64,
    /// The part of `table_store` whose outermost call was Record.FindFirst.
    pub find_first_store: u64,
    /// Outermost Record.FindFirst calls from AL code.
    pub find_first_calls: u64,
    /// Outermost Record.ReadIsolation calls from AL code that set IsolationLevel::UpdLock.
    pub upd_lock_calls: u64,
    /// Calls of a method of a Record variable from AL code, a temporary one too.
    pub record_calls: u64,
    /// Outermost calls of a method of a temporary Record variable from AL code (one that a trigger or subscriber of a table
    /// call makes counts with that call).
    pub temporary_record_calls: u64,
    /// Calls of a RecordRef, FieldRef or KeyRef method [`measured_ref_method`] names from AL code.
    pub ref_calls: u64,
    /// The RecordRef.Open and RecordRef.Close calls of `ref_calls`.
    pub ref_open_close_calls: u64,
    /// Calls of the session builtin CompanyName from AL code.
    pub company_name_calls: u64,
    /// The RecordRef calls of `other_calls` that the helper of corpus PR 1572 makes (GetLastEntryNoFromTable,
    /// SqFailedSeqFixture.al:189), on a RecordRef of the session's company that is not temporary and that did not fail:
    /// [`RefLoads`].
    pub ref_loads: RefLoads,
    /// Calls of a Page or TestPage method from AL code (counted in no other field).
    pub page_calls: u64,
    /// Calls from AL code of any other RecordRef, FieldRef or KeyRef method, of one on a temporary RecordRef or one of
    /// another company, of a Query method, and of a session builtin other than NumberSequence and CompanyName
    /// (IsolatedStorage, CompanyProperty, UserId, ...).
    pub other_calls: u64,
}

/// The counts behind [`DataAccess`] and when each number sequence was inserted ([`Session::sequence_inserted_at`]).
#[derive(Debug, Default)]
pub(crate) struct Notes {
    access: DataAccess,
    /// How many table calls from AL code are running ([`Session::al_table_call`]).
    table_depth: u32,
    /// Where in `tx_log` each number sequence was last inserted while the log was kept.
    inserted_at: HashMap<Key, usize>,
    /// What the next commit is the commit of ([`Session::al_commit`], [`Session::guarded_run_commit`],
    /// [`Session::isolated_event_commit`]).
    commit_by: Option<CommitBy>,
    /// The transaction ends and test method bodies since [`Session::note_tx_ends`]; `None` while not noted.
    tx_log: Option<Vec<TxPoint>>,
    /// The runner is one al-testrun `failed_test_runner_measured` names ([`Session::note_tx_ends`]).
    measured: bool,
    /// The Restarts and Deletes of number sequences in the open transaction (`tx_changes.rs`).
    tx_changes: HashMap<Key, tx_changes::TxChange>,
    /// What the next rollback inside a run is the rollback of ([`Session::note_rollback_by`]).
    rollback_by: Option<RollbackBy>,
    /// Sequences whose step an asserterror kept, until the next commit or rollback outside a run.
    held: HashSet<Key>,
    /// The sequences of `held` when the last transaction end was a rollback outside a run, until the next end
    /// ([`Session::sequences_held_at_rollback`]).
    held_at_rollback: Vec<Key>,
    /// Sequences whose Delete an asserterror undid, until al-testrun reads them at the test's end
    /// ([`Session::take_deletes_undone`]).
    deletes_undone: Vec<Key>,
}

/// Calls of the RecordRef methods the helper of corpus PR 1572 makes beside the measured ones ([`DataAccess::ref_loads`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefLoads {
    /// RecordRef.ReadIsolation(IsolationLevel::ReadUncommitted).
    pub read_uncommitted: u64,
    /// RecordRef.ReadIsolation(IsolationLevel::UpdLock).
    pub upd_lock: u64,
    /// RecordRef.ReadIsolation with any other level.
    pub other_isolation: u64,
    /// RecordRef.SetLoadFields.
    pub set_load_fields: u64,
}

/// A point of a test codeunit's run that [`Session::note_tx_ends`] notes (al-testrun `isolation/failed_test.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TxPoint {
    pub kind: TxPointKind,
    /// [`Session::data_access`] then.
    pub access: DataAccess,
}

/// What happened at a [`TxPoint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxPointKind {
    /// A test method body began ([`Session::note_body`]).
    BodyStart,
    /// A test method body ended.
    BodyEnd,
    /// A transaction was committed, by this.
    Commit(CommitBy),
    /// A transaction was rolled back.
    Rollback,
}

/// What committed a transaction ([`TxPointKind::Commit`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitBy {
    /// An AL `Commit()` statement.
    Statement,
    /// A guarded `Codeunit.Run`, which commits the run's own transaction.
    GuardedRun,
    /// The subscriber of an isolated event, which commits its own transaction.
    IsolatedEvent,
    /// Anything else (the runner, a TestIsolation boundary, an unguarded run).
    Other,
}

/// The number sequences when a test codeunit's tests begin (after its OnRun), and which of them the session had
/// restarted or deleted by then ([`Session::sequences_at_start`]).
#[derive(Debug, Clone, Default)]
pub struct SequenceStart {
    sequences: HashMap<Key, Sequence>,
    changed: HashMap<Key, (bool, bool)>,
}

impl SequenceStart {
    /// Whether the sequence `key` (company, name) was there.
    pub fn holds(&self, key: &Key) -> bool {
        self.sequences.contains_key(key)
    }

    /// [`Session::unkept_sequences_since`] as it was when `self` was taken.
    pub fn unkept_since(&self, start: &SequenceStart) -> Vec<Key> {
        unkept(start, &self.sequences, &self.changed)
    }
}

/// The sequences of `start` that `now` (with its restart and delete flags `flags`) holds restarted, deleted or otherwise
/// changed, sorted. Numbers taken by `Next` count as kept (SeqCarry.al:26-27).
fn unkept(start: &SequenceStart, now: &HashMap<Key, Sequence>, flags: &HashMap<Key, (bool, bool)>) -> Vec<Key> {
    let mut unkept: Vec<Key> = start
        .sequences
        .iter()
        .filter(|&(k, then)| {
            let untouched = flags.get(k) == start.changed.get(k);
            !(untouched
                && now.get(k).is_some_and(|now| {
                    now == then
                        || (now.seed == then.seed
                            && now.increment == then.increment
                            && now.last.is_some()
                            && now.last != then.last)
                }))
        })
        .map(|(k, _)| k.clone())
        .collect();
    unkept.sort();
    unkept
}

/// The RecordRef, FieldRef and KeyRef methods the cells of corpus PR 1468 call in a failed test (codeunit 76240 "LT SQ Ref Access",
/// SqRecordRefFail.al:44-160: GetNewRecNo, GetNewLineNo, LastLineByRef, InsertByRef, OpenOnly, MockInsert), by catalog id.
pub(crate) fn measured_ref_method(id: &str) -> bool {
    // `FieldRef.SetRange` keeps its catalog id whichever way lowering wrote its arguments (`setrange_convert`).
    let (id, _) = crate::setrange_convert::catalog_id(id);
    matches!(
        id.split('#').next(),
        Some(
            "RecordRef.Open"
                | "RecordRef.Close"
                | "RecordRef.GetTable"
                | "RecordRef.GetTableVariant"
                | "RecordRef.Number"
                | "RecordRef.KeyIndex"
                | "RecordRef.Field"
                | "RecordRef.FindLast"
                | "RecordRef.Insert"
                | "KeyRef.FieldCount"
                | "KeyRef.FieldIndex"
                | "FieldRef.Number"
                | "FieldRef.Value"
                | "FieldRef.SetRange"
        )
    )
}

/// A table call from AL code ([`Session::al_table_call`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableCall<'a> {
    /// A method of a Record variable: `op`, which one as far as [`DataAccess`] counts it; `temporary`, the record is
    /// temporary.
    Record { op: RecordCall, temporary: bool },
    /// A RecordRef, FieldRef or KeyRef method [`measured_ref_method`] names, by catalog id.
    Ref(&'a str),
}

/// A Record method as [`DataAccess`] counts it ([`TableCall::Record`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordCall {
    FindLast,
    FindFirst,
    SetRange,
    /// Record.ReadIsolation setting IsolationLevel::UpdLock.
    UpdLock,
    Other,
}

impl RecordCall {
    /// The Record method `method` called with `args`.
    pub(crate) fn of(method: al_builtins::record_op::RecordOp, args: &[Value]) -> RecordCall {
        use al_builtins::record_op::RecordOp;
        match method {
            RecordOp::FindLast => RecordCall::FindLast,
            RecordOp::FindFirst => RecordCall::FindFirst,
            RecordOp::SetRange => RecordCall::SetRange,
            RecordOp::ReadIsolation if is_level(args, "UpdLock") => RecordCall::UpdLock,
            _ => RecordCall::Other,
        }
    }
}

/// `args` is the IsolationLevel member `level`.
fn is_level(args: &[Value], level: &str) -> bool {
    matches!(args.first(), Some(Value::Option(i, names))
        if usize::try_from(*i).ok().and_then(|i| names.get(i)).is_some_and(|n| n.eq_ignore_ascii_case(level)))
}

impl Session {
    /// What AL code reached of the table data so far.
    pub fn data_access(&self) -> DataAccess {
        DataAccess {
            store: self.store.accesses(),
            ..self.access_notes.access
        }
    }

    /// Runs `call`, the table call `kind` from AL code, and counts it with what it read and wrote ([`DataAccess`]).
    pub(crate) fn al_table_call<V>(
        &mut self,
        kind: TableCall,
        call: impl FnOnce(&mut Session) -> Result<V, Signal>,
    ) -> Result<V, Signal> {
        let outermost = self.access_notes.table_depth == 0;
        let access = &mut self.access_notes.access;
        match kind {
            TableCall::Record { op, temporary } => {
                access.record_calls += 1;
                access.temporary_record_calls += u64::from(temporary && outermost);
                access.find_last_calls += u64::from(op == RecordCall::FindLast && outermost);
                access.find_first_calls += u64::from(op == RecordCall::FindFirst && outermost);
                access.set_range_calls += u64::from(op == RecordCall::SetRange && outermost);
                access.upd_lock_calls += u64::from(op == RecordCall::UpdLock && outermost);
            }
            TableCall::Ref(id) => {
                access.ref_calls += 1;
                // The calls of OpenOnly (SqRecordRefFail.al:141), the one cell of corpus PR 1468 whose RecordRef calls read no table.
                access.ref_open_close_calls += u64::from(matches!(
                    id.split('#').next(),
                    Some("RecordRef.Open" | "RecordRef.Close")
                ));
            }
        }
        let (store, writes) = (self.store.accesses(), self.store.writes());
        self.access_notes.table_depth += 1;
        let out = call(self);
        self.access_notes.table_depth -= 1;
        if self.access_notes.table_depth == 0
            && matches!(
                kind,
                TableCall::Record {
                    op: RecordCall::FindLast,
                    ..
                }
            )
        {
            self.access_notes.access.find_last_failures += u64::from(out.is_err());
        }
        if self.access_notes.table_depth == 0 {
            let read = self.store.accesses().saturating_sub(store);
            let access = &mut self.access_notes.access;
            access.table_store += read;
            access.table_writes += self.store.writes().saturating_sub(writes);
            match kind {
                TableCall::Ref(_) => access.ref_store += read,
                TableCall::Record {
                    op: RecordCall::FindLast,
                    ..
                } => access.find_last_store += read,
                TableCall::Record {
                    op: RecordCall::FindFirst,
                    ..
                } => access.find_first_store += read,
                TableCall::Record { .. } => {}
            }
        }
        out
    }

    /// A RecordRef, FieldRef or KeyRef method called from AL code: a table call when [`measured_ref_method`] names it and its
    /// RecordRef is neither temporary nor of another company than the session's; other access too when it is not, or when
    /// the call failed, which no cell of corpus PR 1468 does ([`DataAccess`]).
    pub(crate) fn counted_ref_op(
        &mut self,
        id: &str,
        mut receiver: Option<&mut Value>,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        if !measured_ref_method(id) {
            self.note_other_access();
            // The RecordRef calls of the helper of corpus PR 1572 beside the measured ones (GetLastEntryNoFromTable,
            // SqFailedSeqFixture.al:189), told apart by the isolation level they set.
            let load: Option<fn(&mut RefLoads) -> &mut u64> =
                match crate::setrange_convert::catalog_id(id).0.split('#').next() {
                    Some("RecordRef.SetLoadFields") => Some(|l| &mut l.set_load_fields),
                    Some("RecordRef.ReadIsolation") if is_level(args, "ReadUncommitted") => {
                        Some(|l| &mut l.read_uncommitted)
                    }
                    Some("RecordRef.ReadIsolation") if is_level(args, "UpdLock") => Some(|l| &mut l.upd_lock),
                    Some("RecordRef.ReadIsolation") => Some(|l| &mut l.other_isolation),
                    _ => None,
                };
            let out = self.ref_op(id, receiver.as_deref_mut(), args);
            if let Some(count) = load.filter(|_| out.is_ok() && self.ref_of_session_table(receiver.as_deref())) {
                *count(&mut self.access_notes.access.ref_loads) += 1;
            }
            return out;
        }
        let out = self.al_table_call(TableCall::Ref(id), |s| s.ref_op(id, receiver.as_deref_mut(), args));
        if out.is_err() || !self.ref_of_session_table(receiver.as_deref()) {
            self.note_other_access();
        }
        out
    }

    /// The RecordRef of `receiver` (a RecordRef, a FieldRef or KeyRef into one, or a closed RecordRef) is not temporary and
    /// opened in the session's company, as the RecordRefs of corpus PR 1468 are.
    fn ref_of_session_table(&self, receiver: Option<&Value>) -> bool {
        let Some(Value::Library(_, lib)) = receiver else {
            return false;
        };
        match &*lib.borrow() {
            LibraryValue::RecordRef(rec)
            | LibraryValue::FieldRef(rec, _)
            | LibraryValue::KeyFieldRef(rec, _)
            | LibraryValue::KeyRef(rec, _) => {
                let rec = rec.borrow();
                !rec.temporary && rec.company.as_deref().is_none_or(|c| c == &*self.cx.company)
            }
            LibraryValue::ClosedRecordRef => true,
            _ => false,
        }
    }

    /// AL code called a RecordRef, FieldRef, KeyRef or Query method, or a session builtin other than NumberSequence
    /// ([`DataAccess::other_calls`]). A call that code of Microsoft's apps makes while a Record call from AL code runs counts
    /// with that Record call instead: the Record writes of corpus PR 1465 ran on BC with Microsoft's apps, whose global
    /// triggers run under a table's first insert (the System Application's, which call CurrentDateTime and CompanyName).
    /// AL code called a Page or TestPage method ([`DataAccess::page_calls`]).
    pub(crate) fn note_page_call(&mut self) {
        self.access_notes.access.page_calls += 1;
    }

    pub(crate) fn note_other_access(&mut self) {
        if self.access_notes.table_depth > 0 && self.running_microsoft_code() {
            return;
        }
        self.access_notes.access.other_calls += 1;
    }

    /// AL code called the session builtin `id`: NumberSequence methods are not counted, CompanyName has its own count
    /// (Library - Utility.GetNewLineNo passes it to RecordRef.Open, SqRecordRefFail.al:80), any other is other access.
    pub(crate) fn note_session_call(&mut self, id: al_ir::BuiltinId) {
        if crate::sessionops::number_sequence_method(id).is_some() {
            return;
        }
        if id != al_builtins::builtin_id::ids::DATABASE_COMPANYNAME_1 {
            return self.note_other_access();
        }
        if !(self.access_notes.table_depth > 0 && self.running_microsoft_code()) {
            self.access_notes.access.company_name_calls += 1;
        }
    }

    /// The innermost running frame is code of Microsoft's apps ([`Session::of_microsoft_app`]).
    fn running_microsoft_code(&self) -> bool {
        self.frames
            .last()
            .is_some_and(|o| self.of_microsoft_app(self.engine.program.objects[*o].app))
    }

    /// How many points [`Session::tx_log`] held when the sequence `key` (company, name) was last inserted while it was kept.
    pub fn sequence_inserted_at(&self, key: &Key) -> Option<usize> {
        self.access_notes.inserted_at.get(key).copied()
    }

    /// `NumberSequence.Insert` makes the sequence `key`, and notes when ([`Session::sequence_inserted_at`]).
    pub(crate) fn insert_sequence(&mut self, key: Key, sequence: Sequence) {
        if let Some(log) = &self.access_notes.tx_log {
            self.access_notes.inserted_at.insert(key.clone(), log.len());
        }
        self.sequences.insert(key, sequence);
    }

    /// From now on every transaction end and test method body is noted ([`Session::tx_log`]), and every
    /// `NumberSequence.Insert`, until [`Session::stop_noting_tx_ends`]; `measured`: the runner is one al-testrun
    /// `failed_test_runner_measured` names.
    pub fn note_tx_ends(&mut self, measured: bool) {
        self.access_notes.tx_log = Some(Vec::new());
        self.access_notes.measured = measured;
        self.access_notes.inserted_at.clear();
    }

    /// Ends [`Session::note_tx_ends`].
    pub fn stop_noting_tx_ends(&mut self) {
        self.access_notes.tx_log = None;
        self.access_notes.measured = false;
        self.access_notes.deletes_undone.clear();
        self.access_notes.held_at_rollback.clear();
        self.access_notes.inserted_at.clear();
    }

    /// Every point noted since [`Session::note_tx_ends`], in order.
    pub fn tx_log(&self) -> Option<&[TxPoint]> {
        self.access_notes.tx_log.as_deref()
    }

    /// A test method body begins (`start`) or ends: noted while [`Session::note_tx_ends`] notes.
    pub fn note_body(&mut self, start: bool) {
        self.end_body_refusals();
        self.note_tx_point(match start {
            true => TxPointKind::BodyStart,
            false => TxPointKind::BodyEnd,
        });
    }

    /// The commit of an AL `Commit()` statement.
    pub(crate) fn al_commit(&mut self) {
        self.access_notes.commit_by = Some(CommitBy::Statement);
        self.commit_transaction();
    }

    /// The commit of a guarded `Codeunit.Run` of the run's own transaction.
    pub(crate) fn guarded_run_commit(&mut self) {
        self.access_notes.commit_by = Some(CommitBy::GuardedRun);
        self.commit_transaction();
    }

    /// The commit of the own transaction of an isolated event's subscriber.
    pub(crate) fn isolated_event_commit(&mut self) {
        self.access_notes.commit_by = Some(CommitBy::IsolatedEvent);
        self.commit_transaction();
    }

    /// A transaction ended (`commit`: by a commit), noted while [`Session::note_tx_ends`] notes.
    pub(crate) fn note_tx_end(&mut self, commit: bool) {
        self.settle_tx_changes(commit);
        let by = self.access_notes.commit_by.take().unwrap_or(CommitBy::Other);
        self.note_tx_point(match commit {
            true => TxPointKind::Commit(by),
            false => TxPointKind::Rollback,
        });
    }

    fn note_tx_point(&mut self, kind: TxPointKind) {
        if self.access_notes.tx_log.is_none() {
            return;
        }
        let access = self.data_access();
        if let Some(log) = &mut self.access_notes.tx_log {
            log.push(TxPoint { kind, access });
        }
    }

    /// The sequences `keys` (company, name) are gone, as a failed test's rollback takes their `NumberSequence.Insert` back.
    pub fn drop_sequences(&mut self, keys: &[Key]) {
        for key in keys {
            self.sequences.remove(key);
        }
    }

    /// The sequences `start` did not hold that were restarted or deleted since, or inserted while [`Session::tx_log`] was
    /// kept and gone now, sorted.
    pub fn new_sequences_changed_since(&self, start: &SequenceStart) -> Vec<Key> {
        let flagged = (self.sequences_changed.keys()).filter(|k| !start.holds(k) && !start.changed.contains_key(*k));
        let removed =
            (self.access_notes.inserted_at.keys()).filter(|k| !start.holds(k) && !self.sequences.contains_key(*k));
        let mut keys: Vec<Key> = flagged.chain(removed).cloned().collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// The number sequences now, for [`Session::drop_sequences_inserted_since`].
    pub fn sequences_at_start(&self) -> SequenceStart {
        SequenceStart {
            sequences: self.sequences.clone(),
            changed: self.change_flags(),
        }
    }

    fn change_flags(&self) -> HashMap<Key, (bool, bool)> {
        self.sequences_changed
            .iter()
            .map(|(k, c)| (k.clone(), (c.restarted, c.deleted)))
            .collect()
    }

    /// A test of a codeunit under TestIsolation = Codeunit failed: every sequence that `start` did not hold is gone,
    /// as its `NumberSequence.Insert` is rolled back with the failed test, also when an earlier test that passed made it
    /// (tests/al-language-isolation-probe/src/IsolProbeAfterRollback.Codeunit.al:58-69 S3 inserts, S4 fails at :77-78,
    /// and S8 :117-123, S9 :125-132 find the sequence gone: oracle/28/xunit-isolation-codeunit.xml, BC 28.4, runner
    /// 61301). Without a failed test in between, a later test of the codeunit sees the new sequence
    /// (codeunits/run-onrun/SeqCarry.al:20-21, under 130450).
    /// Numbers taken by `Next` from a sequence of `start` are kept, as they are when the codeunit's writes are rolled
    /// back at its end (SeqCarry.al:26-27). `None`: no sequence was new. `Some(true)`: the new ones were removed.
    /// `Some(false)`: a sequence of `start` was restarted, deleted or otherwise changed since, which no probe measures;
    /// the new ones are removed, `start`'s are left as they are.
    pub fn drop_sequences_inserted_since(&mut self, start: &SequenceStart) -> Option<bool> {
        let before = self.sequences.len();
        self.sequences.retain(|k, _| start.sequences.contains_key(k));
        let dropped = self.sequences.len() != before;
        let kept = self.unkept_sequences_since(start).is_empty();
        match (dropped, kept) {
            (_, false) => Some(false),
            (true, true) => Some(true),
            (false, true) => None,
        }
    }

    /// The sequences `start` did not hold, sorted. A failed test under TestIsolation = Codeunit is undone back to its
    /// last `Commit()` (tests/al-language-isolation-probe/src/SdIsolCommitError.Codeunit.al:26-27 under runner 61301,
    /// measured for rows); what that leaves of a new sequence after a `Commit()` is not measured. Call before
    /// [`Session::drop_sequences_inserted_since`].
    pub fn sequences_inserted_since(&self, start: &SequenceStart) -> Vec<Key> {
        let mut new: Vec<Key> = self
            .sequences
            .keys()
            .filter(|k| !start.sequences.contains_key(*k))
            .cloned()
            .collect();
        new.sort();
        new
    }

    /// The sequences of `start` that were restarted, deleted or otherwise changed since, sorted: what a failed test's
    /// rollback leaves of them under TestIsolation = Codeunit is not measured. Numbers taken by `Next` count as kept
    /// (SeqCarry.al:26-27).
    pub fn unkept_sequences_since(&self, start: &SequenceStart) -> Vec<Key> {
        unkept(start, &self.sequences, &self.change_flags())
    }
}
