//! The insert queue of a table: a duplicate-key Insert that is pending raises when the queue is sent (al-runner issue 7407).
//!
//! BC queues the statement-form Inserts AL code makes into a table without a Blob or AutoIncrement field and sends the queue with
//! its 5th row, with a Commit and with the next access to the table. A duplicate key shows when the queue is sent. Measured on BC 27.5, 28.5 and
//! 29.0 and pinned by record/duplicate-insert/Y7407DuplicateInsertBuffer.al (:152-330): a duplicate that is the 5th insert after
//! 4 queued rows raises at that Insert (Dup_5thInsert_OfFirstRow); a duplicate that is the 6th insert after 5 inserts, the 7th
//! after 6, and one right after a Commit is pending until the next access (Dup_6thInsert_*, Dup_7thInsert_*,
//! Dup_5thInsert_OfCommittedRow_CommitAfter4, Dup_6thInsert_OfCommittedRow_CommitAfter3); four inserts, then the same four keys
//! again, raise inside the asserterror at the first of the second four (Report_FourRowsTwice_NoRead), five and five at the
//! last of the second five (Report_FiveRowsTwice_NoRead), and four, a Commit and the same four keys raise nothing before the next
//! read (Report_FourRowsTwice_CommitBetween).
//!
//! Not measured, and so outside the queue or refused:
//! - an Insert the platform makes itself (an XmlPort import, the row of a page, of a FlowField, a record link): it is not
//!   counted, sends nothing and raises nothing here. Its duplicate key stays pending as before the queue (an XmlPort import:
//!   xmlports/import/XpTImportBasic.al Import_DuplicateKeyExisting, Import_DuplicateKeyWithinInput);
//! - a valid row that sends the queue while a duplicate is pending (`K1, K2, K1, K3, K4`): in every measured cell the Insert
//!   that sends the queue is itself a duplicate. Whether the valid row stays written is not known: refused.

use super::*;

/// Rows the queue holds when it is sent.
const SEND_AT: usize = 5;

/// The rows one table (object index, company) has queued since its queue was last sent, and whether it was sent in this transaction.
#[derive(Clone, Debug)]
pub(crate) struct InsertQueue {
    table: u32,
    company: Option<String>,
    rows: usize,
    sent: bool,
}

/// What an Insert is to the insert queue (`Session::note_insert_buffer`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InsertKind {
    /// A statement-form Insert of AL code into a table BC queues.
    Queued,
    /// An Insert the platform makes itself: outside the queue.
    Platform,
    /// Every other record method, and an Insert of AL code that is not queued (its result is used, a temporary or CRM
    /// record, a table with a Blob or AutoIncrement field, the page client).
    Other,
}

/// The methods that read or write the table and so send its queue (the ones that also raise a deferred duplicate, `record_op_inner`).
fn sends_queue(method: RecordOp) -> bool {
    matches!(
        method,
        RecordOp::Modify
            | RecordOp::Delete
            | RecordOp::DeleteAll
            | RecordOp::ModifyAll
            | RecordOp::Rename
            | RecordOp::Truncate
            | RecordOp::Get
            | RecordOp::LoadFields
            | RecordOp::Find
            | RecordOp::FindFirst
            | RecordOp::FindLast
            | RecordOp::FindSet
            | RecordOp::Next
            | RecordOp::Count
            | RecordOp::IsEmpty
            | RecordOp::CalcFields
            | RecordOp::CalcSums
    )
}

impl Session {
    /// [`Session::record_op`] for a method AL code calls on a Record or a RecordRef: its Insert may go into the insert queue
    /// (`TxState::insert_from_al`).
    pub(crate) fn record_op_of_al(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        self.tx.insert_from_al = method == RecordOp::Insert;
        self.tx.rename_from_al = method == RecordOp::Rename;
        self.record_op(method, rec, args)
    }

    /// Leaves the error of a duplicate-key Insert of `rec` into table `object` pending, unless one is pending already, and notes
    /// which record's Insert it was (`TxState::deferred_by`).
    pub(super) fn defer_insert(&mut self, rec: &Rec, object: u32, e: AlError) {
        self.tx.deferred_insert.get_or_insert((object as usize, e));
        self.tx.deferred_by = Some(Rc::as_ptr(rec) as usize);
    }

