//! A page background task's reads under a locking ReadIsolation of the rows the test codeunit wrote (al-runner issue 8638).
//!
//! The worker runs as another session (`pages/background.rs`). Under the test runner's TestIsolation = Codeunit every row the
//! test codeunit wrote stays locked to it for the whole codeunit, also after Commit() and in a later test of the codeunit
//! (corpus PR 1498, open: pages/background-tasks/BgTaskReadIsolationLock.al, assumption `bg-task-readisolation-lock`; a row an
//! earlier test inserted and committed, :563; an earlier test's uncommitted write of Isolated Storage,
//! session-system/isolated-storage/IsolatedStorageInBackgroundTask.al:322-326):
//! - a Get under ReadCommitted, RepeatableRead or UpdLock of such a row waits and fails with NDBCS:LockTimeout: a row the test
//!   modified (:175, :190, :205), inserted and committed (:250, :368, :380), modified and committed (:264), or inserted without
//!   a commit (:352), from TestPage.RunPageBackgroundTask and from the page's own CurrPage.EnqueueBackgroundTask (:535);
//! - a FindFirst and a Count under ReadCommitted filtered on a field outside the key scan the table and fail the same, also for
//!   a group that holds no row (:294, :309, :324);
//! - a Get under ReadCommitted of a key no row has answers not found at once (:280); under ReadUncommitted and without a
//!   ReadIsolation the worker reads the row as it is (:220, :235, :339), which is what the reads do without this module;
//! - a row the test session's insert queue still holds (`records/insert_buffer.rs`) is not in the database and holds no lock:
//!   the worker's ReadCommitted Count and Get read the table without it (:583, :606), on the view of the store without the
//!   queued rows (`hide_queued_from_worker`). Only when every row the codeunit wrote of the table is still queued; another
//!   locking read of a table whose queue holds rows refuses.
//!
//! The error is measured only caught by a TryFunction of the worker, where GetLastErrorText gives each line break of the message
//! as CR LF (the probe's `OneLine`). [`CodeunitRowLocks`] notes the rows from the codeunit's first test on
//! ([`Session::begin_codeunit_row_locks`], called by al-testrun `run.rs`); before it, a locking read of a table a write in the
//! store changed refuses. A write or a lock of the test session whose effect is not measured makes the table's locks unknown,
//! and every locking read of it in a worker refuses; so does every read form next to the measured ones on a table the codeunit
//! wrote.

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{Signal, unsupported};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::Session;
use crate::recmeta::{ISOLATION_REPEATABLE_READ, ISOLATION_UPDLOCK};
use crate::state::TxWriteKind;

type Rec = Rc<RefCell<RecordState>>;

/// The IsolationLevel member ReadCommitted (`al_builtins::sysenum`, "isolationlevel").
const ISOLATION_READ_COMMITTED: i32 = 2;

/// The start of every refusal of this module.
const REFUSAL: &str = "page background task read under ReadIsolation";

/// What the test session of the running test codeunit wrote and locked, as a page background task's locking read meets it
/// (the module comment).
#[derive(Default)]
pub(crate) struct CodeunitRowLocks {
    /// `None` until the codeunit's first test, and nothing is noted; `Some(true)` under TestIsolation = Codeunit.
    codeunit_isolation: Option<bool>,
    /// The primary keys an Insert or a Modify of the test session wrote, by table object and company, one per write: a write
    /// is noted without a scan, and only a worker's locking read looks through the keys of its table (`holds`).
    written: HashMap<(u32, Option<String>), Vec<Vec<Value>>>,
    /// The tables (object, in any company) whose locks are not known, with what made them so.
    unknown: Vec<(u32, &'static str)>,
}

/// What the test codeunit holds of one table.
enum TableLocks {
    /// Nothing: a worker reads it as without this module.
    Untouched,
    /// Rows it wrote by Insert or Modify, and nothing whose effect is not known.
    Written,
    /// A write or lock whose effect is not measured.
    Unknown(&'static str),
}

impl CodeunitRowLocks {
    /// Holds what a new session holds (`install_left.rs`).
    pub(crate) fn as_new(&self) -> bool {
        self.codeunit_isolation.is_none() && self.written.is_empty() && self.unknown.is_empty()
    }

    fn mark_unknown(&mut self, table: u32, why: &'static str) {
        if !self.unknown.iter().any(|(t, _)| *t == table) {
            self.unknown.push((table, why));
        }
    }

    /// Notes the row `pk` an Insert or a Modify wrote; a row its table's last write already wrote is not noted again.
    fn note_written(&mut self, table: u32, company: &Option<String>, pk: &[Value]) {
        let keys = self.written.entry((table, company.clone())).or_default();
        if keys.last().is_none_or(|k| !crate::records::same_key(k, pk)) {
            keys.push(pk.to_vec());
        }
    }

