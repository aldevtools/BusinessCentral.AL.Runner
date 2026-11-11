//! Reads past the client's window of 20 rows that run to the end of a long view: `Last` past a full window, the `New` of a
//! list longer than a window (`draft.rs`), and the open of a tree whose view is longer than the window
//! (corpus pages/long-list-open/LastOnLongList.al, al-runner issues 7475 and 7421).

use super::*;

impl Session {
    /// `Last` on a list whose window of 20 rows is full: the client reads the last row it holds again, then the rows after it in
    /// one block, and settles on the last of them (a page with OnFindRecord asks `=` for it). Pinned for 25 rows by
    /// testpage/PnF06FindListMoves.al:FindList_Ascending_25Rows `asc25_last`
    /// (`R20;N(1);R21;...;N(1);R25;N(1);F(=);R25;C`) and FindList_PlainDescending `plain25_last` (`R6;R5;R4;R3;R2;R1;R1;C`). A
    /// block of 20 new rows, a descending view of a page with OnFindRecord, and a list with the blank line are not pinned.
    pub(super) fn last_past_the_window(&mut self, st: &St, find_page: bool) -> Result<Option<bool>, Signal> {
        let rec = self.page_rec(st);
        let whole = self.last_reads_long_view_pinned(st, find_page);
        // An OpenEdit of the long list reads the same (LastOnLongList.al:306 OpenEdit_100Rows_Last `81: 20..100`); its page
        // has no OnNewRecord, so whether the blank line is passed on the way is not seen there and is not pinned with one.
        let new_line_unseen = whole && !self.declares(st, "onnewrecord");
        if (st.borrow().new_line && !new_line_unseen)
            || st.borrow().window_eof
            || (find_page && !rec.borrow().ascending)
        {
            return Ok(None);
        }
        let before = self.xrec_before_move(st);
        self.read_block_past_the_window(st, whole)?;
        // Without OnAfterGetCurrRecord the move ends with the block: `=` is not asked and the row is not read again
        // (pages/list-order/AiListOrderTests.al:958 Paging_Last_Nav `last_log` ends `R:R1025;N:1=0`).
        if !self.declares(st, "onaftergetcurrrecord") {
            return Ok(Some(true));
        }
        if find_page {
            self.nav_find(st, "=")?;
        }
        self.populate_current(st)?;
        // In a key other than the primary key the row is read once more (SgListKeyMovesTests.al SG_Key_ThirtyRows_Moves `last`:
        // the block, then R01 three times; corpus bump PR 738).
        if self.still_reads_once_more(st) {
            self.populate_current(st)?;
        }
        self.current_record_trigger(st, before)?;
        Ok(Some(true))
    }

