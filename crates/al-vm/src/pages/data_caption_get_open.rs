//! The DataCaptionExpression of a Worksheet that AL code opened with `Page.Run(Id, Rec)` and a PageHandler, `Page.RunModal(Id,
//! Rec)` as a statement and a ModalPageHandler, or `Page.Run(Id, Rec)` after a TestPage Trap, over a view of up to three stored
//! rows (al-runner issue 6924). Measured by corpus PR 1401, pages/properties/DataCaptionGetOpen.al (merged, not yet in the pinned
//! corpus); the cells run under the assumption `data-caption-get-open`. The cells of DataCaptionExprWorksheet.al
//! (`data_caption_open.rs`) are tried first; a page in neither family keeps the refusal or main's path there.

use super::data_caption_open::{HandedRecord, Shape};
use super::pinned_shape::{NextEnd, PinnedEnd};
use super::*;
use crate::ui::UiKind;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::record_op::RecordOp;

/// The most rows a measured view had: None, rows A B C (DataCaptionGetOpen.al:9, :338 `runs` 3).
const MOST_ROWS: usize = 3;

/// What the handler or the Trap may do after the open: read the caption and the controls, and close (DataCaptionGetOpen.al:1192-1193
/// in the handler, :376-378 after the Trap).
const GET_OPENED_END: PinnedEnd = PinnedEnd {
    reason: "a call of the test on a page whose DataCaptionExpression ran at the open as DataCaptionGetOpen.al of corpus PR 1401 \
             measured, other than Caption, a control's value read or Close (not pinned)",
    reads: true,
    caption: true,
    part_reads: false,
    one_action: false,
    first_ends: false,
    part_visible: false,
    part_moves: false,
    no_writes: false,
    next: NextEnd::Refused,
    close: true,
    ok: false,
    walk: false,
};

/// The start of a refusal of a page in this family's cells whose expression did what no cell measured.
const UNMEASURED_RUN: &str = "a Worksheet with DataCaptionExpression that AL code opened with a handler or a Trap, in the shape \
                              DataCaptionGetOpen.al of corpus PR 1401 measured";

/// A SetRange to one Code value: `Some(true)` when the value is not empty.
fn code_value(f: &FieldFilter) -> Option<bool> {
    match &f.expr {
        FilterExpr::Range(Some(Value::Code(a)), Some(Value::Code(b))) if a == b => Some(!a.is_empty()),
        _ => None,
    }
}

/// A range of two different Code values (:307 `SetFilter("Code", 'A..B')`).
fn code_range(f: &FieldFilter) -> bool {
    matches!(&f.expr, FilterExpr::Range(Some(Value::Code(a)), Some(Value::Code(b))) if a != b)
}

/// An Integer range: `Some(true)` when both ends are the same value (:311 `SetRange(Seq, 2)`; :304 `SetRange(Seq, 1, 3)`).
fn integer_range(f: &FieldFilter) -> Option<bool> {
    match &f.expr {
        FilterExpr::Range(Some(Value::Integer(a)), Some(Value::Integer(b))) => Some(a == b),
        _ => None,
    }
}

/// The number of rows the view had in the cell of the handed record's `filters`, all of filter group 0 (Seed, :269-312, rows A
/// Seq 1 Kind X, B Seq 2 Kind Y, C Seq 3 Kind X): None, no filter, 3 rows (:9); Two, a SetRange to one Code value that is not
/// empty and an Integer range on another field, 2 rows (:302-305); Range, a range of two Code values, 2 rows (:306-307);
/// EmptyCode, a SetRange to the empty Code value, no row (:308-309); IntRange, a SetRange to one Integer value, 1 row (:310-311).
/// `None`: the filters are in no cell.
fn measured_rows(filters: &[FieldFilter]) -> Option<usize> {
    if filters.iter().any(|f| f.group != 0) {
        return None;
    }
    match filters {
        [] => Some(3),
        [f] if code_range(f) => Some(2),
        [f] if code_value(f) == Some(false) => Some(0),
        [f] if integer_range(f) == Some(true) => Some(1),
        [a, b] if a.field != b.field => {
            let two = |c: &FieldFilter, i: &FieldFilter| code_value(c) == Some(true) && integer_range(i) == Some(false);
            (two(a, b) || two(b, a)).then_some(2)
        }
        _ => None,
    }
}

impl Session {
    /// The page `st` that AL code opened with `bind`'s handler bound, or that a Trap caught (`bind` empty), and the record
    /// `handed`, in a cell of DataCaptionGetOpen.al (`data_caption_get_rows`): the expression runs as the probe saw it before the
    /// handler, and only the caption and control reads and the close follow (`GET_OPENED_END`). `false`: not in a cell, the caller
    /// goes on. The expression's run raising an error refuses: BC raised the error at the open as a wrapped error (:653-654), which
    /// is not modelled.
    pub(super) fn data_caption_get_open(
        &mut self,
        st: &St,
        bind: Option<UiKind>,
        handed: Option<&HandedRecord>,
    ) -> Result<bool, Signal> {
        let Some(handed) = handed else {
            return Ok(false);
        };
        let Some(rows) = self.data_caption_get_rows(st, bind, handed)? else {
            return Ok(false);
        };
        assume(Assumption::DataCaptionGetOpen);
        let ran = self.data_caption_get_runs(st, &rows);
        if let Err(Signal::Error(_)) = ran {
            return unsupported(format!(
                "{UNMEASURED_RUN}: the expression raised an error at the open (BC shows it as a wrapped error, not modelled; \
                 al-runner issue 6924)"
            ));
        }
        ran?;
        let mut s = st.borrow_mut();
        s.data_caption_at_open = true;
        s.pinned_end = Some(GET_OPENED_END);
        Ok(true)
    }

