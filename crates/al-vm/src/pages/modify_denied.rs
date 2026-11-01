//! A row that page code changes on a page whose ModifyAllowed is false, other than through the client's own save
//! (pages/lt-y61-page-probes/Y61ModifyDeniedSave.al).

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// `TestField.DrillDown`, `AssistEdit` and `Lookup` ([`Self::control_navigate`]); after a DrillDown, on a list whose ModifyAllowed is false, what the drill-down's code did to the current row:
    /// - it stored the row itself (`Rec.Modify` in code): the client holds the stored row, so a later First shows it, Next moves
    ///   on and Close raises nothing (Y61ModifyDeniedSave.al:164 G617a_Temp_NotEditable_DrillDownModify_ThenFirst `first shown` =
    ///   `A/9`, `next shown` = `B/1`, a non-editable list over a temporary table);
    /// - it changed the row without storing it: BC leaves the change on the page and records `Page cannot be saved.` as the last
    ///   error at a later step without raising it (:203 G617a_Temp_NotEditable_DrillDownAssign_ThenFirst); that is not built and
    ///   refused.
    pub(super) fn navigate_settled(&mut self, st: &St, idx: usize, m: PageMethod) -> Result<Value, Signal> {
        let r = self.control_navigate(st, idx, m)?;
        if m != PageMethod::DrillDown {
            return Ok(r);
        }
        let shape = {
            let s = st.borrow();
            s.open && s.cursor == Cursor::Row && !s.draft && s.host.is_none()
        };
        if !shape
            || !self.is_list(st)
            || !self.has_source(st)
            || !flag_denied(&self.engine(), st.borrow().object as usize, "ModifyAllowed")?
        {
            return Ok(r);
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let Some(at) = st.borrow().window.iter().position(|w| Self::same_key(&w.key, &key)) else {
            return Ok(r);
        };
        let fields = rec.borrow().fields.clone();
        if Self::same_values(&st.borrow().window[at].stored, &fields) {
            return Ok(r);
        }
        match self.stored_row(&rec)? {
            Some(stored) if Self::same_values(&stored, &fields) => {
                let mut s = st.borrow_mut();
                s.window[at].stored = fields.clone();
                s.window[at].shown = fields.clone();
                drop(s);
                self.set_xrec_fields(st, fields);
                Ok(r)
            }
            _ => unsupported(
                "a drill-down that changed the row without storing it on a list whose ModifyAllowed is false (the save BC tries later is not built)",
            ),
        }
    }
}
