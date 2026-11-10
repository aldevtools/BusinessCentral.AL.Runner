//! Where the refresh after a control's OnValidate changed the filters starts (`refresh_after_validate_filter`), including from
//! the new-row line (gap G5-20).

use super::*;

impl Session {
    /// `Some(true)` on a stored row, `Some(false)` on no row (`Cursor::None`), `None` for a start that is not pinned.
    ///
    /// From the untouched new-row line of an empty view, to a view with rows, the page leaves the line and refreshes as from a
    /// stored row: it stands on the row Rec is on, else the next row of the view after Rec's key, else the one before
    /// (pages/triggers/Y50UpdateFromNewLine.al:556 G520_FromNewLine_ToBatchWithRows `V:done 10000;R:B2/10000;
    /// R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000` as Y17JournalBatchSwitch.al:466 from a row; :574 _NoFind, Rec still the
    /// new-line buffer, lands on 20000; :592 _ThenLast; :610 _UpdateTrue; :646 the same on a List). To a view without rows it
    /// shows the new-row line of that view, as from a stored row (`show_new_line_of_empty_view`; measured, see
    /// `new_line_to_empty_view`).
    pub(super) fn validate_refresh_start(&mut self, st: &St, without_curr: bool) -> Result<Option<bool>, Signal> {
        let cursor = st.borrow().cursor;
        match cursor {
            Cursor::Row => {
                let rec = self.page_rec(st);
                Ok(self.stored_row(&rec)?.is_some().then_some(true))
            }
            Cursor::None => Ok(Some(false)),
            Cursor::NewLine => {
                if !st.borrow().placeholder_line {
                    return Ok(None);
                }
                if self.view_row_count(st)? == 0 {
                    self.new_line_to_empty_view(st, without_curr)?;
                    return Ok(Some(true));
                }
                let mut s = st.borrow_mut();
                s.cursor = Cursor::Row;
                s.draft = false;
                s.placeholder_line = false;
                Ok(Some(true))
            }
        }
    }

    /// From the untouched new-row line of an empty view to a view that is empty too (a Movement Worksheet switched to a
    /// worksheet without lines, al-runner issue 6924): BC shows the new-row line of the new view. OnNewRecord and
    /// OnAfterGetCurrRecord run twice after the OnValidate, Rec holding the values of the new filters; no row is read or
    /// stored, and page code that reads Rec next (an action) sees the new view. Measured on BC 27.5, 28.5 and 29.0 by corpus
    /// pull request 1282, not in the pinned corpus yet (tests/al-language/pages/journal-batch-switch/
    /// EmptyToEmptyBatchSwitch.al MovementWs_EmptyToEmpty_ThenAction `V:start;V:done;N:B/L2;C:B/L2;N:B/L2;C:B/L2;`, current
    /// `T1/B/L2/10000`, action `A:B/L2;...`, result `T1/B/L2`; _NoAction; _SecondTest; JournalWs_EmptyToEmpty_ThenAction with
    /// AutoSplitKey, current `T1/B/L2/0`). Microsoft's Tests-SCM 137293 RequisitionLineForItemWithSKU passes through it. A
    /// refresh that leaves OnAfterGetCurrRecord out is not measured and stays refused.
    fn new_line_to_empty_view(&mut self, st: &St, without_curr: bool) -> Result<(), Signal> {
        if without_curr || !st.borrow().new_line {
            return unsupported(
                "CurrPage.Update in a control's OnValidate that changed the filters of a list on its new-row line, to a view without rows (whether the new row is read again with the values of the new filters is not pinned)",
            );
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NewLineSwitchToEmptyView);
        Ok(())
    }

    /// The same OnValidate when the filters it set are the ones the page had (the name the page already shows): the new-row
    /// line is shown again all the same, OnNewRecord and OnAfterGetCurrRecord twice (EmptyToEmptyBatchSwitch.al
    /// MovementWs_SameName_ThenAction `V:start;V:done;N:A/L1;C:A/L1;N:A/L1;C:A/L1;`). `false` for every other page or state:
    /// the refresh runs as before.
    pub(super) fn same_filters_on_empty_new_line(
        &mut self,
        st: &St,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.placeholder_line && s.new_line && !without_curr
        };
        if !shape
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
            || self.view_row_count(st)? != 0
        {
            return Ok(false);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NewLineSwitchToEmptyView);
        self.show_new_line_of_empty_view(st, without_parts)?;
        Ok(true)
    }

    /// The refresh of a page that offers the new-row line, on a view without rows: it shows that line, OnNewRecord and
    /// OnAfterGetCurrRecord run twice, the key from the filters, nothing is inserted (corpus master 97c29f86,
    /// pages/journal-batch-switch/Y17JournalBatchSwitch.al:485 UpdWs_BatchSwitch_ToEmptyBatch `V:done 20000;N:B3/0 below=Yes;
    /// C:B3/0;N:B3/0 below=Yes;C:B3/0`, :560 without the Find in code).
    pub(super) fn show_new_line_of_empty_view(&mut self, st: &St, without_parts: bool) -> Result<(), Signal> {
        {
            let mut s = st.borrow_mut();
            s.window = Vec::new();
            s.focus = None;
            s.focus_in_row = false;
        }
        let r = (|| {
            self.land_on_new_line(st)?;
            self.land_on_new_line(st)
        })();
        self.torn_down_on_error(st, r)?;
        st.borrow_mut().refresh = false;
        if !without_parts {
            self.sync_parts(st)?;
        }
        Ok(())
    }

    /// `CurrPage.Update(true)` in a control's OnValidate, on the untouched new-row line, after page code moved Rec onto a stored
    /// row (the batch switch with Find('-')): the page leaves the line and modifies that row, OnModifyRecord and the table's
    /// OnModify running inside the OnValidate (Y50UpdateFromNewLine.al:610 G520_FromNewLine_ToBatchWithRows_UpdateTrue
    /// `V:start 0;M:B2/10000;L.OnModify:B2/10000;V:done 10000;...`). True when it did.
    pub(super) fn save_found_row_from_new_line(&mut self, st: &St) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine
                && s.placeholder_line
                && !s.dirty
                && s.host.is_none()
                && s.refresh_by_update
                && !s.refresh_update_false
        };
        if !shape || self.stored_row(&self.page_rec(st))?.is_none() {
            return Ok(false);
        }
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.placeholder_line = false;
            s.dirty = true;
        }
        // The row is written even though it holds what is stored (the buffer before it was the blank new line).
        self.save_row(st)?;
        self.set_xrec(st);
        Ok(true)
    }
}