    fn holds(&self, table: u32, company: &Option<String>, pk: &[Value]) -> bool {
        self.written
            .get(&(table, company.clone()))
            .is_some_and(|keys| keys.iter().any(|k| crate::records::same_key(k, pk)))
    }

    fn of(&self, table: u32, company: &Option<String>) -> TableLocks {
        if let Some((_, why)) = self.unknown.iter().find(|(t, _)| *t == table) {
            return TableLocks::Unknown(why);
        }
        if self.written.contains_key(&(table, company.clone())) {
            return TableLocks::Written;
        }
        TableLocks::Untouched
    }
}

/// The lock timeout of a read that waited on another session's lock on a row of `table`, each line break of the message written
/// as `line_break` (NDBCS:LockTimeout; `\\` in the test session, session-system/isolated-storage/IsolatedStorageInBackgroundTask.al:326;
/// CR LF read in a worker, corpus PR 1498 BgTaskReadIsolationLock.al:70-71, :76-86).
pub(crate) fn lock_timeout(table: &str, line_break: &str) -> Signal {
    crate::transactions::al_error_code(
        &format!(
            "We can't save your changes right now, because a record in table '{table}' is being updated in a transaction done by another session.{line_break}{line_break}You'll have to wait until the other transaction has completed, which may take a while. Please try again later."
        ),
        "NDBCS:LockTimeout",
    )
}

/// The record methods that read a table.
fn reads(method: RecordOp) -> bool {
    use RecordOp::*;
    matches!(
        method,
        Get | GetBySystemId
            | Find
            | FindFirst
            | FindLast
            | FindSet
            | Next
            | CalcFields
            | CalcSums
            | Count
            | CountApprox
            | IsEmpty
    )
}

fn isolation_name(isolation: i32) -> &'static str {
    match isolation {
        ISOLATION_READ_COMMITTED => "ReadCommitted",
        ISOLATION_REPEATABLE_READ => "RepeatableRead",
        _ => "UpdLock",
    }
}

fn refuse<T>(why: &str) -> Result<T, Signal> {
    unsupported(format!(
        "{REFUSAL} of a table the test codeunit wrote or locked: {why} (not measured, corpus PR 1498)"
    ))
}

impl Session {
    /// The test codeunit's first test starts: from here on the rows the test session writes are noted, and what OnRun wrote
    /// (the store's changes since `onrun_from`) has locks that are not known. `codeunit_isolation`: TestIsolation = Codeunit.
    pub fn begin_codeunit_row_locks(&mut self, codeunit_isolation: bool, onrun_from: u64) {
        let mut locks = CodeunitRowLocks {
            codeunit_isolation: Some(codeunit_isolation),
            ..Default::default()
        };
        for table in self.store.tables_changed_since(onrun_from) {
            locks.mark_unknown(table, "a write in OnRun");
        }
        self.row_locks = locks;
    }

    /// A Record write of the test session to the database (`records/stale.rs` `note_write`): an Insert or a Modify of the test's
    /// own code notes its row (a ModifyAll's rows follow it as Modifies); any other write, and a write inside a construct that
    /// begins a transaction of its own (`TxState::tx_world`: the page client's writes too), makes the table's locks unknown.
    pub(crate) fn note_row_lock_write(
        &mut self,
        table: u32,
        company: &Option<String>,
        kind: TxWriteKind,
        pk: &[Value],
    ) {
        let Some(codeunit) = self.row_locks.codeunit_isolation else {
            return;
        };
        let why = match kind {
            _ if !codeunit => "a write under a TestIsolation other than Codeunit",
            _ if self.tx.tx_world > 0 => "a write inside a Codeunit.Run, a TestPage operation, a report or an XmlPort",
            TxWriteKind::Insert | TxWriteKind::Modify => {
                self.row_locks.note_written(table, company, pk);
                return;
            }
            TxWriteKind::ModifyAll => return,
            TxWriteKind::Delete => "a Delete",
            TxWriteKind::Rename | TxWriteKind::Other => "a Rename",
            TxWriteKind::DeleteAll => "a DeleteAll",
            TxWriteKind::Truncate => "a Truncate",
        };
        self.row_locks.mark_unknown(table, why);
    }

    /// `LockTable` of a Record or RecordRef on a database table (`recmeta.rs`).
    pub(crate) fn note_row_lock_table(&mut self, table: u32) {
        if self.row_locks.codeunit_isolation.is_some() && self.pages.bg_worker.is_none() {
            self.row_locks.mark_unknown(table, "a LockTable of the test session");
        }
    }

