//! Reads past the client's window of 20 rows that run to the end of a long view: `Last` past a full window, the `New` of a
//! list longer than a window (`draft.rs`), and the open of a tree whose view is longer than the window
//! (corpus pages/long-list-open/LastOnLongList.al, al-runner issues 7475 and 7421).

use super::pinned_shape::{PinnedCode, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// `Last` on a list whose window of 20 rows is full: the client reads the last row it holds again, then the rows after it in
    /// one block, and settles on the last of them (a page with OnFindRecord asks `=` for it). Pinned for 25 rows by
    /// testpage/PnF06FindListMoves.al:FindList_Ascending_25Rows `asc25_last`
    /// (`R20;N(1);R21;...;N(1);R25;N(1);F(=);R25;C`) and FindList_PlainDescending `plain25_last` (`R6;R5;R4;R3;R2;R1;R1;C`). A
    /// block of 20 new rows, a descending view of a page with OnFindRecord, and a list with the blank line are not pinned.
    pub(super) fn last_past_the_window(&mut self, st: &St, find_page: bool) -> Result<Option<bool>, Signal> {
        let rec = self.page_rec(st);
        let whole = self.last_reads_long_view_pinned(st, find_page)?;
        // On the pinned page an OpenEdit reads the same (LastOnLongList.al:306 OpenEdit_100Rows_Last `81: 20..100`): that page
        // has no OnNewRecord, so the blank line is not seen on the way. Every other list with the blank line keeps the earlier
        // move.
        if (st.borrow().new_line && whole.is_none()) || st.borrow().window_eof || (find_page && !rec.borrow().ascending)
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
    /// of exactly one window, are refused. With `pinned_rows` (the rows a pinned long view holds past the window, see
    /// `last_reads_long_view_pinned` and `tree_open_reads_on_pinned`) the reads go on to the end of the view in one go; a view
    /// that then gives more rows than that is not the pinned one and is refused (a safety net, the predicates counted the rows).
    pub(super) fn read_block_past_the_window(&mut self, st: &St, pinned_rows: Option<usize>) -> Result<(), Signal> {
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
            match pinned_rows {
                None if new_rows == Self::WINDOW => {
                    return unsupported(
                        "a list with a second full block of rows past the window (the reads are not pinned)",
                    );
                }
                Some(n) if new_rows > n => {
                    return unsupported("a long view that grew while the client read it to its end (not pinned)");
                }
                _ => {}
            }
        }
        st.borrow_mut().window_eof = true;
        st.borrow_mut().window_whole = true;
        if new_rows == 0 {
            return unsupported("a list of exactly one window (the reads past it are not pinned)");
        }
        Ok(())
    }

    /// `Last` past a full window reads the last row it holds again, then every row after it in one go (corpus
    /// pages/long-list-open/LastOnLongList.al `reads of Last`). The rule runs only on the pinned scenario; the rows the view
    /// holds past the window when it is that one, else `None` and the earlier behavior (the refusal of a second block, or the
    /// move of a list with the blank line). Every fact below is what the pin has, and all of them are required:
    /// - the page (page 63432, :92): PageType List; no property but its source table; a stored table without code, in its
    ///   primary key, ascending, no filter, no marks; its only code the page trigger OnAfterGetRecord; a repeater of fields of
    ///   Rec; no part, no action, no page extension, no subscriber (`page_code_is`); not a tree; on its own, not a part;
    /// - how it opened and how many rows the view holds: `TestPage.OpenView` with 41 to 100 rows (:253 41 rows `22: 20..41`,
    ///   :269 61 rows `42: 20..61`, :285 100 rows `81: 20..100`); `TestPage.OpenEdit` with 100 rows (:306); a Page variable
    ///   after `Editable(false)` and `LookupMode(true)`, run modally, with 41 to 100 rows (:341, :362, :383); the static
    ///   `Page.RunModal(Page::X, Rec)` with 100 rows (:411);
    /// - where it stands: on the first row of the view, the window the first 20 rows of the view as they are stored, nothing
    ///   typed, no new row;
    /// - what the test did since the open: read control values and `Editable()` (:218, :424), then this `Last`.
    fn last_reads_long_view_pinned(&mut self, st: &St, find_page: bool) -> Result<Option<usize>, Signal> {
        const CODE: PinnedCode = PinnedCode {
            triggers: &["onaftergetrecord"],
            control_triggers: &[],
            properties: &[],
            action_props: None,
            part_props: None,
        };
        let rec = self.page_rec(st);
        let calls = match Self::test_calls(st).as_deref() {
            Some([before @ .., TestCall::Page(PageMethod::Last)]) => before
                .iter()
                .all(|c| matches!(c, TestCall::Read | TestCall::Page(PageMethod::Editable))),
            _ => false,
        };
        // The row counts each pinned way to open the page is pinned for.
        let rows = {
            let s = st.borrow();
            let blank_line = s.new_line;
            match (
                s.opened_by_test,
                s.mode,
                s.modal,
                s.lookup,
                s.run_static,
                s.editable_preset,
            ) {
                (true, PageMode::View, false, false, None, None) if !blank_line => Some(41..=100),
                (true, PageMode::Edit, false, false, None, None) if blank_line => Some(100..=100),
                (false, PageMode::Edit, true, true, Some(false), Some(false)) if !blank_line => Some(41..=100),
                (false, PageMode::Edit, true, true, Some(true), None) if !blank_line => Some(100..=100),
                _ => None,
            }
        };
        let Some(rows) = rows else {
            return Ok(None);
        };
        let shape = calls
            && !find_page
            && self.is_list_page(st)
            && self.tree_indentation(st).is_none()
            && st.borrow().host.is_none()
            && !rec.borrow().temporary
            && self.rec_view_is(st, 0)
            && self.page_code_is(st, &CODE)?;
        if !shape || !self.stands_on_first_row_of_unread_view(st)? {
            return Ok(None);
        }
        let n = self.view_row_count(st)?;
        Ok(rows.contains(&n).then(|| n as usize - Self::WINDOW))
    }

    /// Whether the page stands as a pinned long view does right after it opened: on the first row of the view, a stored row with
    /// nothing typed, with the first window read from the start of the view (`window_is_start_of_unread_view`).
    fn stands_on_first_row_of_unread_view(&mut self, st: &St) -> Result<bool, Signal> {
        let stands = {
            let s = st.borrow();
            s.cursor == Cursor::Row && !s.draft && !s.dirty && s.window_from_start
        };
        Ok(stands && self.window_is_start_of_unread_view(st)?)
    }

    /// Whether the window is the first 20 rows of the view, each as it is stored now, with rows left to read, and Rec is on the
    /// first of them.
    fn window_is_start_of_unread_view(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let on_first = {
            let s = st.borrow();
            !s.window_eof && s.window.len() == Self::WINDOW && Self::same_key(&s.window[0].key, &self.row_key(&rec)?)
        };
        if !on_first {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        let mut more = matches!(
            self.record_op(RecordOp::FindSet, &probe, &mut [])?,
            Value::Boolean(true)
        );
        for i in 0..Self::WINDOW {
            let same = more && {
                let s = st.borrow();
                Self::same_key(&s.window[i].key, &self.row_key(&probe)?)
                    && Self::same_values(&s.window[i].stored, &probe.borrow().fields)
            };
            if !same {
                return Ok(false);
            }
            self.ret_used = true;
            more = matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0);
        }
        Ok(more)
    }

    /// The open of a collapsed tree over a temporary source whose view is longer than the window: after the window and the row
    /// it opens on, the client reads the last row of the window again and then every row after it, and opens on the row it
    /// stood on (LastOnLongList.al:482 Tree_TempSource_60Rows_OpenView_Walk `reads of the open` `62: 1..20,1,20..60`,
    /// `opened on` 1; the walk then goes over the 12 rows without a parent and reads nothing). Only on that scenario
    /// (`tree_open_reads_on_pinned`); every other tree keeps the open window only, as before.
    pub(super) fn tree_open_reads_on(&mut self, st: &St) -> Result<(), Signal> {
        let Some(rows) = self.tree_open_reads_on_pinned(st)? else {
            return Ok(());
        };
        let rec = self.page_rec(st);
        let opened_on = rec.borrow().fields.clone();
        self.read_block_past_the_window(st, Some(rows))?;
        rec.borrow_mut().fields = opened_on;
        Ok(())
    }

    /// The entry condition of the tree rule: the rows the view holds past the window on the pinned scenario, else `None`.
    /// Every fact is what LastOnLongList.al:482 Tree_TempSource_60Rows_OpenView_Walk has, and all of them are required:
    /// - the page (page 63433, :120): PageType List, Editable = false, a temporary source without code, in its primary key,
    ///   ascending, no filter, no marks; ShowAsTree over an Integer field of Rec; its only code the page triggers OnOpenPage
    ///   and OnAfterGetRecord; a repeater of fields of Rec; no part, no action, no page extension, no subscriber
    ///   (`page_code_is`); on its own;
    /// - how it opened: `TestPage.OpenView`, no call of the test since;
    /// - the rows: 60, every fifth one from the first at level 0 and the four after each at level 1 (:153), so the tree shows 12;
    /// - where it stands: OnOpenPage left Rec on the first row of the view (:159), and the window is the first 20 rows.
    fn tree_open_reads_on_pinned(&mut self, st: &St) -> Result<Option<usize>, Signal> {
        const CODE: PinnedCode = PinnedCode {
            triggers: &["onopenpage", "onaftergetrecord"],
            control_triggers: &[],
            properties: &["sourcetabletemporary", "editable"],
            action_props: None,
            part_props: None,
        };
        const ROWS: usize = 60;
        let rec = self.page_rec(st);
        let object = st.borrow().object as usize;
        let opened = {
            let s = st.borrow();
            s.opened_by_test
                && s.mode == PageMode::View
                && !s.modal
                && !s.lookup
                && s.host.is_none()
                && s.test_calls.is_empty()
        };
        let shape = opened
            && self.is_list_page(st)
            && rec.borrow().temporary
            && self.rec_view_is(st, 0)
            && super::flag_denied(&self.engine(), object, "Editable")?
            && self.page_code_is(st, &CODE)?;
        if !shape {
            return Ok(None);
        }
        let Some(level) = self.tree_level_field(st)? else {
            return Ok(None);
        };
        let rows = self.tree_rows(&rec, level)?;
        let levels = rows.len() == ROWS && rows.iter().enumerate().all(|(i, (_, l))| *l == i32::from(i % 5 != 0));
        Ok((levels && self.window_is_start_of_unread_view(st)?).then_some(ROWS - Self::WINDOW))
    }
}
