//! A guarded `Codeunit.Run` from a page action whose record argument is the Rec of a List or Document page, or a
//! `Record.Copy` of it, with the page's SourceTableView filter in filter group 2: the Ends and PlainCommit cells of corpus PR
//! 1622 (errors/ConsistentGuardedRunPageConst.al, codeunit 99562, :690-:1200; not yet in the pinned corpus), run through the
//! assumption `guarded-run-consistent-page-view`. Owns the page's view mark on `RecordState` (`PageViewMark`): where it is set,
//! the record methods that end it, and the code a test page's Invoke gives the error the run end raises. Every other shape
//! gets main's result.
//!
//! Entry points: `Session::mark_page_view` (by the page open), `page_view_ended` (by `record_op_inner`),
//! `Session::page_view_run_end` (by `Session::guarded_run_end_marks`), `Session::page_view_action_code` (by the test page's
//! action invoke).

use super::page_args::PageArgShape;
use super::{ArgFilter, MarkState, RunArg, RunEnd, RunStart};
use crate::Session;
use crate::pages::{PageState, page_of};
use crate::records::Rec;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::FieldFilter;
use al_builtins::native::{AlError, ExceptionKind, Signal};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{PageView, PageViewMark, RecordState, RtType};
use std::cell::RefCell;
use std::rc::Rc;

/// The code a test page's Invoke gives the error (`TestWrapped:CSide`, the Ends cells :690-:1185).
const ACTION_CODE: &str = "TestWrapped:CSide";

impl Session {
    /// The page object is a List or a Document page (the cells' pages, :88-:360) that no page extension extends.
    fn page_view_page(&self, object: usize) -> bool {
        let list_or_document = matches!(
            self.engine.object_facts[object].page_type,
            Some(al_ir::PageType::List | al_ir::PageType::Document)
        );
        let page = self.engine.program.objects[object].page.as_ref();
        list_or_document && page.is_some_and(|p| p.extensions.is_empty())
    }

    /// The page just applied its SourceTableView to `rec` (`page_open`): marks it with its group-2 filters when the test
    /// opened the page with OpenView (every cell, :588-:638), without a record or a sub-page view, and the page is measured.
    /// `opened_without_record` is whether the test opened the page itself, without a record or a sub-page view.
    /// Every open takes a new open id, so a mark of an earlier open never matches the page again.
    pub(crate) fn mark_page_view(&mut self, st: &Rc<RefCell<PageState>>, opened_without_record: bool, rec: &Rec) {
        self.pages.page_opens += 1;
        let open_id = self.pages.page_opens;
        st.borrow_mut().open_id = open_id;
        let object = st.borrow().object as usize;
        if !(opened_without_record && st.borrow().test_open_view() && self.page_view_page(object)) {
            return;
        }
        let mut r = rec.borrow_mut();
        let group2 = group2(&r);
        r.page_view = PageViewMark(Some(Rc::new(PageView {
            page: object as u32,
            open_id,
            group2,
        })));
    }