    /// A Commit (`Session::commit_transaction`): the tables a write the runtime does not note changed have unknown locks.
    pub(crate) fn note_row_locks_commit(&mut self) {
        if self.row_locks.codeunit_isolation.is_some() {
            for table in self.unnoted_tables() {
                self.row_locks.mark_unknown(table, "a write the runtime does not note");
            }
        }
    }

    /// A rollback (`Session::rollback_transaction`), before the store undoes it: the tables of the writes it undoes have
    /// unknown locks.
    pub(crate) fn note_row_locks_rollback(&mut self) {
        if self.row_locks.codeunit_isolation.is_none() {
            return;
        }
        let mut tables: Vec<u32> = self.tx.open.writes.iter().map(|w| w.table).collect();
        tables.extend(self.unnoted_tables());
        for table in tables {
            self.row_locks.mark_unknown(table, "a write rolled back");
        }
    }

    /// The tables changed since the last Commit or rollback by a write that is not a noted Record write.
    fn unnoted_tables(&self) -> std::collections::BTreeSet<u32> {
        if self.writes_all_noted() {
            return Default::default();
        }
        self.store.tables_changed_since(self.tx.commit_counter)
    }

    /// Before a record method (`records.rs` `record_op_gated`): a locking read of the test session makes the table's locks
    /// unknown; a worker's read under ReadCommitted, RepeatableRead or UpdLock meets the test codeunit's locks.
    pub(crate) fn row_locks_read(&mut self, method: RecordOp, rec: &Rec, args: &[Value]) -> Result<(), Signal> {
        if !reads(method) {
            return Ok(());
        }
        let (temporary, isolation, table) = {
            let s = rec.borrow();
            (s.temporary, s.read_isolation, s.table)
        };
        if temporary {
            return Ok(());
        }
        if self.pages.bg_worker.is_none() {
            if self.row_locks.codeunit_isolation.is_some()
                && matches!(isolation, ISOLATION_REPEATABLE_READ | ISOLATION_UPDLOCK)
            {
                self.row_locks.mark_unknown(table, "a locking read of the test session");
            }
            return Ok(());
        }
        if !matches!(
            isolation,
            ISOLATION_READ_COMMITTED | ISOLATION_REPEATABLE_READ | ISOLATION_UPDLOCK
        ) {
            // A worker's read sends nothing of the test session's insert queue (`note_insert_buffer`).
            return Ok(());
        }
        self.worker_locking_read(method, rec, args, isolation)
    }

    /// A worker's read under a locking ReadIsolation, by what the test codeunit holds of the table.
    fn worker_locking_read(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &[Value],
        isolation: i32,
    ) -> Result<(), Signal> {
        let (table, company, auto_calc) = {
            let s = rec.borrow();
            (s.table, s.company.clone(), !s.auto_calc.is_empty())
        };
        let calc = auto_calc || matches!(method, RecordOp::CalcFields | RecordOp::CalcSums);
        if self.row_locks.codeunit_isolation.is_none() {
            // Before the test codeunit's first test nothing is noted. A table no write in the store changed holds no lock of
            // the test session, and the read keeps its answer; one that a write changed refuses.
            let changed = self.store.tables_changed_since(0);
            if changed.contains(&table) || (calc && !changed.is_empty()) {
                return refuse("a test session whose writes are not noted");
            }
            return Ok(());
        }
        // A row the insert queue holds is not in the database yet and holds no lock: when every row the codeunit wrote of the
        // table is still queued, a Get and a Count under ReadCommitted read the view of the store without them
        // (`hide_queued_from_worker`): Count leaves them out (BgTaskReadIsolationLock.al:583), Get does not find them (:606).
        if self.queue_holds_rows(table) {
            let only_queued = isolation == ISOLATION_READ_COMMITTED
                && matches!(method, RecordOp::Get | RecordOp::Count)
                && !calc
                && !self.row_locks.unknown.iter().any(|(t, _)| *t == table)
                && self
                    .row_locks
                    .written
                    .get(&(table, company.clone()))
                    .is_none_or(|keys| {
                        keys.iter().all(|k| {
                            self.queued_in_store(table, &company)
                                .iter()
                                .any(|q| crate::records::same_key(q, k))
                        })
                    });
            if only_queued {
                assume(Assumption::BgTaskReadIsolationLock);
                return Ok(());
            }
            return unsupported(format!(
                "{REFUSAL} of a table the test codeunit wrote or locked: a row the test session's insert queue still holds, in a form no cell measures (corpus PR 1498 measures a ReadCommitted Get and Count while every row the codeunit wrote of the table is queued)"
            ));
        }
        let unnoted = self.unnoted_tables();
        let wrote = !self.row_locks.written.is_empty() || !self.row_locks.unknown.is_empty() || !unnoted.is_empty();
        if wrote && calc {
            return refuse("CalcFields, CalcSums or SetAutoCalcFields after the test codeunit wrote rows");
        }
        if unnoted.contains(&table) {
            return refuse(
                "the test codeunit's locks on the table are not known after a write the runtime does not note",
            );
        }
        match self.row_locks.of(table, &company) {
            TableLocks::Untouched => Ok(()),
            TableLocks::Unknown(why) => refuse(&format!(
                "the test codeunit's locks on the table are not known after {why}"
            )),
            TableLocks::Written => self.read_of_written_table(method, rec, args, isolation),
        }
    }

