//! An action of a list with OnAfterGetRecord and OnAfterGetCurrRecord set a filter that leaves the view without rows and called
//! CurrPage.Update(false) or Update(true) (al-runner issue 6945). The page shows a blank row and runs no OnAfterGetCurrRecord.
//! After the update OnAfterGetRecord runs for rows of the old view the new one no longer holds: on BC 27 and 29 once, for the row
//! before the row the page stood on (the row after it when there is none); on BC 28 for every other row, from the row before it up
//! to the first, then the rows after it (corpus pages/actions/ActionFilterEmptyView.al:226 Action_NoRowsLeft_UpdateFalse_Moves
//! `[|]:R1;a1;u1;R2`, 28.5 `R1;a1;u1;R2;R3;R4`; :322 Action_NoRowsLeft_FromLaterRow_UpdateFalse `R3;a3;u3;R2`, 28.5
//! `R3;a3;u3;R2;R1;R4`; _UpdateTrue alike). First, Next, Last, Previous, GoToRecord and GoToKey then answer false on the blank
//! row (:226, :251 _Moves, :276, :299 _GoTo).
//!
//! Pinned for a list whose view had no filter before the action, read from a row of fewer rows than a window; any other shape is
//! refused.
//!
//! Also the refusals of two shapes after an action that no corpus case pins: the first row of a list moved out of its view
//! (al-runner issue 8343, `refuse_first_row_left_view_after_action`, with `stands_on_first_of_view`, `has_row_before`,
//! `row_left_view`), and rows stored into an empty AutoSplitKey list with Rec left on one (al-runner issue 8344,
//! `refuse_rows_stored_with_rec_on_one`).

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether control `idx` reads blank: a control bound to the page's record on a page that shows no row (`shows_no_row`), and
    /// after an action left the view without rows any control in the repeater, a page variable too
    /// (pages/actions/ActionFilterEmptyView.al:226 Action_NoRowsLeft_UpdateFalse_Moves `[|]`).
    /// On the read-only temporary List of `temp_list_refill` with no row (an action refilled it with none, or a filter no row
    /// matches) every control of the repeater reads blank, an expression and a page variable too (testpage/
    /// TempListExpressionRefill.al:347, :426-431, :442-447, :488, :532-537).
    pub(super) fn value_reads_blank(&self, st: &St, idx: usize) -> bool {
        let no_row = st.borrow().cursor == Cursor::None;
        let blank_row = st.borrow().empty_after_action && no_row;
        ((blank_row || (no_row && self.temp_list_refill_pinned(st))) && self.in_repeater(st, idx))
            || self.shows_no_row(st, idx)
    }

    pub(super) fn empty_view_after_action(&mut self, st: &St, rec: &Rec) -> Result<(), Signal> {
        let refuse = |why: &str| {
            unsupported(format!(
                "an action left the list's view without rows: {why} (not pinned)"
            ))
        };
        let Some((filters, key, adhoc, ascending)) = st.borrow().view_before_action.clone() else {
            return refuse("the view before the action was not kept");
        };
        if !filters.is_empty() || adhoc.is_some() {
            return refuse("the view had filters before the action");
        }
        let old = || -> Rec {
            let mut r = rec.borrow().clone();
            r.filters = Vec::new();
            r.key = key;
            r.ascending = ascending;
            Rc::new(RefCell::new(r))
        };
        // Every other row on BC 28, one row on BC 27 and from BC 29 (ActionFilterEmptyView.al:226, :322).
        let every_row = self.has(al_version::Gate::EmptyActionViewEveryRowFrom28)
            && !self.has(al_version::Gate::EmptyActionViewOneRowAgainFrom29);
        let mut reads = Vec::new();
        for step in [-1, 1] {
            let probe = old();
            loop {
                self.ret_used = true;
                if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)])?, Value::Integer(n) if n != 0)
                {
                    break;
                }
                reads.push(probe.borrow().fields.clone());
                if !every_row || reads.len() >= Self::WINDOW {
                    break;
                }
            }
            if !reads.is_empty() && !every_row {
                break;
            }
        }
        if reads.len() >= Self::WINDOW {
            return refuse("the view before the action held a full window of rows or more");
        }
        for fields in reads {
            rec.borrow_mut().fields = fields;
            self.populate_current(st)?;
        }
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::None;
            s.window = Vec::new();
            s.focus = None;
            s.focus_in_row = false;
            s.empty_after_action = true;
        }
        let table = rec.borrow().table as usize;
        let fresh = self.initial_fields(table);
        rec.borrow_mut().fields = fresh;
        Ok(())
    }

    /// The row a list settles on when the action's filter left the stored row it stood on outside the view: the next row of the
    /// view, else the one before it (pages/list-order/AiListOrderTests.al:335 DropCurrent_RowsAfter_Plain and :301 _FindOnly
    /// read R1003 first, :501 DropCurrent_NoRowsAfter_Plain and :467 _FindOnly read R1005 first). `None` when the row is in the
    /// view; a view left with no row is refused.
    pub(super) fn row_after_view_left(&mut self, rec: &Rec) -> Result<Option<Vec<Value>>, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(None);
        }
        for step in [1, -1] {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)])?, Value::Integer(n) if n != 0)
            {
                return Ok(Some(probe.borrow().fields.clone()));
            }
        }
        unsupported(
            "an action left the list's view without rows: the row it stood on left the view and no row is left (not pinned)",
        )
    }

    /// Whether the list stands on the first row of a window read from the start of its view, and the action put no row before
    /// it; an action that changed the view can put rows before it, and then the list reads as on a later row (al-runner issue
    /// 6470: testpage/Y15ActionWidensView.al:199 `after action` `A20/6 28:A20-A01`).
    pub(super) fn stands_on_first_of_view(&mut self, st: &St, rec: &Rec, key: &[Value]) -> Result<bool, Signal> {
        let first = {
            let s = st.borrow();
            s.window_from_start
                && s.window.len() < Self::WINDOW
                && s.window.first().is_some_and(|w| Self::same_key(&w.key, key))
        };
        Ok(first && !(self.view_changed_by_action(st) && self.has_row_before(rec)?))
    }

    /// The refresh after an action (before its reads) of a top-level list that reads its rows from the table and declares
    /// exactly one of OnAfterGetRecord and OnAfterGetCurrRecord, standing on the stored first row of a window read from the
    /// start, which the action moved out of the view (the page's filters no longer hold it): BC reads the view again and
    /// First() finds no row, where the runtime kept the client on the row (al-runner issue 8343, the reporter's repro on BC
    /// 29.1 with OnAfterGetRecord; the issue says OnAfterGetCurrRecord alone fails the same way). Not pinned: refused, with
    /// CurrPage.Update(false), Update(true) or none. With OnAfterGetCurrRecord alone only when the action left Rec's view as it
    /// was (same filters, key and order: `view_changed_by_action` false); where the action changed the view the runtime already
    /// finds no row, and the page keeps that path. A list without row triggers, with both, or on a later row keeps its path.
    pub(super) fn refuse_first_row_left_view_after_action(&mut self, st: &St) -> Result<(), Signal> {
        let on_stored_row = {
            let s = st.borrow();
            s.open && s.host.is_none() && s.cursor == Cursor::Row && !s.draft
        };
        let refused_triggers = match (
            self.declares(st, "onaftergetrecord"),
            self.declares(st, "onaftergetcurrrecord"),
        ) {
            (true, false) => true,
            (false, true) => !self.view_changed_by_action(st),
            _ => false,
        };
        if !(on_stored_row && refused_triggers && self.is_list(st) && self.has_source(st)) {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_none() {
            return Ok(());
        }
        let key = self.row_key(&rec)?;
        if self.stands_on_first_of_view(st, &rec, &key)? && self.row_left_view(&rec)? {
            return unsupported(
                "the reads after an action of a list whose filters no longer hold the row it stands on, from the start of its view (not pinned)",
            );
        }
        Ok(())
    }

    /// An action stored more than one row into an empty top-level AutoSplitKey list (a Worksheet or a List) over a table that is
    /// not temporary, standing on its new-row line (`edit_list`: the `EmptyShape::EditList` shape), and also put Rec on a stored
    /// row (`Rec.FindSet()`): BC's next New() takes the key between that row and the next one (the reporter's repro of al-runner
    /// issue 8344 on BC 29.1, a Worksheet, 15000 between 10000 and 20000), where the runtime went on as after any action and New()
    /// took a key that is stored, on a List too. What the list holds then is not pinned: refused. A temporary table and a part
    /// keep their paths.
    pub(super) fn refuse_rows_stored_with_rec_on_one(
        &mut self,
        st: &St,
        rec: &Rec,
        edit_list: bool,
    ) -> Result<(), Signal> {
        let shape = edit_list && st.borrow().host.is_none() && self.has_auto_split_key(st);
        if shape && !rec.borrow().temporary && self.stored_row(rec)?.is_some() {
            return unsupported(
                "an action that stored rows into an empty list standing on its new-row line and left Rec on a stored row (not pinned)",
            );
        }
        Ok(())
    }

    /// Whether the view of `rec` holds a row before the row it stands on (asked of the table; Rec is put back).
    pub(super) fn has_row_before(&mut self, rec: &Rec) -> Result<bool, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        Ok(matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(-1)])?, Value::Integer(n) if n != 0))
    }

    /// Whether the view of `rec` (its filters, read from the table) no longer holds the row `rec` stands on, as a second
    /// variable with the same filters sees it: without the record's own position, which lets `Find('=')` find a row its
    /// variable moved out of the filter (record/TestFindSetSortKeyModify.al:OwnFilter_ModifiedOutOfIt_FindStillFindsTheRow).
    pub(super) fn row_left_view(&mut self, rec: &Rec) -> Result<bool, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        probe.borrow_mut().position = None;
        self.ret_used = true;
        Ok(!matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ))
    }
}
