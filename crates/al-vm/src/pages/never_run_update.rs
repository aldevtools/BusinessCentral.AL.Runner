//! `CurrPage.Update` and `SaveRecord` of a Page variable the client does not show (never ran, or its modal page closed): they do
//! nothing (see the arm in `pageops.rs` that calls [`Session::never_run_update`] for the pins), except one case that is refused.

use super::*;
use al_builtins::page_op::PageOp;

impl Session {
    /// Never ran, `Update(true)` or `Update()` after the page's code set fields of a new record of its source table: BC inserts
    /// the record, through OnInsertRecord and the table's OnInsert, numbered by AutoSplitKey (corpus branch
    /// al-runtime-rs-staging-r6951 format-evaluate/tablefilter-neutral/TableFilterEnumForms.al
    /// UpdateNeverRunPage_NewRecord_UpdateTrue, _UpdateNoArgument; issue 6951; the Base Application "Table Filter" page fills
    /// its lines this way). Not implemented: refused, so such a page does not silently hold no rows. With the blank record it
    /// does nothing (pages/currpage/UnopenedCurrPage.al Direct_Part_UpdateTrue), and so do `Update(false)` and `SaveRecord`.
    pub(super) fn never_run_update(&mut self, method: PageOp, st: &St, args: &[Value]) -> Result<Value, Signal> {
        let saves = method != PageOp::PageSaveRecord && !matches!(args.first(), Some(Value::Boolean(false)));
        if saves && !st.borrow().ran && st.borrow().instance.is_some() && self.has_source(st) {
            let rec = self.page_rec(st);
            let table = rec.borrow().table as usize;
            let fresh = self.initial_fields(table);
            // A stored record handed over with SetRecord is no new record (UnopenedCurrPage.al:299
            // Update_Then_RunModal_Card_HandlerReads `update-true` = `start end`).
            if !Self::same_values(&rec.borrow().fields, &fresh) && !self.row_exists(&rec)? {
                return unsupported(
                    "CurrPage.Update(true) or Update() of a page variable that never ran, over a new record its code filled (BC inserts it; not implemented)",
                );
            }
        }
        Ok(Value::Null)
    }
}
