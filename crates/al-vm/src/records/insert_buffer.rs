//! The insert queue of a table: a duplicate-key Insert that is pending raises when the queue is sent (al-runner issue 7407).
//!
//! BC queues the statement-form Inserts of a table without a Blob or AutoIncrement field and sends the queue with its 5th row, with
//! a Commit and with the next access to the table. A duplicate key shows when the queue is sent. Measured on BC 27.5, 28.5 and
//! 29.0 (corpus pull request 1279, record/duplicate-insert/Y7407DuplicateInsertBuffer.al): a duplicate that is the 5th insert after
//! 4 queued rows raises at that Insert (Dup_5thInsert_OfFirstRow); a duplicate that is the 6th insert after 5 inserts, the 7th
//! after 6, and one right after a Commit is pending until the next access (Dup_6thInsert_*, Dup_7thInsert_*,
//! Dup_5thInsert_OfCommittedRow_CommitAfter4, Dup_6thInsert_OfCommittedRow_CommitAfter3); four inserts, then the same four keys
//! again, raise inside the asserterror at the first of the second four (Report_FourRowsTwice_NoRead), five and five at the
//! last of the second five (Report_FiveRowsTwice_NoRead), and four, a Commit and the same four keys raise nothing before the next
//! read (Report_FourRowsTwice_CommitBetween).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

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
    /// Whether an unguarded Insert into `rec`'s table is queued by the platform: a database table without a Blob or AutoIncrement
    /// field, outside the page client and XmlPort imports (which raise a duplicate at once).
    pub(super) fn insert_is_queued(&mut self, rec: &Rec) -> bool {
        if rec.borrow().temporary || rec.borrow().crm || self.tx.immediate_errors {
            return false;
        }
        match self.table(rec) {
            Ok(t) => !self.has_blob_field(&t) && matches!(self.auto_increment_field(&t), Ok(None)),
            Err(_) => false,
        }
    }

    /// Follows the insert queue of `rec`'s table after `method` ran (`Session::record_op`). `queued` says the Insert was a statement
    /// form that BC queues. The 5th queued row sends the queue: a duplicate-key Insert pending for the table raises here.
    pub(crate) fn note_insert_buffer(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        queued: bool,
        r: Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        if rec.borrow().temporary || (method != RecordOp::Insert && !sends_queue(method)) {
            return r;
        }
        let (table, company) = {
            let s = rec.borrow();
            (s.table, s.company.clone())
        };
        let at = self
            .tx
            .insert_buffer
            .iter()
            .position(|q| q.table == table && q.company == company);
        if method != RecordOp::Insert || r.is_err() || !queued {
            // A read or a write of the table, a guarded Insert or a failed one sends what is queued.
            if let Some(i) = at {
                self.tx.insert_buffer[i].rows = 0;
            }
            return r;
        }
        let i = at.unwrap_or_else(|| {
            self.tx.insert_buffer.push(InsertQueue {
                table,
                company,
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
            // A duplicate that stays pending after the queue was sent once in this transaction is the other half of the rule.
            if pending && self.tx.insert_buffer[i].sent {
                assume(Assumption::InsertQueueSentAtFifthRow);
            }
            return r;
        }
        self.tx.insert_buffer[i].rows = 0;
        self.tx.insert_buffer[i].sent = true;
        if !pending {
            return r;
        }
        assume(Assumption::InsertQueueSentAtFifthRow);
        // The After event and trigger of the Insert that sent the queue: whether they run is not measured.
        let t = self.table(rec)?;
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
