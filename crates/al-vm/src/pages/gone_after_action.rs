//! A plain list after an action deleted the row it stood on while the table still has another row of the view. The page settles
//! on the next row (else the previous one) and reads it as a move does, `r;r;c` with the deleted row as xRec; the refresh after
//! the action then reads the rows from there without the extra first read, so the log is two such blocks
//! (pages/W1Update.al:577 List_ActionDeletesOneOfTwoRows_WithoutUpdate `r1;action DeleteRowNoUpdate done;r2;r2;c2;r2;r2;c2;`;
//! misc/lt-probes-needed-7/PnGRefresh.al:168 ListLosesOneOfTwoRowsWithoutUpdate `AGR(U2,Upd 2);AGR(U2,Upd 2);AGCR(U2,U1);AGR(U2,Upd 2);
//! AGR(U2,Upd 2);AGCR(U2,U2)`). The client no longer holds the deleted row, so First stands on the row that is left and reads nothing
//! (W1Update.al:574 `first` Yes, `id_after_first` 2; PnGRefresh.al:159 `first-no` U2).
//!
//! Pinned for a top-level list with both row triggers, no OnFindRecord or OnNextRecord, that held its whole view of fewer rows than
//! a window, where exactly the row Rec stood on is gone. Any other shape keeps `arrive` and the rows as they were.

use super::*;

impl Session {
    fn gone_current_shape(&self, st: &St) -> bool {
        let shape = {
            let s = st.borrow();
            s.open
                && s.host.is_none()
                && s.parts.is_empty()
                && s.cursor == Cursor::Row
                && !s.draft
                && !s.dirty
                && !s.stale
                && s.window_from_start
                && s.window.len() >= 2
                && s.window.len() < Self::WINDOW
        };
        shape
            && self.has_source(st)
            && self.is_list(st)
            && self.declares(st, "onaftergetrecord")
            && self.declares(st, "onaftergetcurrrecord")
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
            && self.tree_indentation(st).is_none()
            && {
                let rec = self.page_rec(st);
                let r = rec.borrow();
                !r.marked_only && !r.temporary
            }
    }

    /// The client reads the row an action left Rec on, after the row it stood on was deleted, as a move onto it, and no longer
    /// holds the deleted row. `false`: not the pinned shape, nothing was done.
    fn settle_after_gone_current(&mut self, st: &St) -> Result<bool, Signal> {
        if !self.gone_current_shape(st) {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let before_key = {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            probe.borrow_mut().fields = self.page_xrec_fields(st);
            self.row_key(&probe)?
        };
        let now_key = self.row_key(&rec)?;
        let len = st.borrow().window.len();
        let mut gone = Vec::new();
        for i in 0..len {
            if !self.held_row_alive(st, i)? {
                gone.push(i);
            }
        }
        let [gone_at] = gone[..] else {
            return Ok(false);
        };
        let now_at = st.borrow().window.iter().position(|w| Self::same_key(&w.key, &now_key));
        let Some(now_at) = now_at.filter(|n| *n != gone_at) else {
            return Ok(false);
        };
        if !Self::same_key(&st.borrow().window[gone_at].key, &before_key) {
            return Ok(false);
        }
        st.borrow_mut().window.remove(gone_at);
        let now_at = now_at - usize::from(gone_at < now_at);
        st.borrow_mut().draft = false;
        st.borrow_mut().stale = false;
        self.read_client_row(st, now_at)?;
        st.borrow_mut().gone_row_settled = true;
        Ok(true)
    }

    /// `arrive` after the row an action deleted was left for the next row (see `settle_after_action`).
    pub(super) fn arrive_after_gone_row(&mut self, st: &St) -> Result<(), Signal> {
        if self.settle_after_gone_current(st)? {
            return Ok(());
        }
        self.arrive(st)
    }

    /// `read_page_again` after an action that left Rec on another stored row than the one it deleted (TD_DeleteOneOfTwo_MoveToOther_*).
    pub(super) fn read_page_again_after_gone(&mut self, st: &St) -> Result<(), Signal> {
        if st.borrow().open && st.borrow().cursor == Cursor::Row && !st.borrow().draft {
            let rec = self.page_rec(st);
            if self.stored_row(&rec)?.is_some() {
                self.settle_after_gone_current(st)?;
            }
        }
        self.read_page_again(st)
    }

    /// Whether the refresh of a list reads the row it stands on before the others: not right after `settle_after_gone_current`.
    pub(super) fn reads_row_first(&self, st: &St) -> bool {
        !std::mem::take(&mut st.borrow_mut().gone_row_settled)
    }
}
