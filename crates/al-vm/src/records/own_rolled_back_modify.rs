//! A Modify through a buffer whose own last Modify a rollback undid (al-runner issue 7464).

use super::*;

impl Session {
    /// The checks of a Modify on the buffer's SystemRowVersion against the stored row `stored` (`None`: no row with the key). A
    /// buffer older than a committed change to its row cannot be saved (record/TestRecordStaleBufferWrite.al); before a commit
    /// the last writer wins. `Some`: the answer of a Modify whose own write a rollback undid (the two functions below).
    pub(super) fn modify_version_checks(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        cx: &al_data::AccessContext,
        pk: &[Value],
        run: bool,
        stored: Option<&[Value]>,
    ) -> Result<Option<Value>, Signal> {
        let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005) else {
            return Ok(None);
        };
        let Some(row) = stored else {
            return self.modify_own_rolled_back_insert(t, rec, i, run);
        };
        let version = |v: &Value| if let Value::BigInteger(n) = v { *n } else { 0 };
        let v = (version(&rec.borrow().fields[i]), version(&row[i]));
        self.refuse_stale_modify(t, rec, cx, pk, run, v)?;
        self.modify_own_rolled_back_write(t, rec, i, v, run)
    }

    /// The buffer still holds what its own last Modify wrote and a rollback undid that write: the Modify answers true and writes
    /// nothing, so the row keeps what the rollback restored (record/StaleBuffer.al:76-86); a buffer changed since writes the whole
    /// row (:167-168), which the caller's normal path does. `versions` are the buffer's and the stored row's SystemRowVersion.
    ///
    /// The events and triggers of that Modify: [`Session::own_rolled_back_events`]. `None` when the buffer is not in this state.
    pub(super) fn modify_own_rolled_back_write(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        version_field: usize,
        versions: (i64, i64),
        run: bool,
    ) -> Result<Option<Value>, Signal> {
        let version = |v: &Value| if let Value::BigInteger(n) = v { *n } else { 0 };
        let own_image = {
            let r = rec.borrow();
            !r.temporary
                && versions.0 > versions.1
                && r.last_modify
                    .as_ref()
                    .is_some_and(|img| version(&img[version_field]) == versions.0)
        };
        if !own_image {
            return Ok(None);
        }
        let img = rec.borrow().last_modify.clone().expect("checked");
        if !Session::modify_changes_nothing(t, &img, rec) {
            return Ok(None);
        }
        self.own_rolled_back_events(t, rec, run).map(Some)
    }

    /// The same Modify when the rollback also undid the Insert of the buffer's row (no Commit after it), and nothing was written
    /// since the rollback: it answers true and writes nothing, the row stays gone, and the OnBeforeModifyEvent and
    /// OnAfterModifyEvent subscribers run once each (record/events/Y7464ModifyAfterRolledBackModify.al:281-304
    /// Modify_AfterRolledBackInsertAndModify, BC 27.5, 28.5 and 29.0). Measured only for a table with such subscribers; a table
    /// without them is refused. `None` when the buffer is not in
    /// this state.
    pub(super) fn modify_own_rolled_back_insert(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        version_field: usize,
        run: bool,
    ) -> Result<Option<Value>, Signal> {
        let version = |v: &Value| if let Value::BigInteger(n) = v { *n } else { 0 };
        let own_image = {
            let r = rec.borrow();
            let mine = version(&r.fields[version_field]);
            !r.temporary
                && mine > 0
                && r.last_modify
                    .as_ref()
                    .is_some_and(|img| version(&img[version_field]) == mine)
        };
        if !own_image || !self.tx.rolled_back || !self.tx.writes.is_empty() || !self.writes_all_noted() {
            return Ok(None);
        }
        let img = rec.borrow().last_modify.clone().expect("checked");
        // A buffer changed since raises RecordNotFound, which the caller's normal path does (json/json/P101Streams.al:174-176,
        // :193-194 `af198`: a Blob written again after the rollback).
        if !Session::modify_changes_nothing(t, &img, rec) {
            return Ok(None);
        }
        if !(self.listened(t, "onbeforemodifyevent", None)? || self.listened(t, "onaftermodifyevent", None)?) {
            return unsupported(format!(
                "a Modify of an unchanged {} buffer whose own Modify and the Insert of its row a rollback undid, on a table without Modify event subscribers: not pinned",
                t.name
            ));
        }
        // Modify(true) here is measured only after a Commit kept the Insert (Y7464ModifyAfterRolledBackModify.al:338-361).
        if run && self.table_trigger(t, "OnModify").is_some() {
            return unsupported(format!(
                "Modify(true) of an unchanged {} buffer whose own Modify and the Insert of its row a rollback undid, on a table with an OnModify trigger: not pinned",
                t.name
            ));
        }
        self.own_rolled_back_events(t, rec, run).map(Some)
    }

    /// The Modify of a buffer its own rolled-back write left: true, nothing written. With OnBeforeModifyEvent or
    /// OnAfterModifyEvent subscribers the events run once each, with Rec and xRec both the buffer and the RunTrigger the call
    /// gave; the stored row stays as the rollback left it (record/events/Y7464ModifyAfterRolledBackModify.al:124-280, :308-336:
    /// _BufferUnchanged, _ModifyFalse, _NoAssignment, _OtherFieldReset, _TwoModifiesBeforeTheError, _CommitBetweenModifies; BC
    /// 27.5, 28.5, 29.0). Modify(true) on a table with its own OnModify trigger and both subscribers: the trigger runs between
    /// the events with xRec the buffer, and nothing is written (:338-361 ModifyTrue_AfterRolledBackModify_BufferUnchanged).
    /// A trigger without both subscribers, a table extension or database trigger, and a trigger that changes Rec are not
    /// measured: refused.
    fn own_rolled_back_events(&mut self, t: &TableDef, rec: &Rec, run: bool) -> Result<Value, Signal> {
        let own_trigger = if run { self.table_trigger(t, "OnModify") } else { None };
        let mut ext_trigger = false;
        for n in ["OnBeforeModify", "OnModify", "OnAfterModify"] {
            ext_trigger |= run && !self.ext_triggers(t, n).is_empty();
        }
        if ext_trigger || self.global_enabled(t, GlobalOp::Modify)? {
            return unsupported(format!(
                "a Modify of a {} buffer that its own rolled-back write left, with table extension or database triggers: not pinned",
                t.name
            ));
        }
        let before = self.listened(t, "onbeforemodifyevent", None)?;
        let after = self.listened(t, "onaftermodifyevent", None)?;
        if own_trigger.is_some() && !(before && after) {
            return unsupported(format!(
                "Modify(true) of a {} buffer that its own rolled-back write left, whose table has an OnModify trigger but not both Modify event subscribers: not pinned",
                t.name
            ));
        }
        if before || after {
            let image = rec.borrow().fields.clone();
            let xrec = Self::xrec_handle(rec, image.clone());
            self.table_event(t, "onbeforemodifyevent", rec, Some(&xrec), run, None)?;
            // What the answer is when a subscriber or the trigger changes the buffer is not measured.
            let same = |r: &Rec| r.borrow().fields.iter().zip(&image).all(|(a, b)| a.identical(b));
            if !same(rec) || !same(&xrec) {
                return unsupported(format!(
                    "a Modify of a {} buffer that its own rolled-back write left, whose OnBeforeModifyEvent subscriber changes Rec or xRec: not pinned",
                    t.name
                ));
            }
            if let Some(tr) = own_trigger {
                self.run_trigger(t.object as usize, tr, rec, Some(&xrec))?;
                if !same(rec) || !same(&xrec) {
                    return unsupported(format!(
                        "Modify(true) of a {} buffer that its own rolled-back write left, whose OnModify trigger changes Rec or xRec: not pinned",
                        t.name
                    ));
                }
            }
            self.table_event(t, "onaftermodifyevent", rec, Some(&xrec), run, None)?;
        }
        Ok(Value::Boolean(true))
    }
}
