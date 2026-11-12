//! `First()` and `Last()` that read nothing: they stand on the row they would go to already, or the view holds no row. The
//! pinned shapes, each with its own predicate (`end_move_without_read`, asked by `page_move`).

use super::part_filter_find::FindPartAt;
use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// What `how` (a move) answers on `st` when it reads nothing; `None` when it reads.
    pub(super) fn end_move_without_read(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        if !matches!(how, PageMethod::First | PageMethod::Last) {
            return Ok(None);
        }
        if let Some(answer) = self.find_part_window_end_move(st, how)? {
            return Ok(Some(answer));
        }
        Ok((how == PageMethod::First && self.first_on_first_row_of_a_linked_list(st)?).then_some(true))
    }

    /// `First()` on a ListPart with OnFindRecord, OnNextRecord, OnAfterGetRecord and OnAfterGetCurrRecord (`find_list_part`) that stands on the first row of a window that starts at the view's start
    /// (`PageState::find_window_at_view_start`): it reads nothing and answers Yes (PartFilterExpressionControl.al:356
    /// Periods_FilterOneDay_First_Read `row log` `[]`, :414 Periods_FilterThreeDays_FirstNextNextNext `row 1 log`, :676
    /// EntriesFind_FilterOneDay_First_Read `row log` `[]`, :710 EntriesFind_ReadBeforeFilter_ThenFilter_ThenFirst `after First log`
    /// `[]`, :758 EntriesFind_FilterOneDay_HostMovesToNextItem `I3 after First log` `[]`). `Last()` on the last row of such a
    /// window that holds the view to its end does the same (:414 `last row log` `[]` after Next answered No). Only after the
    /// calls of those tests (`find_part_calls`): `Last()` after a `Next` that moved, or `First()` after `Next` and `Previous`,
    /// reads as before. The window
    /// is the one of the last landing (`PageState::find_window_at_view_start`, set by `part_window_read_starts`, `open_read_steps_back`
    /// and `filter_changed`).
    fn find_part_window_end_move(&self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        let (at_start, cursor, draft, dirty, to_end) = {
            let s = st.borrow();
            (
                s.find_window_at_view_start,
                s.cursor,
                s.draft,
                s.dirty,
                s.view_read_to_end,
            )
        };
        let at = match how {
            PageMethod::First => FindPartAt::First,
            _ => FindPartAt::Last,
        };
        if !at_start || draft || dirty || !Self::find_part_calls(st, at) || !self.find_list_part(st)? {
            return Ok(None);
        }
        // The filter left no row: First answers No and reads nothing (PartFilterExpressionControl.al:785
        // EntriesFind_FilterAdmitsNoRow_First `First` `No`, `First log` `[]`). Last there is not pinned.
        if cursor != Cursor::Row {
            let empty = st.borrow().window.is_empty();
            return Ok((how == PageMethod::First && empty).then_some(false));
        }
        if how == PageMethod::Last && !to_end {
            return Ok(None);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let s = st.borrow();
        let end = if how == PageMethod::First {
            s.window.first()
        } else {
            s.window.last()
        };
        Ok(end.is_some_and(|w| Self::same_key(&w.key, &key)).then_some(true))
    }

    /// Whether `First()` on a list an action opened through `RunPageLink` stands on the first row of its view already: it then
    /// answers Yes and runs no OnAfterGetCurrRecord (testpage/SjRunPageLinkOpenFind.al:337-350, :368-410
    /// `agcr row=4;agcr row=3;...`: the handler's First() on the row the page opened on adds no `agcr`; :412-425 from another row
    /// it does). Pinned for a list that declares OnAfterGetCurrRecord and no other row trigger, not a part, whose OnOpenPage
    /// moved Rec with FindFirst or FindLast, and for one whose OnOpenPage did not move Rec (testpage/R7138RunPageLinkCopyGet.al:210,
    /// :218 `agcr O3;...;handler;agcr O4`: OnAfterGetCurrRecord runs once at the open, First() on O3 runs it not again).
    fn first_on_first_row_of_a_linked_list(&mut self, st: &St) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.open
                && s.run_page_link_opened
                && s.host.is_none()
                && s.cursor == Cursor::Row
                && !s.draft
                && !s.dirty
                && !s.new_line
                && s.after_new.is_none()
        } && self.is_list(st)
            && !["onaftergetrecord", "onfindrecord", "onnextrecord"]
                .iter()
                .any(|t| self.declares(st, t));
        if !shape {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            return Ok(false);
        }
        if !Self::same_key(&self.row_key(&probe)?, &self.row_key(&rec)?) {
            return Ok(false);
        }
        Ok(true)
    }
}
