//! The reads of a move onto a row the client holds (`read_client_row`), split out of rows.rs.

use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// A move onto a row the client holds reads that row from the table again, so a change made elsewhere since the window
    /// was read shows (pages/W1Update.al:836-840 List_BufferLeftByOnAfterGetRecord_AfterTheStoredRowChanged `name_row2 =
    /// changed`; misc/lt-probes-needed-7/PnGRefresh.al:268-269 BufferEditOfOnAfterGetRecordOnAList). Pinned for a page that
    /// reads its rows from the table; a page with OnFindRecord or OnNextRecord keeps the row it holds (what the client asks
    /// such a page for is not pinned). A held row the table no longer has is refused (not pinned).
    pub(super) fn reread_held_row(&mut self, st: &St, target: usize) -> Result<(), Signal> {
        if self.declares(st, "onfindrecord") || self.declares(st, "onnextrecord") {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let mut key = st.borrow().window[target].key.clone();
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Get, &rec, &mut key)?, Value::Boolean(true)) {
            let stored = st.borrow().window[target].stored.clone();
            rec.borrow_mut().fields = stored;
            return unsupported("a move onto a row the client holds that the table no longer has (not pinned)");
        }
        let fresh = rec.borrow().fields.clone();
        if let Some(w) = st.borrow_mut().window.get_mut(target) {
            w.stored = fresh;
        }
        Ok(())
    }

    /// The reads of a move onto row `target` of the rows the client holds (see `move_in_client_rows`).
    pub(super) fn read_client_row(&mut self, st: &St, target: usize) -> Result<(), Signal> {
        // First, Last, GoToKey and GoToRecord in a key other than the primary key read the row once more too
        // (pages/lt-sg-list-key-moves/SgListKeyMovesTests.al SG_Key_FourRows_Moves `last`, `first`, `gotokey_b`, `gotorecord_c`).
        let once_more = self.still_reads_once_more(st);
        self.read_client_row_in(st, target, once_more)
    }

    pub(super) fn read_client_row_in(&mut self, st: &St, target: usize, once_more: bool) -> Result<(), Signal> {
        let before = self.xrec_before_move(st);
        let rec = self.page_rec(st);
        let stored = st.borrow().window[target].stored.clone();
        rec.borrow_mut().fields = stored;
        // A list with OnAfterGetRecord alone reads nothing when the cursor moves onto a row the client holds (pages/lt-testpage-list-rows/
        // ListRowsOpenAG.al:OpenView_OnlyRec5Rows_ReadsAtOpen `after-next-total = 0`).
        // The same holds with OnFindRecord and OnNextRecord declared: pages/lt-f3-pages/F3ListTests.al:24
        // Nav_TwelveRows_OpenLastPreviousFirstNext (`after_last`, `after_previous3`, `after_first`, `after_next2` all empty).
        // A list (or list part) with OnFindRecord, a filtered SourceTableView and no row trigger asks the page nothing either
        // (corpus branch al-runtime-rs-staging-qc eca98051, pagefindrecord/QcFindViewTests.al:97 ViewList_FirstNextLastPrevious,
        // :236 NextList_NextLog, :540 DocPart_OpenEdit_LinesOfH2: the logs are empty).
        if !self.declares(st, "onaftergetcurrrecord")
            && (self.declares(st, "onaftergetrecord") || self.filtered_find_list_without_row_triggers(st))
        {
            let shown = st.borrow().window[target].shown.clone();
            rec.borrow_mut().fields = shown;
            self.set_xrec(st);
            return Ok(());
        }
        self.reread_held_row(st, target)?;
        self.populate_current_pair_first(st)?;
        if self.declares(st, "onfindrecord") {
            self.nav_find(st, "=")?;
        }
        self.populate_current(st)?;
        if once_more {
            self.populate_current(st)?;
        }
        let shown = rec.borrow().fields.clone();
        if let Some(w) = st.borrow_mut().window.get_mut(target) {
            w.shown = shown;
        }
        self.current_record_trigger(st, before)
    }

    pub(super) fn go_to_key_in_client_rows(&mut self, st: &St, want: &[Value]) -> Result<Option<bool>, Signal> {
        if !self.declares(st, "onaftergetcurrrecord")
            || !self.declares(st, "onaftergetrecord")
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
            || !self.is_list(st)
            || st.borrow().host.is_some()
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || st.borrow().stale
            || !st.borrow().window_from_start
            || st.borrow().window.is_empty()
        {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let Some(origin) = st.borrow().window.iter().position(|w| Self::same_key(&w.key, &key)) else {
            return Ok(None);
        };
        let at_key = |s: &Self, i: usize| -> Option<bool> {
            super::rows::s_window_key(s, st, i).map(|k| Self::same_key(&k, want))
        };
        // A key equal to the first row the list stands on: the reads are not pinned, the generic move stays.
        if origin == 0 && at_key(self, 0) == Some(true) {
            return Ok(None);
        }
        let mut found = false;
        if origin != 0 {
            let r = self.arrive_held(st, 0);
            self.torn_down_on_error(st, r)?;
            found = at_key(self, 0) == Some(true);
        }
        let mut idx = 1;
        let mut gone_seen = false;
        while !found && idx < st.borrow().window.len() {
            // The key asked for was deleted by code after the list read it, and the cursor stands on the first row: the first step
            // of the scan is a move onto a gone row (pages/lt-y80-gone-rows/Y80GoneRows.al:1922 Y80_Moved_All_GoToKeyDeleted, :2205
            // Y80_Moved_NoNew_GoToKeyDeleted); the scan goes on over the rows the page holds afterwards.
            if idx == 1 && origin == 0 && !gone_seen && at_key(self, 1) == Some(true) {
                gone_seen = self.moved_onto_deleted(st, PageMethod::Next, 0, 1, false)?.is_some();
                if gone_seen {
                    continue;
                }
            }
            let r = self.arrive_held(st, idx);
            self.torn_down_on_error(st, r)?;
            found = at_key(self, idx) == Some(true);
            idx += 1;
        }
        // A miss whose walk ended on the row the list stood on stays there without another move (pages/
        // lt-qr-onopenpage-moves-rec/SuRefillKey.al Id_Filter_Refill `goto 1 outside` `No/4:R2;R2;C2;R3;R3;C3;R4;R4;C4;`).
        let last = st.borrow().window.len() - 1;
        if !found && gone_seen && st.borrow().new_line {
            // The scan over the rows ends on the new-row line (Y80_Moved_All_GoToKeyDeleted `...R:40;R:40;C:40;N:0;C:0;R:10;R:10;C:10`).
            self.land_on_new_line(st)?;
            let mut s = st.borrow_mut();
            s.draft = false;
            s.cursor = Cursor::Row;
        }
        if !found && origin != last {
            let r = self.arrive_held(st, origin);
            self.torn_down_on_error(st, r)?;
        }
        Ok(Some(found))
    }
}
