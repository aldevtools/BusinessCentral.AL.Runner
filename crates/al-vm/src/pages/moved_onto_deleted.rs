//! A move of a plain list onto a row that a deletion by code took from the table after the list read it. The pinned values are
//! the Y80_Moved_All_* and Y80_Moved_NoNew_* tests of pages/lt-y80-gone-rows/Y80GoneRows.al (:1677 Next, :1715 Previous, :1785
//! First, :1854 Last, :1922 GoToKey on the deleted key, and the same five for the list without the new-row line from :1960).
//! The list stands on row `c` and one other row of the rows it holds is gone; `c` is the row the cursor stays on or moves from.
//!
//! BC 27 and BC 29: the move logs `R:c;C:c` (not for Last on a list with the new-row line, which logged `N:0;C:0` first), then the
//! page reads its rows again around `c`: `c` and the live rows after it (when there are any), `c` and the live rows before it up
//! (when there are any), then `c` twice and OnAfterGetCurrRecord. The move answers true, the cursor stays on `c`, and the client
//! holds the live rows (a later Next, Previous, First, Last or GoToKey is the plain move over them).
//! BC 28: the move logs `R:c;C:c`; Next logs it once more and stays on `c`; Previous, First and Last go on to the next live row
//! in their direction (the plain move: `R:t;R:t;C:t`). Either way the gone row is no longer held afterwards.
//! Pinned only for the shape of those pages: a plain editable list on a database table in its primary key, no host, no
//! OnFindRecord or OnNextRecord, a whole view of fewer rows than a window, exactly one row gone, `c` alive. Anything else
//! keeps the old refusal (`reread_held_row`).

use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether the row held at `idx` is still in the table (the page's Rec is left as it was).
    fn held_row_alive(&mut self, st: &St, idx: usize) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let saved = rec.borrow().fields.clone();
        let mut key = st.borrow().window[idx].key.clone();
        self.ret_used = true;
        let found = self.record_op(RecordOp::Get, &rec, &mut key);
        rec.borrow_mut().fields = saved;
        Ok(matches!(found?, Value::Boolean(true)))
    }

    /// The shape the corpus pins (see the module comment).
    fn moved_onto_deleted_shape(&self, st: &St) -> bool {
        let rec = self.page_rec(st);
        let r = rec.borrow();
        let s = st.borrow();
        self.is_list(st)
            && s.host.is_none()
            && s.parts.is_empty()
            && s.cursor == Cursor::Row
            && !s.draft
            && !s.stale
            && !s.action_ran
            && s.open
            && s.window_from_start
            && !s.modal
            && s.window.len() >= 2
            && s.window.len() < Self::WINDOW
            && s.moved_rec_open_partial.is_none()
            && r.key == 0
            && !r.temporary
            && self.has_source(st)
            && self.declares(st, "onaftergetrecord")
            && self.declares(st, "onaftergetcurrrecord")
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
            && self.page_editable(st)
            && self.tree_indentation(st).is_none()
    }

    /// The move `how` of a list standing on the held row `at`, aimed at the held row `target`: `Ok(None)` when the page or the
    /// state is not the pinned shape or the target is still there (the plain move goes on); `Ok(Some(answer))` when the move
    /// was done here. `line_ran`: Last on a list with the new-row line has landed on the line already (`N:0;C:0`).
    pub(super) fn moved_onto_deleted(
        &mut self,
        st: &St,
        how: PageMethod,
        at: usize,
        target: usize,
        line_ran: bool,
    ) -> Result<Option<bool>, Signal> {
        if !self.moved_onto_deleted_shape(st) || self.still_reads_once_more(st) {
            return Ok(None);
        }
        let len = st.borrow().window.len();
        if at >= len || target >= len || self.held_row_alive(st, target)? {
            return Ok(None);
        }
        let mut alive = Vec::with_capacity(len);
        for i in 0..len {
            alive.push(i != target && self.held_row_alive(st, i)?);
        }
        if !alive[at] || alive.iter().filter(|a| !**a).count() != 1 {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let before = self.xrec_before_move(st);
        // The client re-reads the row the cursor stands on.
        let on_row = |s: &mut Self, rec: &Rec| -> Result<(), Signal> {
            let stored = st.borrow().window[at].stored.clone();
            rec.borrow_mut().fields = stored;
            s.reread_held_row(st, at)?;
            Ok(())
        };
        on_row(self, &rec)?;
        let kept: Vec<usize> = (0..len).filter(|i| alive[*i]).collect();
        let c_in_kept = kept.iter().position(|i| *i == at).unwrap_or(0);
        if !line_ran {
            self.populate_current(st)?;
            self.current_record_trigger(st, before.clone())?;
        }
        let stays = self.has(al_version::Gate::MovedOntoDeletedStaysFrom28)
            && !self.has(al_version::Gate::MovedOntoDeletedRefreshesAgainFrom29);
        if stays {
            st.borrow_mut().window.remove(target);
            if how == PageMethod::Next {
                on_row(self, &rec)?;
                self.populate_current(st)?;
                self.current_record_trigger(st, before)?;
                return Ok(Some(true));
            }
            let now = kept.len();
            let next_target = match how {
                PageMethod::First => 0,
                PageMethod::Last => now - 1,
                PageMethod::Previous if c_in_kept > 0 => c_in_kept - 1,
                _ => {
                    return unsupported(
                        "a move onto a deleted row with no live row on that side of the cursor (not pinned)",
                    );
                }
            };
            if next_target == c_in_kept {
                return unsupported("a move onto a deleted row that ends on the row the cursor stands on (not pinned)");
            }
            self.read_client_row(st, next_target)?;
            // Previous from the blank line afterwards reads the row (Y80_Moved_All_LastOntoDeleted BC 28 `previous log`).
            st.borrow_mut().refreshed_after_gone_move = true;
            return Ok(Some(true));
        }
        // BC 27 and BC 29: read the rows again around the cursor row.
        let read_from_c =
            |s: &mut Self, up: bool, expect: usize| -> Result<Vec<al_builtins::value::WindowRow>, Signal> {
                let mut rows = Vec::new();
                if expect == 0 {
                    return Ok(rows);
                }
                on_row(s, &rec)?;
                s.populate_current(st)?;
                while s.nav_next(st, if up { -1 } else { 1 })? {
                    let key = s.row_key(&rec)?;
                    let stored = rec.borrow().fields.clone();
                    s.populate_current(st)?;
                    let shown = rec.borrow().fields.clone();
                    rows.push(al_builtins::value::WindowRow { key, stored, shown });
                    if rows.len() > expect {
                        break;
                    }
                }
                if rows.len() != expect {
                    return unsupported(
                        "a move onto a deleted row where the rows read again differ from the rows held (not pinned)",
                    );
                }
                Ok(rows)
            };
        let after = read_from_c(self, false, kept.len() - 1 - c_in_kept)?;
        let above = read_from_c(self, true, c_in_kept)?;
        on_row(self, &rec)?;
        self.populate_current(st)?;
        self.populate_current(st)?;
        let shown = rec.borrow().fields.clone();
        let stored = st.borrow().window[at].stored.clone();
        let key = st.borrow().window[at].key.clone();
        self.current_record_trigger(st, before)?;
        let mut window: Vec<al_builtins::value::WindowRow> = above.into_iter().rev().collect();
        window.push(al_builtins::value::WindowRow { key, stored, shown });
        window.extend(after);
        {
            let mut s = st.borrow_mut();
            s.window = window;
            s.refreshed_after_gone_move = true;
        }
        Ok(Some(true))
    }
}
