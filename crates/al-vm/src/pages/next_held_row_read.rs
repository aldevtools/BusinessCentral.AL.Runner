//! A SetValue on a Worksheet row that `TestPage.Next()` moved onto without reading it (al-runner issue 7783).
//!
//! A `Next` onto a row the list already holds runs no read trigger on a page whose only read trigger is OnAfterGetRecord
//! (`server_lag`, events/extension-events/PageExtensionSender.al:643). A SetValue on such a row of a Worksheet, on a repeater
//! control bound to a page variable with an OnValidate, first reads the row: OnAfterGetRecord runs for it, so the per-line
//! variables it loads belong to that row when OnValidate runs. Not pinned by the corpus: Microsoft's Subscription Billing test
//! `Contract Renewal Test.RenewalTermEnteredValueIsRetainedOnPage` needs it (assumption `next-held-row-read-before-set-value`).
//! Only that test's shape reads: the first SetValue right after the `Next` that started the lag, on the clean row it reached.
//! Every other shape keeps the earlier behavior, no read: another TestPage method or move after the `Next` (Previous, First,
//! GoToKey), a second `Next`, a control write before the SetValue, or a changed row (al-runner PR 7787 review).

use super::*;

impl Session {
    /// After a `Next` that started `server_lag` (`page_move_noting_server_row`): remembers the row it reached on a Worksheet.
    pub(super) fn note_next_held_row(&mut self, st: &St) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        let shape = {
            let s = st.borrow();
            s.server_lag.is_some() && s.cursor == Cursor::Row && s.host.is_none()
        } && self.engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet);
        if shape {
            let key = self.row_key(&self.page_rec(st))?;
            st.borrow_mut().next_held_row = Some(key);
        }
        Ok(())
    }

    /// `TestField.SetValue` (the SetValue arm of `test_control_op`): reads the held row first in the shape the module names.
    pub(super) fn set_value_reading_held_row(
        &mut self,
        st: &St,
        idx: usize,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        self.read_held_row_before_set_value(st, idx)?;
        self.control_set_value(st, idx, args)
    }

    /// Reads the row before a SetValue on control `idx` in the shape the module names. Takes the row `note_next_held_row` noted,
    /// so only the first SetValue after the `Next` can read.
    fn read_held_row_before_set_value(&mut self, st: &St, idx: usize) -> Result<(), Signal> {
        let Some(reached) = st.borrow_mut().next_held_row.take() else {
            return Ok(());
        };
        let engine = self.engine();
        let shape = {
            let s = st.borrow();
            s.server_lag.is_some() && s.cursor == Cursor::Row && !s.draft && !s.dirty && s.host.is_none()
        } && matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Var(_))
            && page_of(&engine, st).controls[idx].trigger("onvalidate").is_some()
            && self.in_repeater(st, idx)
            && Self::same_values(&self.row_key(&self.page_rec(st))?, &reached);
        if !shape {
            return Ok(());
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NextHeldRowReadBeforeSetValue);
        self.populate_current(st)
    }
}
