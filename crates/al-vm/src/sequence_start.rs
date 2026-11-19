//! The number sequences as a test codeunit under TestIsolation = Codeunit found them, and what a failed test of it
//! takes back (al-testrun `isolation::after_test`).

use crate::Session;
use crate::sessionops::Sequence;
use std::collections::HashMap;

type Key = (String, String);

/// What a test reached of the table data, counted from the session's start ([`Session::data_access`]): two values compared
/// say whether a test between them read or wrote a table (al-testrun `isolation.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataAccess {
    /// Reads and writes of the database's tables, by any caller.
    pub store: u64,
    /// Calls of a method of a Record variable from AL code, a temporary one too.
    pub record_calls: u64,
    /// Calls of a method of a RecordRef, FieldRef or KeyRef, of a Query, and of IsolatedStorage, from AL code.
    pub other_calls: u64,
}

/// The counts behind [`DataAccess`] and when each number sequence was inserted ([`Session::sequence_inserted_at`]).
#[derive(Debug, Default)]
pub(crate) struct Notes {
    pub(crate) record_calls: u64,
    pub(crate) other_calls: u64,
    inserted_at: HashMap<Key, u64>,
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
}

impl Session {
    /// What AL code reached of the table data so far.
    pub fn data_access(&self) -> DataAccess {
        DataAccess {
            store: self.store.accesses(),
            record_calls: self.access_notes.record_calls,
            other_calls: self.access_notes.other_calls,
        }
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
        let flags = self.change_flags();
        let mut unkept: Vec<Key> = start
            .sequences
            .iter()
            .filter(|&(k, then)| {
                let untouched = flags.get(k) == start.changed.get(k);
                !(untouched
                    && self.sequences.get(k).is_some_and(|now| {
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
}
