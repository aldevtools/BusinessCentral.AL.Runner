//! The one rule for a write through a stale record buffer (al-runner issues 7657 and 6924).
//!
//! A buffer is stale when the last Commit left its row newer than the buffer. Measured on BC 27.5, 28.5 and 29.0
//! (record/crud/StaleRuleMatrix.al, 133 cells, the same on all three) together with the earlier pins
//! (record/TestRecordStaleBufferWrite.al, record/crud/SbStaleOwnModify.al, Y27StaleOtherVarTests.al, Y27StaleOwnInsertMore.al,
//! Y76StaleBuffers.al, StaleTwoCommitsOtherVars.al, record/stale-find/Y89StaleFind.al):
//! 1. A buffer that read an already committed version is refused, whatever the open transaction did.
//! 2. The open transaction starts at the last Commit, the last return of `if Codeunit.Run` (which commits) or the last rollback;
//!    only what happened after that point counts. The number of Commits since the buffer's write does not matter.
//! 3. Row contact (the row modified through any variable, or read by Get under LockTable): written, the stale buffer winning.
//! 4. Table contact (LockTable on the table, or an Insert, Modify, Delete, Rename, ModifyAll or DeleteAll that wrote another row
//!    of the table): written.
//! 5. Otherwise refused with DB:RecordChanged (`Session::record_changed`); Modify(true) runs OnModify first
//!    (`Session::stale_own_insert_modify_refused`).
//!
//! Every input the measured cells did not cover is refused (`OpenTxShape::unknown`), never decided by the rule.
//! After a locking ReadIsolation read in the open transaction the rule decides one shape only (`read_of_others_only`); the other
//! measured shapes are single cells (`locking_read_cells.rs`).

use super::*;
use crate::state::{TxWrite, TxWriteKind};
use al_builtins::value::RowOrigin;

/// Whether two primary keys hold the same values.
pub(crate) fn same_key(a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| al_builtins::ops::compare(x, y).is_ok_and(|o| o == std::cmp::Ordering::Equal))
}

/// What trapped an error whose rollback the rule knows (`Session::trapped_error_rollback`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrappedRollback {
    /// `asserterror` (StaleRuleMatrix.al:206 Rollback).
    AssertError,
    /// A failed `if Codeunit.Run` whose rollback undid no write (record/crud/CodeunitRunTableNoWrites.al:205, :267).
    GuardedRun,
}

/// What the open transaction did to a stale buffer's row and table, as the rule reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Contact {
    /// Rule step 3.
    Row,
    /// Rule step 4.
    Table,
    /// Rule step 5.
    Nothing,
}

