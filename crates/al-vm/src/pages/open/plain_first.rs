//! First on the first row of a plain list that has no OnAfterGetRecord.

use super::*;

impl Session {
    /// First on the first row a list shows, on a list whose only row trigger is OnAfterGetCurrRecord (no OnAfterGetRecord,
    /// OnFindRecord or OnNextRecord), reads nothing: the page opened on that row already
    /// (pages/triggers/Y42RepeaterNewLine.al:313 NrEd_ListRepeaterFalse_NextPastLast `First` adds no `agcr`,
    /// and :381, :483, :511 with a repeater whose Editable is false, a false or a true page variable). With OnAfterGetRecord
    /// the client's rows answer it, `move_in_client_rows`.
    pub(in crate::pages) fn plain_list_first_on_first_row(&mut self, st: &St) -> Result<bool, Signal> {
        if !self.is_list(st)
            || self.declares(st, "onaftergetrecord")
            || !self.declares(st, "onaftergetcurrrecord")
            || ["onfindrecord", "onnextrecord"].iter().any(|t| self.declares(st, t))
        {
            return Ok(false);
        }
        let shape = {
            let s = st.borrow();
            s.cursor == Cursor::Row && !s.draft && !s.stale && !s.dirty && s.host.is_none() && s.window_from_start
        };
        if !shape {
            return Ok(false);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let first = st.borrow().window.first().map(|w| w.key.clone());
        Ok(first.is_some_and(|f| Self::same_key(&f, &key)))
    }
}
