//! A Worksheet over a temporary source that empties and refills Rec from a filter field's OnValidate or from an action, the
//! shape of page 5530 "Item Availability by Event" (corpus pages/lt-outside-worksheet/RmTempRefill.al, al-runner issues 5779
//! and 5780): what the client reads afterwards and which row it shows.

use super::*;
use al_builtins::record_op::RecordOp;
use al_builtins::value::WindowRow;

impl Session {
    /// The pinned shape: a top-level Worksheet over a temporary source in its primary key without filters, with OnAfterGetRecord
    /// and OnAfterGetCurrRecord and without OnFindRecord or OnNextRecord, standing on a stored row.
    pub(super) fn temp_refill_shape(&mut self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        if engine.object_facts[object].page_type != Some(al_ir::PageType::Worksheet)
            || !self.has_source(st)
            || !self.is_list(st)
            || st.borrow().host.is_some()
            || !st.borrow().open
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || !self.declares(st, "onaftergetrecord")
            || !self.declares(st, "onaftergetcurrrecord")
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
        {
            return false;
        }
        let rec = self.page_rec(st);
        let r = rec.borrow();
        r.temporary && r.key == 0 && r.ascending && r.filters.is_empty()
    }

    /// Whether the rows the client holds are no longer what the table has (a row gone or changed).
    pub(super) fn held_rows_changed(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let held: Vec<WindowRow> = st.borrow().window.clone();
        for w in &held {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            probe.borrow_mut().filters.clear();
            let mut pk = w.key.clone();
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Get, &probe, &mut pk)?, Value::Boolean(true))
                || !Self::same_values(&probe.borrow().fields, &w.stored)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The rows of the view from its start, up to `limit` of them.
    fn view_rows(&mut self, rec: &Rec, limit: usize) -> Result<Vec<Vec<Value>>, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        let mut rows = Vec::new();
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            return Ok(rows);
        }
        loop {
            rows.push(probe.borrow().fields.clone());
            if rows.len() >= limit {
                return Ok(rows);
            }
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0)
            {
                return Ok(rows);
            }
        }
    }

    /// Reads row `fields` with OnAfterGetRecord and gives the row the client then holds.
    fn read_refilled_row(&mut self, st: &St, rec: &Rec, fields: &[Value]) -> Result<WindowRow, Signal> {
        rec.borrow_mut().fields = fields.to_vec();
        let key = self.row_key(rec)?;
        self.populate_current(st)?;
        Ok(WindowRow {
            key,
            stored: fields.to_vec(),
            shown: rec.borrow().fields.clone(),
        })
    }

    /// After a filter field's OnValidate refilled the rows: the client stays on the row it stood on when the refill has its key,
    /// else on the row Rec was left on. From the first row of the view it reads the rows from there on (up to a window), then the
    /// row twice and runs OnAfterGetCurrRecord (RmTempRefill.al:215 RM_Filter_B_FromOpen `R:B1;R:B2;R:B1;R:B1;C:B1;`, :254 from
    /// row 23, :302 RM_Filter_BThenCleared `R:A1;...;R:A20;R:A1;R:A1;C:A1;` then 27 rows). From a later row of a view the window
    /// holds whole it reads the row, the rows from it on, the rows before it, then the row twice (:279 RM_Filter_B_FromRow2_Previous
    /// `R:B2;R:B2;R:B1;R:B2;R:B2;C:B2;`, shows B2). A held row that is gone while Rec stands on a row other than the first of the
    /// view, and a later row of a longer view, are not pinned.
    pub(super) fn reread_after_refill_by_validate(&mut self, st: &St, held: &[Value]) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, Self::WINDOW + 1)?;
        let table = self.table(&rec)?;
        let at_key = |k: &[Value]| rows.iter().position(|r| Self::same_key(&table.primary_key(r), k));
        let at = match at_key(held) {
            Some(i) => i,
            None => {
                let now = self.row_key(&rec)?;
                match at_key(&now) {
                    Some(0) => 0,
                    _ => {
                        return unsupported(
                            "a refill of a temporary Worksheet by a field's OnValidate that left the row the client stood on gone and Rec on a row other than the first of the view (not pinned)",
                        );
                    }
                }
            }
        };
        if at > 0 && rows.len() > Self::WINDOW {
            return unsupported(
                "a refill of a temporary Worksheet by a field's OnValidate that keeps the client on a later row of a view longer than a window (not pinned)",
            );
        }
        self.read_refilled_rows(st, &rows, at)
    }

    /// After an action that refilled the rows (or moved Rec) and left Rec on another row: after the read of the row the client
    /// stood on that comes before every action, the client reads Rec's row, then reads as from the first row of the view
    /// (RmTempRefill.al:323 RM_Action_Recalculate_FromRow23 `R:A23;R:A1;R:A1;R:A2;...;R:A20;R:A1;R:A1;C:A1;`, :367 with
    /// CurrPage.Update(false) the same, :388 from row 2 `R:A2;R:A1;...`, :407 after filter B `R:B2;R:B1;R:B1;R:B2;R:B1;R:B1;C:B1;`).
    /// It shows Rec's row. Rec on a row other than the first of the view, and a row the client stood on that the table no longer
    /// has, are not pinned.
    pub(super) fn reread_after_refill_by_action(&mut self, st: &St, held: &[Value]) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, Self::WINDOW + 1)?;
        let table = self.table(&rec)?;
        let now = self.row_key(&rec)?;
        if rows
            .first()
            .is_none_or(|r| !Self::same_key(&table.primary_key(r), &now))
        {
            return unsupported(
                "an action on a temporary Worksheet that left Rec on a row other than the first of its view (not pinned)",
            );
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        probe.borrow_mut().filters.clear();
        let mut pk = held.to_vec();
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Get, &probe, &mut pk)?, Value::Boolean(true)) {
            return unsupported(
                "an action on a temporary Worksheet after which the row the client stood on is gone (not pinned)",
            );
        }
        self.read_refilled_row(st, &rec, &rows[0])?;
        self.read_refilled_rows(st, &rows, 0)
    }

    /// The reads from row `at` of `rows` (the view from its start), then OnAfterGetCurrRecord on it; the client then holds the
    /// rows it read.
    fn read_refilled_rows(&mut self, st: &St, rows: &[Vec<Value>], at: usize) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let before = self.xrec_before_move(st);
        let window_end = rows.len().min(at + Self::WINDOW);
        let mut held = Vec::new();
        if at > 0 {
            self.read_refilled_row(st, &rec, &rows[at])?;
        }
        for r in &rows[at..window_end] {
            held.push(self.read_refilled_row(st, &rec, r)?);
        }
        if at > 0 {
            if window_end > at + 1 {
                self.read_refilled_row(st, &rec, &rows[at])?;
            }
            let mut above = Vec::new();
            for r in rows[..at].iter().rev() {
                above.push(self.read_refilled_row(st, &rec, r)?);
            }
            above.reverse();
            above.extend(held);
            held = above;
        }
        self.read_refilled_row(st, &rec, &rows[at])?;
        let current = self.read_refilled_row(st, &rec, &rows[at])?;
        let whole = rows.len() <= Self::WINDOW && held.len() == rows.len();
        {
            let mut s = st.borrow_mut();
            if let Some(w) = held.get_mut(at) {
                w.shown = current.shown;
            }
            s.window = held;
            s.window_from_start = true;
            s.window_eof = whole;
            s.window_whole = whole;
            s.view_read_to_end = false;
            s.window_read_up = false;
            s.window_top_unread = false;
            s.opened_on_moved_rec = false;
            s.settled_first_row = false;
            s.refresh = false;
            s.cursor = Cursor::Row;
        }
        self.current_record_trigger(st, before)
    }
}
