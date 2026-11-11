//! A SetValue on a Worksheet row that `TestPage.Next()` moved onto without reading it (al-runner issue 7783).
//!
//! A `Next` onto a row the list already holds runs no read trigger on a page whose only read trigger is OnAfterGetRecord
//! (`server_row_lag`, events/extension-events/PageExtensionSender.al:643). A SetValue on such a row of a Worksheet, on a repeater
//! control bound to a page variable with an OnValidate, first reads the row: OnAfterGetRecord runs for it, so the per-line
//! variables it loads belong to that row when OnValidate runs. Not pinned by the corpus: Microsoft's Subscription Billing test
//! `Contract Renewal Test.RenewalTermEnteredValueIsRetainedOnPage` needs it (assumption `next-held-row-read-before-set-value`).
//! Every other shape keeps the earlier behavior: no read.

use super::*;

impl Session {
    /// Reads the row before a SetValue on control `idx` in the shape the module names.
    pub(super) fn read_held_row_before_set_value(&mut self, st: &St, idx: usize) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let shape = {
            let s = st.borrow();
            s.server_lag.is_some() && s.cursor == Cursor::Row && !s.draft && s.host.is_none()
        } && engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet)
            && matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Var(_))
            && page_of(&engine, st).controls[idx].trigger("onvalidate").is_some()
            && self.in_repeater(st, idx);
        if !shape {
            return Ok(());
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NextHeldRowReadBeforeSetValue);
        self.populate_current(st)
    }
}
