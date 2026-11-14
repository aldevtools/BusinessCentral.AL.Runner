//! The DataCaptionExpression of a Worksheet that AL code opened with `Page.Run(Id, Rec)` and a PageHandler, `Page.RunModal(Id,
//! Rec)` as a statement and a ModalPageHandler, or `Page.Run(Id, Rec)` after a TestPage Trap, over a view of up to three stored
//! rows (al-runner issue 6924). Measured by corpus PR 1401, pages/properties/DataCaptionGetOpen.al (merged, not yet in the pinned
//! corpus); the cells run under the assumption `data-caption-get-open`. The cells of DataCaptionExprWorksheet.al
//! (`data_caption_open.rs`) are tried first; a page in neither family keeps the refusal or main's path there.

use super::data_caption_open::{HandedRecord, OPENED_END, Shape};
use super::*;
use crate::ui::UiKind;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::record_op::RecordOp;

/// The most rows a measured view had: None, rows A B C (DataCaptionGetOpen.al:9, :338 `runs` 3).
const MOST_ROWS: usize = 3;

/// The start of a refusal of a page in this family's cells whose expression did what no cell measured.
const UNMEASURED_RUN: &str = "a Worksheet with DataCaptionExpression that AL code opened with a handler or a Trap, in the shape \
                              DataCaptionGetOpen.al of corpus PR 1401 measured";

/// A filter of the handed record in a measured form: a range of two Code values (SetRange to one value, also the empty one,
/// DataCaptionGetOpen.al:303, :309; SetFilter 'A..B', :307) or of two Integer values (SetRange Seq 2, :311; SetRange Seq 1..3,
/// :304), in filter group 0.
fn measured_filter(f: &FieldFilter) -> bool {
    f.group == 0
        && matches!(
            &f.expr,
            FilterExpr::Range(Some(Value::Code(_)), Some(Value::Code(_)))
                | FilterExpr::Range(Some(Value::Integer(_)), Some(Value::Integer(_)))
        )
}

/// A SetRange to one Code value that is not empty (:303 `SetRange(Kind, 'X')`).
fn one_code_value(f: &FieldFilter) -> bool {
    matches!(&f.expr, FilterExpr::Range(Some(Value::Code(a)), Some(Value::Code(b))) if a == b && !a.is_empty())
}

/// An Integer range (:304 `SetRange(Seq, 1, 3)`).
fn integer_range(f: &FieldFilter) -> bool {
    matches!(
        &f.expr,
        FilterExpr::Range(Some(Value::Integer(_)), Some(Value::Integer(_)))
    )
}

/// The handed record's filters in a measured combination: none (:9, `None`), one filter in a measured form (:307, :309, :311), or
/// the two of Two: a SetRange to one Code value and an Integer range on another field (:302-305).
fn measured_filters(filters: &[FieldFilter]) -> bool {
    match filters {
        [] => true,
        [f] => measured_filter(f),
        [a, b] => {
            a.field != b.field
                && a.group == 0
                && b.group == 0
                && ((one_code_value(a) && integer_range(b)) || (one_code_value(b) && integer_range(a)))
        }
        _ => false,
    }
}

impl Session {
    /// The page `st` that AL code opened with `bind`'s handler bound, or that a Trap caught (`bind` empty), and the record
    /// `handed`, in a cell of DataCaptionGetOpen.al (`data_caption_get_rows`): the expression runs as the probe saw it before the
    /// handler, and only the caption and control reads and the close follow (`OPENED_END`). `false`: not in a cell, the caller
    /// goes on. The expression's run raising an error or writing to the database refuses: BC raised the error at the open as
    /// a wrapped error (:653-654), which is not modelled, and no cell wrote.
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
        if self.store.change_counter() != handed.writes {
            return unsupported(format!(
                "{UNMEASURED_RUN}: the expression wrote to the database at the open (no measured cell writes; al-runner issue 6924)"
            ));
        }
        let mut s = st.borrow_mut();
        s.data_caption_at_open = true;
        s.pinned_end = Some(OPENED_END);
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
    /// - the run: a PageHandler bound to AL code's static `Page.Run(Id, Rec)` (:334), a ModalPageHandler to `Page.RunModal(Id,
    ///   Rec)` with the page's id, as a statement (:353), or `Page.Run(Id, Rec)` after the test's `Trap` (:373-374); a top-level page, open, not
    ///   opened by the test; the record as `data_caption_handed` requires, its filters in a measured combination
    ///   (`measured_filters`);
    /// - the page: a Worksheet without DelayedInsert (:56, :108, :157), its own DataCaptionExpression calling a procedure of the
    ///   page with no argument (:59), no property but the plain ones and DataCaptionExpression, the trigger OnOpenPage only
    ///   (:100), no extension, no subscriber; controls bound to fields of Rec or to page variables, no control trigger (:64-75);
    /// - what OnOpenPage did (`data_caption_open_left`, editable: :100-103 sets nothing); the expression has not run;
    /// - 0 to 3 rows in the view (:9-10).
    fn data_caption_get_rows(
        &mut self,
        st: &St,
        bind: Option<UiKind>,
        handed: &HandedRecord,
    ) -> Result<Option<Vec<Vec<Value>>>, Signal> {
        let run = {
            let s = st.borrow();
            let top = !s.opened_by_test && s.host.is_none() && s.open && !s.lookup;
            let run = match (bind, s.modal) {
                (Some(UiKind::Page), false) => true,
                (Some(UiKind::ModalPage), true) => s.statement_run && !s.lookup_page_from_table,
                (None, false) => s.caught_by_trap,
                _ => false,
            };
            top && run
        };
        if !run || !measured_filters(&handed.filters) {
            return Ok(None);
        }
        if self.data_caption_open_shape(st) != Ok(Shape::Worksheet)
            || !self.page_declares_within(st, &["onopenpage"], &["datacaptionexpression"])
        {
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
        Ok((rows.len() <= MOST_ROWS).then_some(rows))
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
