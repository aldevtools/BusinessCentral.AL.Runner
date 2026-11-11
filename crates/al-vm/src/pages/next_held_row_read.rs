//! A SetValue on a Worksheet row that `TestPage.Next()` moved onto without reading it (al-runner issue 7783).
//!
//! A `Next` onto a row the list already holds runs no read trigger on a page whose only read trigger is OnAfterGetRecord
//! (`server_lag`, events/extension-events/PageExtensionSender.al:643). A SetValue on such a row of a Worksheet, on a repeater
//! control bound to a page variable with an OnValidate, first reads the row: OnAfterGetRecord runs for it, so the per-line
//! variables it loads belong to that row when OnValidate runs. Not pinned by the corpus: Microsoft's Subscription Billing test
//! `Contract Renewal Test.RenewalTermEnteredValueIsRetainedOnPage` needs it (assumption `next-held-row-read-before-set-value`).
//!
//! Only that test's shape reads, and `held_row_read_pinned` is the one place that says what the shape is (an allowlist: each
//! condition names the line of the test or of its page it comes from; file names below are those of Microsoft's BCApps
//! repository, `Test/Contract Renewal/ContractRenewalTest.Codeunit.al` and `App/Contract Renewal/Pages/
//! ContractRenewalSelection.Page.al` of Subscription Billing). Every other shape keeps the earlier behavior, no read
//! (al-runner PR 7787 reviews).

use super::*;

impl Session {
    /// The page of the shape: a Worksheet (ContractRenewalSelection.Page.al:10) that the test's own OpenEdit opened
    /// (ContractRenewalTest.Codeunit.al:867). Not a List or a part, not OpenView or OpenNew, not a page AL code ran that the
    /// test trapped (`Trap` and a Page variable's `Run`), not a page `Page.RunModal` runs for a ModalPageHandler.
    fn held_row_page_pinned(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet) && st.borrow().test_open_edit()
    }

    /// A `Next` started `server_lag`, moved and wrote nothing (`page_move_noting_server_row`): on the page of the shape,
    /// remembers the row it moved onto and the store's write mark.
    pub(super) fn note_next_held_row(&mut self, st: &St) -> Result<(), Signal> {
        if self.held_row_page_pinned(st) && st.borrow().cursor == Cursor::Row {
            let key = self.row_key(&self.page_rec(st))?;
            st.borrow_mut().next_held_row = Some((key, self.write_mark()));
        }
        Ok(())
    }

    /// `TestField.SetValue` (the SetValue arm of `test_control_op`): reads the held row first in the shape
    /// `held_row_read_pinned` names.
    pub(super) fn set_value_reading_held_row(
        &mut self,
        st: &St,
        idx: usize,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        // Taken, so only the first SetValue after the `Next` can read.
        let noted = st.borrow_mut().next_held_row.take();
        if let Some((reached, writes)) = noted
            && self.held_row_read_pinned(st, idx, &reached, writes)?
        {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NextHeldRowReadBeforeSetValue);
            self.populate_current(st)?;
        }
        self.control_set_value(st, idx, args)
    }

    /// Whether a SetValue on control `idx` is the one of Microsoft's test: every condition must hold, and a shape that misses
    /// one keeps the earlier behavior (no read). `reached` and `writes` are what `note_next_held_row` took.
    ///
    /// Conditions that are true because the note exists (each of these clears or never sets `next_held_row`):
    /// - the call on the page before this SetValue was `Next()` (ContractRenewalTest.Codeunit.al:878, then :879): any other
    ///   TestPage method (`test_page_op_held`), any other TestField or TestAction call, a read among them
    ///   (`test_control_op_held`), a control write (`control_set_value`) and a procedure call through a `Page` variable
    ///   (`enter_server_row`) clear the note;
    /// - that `Next` moved onto another row, wrote nothing (it saved no change of the row it left; the test's first line
    ///   holds no typed Rec field, :871-:874) and ran no read trigger: it started `server_lag` on a list whose only read
    ///   trigger is OnAfterGetRecord (ContractRenewalSelection.Page.al:280; `page_move_noting_server_row`). A `Next` on the
    ///   last or only row, and a second `Next` in a row, note nothing.
    fn held_row_read_pinned(
        &mut self,
        st: &St,
        idx: usize,
        reached: &[Value],
        writes: (i64, u64),
    ) -> Result<bool, Signal> {
        // The page: a Worksheet the test opened with OpenEdit (`held_row_page_pinned`).
        let page = self.held_row_page_pinned(st);
        // The row: still the stored, unchanged row no trigger read since the `Next` (the test types nothing else on the line
        // first, ContractRenewalTest.Codeunit.al:878-:879): not a new line, no pending change, the lag still on.
        let row = {
            let s = st.borrow();
            s.server_lag.is_some() && s.cursor == Cursor::Row && !s.draft && !s.dirty
        };
        // The control: in the repeater, bound to a page variable, with an OnValidate (RenewalTermCtrl,
        // ContractRenewalSelection.Page.al:68 and :76).
        let engine = self.engine();
        let control = &page_of(&engine, st).controls[idx];
        let control = matches!(control.binding, ir::Binding::Var(_))
            && control.trigger("onvalidate").is_some()
            && self.in_repeater(st, idx);
        // Nothing between the `Next` and the SetValue (the test has no statement between :878 and :879): the store took no
        // insert, modify or delete since, through any record variable, so the row is stored as the `Next` found it.
        let untouched = self.write_mark() == writes;
        // The page still stands on the row the `Next` moved onto.
        Ok(page && row && control && untouched && Self::same_values(&self.row_key(&self.page_rec(st))?, reached))
    }
}