    /// The runs of the expression before the handler (DataCaptionGetOpen.al): on an empty view twice, and the handler reads the
    /// globals it assigned (:526 `runs` 2, `g=cap-ran`; :1109 `DC=none` twice); on a view of rows once per row in key order with
    /// Rec on that row, and the handler reads the globals as they were before (:337-338 three runs, `g=`; :977 `DC=Alpha`, `DC=Beta`,
    /// `DC=Gamma`; :463 two; :589 one). The caption shows the value of the run on the first row (:977 `cap=DCG WS Own - Alpha`).
    fn data_caption_get_runs(&mut self, st: &St, rows: &[Vec<Value>]) -> Result<(), Signal> {
        if rows.is_empty() {
            self.data_caption_expression_value(st)?;
            let value = self.data_caption_expression_value(st)?;
            st.borrow_mut().data_caption_value = Some(value);
            return Ok(());
        }
        let rec = self.page_rec(st);
        let shown = rec.borrow().fields.clone();
        let mut first = None;
        let mut ran = Ok(());
        for row in rows {
            rec.borrow_mut().fields = row.clone();
            ran = self.run_data_caption_held_back(st);
            if ran.is_err() {
                break;
            }
            if first.is_none() {
                first = st.borrow().data_caption_value.clone();
            }
        }
        rec.borrow_mut().fields = shown;
        ran?;
        st.borrow_mut().data_caption_value = first;
        Ok(())
    }

    /// The rows of the view when page `st` is in a measured cell of DataCaptionGetOpen.al, every fact required as it has them:
    /// - the page is one main refused at the open: its expression can raise an error or leave something behind
    ///   (`data_caption_expression_risk`); a page whose expression can do neither keeps main's open;
    /// - the run: a PageHandler bound to AL code's static `Page.Run(Id, Rec)` (:334), a ModalPageHandler to `Page.RunModal(Id,
    ///   Rec)` with the page's id, as a statement (:353), or `Page.Run(Id, Rec)` after the test's `Trap` (:373-374); a top-level
    ///   page, open, not opened by the test; the record as `data_caption_handed` requires;
    /// - the page: a Worksheet without DelayedInsert (:56, :108, :157), its own DataCaptionExpression calling a procedure of the
    ///   page with no argument (:59), no property but the plain ones and DataCaptionExpression, the trigger OnOpenPage and no other
    ///   (:100), no extension, no subscriber; controls bound to fields of Rec or to page variables, no control trigger (:64-75);
    /// - the expression and what it calls do only what the measured bodies do, and under RunModal do not read Rec
    ///   (`data_caption_body_in_cells`);
    /// - what OnOpenPage did (`data_caption_open_left`, editable: :100-103 sets nothing); the expression has not run;
    /// - the handed record's filters and the number of rows in the view as one cell has them (`measured_rows`).
    fn data_caption_get_rows(
        &mut self,
        st: &St,
        bind: Option<UiKind>,
        handed: &HandedRecord,
    ) -> Result<Option<Vec<Vec<Value>>>, Signal> {
        let (run, modal) = {
            let s = st.borrow();
            let top = !s.opened_by_test && s.host.is_none() && s.open && !s.lookup;
            let run = match (bind, s.modal) {
                (Some(UiKind::Page), false) => true,
                (Some(UiKind::ModalPage), true) => s.statement_run && !s.lookup_page_from_table,
                (None, false) => s.caught_by_trap,
                _ => false,
            };
            (top && run, s.modal)
        };
        let Some(want) = measured_rows(&handed.filters) else {
            return Ok(None);
        };
        if !run || self.data_caption_expression_risk(st).is_none() {
            return Ok(None);
        }
        let open_page_only = {
            let engine = self.engine();
            matches!(page_of(&engine, st).triggers.as_slice(), [(n, _)] if n == "onopenpage")
        };
        if !open_page_only
            || self.data_caption_open_shape(st) != Ok(Shape::Worksheet)
            || !self.page_declares_within(st, &["onopenpage"], &["datacaptionexpression"])
        {
            return Ok(None);
        }
        let Some(routine) = self.data_caption_routine(st) else {
            return Ok(None);
        };
        if self.data_caption_body_in_cells(st, routine, modal).is_err() {
            return Ok(None);
        }
        if self.data_caption_open_left(st, handed) != Ok(true) {
            return Ok(None);
        }
        let not_run = {
            let s = st.borrow();
            !s.data_caption_due && s.data_caption_value.is_none() && s.data_caption_pending.is_empty()
        };
        if !not_run {
            return Ok(None);
        }
        let rows = self.handed_view_rows(st, handed)?;
        Ok((rows.len() == want).then_some(rows))
    }

    /// The fields of the rows of page `st`'s table that pass the handed record's filters, in key order; at most one more than
    /// `MOST_ROWS`.
    fn handed_view_rows(&mut self, st: &St, handed: &HandedRecord) -> Result<Vec<Vec<Value>>, Signal> {
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let mut p = probe.borrow_mut();
            p.filters = handed.filters.clone();
            p.marks.clear();
            p.marked_only = false;
        }
        let mut rows = Vec::new();
        self.ret_used = true;
        let mut found = self.record_op(RecordOp::FindSet, &probe, &mut [])?;
        while matches!(found, Value::Boolean(true)) && rows.len() <= MOST_ROWS {
            rows.push(probe.borrow().fields.clone());
            self.ret_used = true;
            found = match self.record_op(RecordOp::Next, &probe, &mut [])? {
                Value::Integer(n) => Value::Boolean(n != 0),
                other => other,
            };
        }
        Ok(rows)
    }
}
