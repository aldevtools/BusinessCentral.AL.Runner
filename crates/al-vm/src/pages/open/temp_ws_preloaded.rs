//! A temporary Worksheet without OnOpenPage whose rows a procedure of the page variable inserted before `Run()` or
//! `RunModal()`, in the key of its SourceTableView (al-runner issue 7420; corpus pages/run-modal/PageVarOpenRow.al).

use super::*;
use crate::pages::pinned_shape::{PinnedCode, TestCall};
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

/// A control validation on a page `open_temp_ws_preloaded` opened (`Session::temp_ws_edit_begin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempWsEdit {
    /// The pinned edit: `CurrPage.Update(false)` in it drops the edit and reads the view again.
    Pinned,
    /// The pinned edit, but on another row than the first of the view: `CurrPage.Update(false)` in it is refused.
    OffFirstRow,
}

impl Session {
    /// The entry condition of the open below: every fact is what PageVarOpenRow.al:942 (and :970, :998) has, and all of them
    /// are required. Every other page keeps the open it had.
    /// - the page (page 63427, :402): PageType Worksheet, InsertAllowed = false, a temporary source without code,
    ///   SourceTableView = sorting(the table's second key, one field), ascending, no filter, no marks; its only code the page
    ///   trigger OnAfterGetRecord and one control's OnValidate; a repeater of fields of Rec; no part, no action, no page
    ///   extension, no subscriber (`page_code_is`); not a tree; on its own;
    /// - how it opened: `Run()` (:942, :998) or `RunModal()` (:970) of a Page variable from other code, not in lookup mode;
    /// - the rows: a procedure of the page inserted them before the run (`temp_worksheet_set_before_open`), 2 (:998) to 4
    ///   (:942), and Rec stands on the one inserted last, as it is stored: the first (:998) or the second (:942) row of the view.
    pub(super) fn temp_ws_preloaded_without_onopenpage(&mut self, st: &St) -> Result<bool, Signal> {
        const CODE: PinnedCode = PinnedCode {
            triggers: &["onaftergetrecord"],
            control_triggers: &["onvalidate"],
            properties: &["sourcetabletemporary", "sourcetableview", "insertallowed"],
            action_props: &[],
            table_triggers: &[],
        };
        let rec = self.page_rec(st);
        let opened = {
            let s = st.borrow();
            s.host.is_none()
                && !s.lookup
                && !s.opened_by_test
                && !s.run_by_currpage
                && s.run_static != Some(true)
                && s.set_record.is_none()
                && s.test_calls.is_empty()
        };
        let shape = opened
            && self.temp_worksheet_set_before_open(st)
            && self.rec_view_is(st, 1)
            && self.table(&rec)?.keys[1].len() == 1
            && self.tree_indentation(st).is_none()
            && super::super::flag_denied(&self.engine(), st.borrow().object as usize, "InsertAllowed")?
            && self.page_code_is(st, &CODE)?;
        if !shape {
            return Ok(false);
        }
        // Rec is a stored row as stored, the first or the second of 2 to 4 rows.
        let Some(stored) = self.stored_row(&rec)? else {
            return Ok(false);
        };
        if !Self::same_values(&stored, &rec.borrow().fields) || !(2..=4).contains(&self.view_row_count(st)?) {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        let above = self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(-2)])?;
        Ok(matches!(above, Value::Integer(0 | -1)))
    }

    /// The page opens on the row Rec was left on. The client reads that row first, then the rows from it to the end of the view,
    /// the row again, then each row above it; it holds the whole view and runs no read when the test walks it
    /// (PageVarOpenRow.al:942 TempWs_SecondaryKeyView_FilledBeforeRun_Run `log at handler` `R:D R:D R:A R:C R:D R:B `,
    /// `opened on` D, `log of the walk` empty; :970 the same run modally; :998 Rec on the first row of the view
    /// `R:B R:B R:A R:B `). Only on the pinned scenario (`temp_ws_preloaded_without_onopenpage`, which looked at the rows): the
    /// refusals below are a safety net and are not reached.
    pub(super) fn open_temp_ws_preloaded(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let mut pk = self.row_key(&rec)?;
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Get, &rec, &mut pk)?, Value::Boolean(true)) {
            return unsupported(
                "a temporary Worksheet whose row was set before it opened, on a primary key no row has (not pinned)",
            );
        }
        let start = rec.borrow().fields.clone();
        self.populate_current(st)?;
        rec.borrow_mut().fields = start.clone();
        let mut below = Vec::new();
        loop {
            below.push(self.read_window_row(st)?);
            if !self.nav_next(st, 1)? {
                break;
            }
            if below.len() == Self::WINDOW {
                return unsupported(
                    "a temporary Worksheet whose row was set before it opened, with more rows than a window holds (not pinned)",
                );
            }
        }
        rec.borrow_mut().fields = start.clone();
        self.populate_current(st)?;
        let shown = rec.borrow().fields.clone();
        below[0].shown = shown.clone();
        rec.borrow_mut().fields = start.clone();
        let mut above = Vec::new();
        while self.nav_next(st, -1)? {
            if above.len() + below.len() == Self::WINDOW {
                return unsupported(
                    "a temporary Worksheet whose row was set before it opened, with more rows than a window holds (not pinned)",
                );
            }
            above.push(self.read_window_row(st)?);
        }
        above.reverse();
        above.extend(below);
        rec.borrow_mut().fields = shown;
        self.set_xrec_fields(st, start);
        {
            let mut s = st.borrow_mut();
            s.window = above;
            s.window_from_start = true;
            s.window_eof = true;
            s.window_whole = true;
            s.cursor = Cursor::Row;
            s.draft = false;
            s.temp_ws_preloaded = true;
        }
        self.current_record_trigger(st, None)
    }

    /// Reads the row Rec stands on (OnAfterGetRecord) as a row of the client's window; Rec is left on the stored row.
    fn read_window_row(&mut self, st: &St) -> Result<crate::pages::WindowRow, Signal> {
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let stored = rec.borrow().fields.clone();
        self.populate_current(st)?;
        let shown = std::mem::replace(&mut rec.borrow_mut().fields, stored.clone());
        Ok(crate::pages::WindowRow { key, stored, shown })
    }

    /// A control validation starts on field `pos` of Rec: whether it is the edit the pin of the page has
    /// (PageVarOpenRow.al:947 `Ws.First(); Ws.Name.SetValue('changed')` in `ReadWs` at :903), noted for
    /// `refresh_temp_ws_preloaded`. All of it is required: the page opened by `open_temp_ws_preloaded`; the test read control
    /// values and called First and Next since (:906-:913), and this is the first value it types; the row held no unsaved edit;
    /// the field is in neither the primary key nor the view's key; the page stands on the first row of the view. On another
    /// row the edit is `OffFirstRow`; anything else is no pinned edit and keeps the earlier behavior.
    pub(in crate::pages) fn temp_ws_edit_begin(
        &mut self,
        st: &St,
        pos: usize,
        was_dirty: bool,
    ) -> Result<bool, Signal> {
        st.borrow_mut().temp_ws_edit = None;
        if !st.borrow().temp_ws_preloaded || was_dirty || st.borrow().cursor != Cursor::Row {
            return Ok(false);
        }
        let calls = match Self::test_calls(st).as_deref() {
            Some([before @ .., TestCall::Write(_)]) => before.iter().all(|c| {
                matches!(
                    c,
                    TestCall::Read | TestCall::Page(PageMethod::First) | TestCall::Page(PageMethod::Next)
                )
            }),
            _ => false,
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if !calls || table.keys[0].contains(&pos) || table.keys[rec.borrow().key].contains(&pos) {
            return Ok(false);
        }
        let key = self.row_key(&rec)?;
        let mut s = st.borrow_mut();
        let first = s.window.first().is_some_and(|w| Self::same_key(&w.key, &key));
        let moves = Self::moves_and_writes(&s.test_calls);
        s.temp_ws_edit = Some((
            moves,
            if first {
                TempWsEdit::Pinned
            } else {
                TempWsEdit::OffFirstRow
            },
        ));
        Ok(true)
    }

    /// The calls of the test that are not reads of a control's value.
    fn moves_and_writes(calls: &[TestCall]) -> usize {
        calls.iter().filter(|c| **c != TestCall::Read).count()
    }

    /// `CurrPage.Update(false)` in the pinned edit (`temp_ws_edit_begin`), standing on the first row of the view: the client
    /// reads the row, then every row from it to the end of the view, then the row again (PageVarOpenRow.al:947
    /// `log of the edit` `R:B R:B R:D R:A R:C R:B `, :975 run modally, :1003 two rows `R:B R:B R:A R:B `). From another row it
    /// is not pinned and refused. False when the refresh is not the one of that edit: `Update(true)`, an Update outside a
    /// control validation, a later call of the test.
    pub(in crate::pages) fn refresh_temp_ws_preloaded(&mut self, st: &St) -> Result<bool, Signal> {
        let edit = {
            let s = st.borrow();
            match s.temp_ws_edit {
                Some((moves, edit))
                    if s.temp_ws_preloaded
                        && s.refresh_update_false
                        && moves == Self::moves_and_writes(&s.test_calls) =>
                {
                    edit
                }
                _ => return Ok(false),
            }
        };
        st.borrow_mut().temp_ws_edit = None;
        if edit == TempWsEdit::OffFirstRow {
            return unsupported(
                "CurrPage.Update on a temporary Worksheet whose rows were set before it opened, off its first row (not pinned)",
            );
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let mut pk = key;
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut pk)?;
        let start = rec.borrow().fields.clone();
        self.populate_current(st)?;
        rec.borrow_mut().fields = start.clone();
        let mut rows = Vec::new();
        loop {
            rows.push(self.read_window_row(st)?);
            if !self.nav_next(st, 1)? {
                break;
            }
            if rows.len() == Self::WINDOW {
                return unsupported(
                    "a temporary Worksheet whose row was set before it opened, with more rows than a window holds (not pinned)",
                );
            }
        }
        rec.borrow_mut().fields = start.clone();
        self.populate_current(st)?;
        rows[0].shown = rec.borrow().fields.clone();
        self.set_xrec_fields(st, start);
        {
            let mut s = st.borrow_mut();
            s.window = rows;
            s.refresh = false;
        }
        self.current_record_trigger(st, None)?;
        Ok(true)
    }
}
