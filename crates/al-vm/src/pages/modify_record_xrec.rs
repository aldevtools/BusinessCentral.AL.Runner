//! The xRec a `Rec.Validate` in the page's own OnModifyRecord passes to the table field's OnValidate.
//!
//! While a page saves a changed row, its OnModifyRecord sees Rec as typed and xRec as the row the page read. A `Rec.Validate(Field)`
//! the trigger calls on the page's own Rec hands the field's OnValidate that same xRec, not Rec as it was before the call: the
//! table's trigger reads `xrec=1+1` while Rec holds `2+4+1` (pages/currpage/Y7428OnModifyRecordAfterUpdate.al:203 Y7428_T0, :225
//! T1, :269 T3, :291 T4 and :313 Direct, the row saved by a move, CurrPage.Update, CurrPage.SaveRecord and a table-bound
//! control; :337 Action_AssignUpdateTrue, :381 AssignSave, :425 ValidateUpdateTrue and :447 AssignOnly, the change made by an
//! action; :471 TypedTwice_And_SameValue, the second save sees the first save's row; al-runner issue 7428).
//!
//! Only the page's own OnModifyRecord trigger is pinned. A Validate of the page's Rec in any other page code (an action, a control's
//! OnValidate, another row trigger, a page extension's trigger) or in code the trigger calls keeps the rule of every other record:
//! xRec is the record before the call (`records/nested_validate.rs` `validate_before_image`).

use super::*;

/// The page's Rec (by address) whose OnModifyRecord runs now, the page object and the page's xRec record.
pub(crate) struct ModifyRecordXRec {
    rec: usize,
    page: usize,
    xrec: Rec,
}

impl Session {
    /// Runs the OnModifyRecord trigger call `f` of page `st` with the page's xRec noted for `modify_record_validate_xrec`.
    pub(super) fn with_modify_record_xrec<T>(
        &mut self,
        st: &St,
        f: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let entry = self.page_xrec_record(st).map(|xrec| ModifyRecordXRec {
            rec: Rc::as_ptr(&self.page_rec(st)) as usize,
            page: st.borrow().object as usize,
            xrec,
        });
        let Some(entry) = entry else {
            return f(self);
        };
        self.pages.modify_record_xrec.push(entry);
        let r = f(self);
        self.pages.modify_record_xrec.pop();
        r
    }

    /// The before-image a `Validate` of record `handle` (its address) takes: the page's xRec when `handle` is the Rec of the page
    /// whose OnModifyRecord runs now and the call comes from that page's own code (see the module comment).
    pub(crate) fn modify_record_validate_xrec(&self, handle: usize) -> Option<Vec<Value>> {
        let e = self.pages.modify_record_xrec.last()?;
        (e.rec == handle && self.frames.last() == Some(&e.page)).then(|| e.xrec.borrow().fields.clone())
    }

    /// The page's xRec record, `None` on a page without a source table.
    fn page_xrec_record(&self, st: &St) -> Option<Rec> {
        if !self.has_source(st) {
            return None;
        }
        let slot = self.engine.program.objects[st.borrow().object as usize].xrec_slot?;
        let inst = st.borrow().instance.clone()?;
        let v = inst.borrow().globals[slot].clone();
        match v {
            Value::Record(x) => Some(x),
            _ => None,
        }
    }
}
