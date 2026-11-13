//! A List over a temporary source table whose action deleted the row it stood on while rows are left (al-runner issue 7940).
//! On the probe's read-only list BC lands on the row BEFORE the deleted one, where a list over a stored table lands on the row
//! after it (pages/W1Update.al:577): open corpus PR 1367, testpage/lifecycle/ReopenSameVariableState.al:469
//! TempListRowGone_CloseReopen_DeleteAgain (`cursor-1` R1, `del-1` `[del(R2) agcr(R1) agcr(R1)]`, `next-in-1` R3, the same on the
//! second open). Only that cell runs, under an assumption; every other such list is refused, because the landing main gives
//! it (the row after) is the one the probe showed wrong.

use super::pinned_shape::{PinnedCode, TestCall};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

/// ReopenSameVariableState.al:151 "PG6 FL Temp List": OnOpenPage fills Rec, OnAfterGetCurrRecord and OnClosePage log, fields of
/// Rec in a repeater, one action whose OnAction deletes Rec; `Editable = false`, `SourceTableTemporary = true`; the table has
/// OnModify only (:33). `action_props` lets the action in: it sets ApplicationArea, never an expression.
const CODE: PinnedCode = PinnedCode {
    triggers: &["onopenpage", "onaftergetcurrrecord", "onclosepage"],
    control_triggers: &["onaction"],
    properties: &["editable", "sourcetabletemporary"],
    action_props: &["applicationarea"],
    table_triggers: &["onmodify"],
};

impl Session {
    /// Settles a top-level List over a temporary table after its action deleted the row Rec stands on: on the probe's cell it
    /// lands on the row before and reads it as main's landing does (`arrive_after_gone_row`); `false` when the page is not such
    /// a list or no row is left (main's path); refused in every other shape.
    pub(super) fn settle_gone_row_of_temp_list(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        let object = st.borrow().object as usize;
        let family = self.engine.object_facts[object].page_type == Some(al_ir::PageType::List)
            && rec.borrow().temporary
            && st.borrow().host.is_none();
        if !family {
            return Ok(false);
        }
        let before = self.neighbor_row(rec, -1)?;
        let after = self.neighbor_row(rec, 1)?;
        if before.is_none() && after.is_none() {
            return Ok(false);
        }
        let middle_of_three = before.is_some() && after.is_some() && self.view_row_count(st)? == 2;
        if !(middle_of_three && self.gone_middle_cell(st, rec)?) {
            return unsupported(
                "a list over a temporary table after its action deleted the row it stood on, rows left (measured only for the probe's read-only list, middle row of three)",
            );
        }
        assume(Assumption::TempReadOnlyListGoneMiddleRowLandsBefore);
        if let Some(fields) = before {
            rec.borrow_mut().fields = fields;
        }
        let r = self.arrive_after_gone_row(st);
        self.torn_down_on_error(st, r)?;
        Ok(true)
    }

    /// The fields of the row `step` (1 or -1) from Rec's key in its view, read on a copy of Rec.
    fn neighbor_row(&mut self, rec: &Rec, step: i32) -> Result<Option<Vec<Value>>, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        let moved = matches!(
            self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)])?,
            Value::Integer(n) if n == step
        );
        Ok(moved.then(|| probe.borrow().fields.clone()))
    }

    /// The facts of the probe's cell, each by equality:
    /// - the page's code is `CODE` (`page_code_is`), `Editable = false`, at the top level without parts, opened by the test with
    ///   OpenView, not modal, not a lookup;
    /// - the view is the primary key without filters or marks (`rec_view_is`), and the client held the whole view of three rows
    ///   from its start, Rec's key the middle one;
    /// - the test's calls since the open: `GoToKey`, then the action's `Invoke`, with one control read before the `GoToKey` on the
    ///   second open (:474-477, :488-492).
    fn gone_middle_cell(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        let object = st.borrow().object as usize;
        let opened = {
            let s = st.borrow();
            s.open
                && s.opened_by_test
                && s.mode == PageMode::View
                && !s.modal
                && !s.lookup
                && s.parts.is_empty()
                && s.window_from_start
                && s.window.len() == 3
        };
        if !(opened && flag_denied(&self.engine(), object, "Editable")? && self.rec_view_is(st, 0)) {
            return Ok(false);
        }
        let key = self.row_key(rec)?;
        if !Self::same_key(&st.borrow().window[1].key, &key) {
            return Ok(false);
        }
        let calls = matches!(
            Self::test_calls(st).as_deref(),
            Some(
                [TestCall::Page(PageMethod::GoToKey), TestCall::Other(PageMethod::Invoke)]
                    | [
                        TestCall::Read,
                        TestCall::Page(PageMethod::GoToKey),
                        TestCall::Other(PageMethod::Invoke)
                    ]
            )
        );
        Ok(calls && self.page_code_is(st, &CODE)?)
    }
}