    /// The end of a guarded run in the cells' shape: `Some` with the result when the shape is measured, `None` to keep
    /// main's result. `top_level` is whether no other guarded run runs; `no_commit` whether nothing committed since the start.
    pub(super) fn page_view_run_end(
        &mut self,
        start: &RunStart,
        end: RunEnd,
        after: &[(u32, MarkState)],
        top_level: bool,
        no_commit: bool,
    ) -> Option<Option<Signal>> {
        let RunArg::Record {
            table,
            temporary: false,
            filter: ArgFilter::Page(PageArgShape::PageView),
        } = start.arg
        else {
            return None;
        };
        let arg = start.arg_record?;
        let run_shape = top_level && no_commit && start.marks.is_empty() && !start.runs_inside && arg.filters_kept();
        // One mark on another table (GPC M), held by the database variable that set it, which is not the argument.
        let one_mark = match (after, self.tx.open.inconsistent.as_slice()) {
            ([(t, MarkState::Marked)], [m]) => {
                let by_arg = m.by.upgrade().is_some_and(|b| Rc::ptr_eq(&b, &arg.rec));
                *t == m.table && *t != table && m.by_database && !by_arg
            }
            _ => false,
        };
        let mark = arg.rec.borrow().page_view.0.clone()?;
        let now = group2(&arg.rec.borrow());
        let view_kept = mark.group2.len() == now.len() && mark.group2.iter().zip(&now).all(|(a, b)| same_filter(a, b));
        let in_action = self.run_by_action_method(mark.open_id, start.caller_frame, table)
            && self.page_view_page(mark.page as usize);
        if !(run_shape && one_mark && view_kept && in_action) {
            return None;
        }
        match end {
            // Ends :690: the run does not return; the error names GPC M.
            RunEnd::Succeeded => {
                assume(Assumption::GuardedRunConsistentPageView);
                let mut e = self.inconsistent_error(after[0].0 as usize);
                if let Signal::Error(err) = &mut e {
                    err.exception = ExceptionKind::ConsistentPageViewEnd;
                }
                Some(Some(e))
            }
            // PlainCommit :705: the run returns false with the Commit's error (code DB:TableInConsistent).
            RunEnd::Failed("DB:TableInConsistent") => {
                assume(Assumption::GuardedRunConsistentPageView);
                Some(None)
            }
            _ => None,
        }
    }

    /// The cells' call path (:117-:121, :464-:486): the OnAction trigger of the page open `open_id` calls a codeunit method
    /// with no attribute and one parameter, a `var` Record of the run argument's table `table` (`RunCopy(var PageRec:
    /// Record "GPC Z")` :466), and that method calls `Codeunit.Run` (its frame is `caller_frame - 1`).
    fn run_by_action_method(&self, open_id: u64, caller_frame: usize, table: u32) -> bool {
        let Some(action) = self.page_action_frame(open_id) else {
            return false;
        };
        let method = action + 1;
        let (Some(&object), Some(call)) = (self.frames.get(method), self.call_stack.get(method)) else {
            return false;
        };
        let object = &self.engine.program.objects[object];
        let routine = &object.routines[call.routine];
        caller_frame == method + 1
            && self.call_stack.len() == self.frames.len()
            && object.kind == al_ir::ObjKind::Codeunit
            && !call.event
            && !routine.is_trigger
            && routine.attributes.is_empty()
            && routine.by_var == [true]
            && routine
                .locals
                .first()
                .is_some_and(|p| p.ty == RtType::Record(table, false))
    }

    /// The code a test page's Invoke gives `e` from action `idx`: `TestWrapped:CSide` for the error a page view run end
    /// raised from an action of the page itself (Ends :690-:1185), `fallback`'s code (main's) for every other error.
    pub(crate) fn page_view_action_code(
        &self,
        st: &Rc<RefCell<PageState>>,
        idx: usize,
        e: &AlError,
        fallback: fn(&str, &str) -> String,
    ) -> String {
        let object = st.borrow().object as usize;
        let own_action = page_of(&self.engine, st).controls[idx].owner.is_none();
        let raised = e.exception == ExceptionKind::ConsistentPageViewEnd && e.code == "DB:TableInConsistent";
        if raised && own_action && self.page_view_page(object) {
            return ACTION_CODE.into();
        }
        fallback(&e.code, &e.message)
    }
}

/// Whether the record method ends the page's view mark: Reset, SetView, Clear, CopyFilters, and a SetRange or SetFilter in
/// filter group 2 (`filter_group` is the record's current filter group).
pub(crate) fn page_view_ended(method: RecordOp, filter_group: i32) -> bool {
    match method {
        RecordOp::Reset | RecordOp::SetView | RecordOp::Clear | RecordOp::CopyFilters => true,
        RecordOp::SetRange | RecordOp::SetFilter => filter_group == 2,
        _ => false,
    }
}

/// The record's filters in filter group 2.
fn group2(r: &RecordState) -> Vec<FieldFilter> {
    r.filters.iter().filter(|f| f.group == 2).cloned().collect()
}

/// The same filter: field, group and expression, values by identity.
fn same_filter(a: &FieldFilter, b: &FieldFilter) -> bool {
    a.field == b.field && a.group == b.group && a.expr.identical(&b.expr)
}
