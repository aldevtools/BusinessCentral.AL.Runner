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
//!   that sends the queue is itself a duplicate. Whether the valid row stays written is not known: refused;
//! - a unique secondary key that a statement-form Insert into such a table breaks (al-runner issue 8287): refused at the Insert;
//! - GetBySystemId, CountApprox, a Query (Open, Read, SaveAsXml, SaveAsJson, SaveAsCsv, also the static Query.SaveAsXml and
//!   Query.SaveAsCsv) and a FlowField that read a table whose Insert error is deferred (al-runner issue 8288): refused
//!   (`refuse_unmeasured_read_while_deferred`). The TableRelation check of a Validate that reads such a table runs as before the
//!   queue (main; not measured);
//! - an `asserterror` body that ends without an error while an Insert error is deferred (al-runner issue 8267); one left by
//!   exit, break or continue is refused where the leave runs (al-runner issue 8289). The slot holds a duplicate key or a Date or DateTime its SQL column cannot hold
//!   (`sql_range.rs` `insert_sql_range`). In every measured cell the deferred error comes out inside the body: a duplicate at a
//!   read of the table (Y7407DuplicateInsertBuffer.al:98-105, :386) or at the Insert that sends the queue (:277, :301); the
//!   SQL-range kind at a read or a Commit, and every body of record/filters/SqlDateFloor1753.al ends in Error(). Whether the
//!   leave of the body sends it, so that the asserterror catches it, is not known: refused (`refuse_pending_at_assert_end`).

use super::*;
use crate::state::WriteOfAl;

/// Rows the queue holds when it is sent.
const SEND_AT: usize = 5;

/// The rows one table (object index, company) has queued since its queue was last sent.
#[derive(Clone, Debug)]
pub(crate) struct InsertQueue {
    table: u32,
    company: Option<String>,
    /// The primary keys of the rows queued since the queue was last sent, one per queued Insert (a duplicate's too): the 5th
    /// sends the queue, and a page background task's worker does not see them (`hide_queued_from_worker`).
    keys: Vec<Vec<Value>>,
}

impl InsertQueue {
    fn new(table: u32, company: Option<String>) -> InsertQueue {
        InsertQueue {
            table,
            company,
            keys: Vec::new(),
        }
    }

    /// The queue is sent: its rows reach the database, other sessions see them, and the count of five starts again.
    fn send(&mut self) {
        self.keys.clear();
    }
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

/// The record methods of a page background task's worker that run on a view without the rows the test session's insert queue
/// holds (pages/background-tasks/BgTaskInsertQueue.al:234-481, :587-657): the ones that send the queue in the test session, and
/// GetBySystemId (:624).
fn worker_hides(method: RecordOp) -> bool {
    sends_queue(method) || method == RecordOp::GetBySystemId
}

/// The methods that read or write the table and so send its queue (the ones that also raise a deferred duplicate, `record_op_inner`).
pub(super) fn sends_queue(method: RecordOp) -> bool {
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
        self.tx.get_from_al = method == RecordOp::Get && !self.through_recordref;
        self.tx.write_of_al = match method {
            _ if self.through_recordref => None,
            RecordOp::Insert => Some(WriteOfAl::Insert),
            RecordOp::Modify => Some(WriteOfAl::Modify),
            RecordOp::ModifyAll => Some(WriteOfAl::ModifyAll),
            _ => None,
        };
        let r = self.record_op(method, rec, args);
        self.tx.write_of_al = None;
        self.tx.get_from_al = false;
        r
    }

    /// Whether the write starting now is the statement `w` of AL code on a Record variable (`TxState::write_of_al`); takes the
    /// mark only when it is.
    pub(crate) fn take_write_of_al(&mut self, w: WriteOfAl) -> bool {
        let is = self.tx.write_of_al == Some(w);
        if is {
            self.tx.write_of_al = None;
        }
        is
    }

