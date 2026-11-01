//! GoToKey and GoToRecord on lists whose client walks to the key row by row: a list with OnAfterGetCurrRecord only, and the lists
//! that serve their rows from OnFindRecord and OnNextRecord with a full first window of rows.

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// GoToKey on a list that declares OnAfterGetCurrRecord and no OnAfterGetRecord: from a row other than the first the client goes to the
    /// first row, then steps on row by row until the key is there, each arrival one current-record read; standing on the first row it
    /// does not read that row again (pages/lt-testpage-list-rows/ListRowsWindow.al:GoToKeyLastThenRow2_OnlyCurr41Rows `C2;...;C41`, then
    /// back to row 2: `C1;C2`). A key the list does not hold walks every row the same way and then moves back to the row it started
    /// from, one more read unless that is the last row; the answer is false, and with the result unused the row error
    /// (pages/lt-y61-page-probes/Y61GoToKeyEdges.al:334 G613_Current_Miss_FromFirstRow `C:002;C:003;C:001`, :408
    /// G613_Current_Miss_FromLaterRow `C:001;C:002;C:003` from the last row and `C:001;C:002;C:003;C:002` from row 2, :444 one row:
    /// nothing read, :365 a temporary source). A key equal to the row the cursor stands on (not the first) is not pinned.
    pub(super) fn go_to_key_current_only(
        &mut self,
        st: &St,
        want: &[Value],
        result_used: bool,
    ) -> Result<Option<bool>, Signal> {
        if !self.declares(st, "onaftergetcurrrecord")
            || self.declares(st, "onaftergetrecord")
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
        let cur = self.row_key(&rec)?;
        let on_first = st.borrow().window.first().is_some_and(|w| Self::same_key(&w.key, &cur));
        if Self::same_key(&cur, want) && !on_first {
            return Ok(None);
        }
        let origin_fields = rec.borrow().fields.clone();
        let mut found = false;
        let mut at = true;
        if !on_first {
            at = self.nav_find(st, "-")?;
            if at {
                st.borrow_mut().cursor = Cursor::Row;
                let r = self.arrive(st);
                self.torn_down_on_error(st, r)?;
                found = Self::same_key(&self.row_key(&rec)?, want);
            }
        }
        while at && !found {
            at = self.nav_next(st, 1)?;
            if !at {
                break;
            }
            let r = self.arrive(st);
            self.torn_down_on_error(st, r)?;
            found = Self::same_key(&self.row_key(&rec)?, want);
        }
        if !found {
            if !Self::same_key(&self.row_key(&rec)?, &cur) {
                rec.borrow_mut().fields = origin_fields;
                let r = self.arrive(st);
                self.torn_down_on_error(st, r)?;
            }
            if !result_used {
                return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
            }
            return Ok(Some(false));
        }
        Ok(Some(true))
    }

    /// GoToKey for a key past a full window of a list with OnFindRecord, OnNextRecord and OnAfterGetRecord (no OnAfterGetCurrRecord)
    /// that holds its first 20 rows: the client reads the last two rows it holds again, without asking the page, then asks
    /// OnNextRecord(1) and reads each row after them to the end of the view; the cursor lands on the key without another read
    /// (pages/lt-ai-list-order/AiListOrderTests.al:999 Paging_GoToKey_Nav `R:R1019;R:R1020;N:1=1;R:R1021;...;R:R1025;N:1=0`, 25
    /// rows). The walk on from there asks nothing (:1020 Paging_GoToKeyThenWalk_Nav). A second full block and a key the view does
    /// not have are refused (not pinned). `None` when the page or its window is not of that kind.
    pub(super) fn go_to_key_past_a_full_find_window(
        &mut self,
        st: &St,
        want: &[Value],
    ) -> Result<Option<bool>, Signal> {
        let shape = {
            let s = st.borrow();
            s.host.is_none()
                && s.window_from_start
                && !s.window_eof
                && !s.window_whole
                && s.window.len() == Self::WINDOW
        };
        if !shape
            || !self.declares(st, "onnextrecord")
            || self.declares(st, "onaftergetcurrrecord")
            || self.in_window(st, want)
        {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        for i in Self::WINDOW - 2..Self::WINDOW {
            let stored = st.borrow().window[i].stored.clone();
            rec.borrow_mut().fields = stored;
            self.populate_current(st)?;
            let shown = rec.borrow().fields.clone();
            st.borrow_mut().window[i].shown = shown;
        }
        let mut new_rows = 0;
        while self.nav_next(st, 1)? {
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            let shown = rec.borrow().fields.clone();
            st.borrow_mut()
                .window
                .push(al_builtins::value::WindowRow { key, stored, shown });
            new_rows += 1;
            if new_rows == Self::WINDOW {
                return unsupported(
                    "GoToKey past a second full block of rows of a find list (the reads are not pinned)",
                );
            }
        }
        {
            let mut s = st.borrow_mut();
            s.window_eof = true;
            s.window_whole = true;
            s.view_read_to_end = true;
        }
        let at = st.borrow().window.iter().position(|w| Self::same_key(&w.key, want));
        let Some(at) = at else {
            return unsupported(
                "GoToKey for a key a find list does not hold, past a full window (the reads are not pinned)",
            );
        };
        let shown = st.borrow().window[at].shown.clone();
        rec.borrow_mut().fields = shown;
        self.set_xrec(st);
        Ok(Some(true))
    }
    /// GoToKey / GoToRecord on a list with OnFindRecord, OnNextRecord, OnAfterGetRecord and OnAfterGetCurrRecord that holds a full
    /// first window of rows (the shape of Item List): from the first row the client steps to the next rows one move at a time
    /// (`R:x;F:==Yes;R:x;C:x`); arriving on the last row it holds it first reads that row and the rest of the view with
    /// OnNextRecord(1) (`R:R020;N:1=1;R:R021;...;R:R030;N:1=0`), then moves on. From another row the walk starts with a move to the
    /// first row; a key the view does not have walks every row and goes back to the row it started from and answers false. Pinned for 30
    /// rows by pages/lt-y41-page-probes/Y41GoToKeyFindListCurr.al:157 GK_FindList_Curr_PastWindow_GoToRecord, :178 _GoToKey, :199
    /// _LastRow, :213 _Missing, :227 _FromOtherRow (the second walk), :253 _ThenNext, and by pages/lt-y61-page-probes/
    /// Y61GoToKeyEdges.al: :212 a key in the second block of 45 rows (arriving on row 20 reads one full block of 20 more rows,
    /// `R:020;N:1=1;R:021;...;R:040`), :241 the first row while standing on it reads nothing and from row 10 is one move, :270 the
    /// current row (not the first) walks from the first row, :299 a miss from row 10 walks back to row 10 (`...;R:010;F:==Yes;R:010;
    /// C:010`), and with the result unused raises the row error after the same walk over the view now held whole. A third block of
    /// rows and a view of exactly 20 rows are refused. `None` when the page or its window is not of that kind.
    pub(super) fn go_to_key_find_list_curr(
        &mut self,
        st: &St,
        want: &[Value],
        result_used: bool,
    ) -> Result<Option<bool>, Signal> {
        let shape = {
            let s = st.borrow();
            s.host.is_none()
                && s.cursor == Cursor::Row
                && !s.draft
                && !s.stale
                && s.window_from_start
                && (s.window.len() > Self::WINDOW || (s.window.len() == Self::WINDOW && !s.window_eof))
        };
        let triggers = [
            "onfindrecord",
            "onnextrecord",
            "onaftergetrecord",
            "onaftergetcurrrecord",
        ];
        if !shape || !self.is_list(st) || !triggers.iter().all(|t| self.declares(st, t)) {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let cur = self.row_key(&rec)?;
        let Some(origin) = st.borrow().window.iter().position(|w| Self::same_key(&w.key, &cur)) else {
            return Ok(None);
        };
        let first_is_want = Self::same_key(&st.borrow().window[0].key, want);
        if first_is_want && origin == 0 {
            return Ok(Some(true));
        }
        let mut found = false;
        if origin != 0 {
            let r = self.arrive_held(st, 0);
            self.torn_down_on_error(st, r)?;
            found = first_is_want;
        }
        let mut idx = 1;
        while !found && idx < st.borrow().window.len() {
            {
                let s = st.borrow();
                if idx + 1 == s.window.len() && s.window.len() >= 2 * Self::WINDOW && !s.window_eof {
                    return unsupported(
                        "GoToKey past a second full block of rows of a find list with OnAfterGetCurrRecord (not pinned)",
                    );
                }
            }
            let r = self.arrive_held(st, idx);
            self.torn_down_on_error(st, r)?;
            found = Self::same_key(&st.borrow().window[idx].key, want);
            idx += 1;
        }
        if !found {
            let r = self.arrive_held(st, origin);
            self.torn_down_on_error(st, r)?;
            if !result_used {
                return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
            }
        }
        Ok(Some(found))
    }

    /// Whether a move on a list that holds more than its first window of rows, not its whole view, is not pinned (`move_in_client_rows`
    /// leaves it to the generic move). Pinned: Next between rows held below the second block of a find list with OnFindRecord,
    /// OnNextRecord, OnAfterGetRecord and OnAfterGetCurrRecord, after a GoToKey walked into it (pages/lt-y61-page-probes/
    /// Y61GoToKeyEdges.al:212 G613_Find_45Rows_KeyInSecondBlock `next log` = `R:034;F:==Yes;R:034;C:034`). Next onto the last row held
    /// (which reads another block) and the other moves are not.
    pub(super) fn window_past_first_block_unpinned(&mut self, st: &St, how: PageMethod) -> Result<bool, Signal> {
        let past = {
            let s = st.borrow();
            s.window.len() > Self::WINDOW && !s.window_whole && !s.window_read_up
        };
        let triggers = [
            "onfindrecord",
            "onnextrecord",
            "onaftergetrecord",
            "onaftergetcurrrecord",
        ];
        if !past
            || how != PageMethod::Next
            || st.borrow().host.is_some()
            || !triggers.iter().all(|t| self.declares(st, t))
        {
            return Ok(past);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let s = st.borrow();
        let at = s.window.iter().position(|w| Self::same_key(&w.key, &key));
        Ok(!at.is_some_and(|at| at + 2 < s.window.len()))
    }
}
