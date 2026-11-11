//! A move off the new-row line of a list that nothing was typed on.
//!
//! On a List or a ListPlus page of its own (not a part, not a tree) with OnAfterGetRecord and OnAfterGetCurrRecord and without
//! OnFindRecord and OnNextRecord, a move off the untouched new-row line `New()` started reads the row it lands on twice and then
//! runs OnAfterGetCurrRecord; it reads no other row (testpage/NewRowThenMove.al, al-runner issue 7452):
//! - First: the first row (:540 List_TwoStoredRows_OpenEdit_New_First and :663 ListPlus_TwoStoredRows_OpenEdit_New_First
//!   `log after FIRST` = `R:S1 R:S1 C:S1`; :444 List_TypedRow_New_First and :581 ListPlus_TypedRow_New_First on a list opened with
//!   OpenNew whose typed row was saved), also when CurrPage.Update(true) in a control's OnValidate saved what was typed on the
//!   line (:639 ListPlus_TypedRow_New_SecondRowTyped_First). Over an empty view the line is started again (OnNewRecord,
//!   OnAfterGetCurrRecord) and First answers No (:562 List_EmptyTable_OpenEdit_New_First `N C: `).
//! - Previous: the last row (:480 List_TypedRow_New_Previous, :617 ListPlus_TypedRow_New_Previous).
//! - Last: the line is reached twice before the client settles on the last row (:462 List_TypedRow_New_Last, :599
//!   ListPlus_TypedRow_New_Last `N C: N C: R:FORMULA1 R:FORMULA1 C:FORMULA1`).
//! - GoToKey onto a stored row (:516 List_TypedRow_New_GoToKey).
//!
//! The pinned views hold no row, one or two rows (a longer view is not pinned); every other shape keeps the moves of `rows.rs`. Also here: Previous off the
//! new-row line of a list opened on a moved Rec (see `previous_off_new_line_onto_held_row`).

use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether a move from where `st` stands is one of the pinned moves off the new-row line `New()` started: the line untouched
    /// (below a row the page saved on a list opened with OpenNew, see `open_new_list_untouched`), or, with `saved_line`, the line
    /// after the page's code saved what was typed on it; on a list of the pinned shape whose view has at most two rows.
    pub(super) fn off_new_line_pinned(&mut self, st: &St, saved_line: bool) -> Result<bool, Signal> {
        let line = {
            let s = st.borrow();
            let untouched = s.cursor == Cursor::NewLine
                && s.draft
                && !s.placeholder_line
                && (s.after_new.is_some() || s.mode != PageMode::New);
            let saved = saved_line && s.cursor == Cursor::Row && !s.draft && s.after_new.is_some();
            (untouched || saved) && !s.dirty && s.host.is_none()
        };
        let shape = line
            && self.is_list(st)
            && self.declares(st, "onaftergetrecord")
            && self.declares(st, "onaftergetcurrrecord")
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
            && self.tree_indentation(st).is_none()
            && matches!(
                self.engine.object_facts[st.borrow().object as usize].page_type,
                Some(al_ir::PageType::List | al_ir::PageType::ListPlus)
            );
        Ok(shape && self.view_row_count(st)? <= 2)
    }

    /// First, Previous or Last off the untouched new-row line (see the module comment); `None` for another shape or move.
    pub(super) fn move_off_untouched_new_line(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        if !self.off_new_line_pinned(st, how == PageMethod::First)? {
            return Ok(None);
        }
        let which = match how {
            PageMethod::First => "-",
            PageMethod::Previous | PageMethod::Last => "+",
            _ => return Ok(None),
        };
        // Only First is pinned over an empty view; Previous and Last there keep the moves of `rows.rs`.
        if how != PageMethod::First && self.view_row_count(st)? == 0 {
            return Ok(None);
        }
        st.borrow_mut().after_new = None;
        if how == PageMethod::Last {
            self.land_on_new_line(st)?;
            self.land_on_new_line(st)?;
        }
        if self.nav_find(st, which)? {
            self.read_row_off_new_line(st)?;
            return Ok(Some(true));
        }
        self.land_on_new_line(st)?;
        Ok(Some(false))
    }

    /// GoToKey onto a stored row of the view off the untouched new-row line, once the caller found the move pinned
    /// (`off_new_line_pinned` before the GoToKey's save); `None` for a key no row of the view has.
    pub(super) fn go_to_key_off_untouched_new_line(&mut self, st: &St, want: &[Value]) -> Result<Option<bool>, Signal> {
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let table = self.table(&rec)?;
            let mut p = probe.borrow_mut();
            for (&f, v) in table.keys[0].iter().zip(want) {
                p.fields[f] = v.clone();
            }
        }
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(None);
        }
        st.borrow_mut().after_new = None;
        let found = probe.borrow().fields.clone();
        rec.borrow_mut().fields = found;
        self.read_row_off_new_line(st)?;
        Ok(Some(true))
    }

    /// The row Rec landed on is read twice, then OnAfterGetCurrRecord runs. The rows the client holds stay as they are.
    fn read_row_off_new_line(&mut self, st: &St) -> Result<(), Signal> {
        let before = self.xrec_before_move(st);
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.stale = false;
        }
        self.populate_current(st)?;
        self.populate_current(st)?;
        self.current_record_trigger(st, before)
    }

    /// Previous off the new-row line of a list opened on the row its OnOpenPage left Rec on goes back to the last row it holds as a
    /// move between its rows does: the row read twice, then OnAfterGetCurrRecord (corpus
    /// pages/onopenpage-moves-rec/QrMovedTests.al:585 QR_TempList_PageRun_NoMove `previous log` = `R:D;R:D;C:D;`, :615
    /// QR_List_RunModal_FindLast). `Some(true)` when it did; other lists keep the earlier move. Rec stands on the last row of the
    /// view (`found` by the caller's find).
    pub(super) fn previous_off_new_line_onto_held_row(&mut self, st: &St, found: bool) -> Result<Option<bool>, Signal> {
        if !(found
            && (st.borrow().opened_on_moved_rec || st.borrow().refreshed_after_gone_move)
            && st.borrow().host.is_none())
        {
            return Ok(None);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let last = st.borrow().window.len().checked_sub(1);
        let Some(at) = last.filter(|&at| Self::same_key(&st.borrow().window[at].key, &key)) else {
            return Ok(None);
        };
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.stale = false;
            s.refreshed_after_gone_move = false;
        }
        self.read_client_row(st, at)?;
        Ok(Some(true))
    }
}