    /// Leaves the error of a duplicate-key Insert of `rec` into table `object` pending, unless one is pending already, and notes
    /// which record's Insert it was (`TxState::deferred_by`).
    pub(super) fn defer_insert(&mut self, rec: &Rec, object: u32, e: AlError) {
        self.tx.open.deferred_insert.get_or_insert((object as usize, e));
        self.tx.deferred_by = Some(Rc::as_ptr(rec) as usize);
    }

    /// The end of an `asserterror` body without an error (`op_assert`): an Insert error still deferred here (a duplicate key,
    /// or a value its SQL column cannot hold) is refused. In every measured cell it comes out inside the body
    /// (Y7407DuplicateInsertBuffer.al:98-105, :277, :301, :386; record/filters/SqlDateFloor1753.al, whose bodies end in Error()).
    /// No cell leaves the body with it deferred.
    pub(crate) fn refuse_pending_at_assert_end(&self) -> Result<(), Signal> {
        match &self.tx.open.deferred_insert {
            Some((t, _)) => unsupported(format!(
                "an asserterror whose body ends without an error while the error of an Insert into {} is deferred (a duplicate key, or a value its SQL column cannot hold): whether the asserterror catches it is not pinned (docs/probes-needed.md)",
                self.engine.program.objects[*t].name
            )),
            None => Ok(()),
        }
    }

    /// A read of `table` (object index) that no corpus case measures while an Insert error into it is deferred (al-runner issue
    /// 8288): `read` names it. The measured reads send the queue and raise it (`sends_queue`). Refused. It is also a read outside
    /// the record methods that a page background task's worker makes on a view without the queued rows
    /// (`note_indirect_read_of_queue`).
    pub(crate) fn refuse_unmeasured_read_while_deferred(&mut self, table: usize, read: &str) -> Result<(), Signal> {
        self.note_indirect_read_of_queue(table, true, read)?;
        self.refuse_read_while_deferred(table, read)
    }

    /// A read of `table` (object index) outside the record methods: a FlowField, a Query, CountApprox or GetBySystemId (`view`),
    /// or the TableRelation check of a Validate. The test session's read sends the queue, as its record methods do
    /// (pages/background-tasks/BgTaskInsertQueue.al:481, :708, :730, :747, :765, :783). A page background task's worker makes
    /// the reads of `view` on a view without the queued rows (`hide_queued_in_view`, `hide_queued_from_worker`: :465, :606,
    /// :624, :640, :657); its TableRelation check while the queue holds rows is not measured, and refused.
    pub(crate) fn note_indirect_read_of_queue(&mut self, table: usize, view: bool, read: &str) -> Result<(), Signal> {
        let worker = self.pages.bg_worker.is_some();
        let name = &self.engine.program.objects[table].name;
        for q in self
            .tx
            .open
            .insert_buffer
            .iter_mut()
            .filter(|q| q.table as usize == table && !q.keys.is_empty())
        {
            if !worker {
                q.send();
            } else if !view {
                return unsupported(format!(
                    "{read} of {name} by a page background task's worker while the test session's insert queue holds rows of it (not measured)"
                ));
            }
        }
        Ok(())
    }

    /// Whether the test session's insert queue of `table` (object id) holds rows.
    pub(super) fn queue_holds_rows(&self, table: u32) -> bool {
        self.tx
            .open
            .insert_buffer
            .iter()
            .any(|q| q.table == table && !q.keys.is_empty())
    }

