//! A Modify(true) through a stale buffer the stale-buffer rule refuses (`records/stale_rule.rs`): the table's OnModify trigger
//! runs on the buffer, with xRec the buffer itself, then the write raises DB:RecordChanged and the row stays. Measured for the
//! buffers of the variable's own write and for a committed-read buffer, at 1, 2 and 4 Commits, beside Inserts, Modifies,
//! Deletes, DeleteAll and LockTable of another table, after a Get, a FindSet, a rollback and a guarded run (record/crud/
//! StaleRuleMatrix.al: `buf=` one more than before in all 47 refused Modify(true) cells; Y27StaleOtherVarTests.al:269-299,
//! al-runner issue 6522; Y27StaleOwnInsertMore.al:650). A table extension's modify triggers, modify event subscribers and the
//! OnDatabaseModify global trigger are not pinned there: with any of them the error comes before the trigger, as before.

use crate::Session;
use crate::globaltriggers::GlobalOp;
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

impl Session {
    /// The error of a refused stale Modify, after the OnModify trigger (see the module comment). `pk` is the row's key, `theirs`
    /// the stored row's version.
    pub(crate) fn stale_own_insert_modify_refused(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        run: bool,
        pk: &[Value],
        theirs: i64,
    ) -> Result<Signal, Signal> {
        let i = t
            .fields
            .iter()
            .position(|f| f.id == 2_000_000_005)
            .expect("a table with SystemRowVersion");
        let (origin, company) = {
            let r = rec.borrow();
            let mine = if let Value::BigInteger(n) = &r.fields[i] { *n } else { 0 };
            (r.row_origin.filter(|o| o.version == mine), r.company.clone())
        };
        let trigger = self.table_trigger(t, "OnModify");
        let mut hooks = false;
        for n in ["OnBeforeModify", "OnModify", "OnAfterModify"] {
            hooks |= !self.ext_triggers(t, n).is_empty();
        }
        // An OnAfterModify subscriber does not matter: the refused write never reaches it (Y27StaleOwnInsertMore.al:754
        // Y27B_2C_SubscriberLogInTx2_NothingAfter_Refused, `log` 1 from the helper's own Modify only).
        hooks = hooks || self.listened(t, "onbeforemodifyevent", None)? || self.global_enabled(t, GlobalOp::Modify)?;
        let runs_first = match origin {
            // The rule refused it, so the open transaction has a measured shape.
            Some(o) if o.own_write && !o.lock_only => true,
            // A committed-read buffer is refused whatever the open transaction did; the trigger order is measured where the
            // rule knows the shape (StaleRuleMatrix.al CommittedRead_*).
            Some(o) if !o.own_write && run && trigger.is_some() && !hooks => {
                // With a locking ReadIsolation read in the open transaction the order is measured for own-write buffers only
                // (record/crud/CodeunitRunTableNoWrites.al:629-741).
                let write = crate::records::StaleWrite::Modify { run_trigger: run };
                let contact = match self.open_tx_contact(t, &company, pk, o, theirs, write) {
                    Ok(_) if self.locking_read_in_tx() => Err("after a ReadIsolation read that locks"),
                    c => c,
                };
                if let Err(why) = contact {
                    return Err(Signal::unsupported(format!(
                        "Modify(true) through a stale {} buffer read from a committed row, {why}: whether OnModify runs first is not pinned",
                        t.name
                    )));
                }
                true
            }
            // A TransferFields buffer or one of no origin: not pinned, the error first, as before.
            _ => false,
        };
        if run && runs_first && !hooks {
            if let Some(tr) = trigger {
                let x = Self::xrec_handle(rec, rec.borrow().fields.to_vec());
                self.run_trigger(t.object as usize, tr, rec, Some(&x))?;
            }
        }
        self.record_changed(t, rec)
    }

