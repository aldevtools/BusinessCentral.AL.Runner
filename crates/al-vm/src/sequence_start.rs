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
use std::collections::HashMap;

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
    inserted_at: HashMap<Key, u64>,
    /// The sequences `Next` or `Range` took numbers from since [`Session::take_numbers_taken`] (al-testrun
    /// `order_hint.rs`, ADR-070 decision 3).
    taken: std::collections::BTreeSet<Key>,
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
    /// A method of a Record variable: `find_last`, it is Record.FindLast; `set_range`, Record.SetRange; `temporary`, the
    /// record is temporary.
    Record {
        find_last: bool,
        set_range: bool,
        temporary: bool,
    },
    /// A RecordRef, FieldRef or KeyRef method [`measured_ref_method`] names, by catalog id.
    Ref(&'a str),
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
            TableCall::Record {
                find_last,
                set_range,
                temporary,
            } => {
                access.record_calls += 1;
                access.temporary_record_calls += u64::from(temporary && outermost);
                access.find_last_calls += u64::from(find_last && outermost);
                access.set_range_calls += u64::from(set_range && outermost);
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
        if self.access_notes.table_depth == 0 && matches!(kind, TableCall::Record { find_last: true, .. }) {
            self.access_notes.access.find_last_failures += u64::from(out.is_err());
        }
        if self.access_notes.table_depth == 0 {
            let read = self.store.accesses().saturating_sub(store);
            let access = &mut self.access_notes.access;
            access.table_store += read;
            access.table_writes += self.store.writes().saturating_sub(writes);
            match kind {
                TableCall::Ref(_) => access.ref_store += read,
                TableCall::Record { find_last: true, .. } => access.find_last_store += read,
                TableCall::Record { find_last: false, .. } => {}
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
            return self.ref_op(id, receiver, args);
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

    /// The value of [`Session::commits`] when the sequence `key` (company, name) was last inserted.
    pub fn sequence_inserted_at(&self, key: &Key) -> Option<u64> {
        self.access_notes.inserted_at.get(key).copied()
    }

    /// `NumberSequence.Insert` makes the sequence `key`, and notes when ([`Session::sequence_inserted_at`]).
    pub(crate) fn insert_sequence(&mut self, key: Key, sequence: Sequence) {
        self.access_notes.inserted_at.insert(key.clone(), self.commits);
        self.sequences.insert(key, sequence);
    }

    /// `NumberSequence.Next` or `Range` took numbers from the sequence `key` ([`Session::take_numbers_taken`]).
    pub(crate) fn note_numbers_taken(&mut self, key: &Key) {
        if !self.access_notes.taken.contains(key) {
            self.access_notes.taken.insert(key.clone());
        }
    }

    /// The sequences (company, name) the session took numbers from since the last call, and starts the next count.
    pub fn take_numbers_taken(&mut self) -> std::collections::BTreeSet<Key> {
        std::mem::take(&mut self.access_notes.taken)
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
