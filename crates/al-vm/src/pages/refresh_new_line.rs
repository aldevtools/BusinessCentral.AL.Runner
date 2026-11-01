//! Where the refresh after a control's OnValidate changed the filters starts (`refresh_after_validate_filter`), including from
//! the new-row line (gap G5-20).

use super::*;

impl Session {
    /// `Some(true)` on a stored row, `Some(false)` on no row (`Cursor::None`), `None` for a start that is not pinned.
    ///
    /// From the untouched new-row line of an empty view, to a view with rows, the page leaves the line and refreshes as from a
    /// stored row: it stands on the row Rec is on, else the next row of the view after Rec's key, else the one before
    /// (pages/lt-y50-update-from-new-line/Y50UpdateFromNewLine.al:556 G520_FromNewLine_ToBatchWithRows `V:done 10000;R:B2/10000;
    /// R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000` as Y17JournalBatchSwitch.al:466 from a row; :574 _NoFind, Rec still the
    /// new-line buffer, lands on 20000; :592 _ThenLast; :610 _UpdateTrue; :646 the same on a List). To a view without rows it is
    /// not pinned.
    pub(super) fn validate_refresh_start(&mut self, st: &St) -> Result<Option<bool>, Signal> {
        let cursor = st.borrow().cursor;
        match cursor {
            Cursor::Row => {
                let rec = self.page_rec(st);
                Ok(self.stored_row(&rec)?.is_some().then_some(true))
            }
            Cursor::None => Ok(Some(false)),
            Cursor::NewLine => {
                if !st.borrow().placeholder_line || self.view_row_count(st)? == 0 {
                    return Ok(None);
                }
                let mut s = st.borrow_mut();
                s.cursor = Cursor::Row;
                s.draft = false;
                s.placeholder_line = false;
                Ok(Some(true))
            }
        }
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
