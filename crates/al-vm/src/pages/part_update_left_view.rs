//! A part whose own `CurrPage.Update` was asked while its view no longer started with the rows the client holds (al-runner
//! issue 8328: a host control's OnValidate calls a part procedure that copies filters onto the part's Rec and runs
//! `CurrPage.Update(false)`; on a host opened for edit no host step carries the refresh out, so the part kept answering from
//! the held rows). Which rows the part shows then is not pinned, so every test call that reads or writes a row of the part is
//! refused until the refresh runs.
//!
//! The answer is decided once, when the Update is asked (`note_part_update_left_view`, by `ask_update_refresh`), and kept in
//! `PageState::update_left_view` with the keys of the held rows; rows the test stores after that keep the answer from the held
//! rows, as before, and so does a step that read the part's rows again (a host action, the test's own `Filter.SetFilter`). It cannot
//! tell who changed the rows before the Update was asked: rows the test stored then refuse too.
//!
//! A pending refresh whose view still starts with the held rows keeps the answer from the held rows, as before: the corpus cases
//! that reach it pass with it (pages/parts/R6889RealShape.al:1346 PG6889RS_Step1_HostCallsPartSetEditable and the Step2* and
//! Step3 cases: the host's OnAfterGetCurrRecord calls `CurrPage.Lines.Page.SetEditable(true)`, whose `CurrPage.Update(false)`
//! still waits when the test calls `First()` on the part; testpage/ScLookupGlobalRecordField.al:644
//! TW_LookupOnValidateUpdatesThePart). A part that serves its rows through OnFindRecord or OnNextRecord is not compared: its
//! rows are not the view of its Rec.

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// Sets `update_left_view` of part `st` to the keys of the held rows as its own `CurrPage.Update` is asked: the client holds
    /// rows read from the start of the view, the part reads its rows from that view, and the view now starts with other rows (or
    /// with more rows, when the held rows were the whole view).
    pub(super) fn note_part_update_left_view(&mut self, st: &St) -> Result<(), Signal> {
        st.borrow_mut().update_left_view = None;
        let (held, short) = {
            let s = st.borrow();
            if s.host.is_none() || !s.open || !s.window_from_start || s.window.is_empty() {
                return Ok(());
            }
            let keys: Vec<Vec<Value>> = s.window.iter().map(|w| w.key.clone()).collect();
            (keys, s.window.len() < Self::WINDOW)
        };
        if !self.has_source(st) || self.declares(st, "onfindrecord") || self.declares(st, "onnextrecord") {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let used = self.ret_used;
        let view = self.view_rows(&rec, held.len() + usize::from(short));
        self.ret_used = used;
        let view = view?;
        let same = view.len() == held.len()
            && view
                .iter()
                .zip(&held)
                .all(|(row, key)| Self::same_key(&table.primary_key(row), key));
        if !same {
            st.borrow_mut().update_left_view = Some(held);
        }
        Ok(())
    }

    /// Refuses test call `m` on part `st` while the refresh its `update_left_view` Update asked for waits and the client still
    /// holds, from the start of the view, the rows it held then. A step that read the rows again or left them as not read from
    /// the start (a host action, the test's own `Filter.SetFilter`) keeps main's answer. Kept too: `Filter` and its `GetFilter`
    /// and `SetFilter`, `CurrentKey` and `Ascending`, `GetField`, `Caption`, `Visible`, `Editable`, `Enabled`.
    pub(super) fn refuse_row_access_on_part_left_by_its_view(&self, st: &St, m: PageMethod) -> Result<(), Signal> {
        let waits = {
            let s = st.borrow();
            s.refresh
                && s.refresh_by_update
                && s.open
                && s.window_from_start
                && s.update_left_view.as_ref().is_some_and(|held| {
                    held.len() == s.window.len() && held.iter().zip(&s.window).all(|(k, w)| Self::same_key(k, &w.key))
                })
        };
        let reads_no_row = matches!(
            m,
            PageMethod::Filter
                | PageMethod::GetFilter
                | PageMethod::SetFilter
                | PageMethod::CurrentKey
                | PageMethod::Ascending
                | PageMethod::GetField
                | PageMethod::Caption
                | PageMethod::Visible
                | PageMethod::Editable
                | PageMethod::Enabled
        );
        if !waits || reads_no_row {
            return Ok(());
        }
        unsupported(format!(
            "{m}() on a part whose view no longer started with the rows it holds when its own CurrPage.Update was asked (which rows it shows then is not pinned)"
        ))
    }
}