    /// A worker's locking read of a table the test codeunit wrote rows of: the measured forms wait on the lock; a Get of a key
    /// it did not write keeps its answer (:280); every other form refuses.
    fn read_of_written_table(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &[Value],
        isolation: i32,
    ) -> Result<(), Signal> {
        let t = self.table(rec)?;
        match method {
            RecordOp::Get => {
                if args.iter().any(|a| matches!(a, Value::Library(..))) {
                    return refuse("a Get by a RecordId");
                }
                if args.len() > t.keys[0].len() {
                    return Ok(());
                }
                let key = self.get_key(rec, &t, args, false)?;
                let company = rec.borrow().company.clone();
                if !self.row_locks.holds(t.object, &company, &key.pk) {
                    return Ok(());
                }
                if key.given < key.pk.len() {
                    return refuse("a Get by fewer values than the primary key has fields");
                }
                self.measured_lock_read(rec, &t, method)?;
            }
            RecordOp::FindFirst | RecordOp::Count if isolation == ISOLATION_READ_COMMITTED => {
                if method == RecordOp::FindFirst && !self.ret_used {
                    return refuse("FindFirst as a statement");
                }
                self.measured_lock_read(rec, &t, method)?;
                Self::measured_group_filters(rec, &t)?;
            }
            _ => return refuse(&format!("{method} under {}", isolation_name(isolation))),
        }
        assume(Assumption::BgTaskReadIsolationLock);
        Err(lock_timeout(&t.name, "\r\n"))
    }

    /// The record, the table and the call of the probe's reads (BgTaskReadIsolationLock.al:22-37, :70, :88-123): a Record variable of the
    /// session company, its table with no key but the primary key and with its name as its Caption, no SetLoadFields,
    /// SetAutoCalcFields, MarkedOnly or other current key, a Get with no filter, read in the TryFunction's own statements
    /// under the default lock timeout.
    fn measured_lock_read(&self, rec: &Rec, t: &TableDef, method: RecordOp) -> Result<(), Signal> {
        let s = rec.borrow();
        let fresh = crate::state::TxState::fresh();
        let why = if self.through_recordref {
            "a read through a RecordRef"
        } else if t.keys.len() != 1 {
            "a table with a key other than the primary key"
        } else if self.source_caption(t) != t.name {
            "a table with a Caption other than its name"
        } else if s.company.is_some() || s.change_company_called {
            "a record of ChangeCompany"
        } else if s.load.request.is_some() || !s.auto_calc.is_empty() || s.marked_only || s.key != 0 {
            "SetLoadFields, SetAutoCalcFields, MarkedOnly or a current key other than the primary key"
        } else if method == RecordOp::Get && !s.filters.is_empty() {
            "a Get of a record with filters"
        } else if !self.in_own_used_try() {
            "a lock timeout that leaves the worker (not caught in the TryFunction that reads)"
        } else if self.tx.lock_timeout != fresh.lock_timeout
            || self.tx.lock_timeout_duration != fresh.lock_timeout_duration
        {
            "a LockTimeout or LockTimeoutDuration of its own"
        } else {
            return Ok(());
        };
        refuse(why)
    }

    /// The filters of the probe's FindFirst and Count (BgTaskReadIsolationLock.al:109-121): at least one, each in filter group 0
    /// on a field outside every key, ascending.
    fn measured_group_filters(rec: &Rec, t: &TableDef) -> Result<(), Signal> {
        let s = rec.borrow();
        let why = if s.filters.is_empty() {
            "a read without a filter"
        } else if s.filters.iter().any(|f| f.group != 0) {
            "a filter in another filter group"
        } else if s.filters.iter().any(|f| t.keys.iter().any(|k| k.contains(&f.field))) {
            "a filter on a key field"
        } else if !s.ascending || s.field_descending != 0 {
            "a descending read"
        } else {
            return Ok(());
        };
        refuse(why)
    }
}
