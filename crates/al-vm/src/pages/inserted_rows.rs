//! Rows inserted while a list is open: what the client holds after an action stored them, and what a GoToKey finds of a row the
//! test stored itself (corpus branch al-runtime-rs-staging-y65 eefaf01b, pages/actions/
//! Y65GoToRecordActionInsert.al, al-runner issue 6671).

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// What an action leaves behind in the list: the window of the old view is dropped, then the rows the action inserted are held.
    pub(super) fn settle_rows_after_action(
        &mut self,
        st: &St,
        view_before: Option<actions::ListView>,
    ) -> Result<(), Signal> {
        self.drop_window_of_the_old_view(st, view_before)?;
        self.hold_rows_an_action_inserted(st)
    }

    /// After an action, the client holds the rows of the view as they are now: a row the action inserted, before the row the list
    /// stands on or after it, is among them, with `CurrPage.Update()`, `Update(false)` and without an update, so GoToRecord
    /// finds it on a Worksheet with both row triggers, with neither, with only one of them and on a List with both
    /// (Y65GoToRecordActionInsert.al:593-657, `insert A, Update()`, `insert A, Update(false)`, `insert A, no Update`, `insert C,
    /// Update()` all `ok:on B Inserted A|C`).
    ///
    /// Pinned for a top-level list that held its whole view and whose view only gained rows; the rows it gains are held as
    /// stored, without a read (what the client reads for them is not pinned). A list that held a full first window of a longer
    /// view holds the first window of the new view the same way: a row inserted before every row or among the rows it held is
    /// found by GoToKey and GoToRecord, with 20, 21 and 60 rows, with and without `CurrPage.Update()`, on a Worksheet with both
    /// row triggers (corpus pages/long-list-open/Y7344ListFirstWindow.al:311 Y7344_Both_InsertBeforeAll_Update, :337
    /// _NoUpdate, :381 Y7344_Both_InsertMiddle_Update `found=Yes now=A|R1010A`; al-runner issue 7344); the rows it held that
    /// the new row pushes past the window are dropped, as a block read past the window reads them again. A row the action
    /// deleted, a tree and a list that serves its rows from OnFindRecord or OnNextRecord keep the rows as they were.
    pub(super) fn hold_rows_an_action_inserted(&mut self, st: &St) -> Result<(), Signal> {
        let shape = {
            let s = st.borrow();
            s.open
                && s.host.is_none()
                && s.cursor == Cursor::Row
                && !s.draft
                && !s.dirty
                && s.window_from_start
                && !s.window.is_empty()
                && !s.window_whole
                && s.window.len() <= Self::WINDOW
        };
        if !shape
            || !self.has_source(st)
            || !self.is_list(st)
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
            || self.tree_indentation(st).is_some()
        {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if rec.borrow().marked_only {
            return Ok(());
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        let full = st.borrow().window.len() == Self::WINDOW;
        let mut rows: Vec<al_builtins::value::WindowRow> = Vec::new();
        self.ret_used = true;
        let mut at = matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("-".into())])?,
            Value::Boolean(true)
        );
        while at {
            if rows.len() == Self::WINDOW {
                if !full {
                    return Ok(());
                }
                break;
            }
            let key = self.row_key(&probe)?;
            let stored = probe.borrow().fields.clone();
            rows.push(al_builtins::value::WindowRow {
                key,
                stored: stored.clone(),
                shown: stored,
            });
            self.ret_used = true;
            at = matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0);
        }
        let held = st.borrow().window.clone();
        let kept = |h: &al_builtins::value::WindowRow| rows.iter().any(|r| Self::same_key(&r.key, &h.key));
        if full {
            // The rows held that the new first window no longer holds must be the last ones held (pushed past the window), and
            // the window must hold a row it did not hold before.
            let pushed_from = held.iter().position(|h| !kept(h)).unwrap_or(held.len());
            let gained = rows
                .iter()
                .any(|r| !held.iter().any(|h| Self::same_key(&h.key, &r.key)));
            if held[pushed_from..].iter().any(kept) || !gained {
                return Ok(());
            }
        } else if !held.iter().all(kept) || rows.len() <= held.len() {
            return Ok(());
        }
        let window = rows
            .into_iter()
            .map(|r| {
                held.iter()
                    .find(|h| Self::same_key(&h.key, &r.key))
                    .cloned()
                    .unwrap_or(r)
            })
            .collect();
        let mut s = st.borrow_mut();
        s.window = window;
        if full {
            // The view gained a row past a full window: it goes on after the window.
            s.window_eof = false;
        }
        Ok(())
    }

    /// A GoToKey on a list without OnAfterGetCurrRecord walks the view from its first row (`page_go_to_key`). When the test stored
    /// a row before the first row the client holds, the rows it holds no longer start at the view's start: the walk finds the new
    /// row as any other, on a Worksheet with neither row trigger and with OnAfterGetRecord only (Y65GoToRecordActionInsert.al:609,
    /// :625 `test inserts A, no action` `ok:on B Inserted A`). Which rows it reads on the way is not pinned.
    pub(super) fn forget_rows_held_behind_a_new_first_row(&mut self, st: &St) -> Result<(), Signal> {
        let shape = {
            let s = st.borrow();
            s.host.is_none() && s.window_from_start && !s.window.is_empty()
        };
        if !shape
            || !self.is_list(st)
            || self.declares(st, "onaftergetcurrrecord")
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
        {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("-".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(());
        }
        let first = self.row_key(&probe)?;
        if st.borrow().window.iter().any(|w| Self::same_key(&w.key, &first)) {
            return Ok(());
        }
        st.borrow_mut().window_from_start = false;
        Ok(())
    }

    /// Where the walk of `page_go_to_key` starts from: Rec's fields, the cursor and whether the row is a draft, after
    /// `forget_rows_held_behind_a_new_first_row`.
    pub(super) fn go_to_key_walk_start(&mut self, st: &St) -> Result<(Vec<Value>, Cursor, bool), Signal> {
        self.forget_rows_held_behind_a_new_first_row(st)?;
        let fields = self.page_rec(st).borrow().fields.clone();
        let s = st.borrow();
        Ok((fields, s.cursor, s.draft))
    }
}
