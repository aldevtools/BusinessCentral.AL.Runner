//! GoToRecord after a marking OnDrillDown with CurrPage.Update(false) (`drilldown_marked_only_refresh`), and the row the refill
//! settles on when the drilldown left Rec outside the marked rows.

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// GoToRecord after an OnDrillDown marked rows, turned MarkedOnly on, left Rec on the last of them and called
    /// `CurrPage.Update(false)` (`drilldown_marked_only_refresh`). The client first refills its rows around the row Rec was left on,
    /// as the refresh after an action reads them from Rec's row (`read_list_again_from_row_as`): in a key other than the primary
    /// key that row once more, then the row, the rows above it from the nearest up, and the row read as a move onto it (three times
    /// in such a key, twice in the primary key) with OnAfterGetCurrRecord. Then:
    /// - a GoToRecord of Rec's row answers true on it (corpus pages/onopenpage-moves-rec/SuRefillKey.al:269
    ///   Name_DrillDown_GoToRecord `goto 3` `Yes/3:R3;R3;R5;R2;R3;R3;R3;C3;`, in the key ByName);
    /// - a GoToRecord of another row walks the marked rows from the first, finds none and answers false, on the row the walk
    ///   ended on (SuRefillKey.al:379 Id_DrillDown_GoToRecord `goto 3` `No/5:R5;R3;R2;R5;R5;C5;R2;R2;C2;R3;R3;C3;R5;R5;C5;`, in
    ///   the primary key; al-runner issue 6008).
    ///
    /// Each answer is pinned in one key only: the other key's, Rec on a row with marked rows after it and more marked rows than
    /// a window holds are refused.
    pub(super) fn go_to_record_after_marked_refill(&mut self, st: &St, want: &[Value]) -> Result<Value, Signal> {
        let Some(left) = st.borrow_mut().marked_refill_in_key.take() else {
            return unsupported("GoToRecord after a marking OnDrillDown without the rows it left (not pinned)");
        };
        self.pages.scope_unpinned.retain(|p| !Rc::ptr_eq(p, st));
        let refuse = |what: &str| -> Result<Value, Signal> {
            unsupported(format!(
                "GoToRecord after an OnDrillDown turned MarkedOnly on and called CurrPage.Update(false) on a list, {what} (not pinned)"
            ))
        };
        let in_key = self.still_reads_once_more(st);
        let before = self.xrec_before_move(st);
        let rec = self.page_rec(st);
        rec.borrow_mut().fields = left;
        let outside = self.marked_refill_outside(st, &rec)?;
        let left_key = self.row_key(&rec)?;
        let of_recs_row = Self::same_key(want, &left_key);
        // With a part the GoToRecord of Rec's row refills as without one, in either key (corpus SuRefillKey.al:518, :562
        // TW_*WithPart_GoToRecordOfRecsRow). A GoToRecord of another row walks the marked rows after the refill: in a key other
        // than the primary key all of them, ending on Rec's row (:541 TW_NameKeyWithPart_GoToRecordOfUnmarkedRow `goto 2`
        // `No/1:R1;R1;R3;R1;R1;R1;C1;R3;R3;R3;C3;R1;R1;R1;C1;`, al-runner issue 6141); in the primary key only a move onto the
        // first marked row, where the page stays (:585 TW_IdKeyWithPart_GoToRecordOfUnmarkedRow `goto 2`
        // `No/1:R3;R1;R3;R3;C3;R1;R1;C1;`). Both are pinned with two marked rows, Rec's row the last.
        let with_parts = !st.borrow().parts.is_empty();
        if with_parts {
            // Pinned by pages/onopenpage-moves-rec/SuRefillKey.al TW_NameKeyWithPart_GoToRecordOfUnmarkedRow, TW_IdKeyWithPart_GoToRecordOfUnmarkedRow, UD_IdKeyWithPart_DrillDownAgainAfterRowLostFlag, UD_IdKeyWithPart_ActionAfterRowLostFlag (was assumption `drilldown-marked-walk-with-part`).
        } else if of_recs_row != in_key {
            return refuse(if in_key {
                "in a key other than the primary key, of a row other than the row Rec was left on"
            } else {
                "in the primary key, of the row Rec was left on"
            });
        }
        if !self.nav_find(st, "=")? {
            return refuse("with Rec left outside the marked rows");
        }
        let key = self.row_key(&rec)?;
        let stored = rec.borrow().fields.clone();
        if self.nav_next(st, 1)? {
            return refuse("with marked rows after the row Rec was left on");
        }
        rec.borrow_mut().fields = stored.clone();
        let mut above = Vec::new();
        while self.nav_next(st, -1)? {
            above.push((self.row_key(&rec)?, rec.borrow().fields.clone()));
            if above.len() >= Self::WINDOW {
                return refuse("with more marked rows than a window holds");
            }
        }
        let reads = if in_key { 2 } else { 1 };
        for i in 0..reads {
            // Rec left outside the marked rows: in another key the first read is still the row Rec was left on
            // (`marked_refill_outside`).
            let read = match &outside {
                Some(left) if i == 0 && in_key => left.clone(),
                _ => stored.clone(),
            };
            rec.borrow_mut().fields = read;
            self.populate_current(st)?;
        }
        let mut held = Vec::new();
        for (k, f) in &above {
            rec.borrow_mut().fields = f.clone();
            self.populate_current(st)?;
            held.push(crate::pages::WindowRow {
                key: k.clone(),
                stored: f.clone(),
                shown: rec.borrow().fields.clone(),
            });
        }
        held.reverse();
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::Get, &rec, &mut key.clone())?,
            Value::Boolean(true)
        ) {
            return refuse("with the row Rec was left on gone");
        }
        let stored = rec.borrow().fields.clone();
        for _ in 0..reads + 1 {
            rec.borrow_mut().fields = stored.clone();
            self.populate_current(st)?;
        }
        let shown = rec.borrow().fields.clone();
        held.push(crate::pages::WindowRow {
            key,
            stored: stored.clone(),
            shown,
        });
        {
            let mut s = st.borrow_mut();
            s.window = held;
            s.window_from_start = true;
            s.window_whole = true;
            s.window_eof = true;
            s.view_read_to_end = true;
            s.cursor = Cursor::Row;
            s.draft = false;
            s.stale = false;
        }
        self.set_xrec_fields(st, stored);
        let r = self.current_record_trigger(st, before);
        self.torn_down_on_error(st, r)?;
        if of_recs_row {
            return Ok(Value::Boolean(true));
        }
        // The walk over the marked rows from the first one, a move onto each (SuRefillKey.al:379 `goto 3`: `R2;R2;C2;R3;R3;C3;
        // R5;R5;C5;` after the refill); it ends on Rec's row, the last, so there is no move back.
        let last = if with_parts && !in_key && outside.is_none() {
            0
        } else {
            st.borrow().window.len() - 1
        };
        for idx in 0..=last {
            let r = self.arrive_held(st, idx);
            self.torn_down_on_error(st, r)?;
        }
        // A false GoToRecord moves no part (`page_method`); here the page did move, and the part follows to the row it ends on
        // (corpus writer UD, branch al-runtime-rs-staging-ud, SuRefillKey.al:630, :722 `goto 2 again` `No/1`, `part` `D1`). What
        // the part reads on the way is not pinned.
        if with_parts {
            self.sync_parts(st)?;
        }
        Ok(Value::Boolean(false))
    }

    /// The drilldown left Rec on a row the marked rows do not hold (al-runner issue 6946: the marking loop walked Rec itself and
    /// ended on the last row of the view, which is not marked). The refill settles on the nearest marked row before it and goes
    /// on as if Rec stood there; in a key other than the primary key the first read is still the row Rec was left on
    /// (pages/onopenpage-moves-rec/DrillDownMarkedRecLeft.al:408 Name_LeaveUnmarked_GoToMarkedThenUnmarked `goto 3`
    /// `Yes[3]:R4;R3;R2;R3;R3;R3;C3`, :646 Pk_LeaveUnmarked_GoToMarkedThenUnmarked `Yes[3]:R3;R2;R3;R3;C3`). Any other row then walks
    /// the marked rows from the first and answers false on that row, in either key (:452 Name_LeaveUnmarked_GoToUnmarked,
    /// :669 Pk_LeaveUnmarked_GoToFirstMarked `No[3]:R3;R2;R3;R3;C3;R2;R2;C2;R3;R3;C3`). Pinned with a part, the row Rec was left on the
    /// last of the view; marked rows after it, no marked row before it or no part are refused. `None` when Rec is on a marked row;
    /// else Rec is moved to that row and the row it was left on is answered.
    fn marked_refill_outside(&mut self, st: &St, rec: &Rec) -> Result<Option<Vec<Value>>, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::Find, &probe, &mut [Value::Text("=".into())])?,
            Value::Boolean(true)
        ) {
            return Ok(None);
        }
        let refuse = |what: &str| -> Result<Option<Vec<Value>>, Signal> {
            unsupported(format!(
                "GoToRecord after an OnDrillDown turned MarkedOnly on and called CurrPage.Update(false) on a list, {what} (not pinned)"
            ))
        };
        if st.borrow().parts.is_empty() {
            return refuse("with Rec left outside the marked rows");
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0) {
            return refuse("with Rec left outside the marked rows, before a marked row");
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(-1)])?, Value::Integer(n) if n != 0) {
            return refuse("with Rec left outside the marked rows, after no marked row");
        }
        let nearest = probe.borrow().fields.clone();
        Ok(Some(std::mem::replace(&mut rec.borrow_mut().fields, nearest)))
    }
}