    /// Notes that a guarded Insert of `rec` answered false (the key existed): the buffer keeps the version it had, and a Modify
    /// through it is refused when the row is newer (record/crud/Y71FailedInsertModify.al).
    pub(crate) fn note_failed_insert(&self, t: &TableDef, rec: &Rec, outcome: &Result<Value, Signal>) {
        if !(self.ret_used && matches!(outcome, Ok(Value::Boolean(false))) && !rec.borrow().temporary) {
            return;
        }
        if let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005) {
            let mut b = rec.borrow_mut();
            b.failed_insert = Some(if let Value::BigInteger(n) = &b.fields[i] { *n } else { 0 });
        }
    }

    /// A buffer whose Insert failed on the existing key and whose row another writer changed since it was read cannot be saved,
    /// with no Commit in between (Y71FailedInsertModify.al:FailedInsert_ThenModify_*). A buffer never read (version 0) writes.
    /// A buffer whose version came from its own Modify of another row writes, and so does one whose version reached it
    /// after the open transaction had written the table (`version_after_table_write`).
    pub(crate) fn check_failed_insert_stale(
        &self,
        t: &TableDef,
        rec: &Rec,
        stored: Option<&[Value]>,
    ) -> Result<(), Signal> {
        let (Some(row), Some(i)) = (stored, t.fields.iter().position(|f| f.id == 2_000_000_005)) else {
            return Ok(());
        };
        let failed = rec.borrow_mut().failed_insert.take();
        let version = |v: &Value| if let Value::BigInteger(n) = v { *n } else { 0 };
        let mine = version(&rec.borrow().fields[i]);
        // Only while the buffer still holds the version it had at the failed Insert (a Get since gives it a new one).
        if failed.is_some_and(|v| v != 0 && v == mine && v < version(&row[i])) {
            // A version the buffer's own Modify of another row gave it (the key changed before the Insert: `if not Insert then
            // Modify` over two rows) is not the row's: the Modify writes, in every measured variant (record/crud/
            // CodeunitRunTableNoWrites.al:828-904 BothRows_Twice_SameNote, _CommitBetween, _NewNote, InRow_Twice,
            // BothRows_ThenOutOnly, InOnly_ThenBothRows; al-runner issue 7798).
            let other_row = rec.borrow().last_modify.as_ref().is_some_and(|img| {
                version(&img[i]) == mine
                    && !crate::records::same_key(&t.primary_key(img), &t.primary_key(&rec.borrow().fields))
            });
            if other_row || self.version_after_table_write(t, rec, mine)? {
                return Ok(());
            }
            return Err(self.record_changed(t, rec)?);
        }
        Ok(())
    }

    /// Whether the buffer's version `mine` reached it after the open transaction had written its table (`RowOrigin::
    /// table_written`, judged then; a Commit since does not matter): the Modify after the failed Insert is then
    /// written. A read before any write of the table, or after a write of another table only, raises (corpus PR 1523,
    /// aldevtools/BusinessCentral.AL.Language.Tests#1523, record/crud/TestRecordFailedInsertMovedKey.al: written :283
    /// P6, :393 Q1a, :414 Q1b, :432 Q2, :481 Q4, :503 Q5, :528 Q6, :561 Q7; raised :169-:261 P1-P5, :315 P7, :338 P8,
    /// :454 Q3). Measured for a version from a Get or from the buffer's own Insert (and its own Modify after it),
    /// copied by `:=`, with TransferFields or not, Modify(true) or Modify(), when the stored row at the buffer's key is
    /// one the open transaction inserted and did not write since (every written cell moves the key to such a row).
    /// Every other shape after a write of the table refuses: a row of the buffer's own key changed after the read is
    /// not measured.
    fn version_after_table_write(&self, t: &TableDef, rec: &Rec, mine: i64) -> Result<bool, Signal> {
        use al_builtins::value::TableWritten;
        let Some(o) = rec.borrow().row_origin.filter(|o| o.version == mine) else {
            return Ok(false);
        };
        let get = o.by_get && !o.from_find && !o.own_modified && !o.lock_only;
        let own_insert = o.own_write && !o.read_in_write_tx && !o.by_get && !o.from_find && !o.lock_only;
        let inserted_row = || {
            let r = rec.borrow();
            let pk = crate::sqltime::rounded_key(&t.primary_key(&r.fields));
            self.writes_all_noted()
                && crate::records::inserted_unwritten(&self.tx.open.writes, t.object, &r.company, &pk)
        };
        match o.table_written {
            TableWritten::No => Ok(false),
            TableWritten::ByRecord if (get || own_insert) && !self.through_recordref && inserted_row() => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::FailedInsertAfterTableWrite);
                Ok(true)
            }
            _ => al_builtins::native::unsupported(format!(
                "Modify after a failed Insert of a {} buffer whose row version came after the open transaction \
                 wrote or locked the table: measured only for a Get or the buffer's own Insert after a Record Insert \
                 or Modify of the table, onto a row the transaction inserted and did not write since (not pinned)",
                t.name
            )),
        }
    }

    /// Refuses a Modify through a stale buffer before its events run (`stale_verdict` Refused). After a failed Insert the events
    /// and triggers run first and the write raises (`check_failed_insert_stale`; Y71FailedInsertModify.al:
    /// FailedInsert_ThenModify_CommitBetween `BeforeModify:No,BeforeModify:No`). `versions`: the buffer's, the stored row's.
    pub(crate) fn refuse_stale_modify(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        cx: &al_data::AccessContext,
        pk: &[Value],
        run: bool,
        versions: (i64, i64),
    ) -> Result<(), Signal> {
        let after_failed_insert = rec.borrow().failed_insert.is_some_and(|v| v != 0 && v == versions.0);
        // A committed-read buffer refused with OnModify first is measured for a Get buffer only (StaleRuleMatrix.al CommittedRead_*
        // `get_committed`); Y89StaleFind.al writes Find buffers with Modify(false) and Delete only (docs/probes-needed.md).
        let find_buffer = rec
            .borrow()
            .row_origin
            .is_some_and(|o| o.version == versions.0 && !o.own_write && o.from_find);
        let write = crate::records::StaleWrite::Modify { run_trigger: run };
        let refused = self.stale_verdict(t, rec, cx, pk, versions.1, write)? == crate::records::StaleVerdict::Refused;
        if refused && run && find_buffer {
            return al_builtins::native::unsupported(format!(
                "Modify(true) of a stale committed-read {} buffer from a Find (not pinned)",
                t.name
            ));
        }
        if refused && !after_failed_insert {
            return Err(self.stale_own_insert_modify_refused(t, rec, run, pk, versions.1)?);
        }
        Ok(())
    }
}