impl Session {
    /// Whether the Modify or Delete `write` through `rec` of the stored row `pk` (whose row version is now `theirs`) may write. See the
    /// module comment for the rule. A buffer newer than the row (a rollback to the last Commit after the buffer's own Modify, as an
    /// asserterror does) is not stale (record/StaleBuffer.al P98_Stale_AfterAsserterror_SameBuffer,
    /// P98_Stale_CommitThenErrorThenModify_Rec).
    pub(crate) fn stale_verdict(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        cx: &al_data::AccessContext,
        pk: &[Value],
        theirs: i64,
        write: StaleWrite,
    ) -> Result<StaleVerdict, Signal> {
        let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005) else {
            return Ok(StaleVerdict::Fresh);
        };
        let (mine, origin, temporary, company) = {
            let r = rec.borrow();
            let v = if let Value::BigInteger(n) = &r.fields[i] { *n } else { 0 };
            (v, r.row_origin, r.temporary, r.company.clone())
        };
        if temporary || mine == 0 {
            return Ok(StaleVerdict::Fresh);
        }
        let committed = self
            .store_for(rec)
            .committed_get(cx, t, pk)
            .and_then(|row| match row.get(i) {
                Some(Value::BigInteger(n)) => Some(*n),
                _ => None,
            });
        // Stale: the row changed since the buffer was read, and the last Commit already held a newer row than the buffer.
        if mine >= theirs || !committed.is_some_and(|c| mine < c) {
            return Ok(StaleVerdict::Fresh);
        }
        match origin.filter(|o| o.version == mine) {
            // A buffer that only got TransferFields from a record (Y76StaleBuffers.al Y76_Part1_Copy_TransferFromRecord).
            Some(o) if o.own_write && o.lock_only => {
                let touched = self.row_touched(t, &company, pk);
                let contact = touched
                    || self.tx.rolled_back
                    || self.tx.lock_tx
                    || self.tx.lock_unpinned.is_some()
                    || self.store_for(rec).written_since_commit(cx, t);
                self.stale_lock_only_verdict(t, &company, pk, o.commits, touched, contact)
            }
            // Rule steps 2 to 5: the variable's own Insert, its own Modify or Rename after its Insert or Get in that transaction
            // (SbStaleOwnModify.al:132-448), or a Get or Find of a row written in the transaction then open
            // (SbStaleOwnModify.al:467-640; StaleRuleMatrix.al GetUncommitted_2C, GetThenOwnModify_2C).
            Some(o) if o.own_write => match self.open_tx_contact(t, &company, pk, o, theirs, write) {
                Ok(Contact::Row | Contact::Table) => Ok(StaleVerdict::Written),
                Ok(Contact::Nothing) => Ok(StaleVerdict::Refused),
                Err(why) => unsupported(format!(
                    "a stale {} buffer from its own write, {why} (not pinned)",
                    t.name
                )),
            },
            // Rule step 1 is measured without a rollback since the last Commit only (StaleRuleMatrix.al Rollback has own-write
            // buffers only).
            Some(_) if self.tx.rolled_back => unsupported(format!(
                "a stale committed-read {} buffer after a rollback (not pinned)",
                t.name
            )),
            // Rule step 1 (TestRecordStaleBufferWrite.al:604, :623, :642; StaleRuleMatrix.al CommittedRead_1C, _2C, _More).
            Some(_) => Ok(StaleVerdict::Refused),
            // Not pinned; Microsoft's tests need it (assumption stale-buffer-other-origin-written): the last writer wins, as
            // before any of this was probed.
            None if theirs > self.tx.committed_row_version => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::StaleBufferOtherOriginWritten);
                Ok(StaleVerdict::Written)
            }
            None => Ok(StaleVerdict::Refused),
        }
    }

    /// Whether the open transaction holds a lock of a locking ReadIsolation read.
    pub(crate) fn locking_read_in_tx(&self) -> bool {
        let tx = &self.tx;
        tx.isolation_lock || tx.lock_unpinned == Some(UNPINNED_LOCKING_READ) || !tx.isolation_lock_tables.is_empty()
    }

    /// Whether the open transaction modified the row `pk` of `t` through any Record variable or read it by Get under LockTable.
    fn row_touched(&self, t: &TableDef, company: &Option<String>, pk: &[Value]) -> bool {
        self.tx
            .rows_touched
            .iter()
            .any(|(o, c, k)| *o == t.object && c == company && same_key(k, pk))
    }

    /// Rule steps 2 to 5 for the `write` through a buffer of origin `o`. `Err` names an input no measured cell covers.
    pub(crate) fn open_tx_contact(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &[Value],
        o: RowOrigin,
        theirs: i64,
        write: StaleWrite,
    ) -> Result<Contact, &'static str> {
        let tx = &self.tx;
        let row_touched = self.row_touched(t, company, pk);
        let table_written = self.table_writes_known(t, company, pk)?;
        // The row written in the open transaction by something that is not a Record Modify of it (a page, a write the
        // runtime does not note): not measured.
        if theirs > tx.committed_row_version && !row_touched {
            return Err("after its row was written in the open transaction by a write that is not a Modify");
        }
        // Locks: LockTable (`locked_tables`) and a locking ReadIsolation read are measured; a RecordRef lock and the other locks
        // the runtime marks as not pinned are not.
        let read_lock = self.locking_read_in_tx();
        let other_lock = tx.recref_lock_tx
            || tx.lock_unpinned.is_some_and(|w| w != UNPINNED_LOCKING_READ)
            || (tx.lock_tx && tx.locked_tables.is_empty() && !read_lock);
        if other_lock {
            return Err("after a lock other than LockTable or a ReadIsolation read in the open transaction");
        }
        if tx.rolled_back {
            return Self::after_rollback(tx, (t, company, pk), row_touched, read_lock, write);
        }
        let read_lock_on_table = tx
            .isolation_lock_tables
            .iter()
            .any(|(ob, c)| *ob == t.object && c == company);
        // Rule step 3, beside any other write or LockTable, also beside a ReadIsolation UpdLock read of another table
        // (StaleRuleMatrix.al RowContact_*, Beside_InsertModify_*, Beside_InsertOnly_* (rm_kr), RowReadUnderLock lg_*;
        // StaleTwoCommitsOtherVars.al:332, :395, :441; Y27StaleOtherVarTests.al Y27_2C_*, Y27_3C_*). Row contact beside a locking
        // ReadIsolation read of its own table is not measured.
        if row_touched {
            if read_lock_on_table {
                return Err("after a ReadIsolation read that locks its table, with contact with its row");
            }
            return Ok(Contact::Row);
        }
        // Without row contact a locking read is measured four Commits after the buffer's own write, as a read of another table
        // beside writes of other tables: it changes nothing and steps 4 and 5 decide (WbStaleUpdLockTests.al:16-25, al-runner
        // issue 6287). Else only the single cells of `locking_read_cells.rs`, each with its own result.
        if read_lock && (read_lock_on_table || !self.read_of_others_only(t, o)) {
            return self
                .locking_read_cell(t, company, pk, o, write)
                .ok_or("after a ReadIsolation read that locks, beside a read, write or lock no measured cell covers");
        }
        // Rule step 4 (StaleRuleMatrix.al TableContact_*, BulkSameTable_*, RowReadUnderLock lf_2c; SbStaleOwnModify.al TE_*,
        // TH_*, Y2_*; Y76StaleBuffers.al Y76_Part2_*; Y89StaleFind.al Y89_Part2_*).
        if tx.locked_tables.contains(&t.object) || table_written {
            return Ok(Contact::Table);
        }
        // Rule step 5: writes and LockTable of other tables only, reads, nothing (StaleRuleMatrix.al NoContact_*,
        // GuardedRun_*, RunStatement_*; TestRecordStaleBufferWrite.al:481, :551, :586; Y27StaleOwnInsertMore.al:650).
        Ok(Contact::Nothing)
    }

    /// Four Commits after the buffer's own write, a read under ReadIsolation UpdLock (Find, as No. Series takes a number) of
    /// another table only, beside writes of other tables only, decides as the same writes after a plain Get: DB:RecordChanged
    /// (record/WbStaleUpdLockTests.al:16-25 UpdLockReadOfAnotherTableFourCommitsLaterRecordChanged, contrast :29-38; al-runner
    /// issue 6287). The lock then comes from ReadIsolation alone (no LockTable), on other tables only (the caller checks that).
    fn read_of_others_only(&self, t: &TableDef, o: RowOrigin) -> bool {
        let tx = &self.tx;
        o.commits + 4 == self.commits && tx.locked_tables.is_empty() && tx.writes.iter().all(|w| w.table != t.object)
    }

    /// Whether the open transaction wrote another row of `t` by a write the rule knows. `Err` for a write it does not know: one
    /// the runtime did not note, a Truncate, a Rename that rewrote related rows, an Insert, Delete or Rename of the row itself, a
    /// ModifyAll or DeleteAll of `t` that stopped, matched no row or included the row (TestRecordStaleBufferWrite.al:17 notes a
    /// Windows and Linux difference for a ModifyAll of the row itself).
    fn table_writes_known(&self, t: &TableDef, company: &Option<String>, pk: &[Value]) -> Result<bool, &'static str> {
        if !self.stale_writes_noted() {
            return Err("after a write of the open transaction the runtime does not track");
        }
        let writes = &self.tx.writes;
        let of_table = |w: &TxWrite| w.table == t.object && w.company == *company;
        let mut table_written = false;
        for (i, w) in writes.iter().enumerate() {
            match w.kind {
                TxWriteKind::Truncate => return Err("after a Truncate in the open transaction"),
                TxWriteKind::Other => return Err("after a Rename that rewrote related rows"),
                TxWriteKind::ModifyAll | TxWriteKind::DeleteAll => {
                    let Some(end) = w.bulk else {
                        return Err("after a ModifyAll or DeleteAll that stopped with an error");
                    };
                    if !of_table(w) {
                        continue;
                    }
                    if end.counter == w.before {
                        return Err("after a ModifyAll or DeleteAll of its table that matched no row");
                    }
                    let rows = &writes[i + 1..i + 1 + end.entries];
                    if rows.iter().any(|r| of_table(r) && same_key(&r.pk, pk)) {
                        return Err("after a ModifyAll or DeleteAll that included its row");
                    }
                    // StaleRuleMatrix.al BulkSameTable_1C, BulkSameTable_2C (Linux CI).
                    table_written = true;
                }
                TxWriteKind::Insert | TxWriteKind::Delete | TxWriteKind::Rename
                    if of_table(w) && same_key(&w.pk, pk) =>
                {
                    return Err("after an Insert, Delete or Rename of its row");
                }
                _ if of_table(w) && !same_key(&w.pk, pk) => table_written = true,
                _ => {}
            }
        }
        Ok(table_written)
    }

    /// Whether every write to the database store since the last Commit or rollback is in `tx.writes`, a DeleteAll the store did in
    /// one step counting as its own note (`TxWrite::bulk`).
    fn stale_writes_noted(&self) -> bool {
        let mut at = self.tx.commit_counter;
        for w in &self.tx.writes {
            if w.before != at {
                return false;
            }
            at = match w.bulk {
                Some(end) if w.kind == TxWriteKind::DeleteAll && end.entries == 0 => end.counter,
                _ => w.after,
            };
        }
        at == self.store.change_counter()
    }

    /// The open transaction began at a rollback of `Error('literal')` in a transaction that held no lock; `row` is the buffer's
    /// table, company and key. Measured:
    /// - after `asserterror` (StaleRuleMatrix.al:206 Rollback): nothing after it refuses (rm_rb_1c, rm_rb_2c, om_rb_2c: contact
    ///   before the rollback does not count), a Get and Modify of the row after it writes (rm_rb_rm_2c);
    /// - after a failed guarded `Codeunit.Run` whose rollback undid no write, for the buffer's `Modify()` only: nothing after it
    ///   refuses (record/crud/CodeunitRunTableNoWrites.al:267 Guarded_ModifyCommitError_ThenModify), and so does one Get of the
    ///   row through another variable (:205 Guarded_ModifyCommitError_ThenHandler: `stored after run` reads the row, then the
    ///   handler codeunit's `Rec.Modify()`).
    ///
    /// Any other rollback or anything else after it (a Get alone after an asserterror; after a guarded run a Delete, a
    /// Modify(true), a second Get, a Get and Modify; a Find, another read) is not measured (corpus PR 1313
    /// StaleEdge_LockThenRollback, StaleEdge_DatabaseErrorRollback; docs/probes-needed.md).
    fn after_rollback(
        tx: &crate::state::TxState,
        row: (&TableDef, &Option<String>, &[Value]),
        row_touched: bool,
        read_lock: bool,
        write: StaleWrite,
    ) -> Result<Contact, &'static str> {
        let (t, company, pk) = row;
        if let Some(why) = tx.rollback_unpinned {
            return Err(why);
        }
        let no_lock = !tx.lock_tx && tx.locked_tables.is_empty() && !read_lock;
        let reads = &tx.reads_after_rollback;
        let row_got_only = reads
            .iter()
            .all(|(o, c, k)| *o == t.object && c == company && k.as_deref().is_some_and(|k| same_key(k, pk)));
        let nothing_written = no_lock && tx.writes.is_empty() && tx.rows_touched.is_empty();
        let plain_modify = write == StaleWrite::Modify { run_trigger: false };
        match tx.rollback_kind {
            Some(TrappedRollback::AssertError) if nothing_written && reads.is_empty() => return Ok(Contact::Nothing),
            Some(TrappedRollback::GuardedRun)
                if plain_modify && nothing_written && reads.len() <= 1 && row_got_only =>
            {
                return Ok(Contact::Nothing);
            }
            Some(TrappedRollback::AssertError) => {
                let row_modified_only = tx.writes.iter().all(|w| {
                    w.kind == TxWriteKind::Modify && w.table == t.object && w.company == *company && same_key(&w.pk, pk)
                });
                if no_lock && row_touched && row_modified_only && row_got_only {
                    return Ok(Contact::Row);
                }
            }
            _ => {}
        }
        Err("after a rollback, then a read, a write or a lock other than a Get and Modify of its row")
    }

    /// A rollback ends the open transaction (`rollback_transaction` calls this first): the stale-buffer rule knows it only when it
    /// is an `asserterror` or a failed `if Codeunit.Run` of an `Error` statement (`trapped_error_rollback`) and the transaction held no lock.
    pub(crate) fn note_rollback_for_stale_rule(&mut self) {
        let tx = &mut self.tx;
        let held_lock = tx.lock_tx || !tx.locked_tables.is_empty() || tx.lock_unpinned.is_some();
        let why = if held_lock {
            "after a rollback of a transaction that held a lock"
        } else {
            "after a rollback other than an asserterror of an Error statement"
        };
        tx.rollback_unpinned = tx.rollback_unpinned.or(Some(why));
    }

    /// The rollback of an `asserterror` whose body raised `e`, or of a failed `if Codeunit.Run` whose run raised `e`. An `Error`
    /// statement written with a literal (`ExceptionKind::Dialog`) in a transaction that held no lock is the rollback the corpus
    /// measures: after an asserterror (StaleRuleMatrix.al:206 `rb`), and after a guarded run that did Modify, Commit and
    /// `Error('literal')`, where a later Modify through the caller's stale buffer raises DB:RecordChanged
    /// (record/crud/CodeunitRunTableNoWrites.al:205 Guarded_ModifyCommitError_ThenHandler, :267 _ThenModify; al-runner issue 7795).
    /// An `Error` with a Label, a variable or StrSubstNo (`DialogComputed`) or another error is not measured (docs/probes-needed.md).
    /// A guarded run whose rollback undid a write, and rollbacks of both kinds in one transaction, are not measured.
    pub(crate) fn trapped_error_rollback(
        &mut self,
        e: &al_builtins::native::AlError,
        kind: TrappedRollback,
    ) -> Result<(), Signal> {
        use al_builtins::native::ExceptionKind;
        let before = self.tx.rollback_unpinned;
        let held_lock = self.tx.lock_tx || !self.tx.locked_tables.is_empty() || self.tx.lock_unpinned.is_some();
        let undid_write = !self.tx.writes.is_empty() || self.store.change_counter() != self.tx.commit_counter;
        let earlier = self.tx.rollback_kind;
        self.rollback_in_run()?;
        if before.is_some() || held_lock || e.exception != ExceptionKind::Dialog {
            return Ok(());
        }
        self.tx.rollback_unpinned = if earlier.is_some_and(|k| k != kind) {
            Some("after rollbacks of an asserterror and of a failed guarded Codeunit.Run")
        } else if kind == TrappedRollback::GuardedRun && undid_write {
            Some("after a failed guarded Codeunit.Run whose rollback undid a write")
        } else {
            self.tx.rollback_kind = Some(kind);
            None
        };
        Ok(())
    }

    /// Notes a database read of `rec` while the open transaction began at a rollback (`after_rollback` reads it): a Get with the
    /// key it named, any other read without one.
    pub(crate) fn note_read_after_rollback(&mut self, method: RecordOp, rec: &Rec, args: &[Value]) {
        if !self.tx.rolled_back || rec.borrow().temporary {
            return;
        }
        let key = match method {
            RecordOp::Get => Some(args.to_vec()),
            RecordOp::GetBySystemId
            | RecordOp::Find
            | RecordOp::FindFirst
            | RecordOp::FindLast
            | RecordOp::FindSet
            | RecordOp::Next
            | RecordOp::Count
            | RecordOp::IsEmpty
            | RecordOp::CalcFields
            | RecordOp::CalcSums => None,
            _ => return,
        };
        let (table, company) = {
            let r = rec.borrow();
            (r.table, r.company.clone())
        };
        self.tx.reads_after_rollback.push((table, company, key));
    }

    /// Whether the open transaction's only writes are Modifies of the row `pk` of `t`, with no rollback and no lock of any kind
    /// (the Delete path, record/crud/Y32StaleDeleteTests.al).
    pub(super) fn stale_row_modified_only(&self, t: &TableDef, company: &Option<String>, pk: &[Value]) -> bool {
        let tx = &self.tx;
        self.no_lock_no_rollback()
            && !tx.writes.is_empty()
            && tx.writes.iter().all(|w| {
                w.kind == TxWriteKind::Modify && w.table == t.object && w.company == *company && same_key(&w.pk, pk)
            })
    }

    /// Whether the open transaction's only writes are Deletes of other rows of `t`, with no rollback and no lock of any kind: a
    /// Delete through the stale buffer then runs its triggers and events as for a fresh buffer
    /// (al-language-testlibraries/test-framework/stale-delete-triggers/WbStaleDeleteTests.al:13-53, al-runner issue 6286).
    pub(super) fn stale_after_other_deletes_only(&self, t: &TableDef, company: &Option<String>, pk: &[Value]) -> bool {
        let tx = &self.tx;
        self.no_lock_no_rollback()
            && !tx.writes.is_empty()
            && tx.writes.iter().all(|w| {
                w.kind == TxWriteKind::Delete && w.table == t.object && w.company == *company && !same_key(&w.pk, pk)
            })
    }

    /// No rollback, no lock of any kind and every write noted since the last Commit.
    fn no_lock_no_rollback(&self) -> bool {
        let tx = &self.tx;
        !tx.rolled_back
            && !tx.lock_tx
            && tx.lock_unpinned.is_none()
            && !tx.isolation_lock
            && tx.locked_tables.is_empty()
            && self.writes_all_noted()
    }
}