    /// Whether an unguarded Insert of AL code into `rec`'s table is queued by the platform: a database table without a Blob or
    /// AutoIncrement field (`TableFacts::insert_unqueued`), outside the page client (which raises a duplicate at once).
    pub(super) fn insert_is_queued(&mut self, rec: &Rec) -> bool {
        let r = rec.borrow();
        !r.temporary
            && !r.crm
            && !self.tx.immediate_errors
            && !self.engine.table_facts[r.table as usize].insert_unqueued
    }

    /// Follows the insert queue of `rec`'s table after `method` ran (`Session::record_op`). `insert` says what an Insert is to
    /// the queue. The 5th queued row sends the queue: a duplicate-key Insert pending for the table raises here.
    pub(crate) fn note_insert_buffer(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        insert: InsertKind,
        r: Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        // Whether this Insert left its own duplicate key pending (`TxState::deferred_by`).
        let own = Rc::as_ptr(rec) as usize;
        let duplicate = method == RecordOp::Insert && self.tx.deferred_by == Some(own);
        if duplicate {
            self.tx.deferred_by = None;
        }
        // Nothing is queued in this transaction: a read or a write has nothing to send.
        if insert != InsertKind::Queued && self.tx.insert_buffer.is_empty() {
            return r;
        }
        if insert == InsertKind::Platform
            || rec.borrow().temporary
            || (method != RecordOp::Insert && !sends_queue(method))
        {
            return r;
        }
        let at = {
            let s = rec.borrow();
            self.tx
                .insert_buffer
                .iter()
                .position(|q| q.table == s.table && q.company == s.company)
        };
        if insert != InsertKind::Queued || r.is_err() {
            // A read or a write of the table, a guarded Insert or a failed one sends what is queued.
            if let Some(i) = at {
                self.tx.insert_buffer[i].rows = 0;
            }
            return r;
        }
        let table = rec.borrow().table;
        let i = at.unwrap_or_else(|| {
            self.tx.insert_buffer.push(InsertQueue {
                table,
                company: rec.borrow().company.clone(),
                rows: 0,
                sent: false,
            });
            self.tx.insert_buffer.len() - 1
        });
        self.tx.insert_buffer[i].rows += 1;
        let pending = self
            .tx
            .deferred_insert
            .as_ref()
            .is_some_and(|(o, _)| *o == table as usize);
        if self.tx.insert_buffer[i].rows < SEND_AT {
            // A duplicate that stays pending after the queue was sent once in this transaction is the other half of the rule
            // (Y7407DuplicateInsertBuffer.al:229 Dup_6thInsert_OfCommittedRow_CommitAfter3).
            return r;
        }
        self.tx.insert_buffer[i].rows = 0;
        self.tx.insert_buffer[i].sent = true;
        if !pending {
            return r;
        }
        let t = self.table(rec)?;
        // In every measured cell the Insert that sends the queue is itself a duplicate (Dup_5thInsert_OfFirstRow,
        // Report_FourRowsTwice_NoRead, Report_FiveRowsTwice_NoRead). A valid row that sends it is written by now: whether BC
        // keeps it when the pending error comes out is not measured.
        if !duplicate {
            return unsupported(format!(
                "an Insert of a valid row into {} that sends the insert queue (its 5th row) while a duplicate-key Insert is pending: not pinned (docs/probes-needed.md)",
                t.name
            ));
        }
        // The After event and trigger of the Insert that sent the queue: whether they run is not measured.
        if self.listened(&t, "onafterinsertevent", None)? || !self.ext_triggers(&t, "OnAfterInsert").is_empty() {
            return unsupported(format!(
                "a duplicate-key Insert into {} sent with the 5th queued row, with an OnAfterInsert subscriber or trigger: not pinned",
                t.name
            ));
        }
        let (_, e) = self.tx.deferred_insert.take().expect("checked");
        self.tx.deferred_raised += 1;
        Err(Signal::Error(Box::new(e)))
    }
}
