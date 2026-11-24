//! The isolated storage and the number sequences at a test runner's checkpoint (al-testrun `isolation::Checkpoint`), and
//! the keys and sequences whose state after the rollback no probe measures: a later test of the codeunit that reaches one
//! is refused at that call (`refuse_unmeasured_key`, `refuse_unmeasured_sequence`), and every other test runs. A session
//! whose sequences as a whole are not known refuses every NumberSequence call (`mark_numbers_unmeasured`).
//!
//! the isolated storage and number sequences at a test runner's checkpoint (TestIsolation =
//! Function), what a restore finds changed among what the checkpoint held (`Unkept`), and the keys
//! and sequences a later test of the codeunit may not reach: `IsolatedStorage` and `NumberSequence`
//! calls on them are refused at the call (al-runner issue 8002). Set by al-testrun
//! `isolation::after_test`, cleared at the codeunit's end. A session whose number sequences as a
//! whole are not known refuses every `NumberSequence` call (`mark_numbers_unmeasured`, set by al-
//! testrun `order_hint::mark_unmeasured_numbers`; al-runner issue 8152). Entry points:
//! `storage_checkpoint`, `restore_storage_checkpoint`, `refuse_unmeasured_key`,
//! `refuse_unmeasured_sequence`, `clear_unmeasured`, `mark_numbers_unmeasured`.

use crate::{Session, sessionops};
use al_builtins::native::{Signal, unsupported};
use std::collections::HashMap;
use std::rc::Rc;

type Key = (String, String);

/// The isolated storage and the number sequences of a session at a test runner's checkpoint
/// ([`Session::storage_checkpoint`]).
#[derive(Debug, Clone, Default)]
pub struct StorageCheckpoint {
    isolated: HashMap<Key, (String, bool, bool)>,
    sequences: HashMap<Key, sessionops::Sequence>,
}

/// What a checkpoint held and a test changed or removed ([`Session::restore_storage_checkpoint`]): isolated storage
/// slots by (scope, key) and number sequences by (company, name).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Unkept {
    pub isolated: Vec<Key>,
    pub sequences: Vec<Key>,
}

/// Isolated storage slots and number sequences a later test of the running codeunit may not reach, each with the reason
/// of its refusal. Matched by the slot's scope or the sequence's company, and by the name without trailing spaces and in
/// any case: a lookup that finds a stored name by a looser match (`trailing_twin`) is refused too.
#[derive(Debug, Default)]
pub(crate) struct Unmeasured {
    isolated: HashMap<Key, String>,
    sequences: HashMap<Key, String>,
    /// Number sequences whose state after a failed test runs on an assumption (`assume_sequences_after_failure`).
    assumed: std::collections::HashSet<Key>,
}

impl Unmeasured {
    /// Holds what a new session holds (`install_left.rs`). Every field is named: a new one does not compile here until
    /// it is decided.
    pub(crate) fn as_new(&self) -> bool {
        let Unmeasured {
            isolated,
            sequences,
            assumed,
        } = self;
        isolated.is_empty() && sequences.is_empty() && assumed.is_empty()
    }
}

/// The keys of `then` whose value `now` does not hold, sorted.
fn unkept<V: PartialEq>(then: &HashMap<Key, V>, now: &HashMap<Key, V>) -> Vec<Key> {
    let mut keys: Vec<Key> = then
        .iter()
        .filter(|&(k, v)| now.get(k) != Some(v))
        .map(|(k, _)| k.clone())
        .collect();
    keys.sort();
    keys
}

fn loose(scope: &str, name: &str) -> Key {
    (scope.to_string(), name.trim_end_matches(' ').to_lowercase())
}

impl Session {
    /// The isolated storage and the number sequences now, for [`Session::restore_storage_checkpoint`] (a test runner's
    /// TestIsolation = Function, al-testrun `isolation::Checkpoint`).
    pub fn storage_checkpoint(&self) -> StorageCheckpoint {
        StorageCheckpoint {
            isolated: self.isolated_storage.clone(),
            sequences: self.sequences.clone(),
        }
    }

