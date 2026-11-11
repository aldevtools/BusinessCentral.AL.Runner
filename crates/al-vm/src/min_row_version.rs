//! `Database.MinimumActiveRowVersion` while the tests of a test codeunit run.
//!
//! What BC shows (corpus probe tests/al-language/metadata/platform-tables/MinimumActiveRowVersionStages.al, corpus PR 1236,
//! measured on BC 27.5, 28.5 and 29.0; header at lines 4-8): the value is fixed when the test codeunit starts, at
//! LastUsedRowVersion() + 1 before any write of the codeunit, and does not move for the rest of the codeunit. `MRV Sequence`.Run
//! (lines 66-122): the first read in a codeunit is LastUsedRowVersion() + 1, the value stays the same through an open Insert
//! and Modify, Commit, more writes, a DeleteAll and its Commit, and in the next test (codeunits 97987, 97988 and 97989 at
//! lines 125-206: the default isolation, RequiredTestIsolation = Codeunit and = Function); the first row inserted after it
//! carries the value as its SystemRowVersion, later rows a greater one, no row is above LastUsedRowVersion(). A first read
//! after three writes of the codeunit, open or committed, is below LastUsedRowVersion() + 1 and the last row is above it
//! (codeunit 97990 at lines 208-246, 97991 at lines 248-287). A new test codeunit takes a new value.

use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;

use crate::Session;

/// The minimum active row version of the running test codeunit ([`Session::freeze_min_active_row_version`]).
#[derive(Default, Clone, Debug, PartialEq)]
pub(crate) struct IsoMin {
    /// The value, once taken.
    value: Option<i64>,
    /// The row versions handed out when the value was taken, or after the last test runner trigger in which only the
    /// runner wrote: a row version handed out since was the tests'.
    mark: i64,
    /// A test read the value, or wrote a row, since it was taken: a test runner trigger no longer moves it.
    held: bool,
    /// A background session committed rows after the value was taken: what the value is then is not pinned.
    unpinned: bool,
}

impl Session {
    /// The value LastUsedRowVersion() + 1 when nothing uncommitted stands in the way: no write transaction is open, every
    /// row version handed out is committed, and the last row version of the test data is known (a configuration package
    /// or a `.bacpac` may hold rows that sort after the session's own). With no row version handed out yet
    /// LastUsedRowVersion() reports 1 while the first row gets 1: no value.
    fn min_row_version_now(&self) -> Option<i64> {
        ((self.engine.test_data.is_none() || self.restored_row_version > 0)
            && self.tx.committed_row_version > 0
            && self.tx.tx_phase != crate::transactions::TX_WRITE
            && self.tx.committed_row_version == self.next_row_version)
            .then(|| self.tx.committed_row_version.max(1) + 1)
    }

    /// The test runner starts the tests of a test codeunit (TestIsolation = Codeunit, or RequiredTestIsolation = Function on
    /// the hub): `MinimumActiveRowVersion` takes the value it has now (LastUsedRowVersion() + 1) and keeps it until the
    /// codeunit ends (the probe in the head of this module). Nothing is taken while the session holds rows it has not
    /// committed, nor over test data whose last row version is not known.
    pub fn freeze_min_active_row_version(&mut self) {
        self.iso_min = IsoMin {
            value: self.min_row_version_now(),
            mark: self.next_row_version,
            ..IsoMin::default()
        };
    }

    /// After a test runner's trigger ran (ADR-066); `before`: the row versions handed out when it began. Its rows are not
    /// the tests': while no test has read the value or written a row, a value taken moves to the state after the trigger
    /// once the trigger's rows are committed (BC passes the corpus's MinimumActiveRowVersion cases,
    /// metadata/platform-tables/PlTRowVersionValue.al, through runner 130450, whose triggers write rows before each test;
    /// the commit after OnBeforeTestRun is pinned by tests/al-language-isolation-probe/src/RpReader.Codeunit.al:94-98,
    /// ADR-066 P4). Once a test read the value or wrote a row, the value stays: the next test of the codeunit reads the
    /// same value (the probe in the head of this module, lines 105-111).
    pub fn runner_trigger_ran(&mut self, before: i64) {
        let m = &mut self.iso_min;
        if m.value.is_none() || m.held {
            return;
        }
        if before != m.mark {
            m.held = true;
            return;
        }
        m.mark = self.next_row_version;
        if let Some(v) = self.min_row_version_now() {
            self.iso_min.value = Some(v);
        }
    }

    /// A background session committed rows ([`crate::bgsession`]): they move the minimum active row version only while
    /// the isolation transaction has not written, which is not pinned.
    pub(crate) fn note_background_commit_for_min_row_version(&mut self) {
        if self.iso_min.value.is_some() {
            self.iso_min.unpinned = true;
        }
    }

    /// `Database.MinimumActiveRowVersion`.
    pub(crate) fn minimum_active_row_version(&mut self) -> Result<Value, Signal> {
        if let Some(min) = self.iso_min.value {
            // LastUsedRowVersion() counts committed rows only. After an Insert and a read of its table it grows on BC
            // (PlTRowVersionValue.al:157-172, `insert open last > start last`), and after a Modify too (PlTDatabase.al:
            // 220-245, `after_modify_greater`): tests compare the two, so the value is refused while the open transaction
            // holds a Delete, Rename or bulk write whose effect on LastUsedRowVersion() is not pinned.
            if let Err(why) = self.last_used_row_version() {
                return unsupported(format!(
                    "Database.MinimumActiveRowVersion after {why} (LastUsedRowVersion is not pinned there)"
                ));
            }
            if self.iso_min.unpinned {
                return unsupported(
                    "Database.MinimumActiveRowVersion after a background session committed rows (not pinned)",
                );
            }
            self.iso_min.held = true;
            return Ok(Value::BigInteger(min));
        }
        // After a Commit that ended a transaction without writes, and with no write since, it is LastUsedRowVersion() + 1
        // (platform/lt-platform-members/PlTRowVersionValue.al:MinimumActiveRowVersion_NoWrite_Committed line 40 and
        // _PrefixedCallEqualsBareCall). Outside the tests of a test codeunit, inside a write transaction or after a Commit
        // that made writes durable, the value is not pinned: refused. platform/lt-platform-members/PlTDatabase.al:250-264
        // pins the bounds 0 <= value <= LastUsedRowVersion() + 1 only.
        if self.tx.tx_phase == crate::transactions::TX_WRITE
            || self.tx.last_commit_wrote
            || self.tx.lock_tx
            || self.tx.lock_unpinned.is_some()
        {
            return unsupported(
                "Database.MinimumActiveRowVersion in a write transaction or after a Commit that made writes durable (the exact value is not pinned; probe wanted)",
            );
        }
        match self.last_used_row_version() {
            Ok(last) => Ok(Value::BigInteger(last + 1)),
            Err(why) => unsupported(format!(
                "Database.MinimumActiveRowVersion after {why} (LastUsedRowVersion is not pinned there)"
            )),
        }
    }
}
