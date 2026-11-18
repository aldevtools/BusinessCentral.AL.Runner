//! A move on a part whose own `CurrPage.Update` still waits for its refresh, after the part's code changed the view so that it no
//! longer starts with the rows the client holds (al-runner issue 8328: a host control's OnValidate calls a part procedure that
//! copies filters onto the part's Rec and runs `CurrPage.Update(false)`; on a host opened for edit no host step carries the
//! refresh out). Which rows the part shows then is not pinned, so the move is refused.
//!
//! A pending refresh whose view still starts with the held rows keeps the answer from the held rows, as before: the corpus cases
//! that reach it pass with it (pages/parts/R6889RealShape.al:1346 PG6889RS_Step1_HostCallsPartSetEditable and the Step2* and
//! Step3 cases: the host's OnAfterGetCurrRecord calls `CurrPage.Lines.Page.SetEditable(true)`, whose `CurrPage.Update(false)`
//! still waits when the test calls `First()` on the part; testpage/ScLookupGlobalRecordField.al:644
//! TW_LookupOnValidateUpdatesThePart).

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// Refuses `how` on part `st` when its own `CurrPage.Update` waits for its refresh, the client holds rows read from the start
    /// of the view, and the view now starts with other rows (or with more rows, when the held rows were the whole view).
    pub(super) fn refuse_move_on_part_left_by_its_view(&mut self, st: &St, how: PageMethod) -> Result<(), Signal> {
        let (held, short) = {
            let s = st.borrow();
            if s.host.is_none()
                || !s.open
                || !s.refresh
                || !s.refresh_by_update
                || !s.window_from_start
                || s.window.is_empty()
            {
                return Ok(());
            }
            let keys: Vec<Vec<Value>> = s.window.iter().map(|w| w.key.clone()).collect();
            (keys, s.window.len() < Self::WINDOW)
        };
        if !self.has_source(st) {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let view = self.view_rows(&rec, held.len() + usize::from(short))?;
        let same = view.len() == held.len()
            && view
                .iter()
                .zip(&held)
                .all(|(row, key)| Self::same_key(&table.primary_key(row), key));
        if same {
            return Ok(());
        }
        unsupported(format!(
            "{how}() on a part whose own CurrPage.Update waits for its refresh, after its code changed the rows its view starts with (which rows it shows then is not pinned)"
        ))
    }
}