    /// Puts the isolated storage and the number sequences back as they were at `cp`. `None`: nothing changed since `cp`.
    /// Otherwise the keys and sequences `cp` already held that changed or went away since: they are put back too, but
    /// only a new key's `IsolatedStorage.Set` and a new sequence's `NumberSequence.Insert` are measured to be rolled back.
    pub fn restore_storage_checkpoint(&mut self, cp: &StorageCheckpoint) -> Option<Unkept> {
        if self.isolated_storage == cp.isolated && self.sequences == cp.sequences {
            return None;
        }
        let changes = Unkept {
            isolated: unkept(&cp.isolated, &self.isolated_storage),
            sequences: unkept(&cp.sequences, &self.sequences),
        };
        self.isolated_storage = cp.isolated.clone();
        self.isolated_committed = cp.isolated.clone();
        self.sequences = cp.sequences.clone();
        Some(changes)
    }

    /// A later test of the running codeunit that reaches isolated storage slot `slot` (scope, key) is refused with
    /// `reason`.
    pub fn refuse_unmeasured_key(&mut self, slot: &Key, reason: String) {
        self.unmeasured.isolated.insert(loose(&slot.0, &slot.1), reason);
    }

    /// A later test of the running codeunit that reaches number sequence `key` (company, name) is refused with `reason`.
    pub fn refuse_unmeasured_sequence(&mut self, key: &Key, reason: String) {
        self.unmeasured.sequences.insert(loose(&key.0, &key.1), reason);
    }

    /// What a failed test of the running codeunit left of number sequences `keys` (company, name) runs on assumption
    /// `SequenceAfterFailedTest` (al-testrun `isolation::after_test`): every later NumberSequence call of the codeunit on
    /// one of them passes through it.
    pub fn assume_sequences_after_failure(&mut self, keys: &[Key]) {
        self.unmeasured.assumed.extend(keys.iter().map(|k| loose(&k.0, &k.1)));
    }

    /// Whether an earlier failed test of the running codeunit left number sequence `key` (company, name) on that
    /// assumption ([`Session::assume_sequences_after_failure`]).
    pub fn sequence_left_by_failure(&self, key: &Key) -> bool {
        self.unmeasured.assumed.contains(&loose(&key.0, &key.1))
    }

    /// The running codeunit's tests ended: the refusals of `refuse_unmeasured_key` and `refuse_unmeasured_sequence`, and
    /// the marks of `assume_sequences_after_failure`, end.
    pub fn clear_unmeasured(&mut self) {
        self.unmeasured = Unmeasured::default();
    }

    /// Refuses an `IsolatedStorage` call on `key` in `scope` that an earlier test of the codeunit left unmeasured.
    pub(crate) fn check_unmeasured_key(&self, scope: &str, key: &str) -> Result<(), Signal> {
        match self.unmeasured.isolated.get(&loose(scope, key)) {
            Some(reason) => unsupported(reason.clone()),
            None => Ok(()),
        }
    }

    /// From now on every NumberSequence call of this session is refused with `reason` (al-testrun
    /// `order_hint::mark_unmeasured_numbers`). Not ended by [`Session::clear_unmeasured`]: it holds for the session.
    pub fn mark_numbers_unmeasured(&mut self, reason: Rc<str>) {
        self.numbers_unmeasured = Some(reason);
    }

    /// Refuses a `NumberSequence` call on `key` that an earlier test of the codeunit left unmeasured, and every call in a
    /// session marked by [`Session::mark_numbers_unmeasured`].
    pub(crate) fn check_unmeasured_sequence(&self, key: &Key) -> Result<(), Signal> {
        if let Some(reason) = &self.numbers_unmeasured {
            return unsupported(reason.to_string());
        }
        let key = loose(&key.0, &key.1);
        if let Some(reason) = self.unmeasured.sequences.get(&key) {
            return unsupported(reason.clone());
        }
        if self.unmeasured.assumed.contains(&key) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::SequenceAfterFailedTest);
        }
        Ok(())
    }
}
