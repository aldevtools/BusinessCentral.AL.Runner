//! The xRec a `Rec.Validate` in the page's own OnModifyRecord passes to the table field's OnValidate.
//!
//! While a page saves a changed row, its OnModifyRecord sees Rec as typed and xRec as the row the page read. A `Rec.Validate(Field)`
//! the trigger calls on the page's own Rec hands the field's OnValidate that same xRec, not Rec as it was before the call: the
//! table's trigger reads `xrec=1+1` while Rec holds `2+4+1` (pages/currpage/Y7428OnModifyRecordAfterUpdate.al:203 Y7428_T0, :225
//! T1, :269 T3, :291 T4 and :313 Direct, the row saved by a move, CurrPage.Update, CurrPage.SaveRecord and a table-bound
//! control; :337 Action_AssignUpdateTrue, :381 AssignSave, :425 ValidateUpdateTrue and :447 AssignOnly, the change made by an
//! action; :471 TypedTwice_And_SameValue, the second save sees the first save's row; al-runner issue 7428).
//!
//! Pinned: the page's own OnModifyRecord trigger of a ListPlus calls `Rec.Validate(Field)` (no value) on the page's Rec. Where the
//! page's xRec and Rec before the call differ, every other Validate of the page's Rec while OnModifyRecord runs is refused: from
//! another page type, with a value (`Rec.Validate(Field, Value)`), from a page extension's OnModifyRecord, from a procedure of
//! the page the trigger calls, or from a codeunit that takes Rec by var (which xRec real BC passes there is not pinned;
//! docs/probes-needed.md). Where both are the same row, it does not matter which one is passed. Outside OnModifyRecord a Validate
//! keeps the rule of every other record: xRec is the record before the call (`records/nested_validate.rs`
//! `validate_before_image`).

use super::*;

/// The page's Rec (by address) whose OnModifyRecord runs now, the page object, its OnModifyRecord routine, whether the page is of the
/// pinned type (ListPlus), and the page's xRec record.
pub(crate) struct ModifyRecordXRec {
    rec: usize,
    page: usize,
    trigger: Option<usize>,
    list_plus: bool,
    xrec: Rec,
}

impl Session {
    /// Runs the OnModifyRecord trigger call `f` of page `st` with the page's xRec noted for `modify_record_validate_xrec`.
    pub(super) fn with_modify_record_xrec<T>(
        &mut self,
        st: &St,
        f: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let engine = self.engine();
        let page = st.borrow().object as usize;
        let entry = self.page_xrec_record(st).map(|xrec| ModifyRecordXRec {
            rec: Rc::as_ptr(&self.page_rec(st)) as usize,
            page,
            trigger: page_of(&engine, st)
                .triggers
                .iter()
                .find(|(n, _)| n == "onmodifyrecord")
                .map(|(_, r)| *r),
            list_plus: matches!(engine.object_facts[page].page_type, Some(al_ir::PageType::ListPlus)),
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

    /// The before-image a `Validate` of record `handle` (its address) takes while the page's OnModifyRecord runs: `Ok(Some)` the
    /// page's xRec for the pinned call, `Ok(None)` when `handle` is not that page's Rec or the page's xRec is the row `before`
    /// already holds; any other Validate of the page's Rec there is refused (see the module comment). `with_value`: the Validate
    /// hands a new value.
    pub(crate) fn modify_record_validate_xrec(
        &self,
        handle: usize,
        before: &[Value],
        with_value: bool,
    ) -> Result<Option<Vec<Value>>, Signal> {
        let Some(e) = self.pages.modify_record_xrec.last() else {
            return Ok(None);
        };
        if e.rec != handle {
            return Ok(None);
        }
        let xrec = e.xrec.borrow().fields.clone();
        if Self::same_values(&xrec, before) {
            return Ok(None);
        }
        let own_trigger = self.frames.last() == Some(&e.page)
            && e.trigger.is_some()
            && self.call_stack.last().map(|c| c.routine) == e.trigger;
        if own_trigger && e.list_plus && !with_value {
            return Ok(Some(xrec));
        }
        unsupported(
            "a Validate of the page's Rec while OnModifyRecord runs, other than Rec.Validate(Field) in the OnModifyRecord trigger of a \
             ListPlus (which xRec it passes is not pinned)",
        )
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
