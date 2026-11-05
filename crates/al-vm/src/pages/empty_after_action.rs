//! An action of a list with OnAfterGetRecord and OnAfterGetCurrRecord set a filter that leaves the view without rows and called
//! CurrPage.Update(false) or Update(true) (al-runner issue 6945). The page shows a blank row and runs no OnAfterGetCurrRecord.
//! After the update OnAfterGetRecord runs for rows of the old view the new one no longer holds: on BC 27 and 29 once, for the row
//! before the row the page stood on (the row after it when there is none); on BC 28 for every other row, from the row before it up
//! to the first, then the rows after it (corpus pages/actions/ActionFilterEmptyView.al:226 Action_NoRowsLeft_UpdateFalse_Moves
//! `[|]:R1;a1;u1;R2`, 28.5 `R1;a1;u1;R2;R3;R4`; :322 Action_NoRowsLeft_FromLaterRow_UpdateFalse `R3;a3;u3;R2`, 28.5
//! `R3;a3;u3;R2;R1;R4`; _UpdateTrue alike). First, Next, Last, Previous, GoToRecord and GoToKey then answer false on the blank
//! row (:226, :251 _Moves, :276, :299 _GoTo).
//!
//! Pinned for a list whose view had no filter before the action, read from a row of fewer rows than a window; any other shape is
//! refused.

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether control `idx` reads blank: a control bound to the page's record on a page that shows no row (`shows_no_row`), and
    /// after an action left the view without rows any control in the repeater, a page variable too
    /// (pages/actions/ActionFilterEmptyView.al:226 Action_NoRowsLeft_UpdateFalse_Moves `[|]`).
    pub(super) fn value_reads_blank(&self, st: &St, idx: usize) -> bool {
        let blank_row = st.borrow().empty_after_action && st.borrow().cursor == Cursor::None;
        (blank_row && self.in_repeater(st, idx)) || self.shows_no_row(st, idx)
    }

    pub(super) fn empty_view_after_action(&mut self, st: &St, rec: &Rec) -> Result<(), Signal> {
        let refuse = || unsupported("an action left the list's view without rows (not pinned)");
        let Some((filters, key, adhoc, ascending)) = st.borrow().view_before_action.clone() else {
            return refuse();
        };
        if !filters.is_empty() || adhoc.is_some() {
            return refuse();
        }
        let old = || -> Rec {
            let mut r = rec.borrow().clone();
            r.filters = Vec::new();
            r.key = key;
            r.ascending = ascending;
            Rc::new(RefCell::new(r))
        };
        let major = self
            .engine
            .target
            .as_ref()
            .map_or(al_version::MIN_BC_MAJOR, |t| t.bc.major);
        let Some(every_row) = al_version::empty_action_view_reads_every_row(major) else {
            return refuse();
        };
        let mut reads = Vec::new();
        for step in [-1, 1] {
            let probe = old();
            loop {
                self.ret_used = true;
                if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)])?, Value::Integer(n) if n != 0)
                {
                    break;
                }
                reads.push(probe.borrow().fields.clone());
                if !every_row || reads.len() >= Self::WINDOW {
                    break;
                }
            }
            if !reads.is_empty() && !every_row {
                break;
            }
        }
        if reads.len() >= Self::WINDOW {
            return refuse();
        }
        for fields in reads {
            rec.borrow_mut().fields = fields;
            self.populate_current(st)?;
        }
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::None;
            s.window = Vec::new();
            s.focus = None;
            s.focus_in_row = false;
            s.empty_after_action = true;
        }
        let table = rec.borrow().table as usize;
        let fresh = self.initial_fields(table);
        rec.borrow_mut().fields = fresh;
        Ok(())
    }

    /// The row a list settles on when the action's filter left the stored row it stood on outside the view: the next row of the
    /// view, else the one before it (pages/list-order/AiListOrderTests.al:335 DropCurrent_RowsAfter_Plain and :301 _FindOnly
    /// read R1003 first, :501 DropCurrent_NoRowsAfter_Plain and :467 _FindOnly read R1005 first). `None` when the row is in the
    /// view; a view left with no row is refused.
    pub(super) fn row_after_view_left(&mut self, rec: &Rec) -> Result<Option<Vec<Value>>, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(None);
        }
        for step in [1, -1] {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)])?, Value::Integer(n) if n != 0)
            {
                return Ok(Some(probe.borrow().fields.clone()));
            }
        }
        unsupported("an action left the list's view without rows (not pinned)")
    }
}
