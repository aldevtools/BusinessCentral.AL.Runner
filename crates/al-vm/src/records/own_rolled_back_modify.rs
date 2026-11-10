//! A Modify through a buffer whose own last Modify a rollback undid (al-runner issue 7464).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

impl Session {
    /// The buffer still holds what its own last Modify wrote and a rollback undid that write: the Modify answers true and writes
    /// nothing, so the row keeps what the rollback restored (record/StaleBuffer.al:76-86); a buffer changed since writes the whole
    /// row (:167-168), which the caller's normal path does. `versions` are the buffer's and the stored row's SystemRowVersion.
    ///
    /// With OnBeforeModifyEvent or OnAfterModifyEvent subscribers the events run once each, with Rec and xRec both the buffer and
    /// the RunTrigger the call gave; the stored row stays as the rollback left it (corpus pull request 1279, record/events/
    /// Y7464ModifyAfterRolledBackModify.al Modify_AfterRolledBackModify_BufferUnchanged, _ModifyFalse, _NoAssignment, _OtherFieldReset,
    /// _TwoModifiesBeforeTheError, _CommitBetweenModifies; BC 27.5, 28.5, 29.0). The OnModify trigger of Modify(true) and the
    /// database triggers are not measured: refused. `None` when the buffer is not in this state.
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
        let mut trigger = run && self.table_trigger(t, "OnModify").is_some();
        for n in ["OnBeforeModify", "OnModify", "OnAfterModify"] {
            trigger |= run && !self.ext_triggers(t, n).is_empty();
        }
        if trigger || self.global_enabled(t, GlobalOp::Modify)? {
            return unsupported(format!(
                "a Modify of a {} buffer that its own rolled-back write left, with triggers: not pinned",
                t.name
            ));
        }
        let before = self.listened(t, "onbeforemodifyevent", None)?;
        let after = self.listened(t, "onaftermodifyevent", None)?;
        if before || after {
            assume(Assumption::ModifyOwnRolledBackWriteEvents);
            let image = rec.borrow().fields.clone();
            let xrec = Self::xrec_handle(rec, image.clone());
            self.table_event(t, "onbeforemodifyevent", rec, Some(&xrec), run, None)?;
            // What the answer is when a subscriber changes the buffer is not measured.
            let same = |r: &Rec| r.borrow().fields.iter().zip(&image).all(|(a, b)| a.identical(b));
            if !same(rec) || !same(&xrec) {
                return unsupported(format!(
                    "a Modify of a {} buffer that its own rolled-back write left, whose OnBeforeModifyEvent subscriber changes Rec or xRec: not pinned",
                    t.name
                ));
            }
            self.table_event(t, "onaftermodifyevent", rec, Some(&xrec), run, None)?;
        }
        Ok(Some(Value::Boolean(true)))
    }
}