    /// The client reads the last row it holds again, then the rows after it in one block (up to 20, the failing last call of
    /// OnNextRecord included) and ends on the last of them with the window holding the whole view. Used by `Last` and by `New` on a
    /// list longer than a window (testpage/ListRowsActionNew.al:NewRowThenAction_Ping30Rows_ReadsLogged: 12 reads
    /// after `New()` on 30 rows, the last row of the window, the ten rows after it, and the blank line). A second full block, and a view
    /// of exactly one window, are refused, unless `long_view` (`last_reads_long_view_pinned`): then the reads go on to the end
    /// of the view in one go.
    pub(super) fn read_block_past_the_window(&mut self, st: &St, long_view: bool) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let last = st.borrow().window[Self::WINDOW - 1].stored.clone();
        rec.borrow_mut().fields = last;
        self.populate_current(st)?;
        let mut new_rows = 0;
        loop {
            if !self.nav_next(st, 1)? {
                break;
            }
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            let shown = rec.borrow().fields.clone();
            st.borrow_mut()
                .window
                .push(crate::pages::WindowRow { key, stored, shown });
            new_rows += 1;
            if new_rows == Self::WINDOW && !long_view {
                return unsupported(
                    "a list with a second full block of rows past the window (the reads are not pinned)",
                );
            }
        }
        st.borrow_mut().window_eof = true;
        st.borrow_mut().window_whole = true;
        if new_rows == 0 {
            return unsupported("a list of exactly one window (the reads past it are not pinned)");
        }
        Ok(())
    }

    /// `Last` past a full window reads the last row it holds again, then every row after it in one go, however many blocks of 20
    /// that is (corpus pages/long-list-open/LastOnLongList.al: 41 rows `reads of Last` `22: 20..41` (:253), 61 rows
    /// `42: 20..61` (:269), 100 rows `81: 20..100` (:285), opened with OpenView, OpenEdit (:306), in a lookup-mode modal run of a
    /// Page variable (:341, :362, :383) and by Page.RunModal (:411)). Pinned only for that page's shape: a List over a stored table in its
    /// primary key, ascending, with OnAfterGetRecord as its only row trigger, opened on its own; every other shape keeps the
    /// refusal of a second block.
    fn last_reads_long_view_pinned(&self, st: &St, find_page: bool) -> bool {
        let rec = self.page_rec(st);
        let r = rec.borrow();
        let s = st.borrow();
        // LastOnLongList.al:92 page 63432: PageType = List, SourceTable over a stored table, only OnAfterGetRecord.
        self.is_list_page(st)
            && self.tree_indentation(st).is_none()
            && s.host.is_none()
            && !find_page
            && !r.temporary
            && r.key == 0
            && r.ascending
            && self.declares(st, "onaftergetrecord")
            && ["onaftergetcurrrecord", "onfindrecord", "onnextrecord"]
                .iter()
                .all(|t| !self.declares(st, t))
    }

    /// The open of a collapsed tree over a temporary source whose view is longer than the window: after the window and the row
    /// it opens on, the client reads the last row of the window again and then every row after it, and opens on the row it
    /// stood on (LastOnLongList.al:482 Tree_TempSource_60Rows_OpenView_Walk `reads of the open` `62: 1..20,1,20..60`,
    /// `opened on` 1; the walk then goes over the 12 rows without a parent and reads nothing). Pinned for that page's shape
    /// (`tree_open_reads_on_pinned`) and while the rows the tree shows fit in one window; a tree that shows more is refused,
    /// as is every other shape with a full window (main's behavior keeps the open window only).
    pub(super) fn tree_open_reads_on(&mut self, st: &St) -> Result<(), Signal> {
        if st.borrow().window.len() < Self::WINDOW || st.borrow().window_eof {
            return Ok(());
        }
        if !self.tree_open_reads_on_pinned(st)? {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let opened_on = rec.borrow().fields.clone();
        self.read_block_past_the_window(st, true)?;
        rec.borrow_mut().fields = opened_on;
        let Some(level) = self.tree_level_field(st)? else {
            return unsupported("a tree whose IndentationColumn is not an Integer field (not pinned)");
        };
        // The 60 rows of the pin show 12; whether a tree that shows a window or more reads on to the end is not seen there.
        let top = st
            .borrow()
            .window
            .iter()
            .filter(|w| matches!(w.stored.get(level), Some(Value::Integer(0))))
            .count();
        if top >= Self::WINDOW {
            return unsupported(
                "a tree whose view shows a full window of rows or more when it opens (which rows it reads is not pinned)",
            );
        }
        Ok(())
    }

    /// LastOnLongList.al:120 page 63433: a List page, ShowAsTree over an Integer field of a temporary source, OnAfterGetRecord
    /// as its only row trigger, opened on its own.
    fn tree_open_reads_on_pinned(&self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        Ok(self.is_list_page(st)
            && st.borrow().host.is_none()
            && rec.borrow().temporary
            && rec.borrow().key == 0
            && rec.borrow().ascending
            && self.tree_level_field(st)?.is_some()
            && self.declares(st, "onaftergetrecord")
            && ["onaftergetcurrrecord", "onfindrecord", "onnextrecord"]
                .iter()
                .all(|t| !self.declares(st, t)))
    }
}
