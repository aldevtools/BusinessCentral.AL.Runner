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
                if let Err(why) = self.open_tx_contact(t, &company, pk, o, theirs) {
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
                let x = Self::xrec_handle(rec, rec.borrow().fields.clone());
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
            return Err(self.record_changed(t, rec)?);
        }
        Ok(())
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
        if self.stale_verdict(t, rec, cx, pk, versions.1)? == crate::records::StaleVerdict::Refused
            && !after_failed_insert
        {
            return Err(self.stale_own_insert_modify_refused(t, rec, run, pk, versions.1)?);
        }
        Ok(())
    }
}