    pub(crate) fn refuse_read_while_deferred(&self, table: usize, read: &str) -> Result<(), Signal> {
        match &self.tx.open.deferred_insert {
            Some((t, _)) if *t == table => unsupported(format!(
                "{read} of {} while the error of an Insert into it is deferred (a duplicate key, or a value its SQL column cannot hold): whether the read raises it is not pinned (docs/probes-needed.md)",
                self.engine.program.objects[table].name
            )),
            _ => Ok(()),
        }
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

    /// The store write of a queued Insert of `rec` (`record_op_inner`): its row goes into the queue now, before the table's
    /// OnAfterInsert trigger and OnAfterInsertEvent subscribers run, so a read of the table inside them sends it, as BC does
    /// (pages/background-tasks/BgTaskInsertQueue.al:691); a subscriber that reads nothing of the table leaves it held back
    /// (:675). `note_insert_buffer` then counts it no more.
    pub(crate) fn queue_stored_row(&mut self, rec: &Rec, t: &TableDef) {
        let own = Rc::as_ptr(rec) as usize;
        if self.tx.queue_pending != Some(own) {
            return;
        }
        self.tx.queue_pending = None;
        let (table, company, key) = {
            let r = rec.borrow();
            (r.table, r.company.clone(), t.primary_key(&r.fields))
        };
        let i = self.insert_queue_of(table, company);
        self.tx.open.insert_buffer[i].keys.push(key);
        self.tx.queue_counted.push(own);
    }

    /// The position of the insert queue of table `table` (object index) and `company`, made when there is none.
    fn insert_queue_of(&mut self, table: u32, company: Option<String>) -> usize {
        let buffer = &mut self.tx.open.insert_buffer;
        match buffer.iter().position(|q| q.table == table && q.company == company) {
            Some(i) => i,
            None => {
                buffer.push(InsertQueue::new(table, company));
                buffer.len() - 1
            }
        }
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
        let own = Rc::as_ptr(rec) as usize;
        // Whether the Insert put its row into the queue at its store write (`queue_stored_row`).
        let counted = match self.tx.queue_counted.iter().rposition(|p| *p == own) {
            Some(i) => {
                self.tx.queue_counted.remove(i);
                true
            }
            None => false,
        };
        // A worker of a page background task runs in a session of its own: its reads send nothing of the test session's queue
        // (pages/background-tasks/BgTaskInsertQueue.al:412).
        if self.pages.bg_worker.is_some() {
            return r;
        }
        // Whether this Insert left its own duplicate key pending (`TxState::deferred_by`).
        let duplicate = method == RecordOp::Insert && self.tx.deferred_by == Some(own);
        if duplicate {
            self.tx.deferred_by = None;
        }
        // Nothing is queued in this transaction: a read or a write has nothing to send.
        if insert != InsertKind::Queued && self.tx.open.insert_buffer.is_empty() {
            return r;
        }
        if insert == InsertKind::Platform
            || rec.borrow().temporary
            || (method != RecordOp::Insert && !sends_queue(method))
        {
            return r;
        }
        let (table, company) = {
            let s = rec.borrow();
            (s.table, s.company.clone())
        };
        if insert != InsertKind::Queued || r.is_err() {
            // A read or a write of the table, a guarded Insert or a failed one sends what is queued.
            if let Some(q) = self
                .tx
                .open
                .insert_buffer
                .iter_mut()
                .find(|q| q.table == table && q.company == company)
            {
                q.send();
            }
            return r;
        }
        let i = self.insert_queue_of(table, company);
        // A duplicate key never reached the store: it is counted here.
        if !counted {
            let key = self.table(rec)?.primary_key(&rec.borrow().fields);
            self.tx.open.insert_buffer[i].keys.push(key);
        }
        let pending = self
            .tx
            .open
            .deferred_insert
            .as_ref()
            .is_some_and(|(o, _)| *o == table as usize);
        if self.tx.open.insert_buffer[i].keys.len() < SEND_AT {
            // A duplicate that stays pending after the queue was sent once in this transaction is the other half of the rule
            // (Y7407DuplicateInsertBuffer.al:229 Dup_6thInsert_OfCommittedRow_CommitAfter3).
            return r;
        }
        self.tx.open.insert_buffer[i].send();
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
        let (_, e) = self.tx.open.deferred_insert.take().expect("checked");
        self.tx.deferred_raised += 1;
        Err(Signal::Error(Box::new(e)))
    }
}

impl Session {
    /// A record method of a page background task's worker on `rec`'s table while the test session's insert queue of that table
    /// holds rows: the worker does not see them (pages/background-tasks/BgTaskInsertQueue.al:234, :262, :277, :312, :361, :412,
    /// :427, :443, and a worker that page code starts, :548; they show once the queue is sent, :247, :294, :331, :393, :566). A
    /// method that reads the store (`worker_hides`) runs on a view of it without those rows (`show_queued_to_test_session`
    /// puts the store back). A Get does so also while the error of a duplicate Insert into the table is deferred: it raises
    /// nothing, and the error comes out later (:800).
    pub(crate) fn hide_queued_from_worker(&mut self, method: RecordOp, rec: &Rec) -> Result<Option<MemStore>, Signal> {
        if self.pages.bg_worker.is_none()
            || self.tx.open.insert_buffer.is_empty()
            || !worker_hides(method)
            || rec.borrow().temporary
        {
            return Ok(None);
        }
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let company = rec.borrow().company.clone();
        let deferred_measured = method == RecordOp::Get;
        self.hide_queued_in_view_of(
            &[(&t, company, cx)],
            "a read of a page background task's worker",
            deferred_measured,
        )
    }

