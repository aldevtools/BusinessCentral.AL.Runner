//! A temporary Worksheet without OnOpenPage whose rows a procedure of the page variable inserted before `Run()` or
//! `RunModal()`, in the key of its SourceTableView (al-runner issue 7420; corpus pages/run-modal/PageVarOpenRow.al).

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// The page's shape as page 63427 of PageVarOpenRow.al:402 has it: a Worksheet over a temporary source whose rows were set
    /// before the open (`temp_worksheet_set_before_open`), in a key other than the primary key, InsertAllowed = false, no
    /// OnOpenPage, OnAfterGetRecord as its only row trigger, outside lookup mode, on its own, not a tree.
    pub(super) fn temp_ws_preloaded_without_onopenpage(&self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let shape = st.borrow().host.is_none()
            && !st.borrow().lookup
            && self.temp_worksheet_set_before_open(st)
            && rec.borrow().key != 0
            && self.tree_indentation(st).is_none()
            && !self.declares_or_ext_declares(st, "onopenpage")
            && self.declares(st, "onaftergetrecord")
            && ["onaftergetcurrrecord", "onfindrecord", "onnextrecord"]
                .iter()
                .all(|t| !self.declares(st, t));
        Ok(shape && super::super::flag_denied(&self.engine(), st.borrow().object as usize, "InsertAllowed")?)
    }

    /// The page opens on the row Rec was left on. The client reads that row first, then the rows from it to the end of the view,
    /// the row again, then each row above it; it holds the whole view and runs no read when the test walks it
    /// (PageVarOpenRow.al:942 TempWs_SecondaryKeyView_FilledBeforeRun_Run `log at handler` `R:D R:D R:A R:C R:D R:B `,
    /// `opened on` D, `log of the walk` empty; :970 the same run modally; :998 Rec on the first row of the view
    /// `R:B R:B R:A R:B `). A view longer than a window, and a Rec on a key no row has, are not pinned.
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

    /// `CurrPage.Update` on that page, standing on the first row of the view: the client reads the row, then every row from it
    /// to the end of the view, then the row again (PageVarOpenRow.al:947 `log of the edit` `R:B R:B R:D R:A R:C R:B `, :975 run
    /// modally, :1003 two rows `R:B R:B R:A R:B `). From another row it is not pinned. False when the page is not that page.
    pub(in crate::pages) fn refresh_temp_ws_preloaded(&mut self, st: &St) -> Result<bool, Signal> {
        if !st.borrow().temp_ws_preloaded {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        if !st.borrow().window.first().is_some_and(|w| Self::same_key(&w.key, &key)) {
            return unsupported(
                "CurrPage.Update on a temporary Worksheet whose rows were set before it opened, off its first row (not pinned)",
            );
        }
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