    /// A Query of a page background task's worker reads its tables on a view without the rows the test session's insert queue
    /// holds (BgTaskInsertQueue.al:640).
    pub(crate) fn hide_queued_from_query(&mut self, def: &al_ir::Query) -> Result<Option<MemStore>, Signal> {
        if self.pages.bg_worker.is_none() || self.tx.open.insert_buffer.is_empty() {
            return Ok(None);
        }
        let engine = self.engine.clone();
        let cx = self.cx.clone();
        let tables: Vec<_> = def
            .items
            .iter()
            .filter_map(|item| engine.tables[item.table].as_ref())
            .map(|t| (&**t, None, cx.clone()))
            .collect();
        self.hide_queued_in_view(&tables, "a Query of a page background task's worker")
    }

    /// A read of `tables` (each with the company of its rows and the context they are read in) by a page background task's
    /// worker: when the test session's insert queue of one of them holds rows, the store is replaced by a view of it without
    /// them, and the store comes back (`show_queued_to_test_session`). Refused, as not measured: a read of a table whose Insert
    /// error is deferred (other than a Get, `hide_queued_from_worker`), or whose queue holds rows under another company value.
    pub(crate) fn hide_queued_in_view(
        &mut self,
        tables: &[(&TableDef, Option<String>, al_data::AccessContext)],
        read: &str,
    ) -> Result<Option<MemStore>, Signal> {
        self.hide_queued_in_view_of(tables, read, false)
    }

    /// [`Session::hide_queued_in_view`]; `deferred_measured`: the read runs also while an Insert error into the table is
    /// deferred.
    fn hide_queued_in_view_of(
        &mut self,
        tables: &[(&TableDef, Option<String>, al_data::AccessContext)],
        read: &str,
        deferred_measured: bool,
    ) -> Result<Option<MemStore>, Signal> {
        if self.pages.bg_worker.is_none() || self.tx.open.insert_buffer.is_empty() {
            return Ok(None);
        }
        let mut view: Option<MemStore> = None;
        for (t, company, cx) in tables {
            let queued: Vec<&InsertQueue> = self
                .tx
                .open
                .insert_buffer
                .iter()
                .filter(|q| q.table == t.object && !q.keys.is_empty())
                .collect();
            let keys = match queued.as_slice() {
                [] => continue,
                [q] if q.company == *company => q.keys.clone(),
                _ => {
                    return unsupported(format!(
                        "{read} of {} while the test session's insert queue holds rows of it under another company value (not measured)",
                        t.name
                    ));
                }
            };
            if !deferred_measured {
                self.refuse_read_while_deferred(t.object as usize, read)?;
            }
            let base = view.as_ref().unwrap_or(&self.store);
            view = Some(
                base.without_rows(cx, t, &keys)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?,
            );
        }
        let Some(view) = view else {
            return Ok(None);
        };
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BgTaskInsertQueue);
        Ok(Some(std::mem::replace(&mut self.store, view)))
    }

    /// The read `hide_queued_in_view` ran on a view of the store without the queued rows has ended: the store is the one before
    /// it, untouched.
    pub(crate) fn show_queued_to_test_session(&mut self, hidden: Option<MemStore>) {
        if let Some(store) = hidden {
            self.store = store;
        }
    }
}
