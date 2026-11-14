//! A List that stood on its new-row line over an empty view, after an action stored one row through another record variable (a
//! processing report the action ran): the list is back on its new-row line with the row behind it (`empty_page_after_action`,
//! `PageState::empty_list_row_behind`). The calls that follow on a page without triggers (al-runner issue 7478;
//! pages/actions/ActionReportInsertsOneRowOnEmptyList.al, BC 27.5, 28.5 and 29.0):
//! - OK closes the page and inserts nothing; the row stays (:98 OneRow_DelayedInsertList_ReportWithRequestPage_Ok, :204
//!   OneRow_PlainList_ReportWithRequestPage_Ok);
//! - Next answers No and the line stays (:128); Previous, Last and GoToKey of the row answer Yes and show the row (:147, :166,
//!   :185); OK after each closes the page and inserts nothing.
//!
//! Those are the answers of the ordinary calls on such a page, so a pinned call goes on as usual. Close stays as before
//! (`pageops.rs`).

use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether `name` on `st`, right after the action that left one row behind the new-row line, is a pinned call, every fact
    /// required:
    /// - the call is OK, Next, Previous, Last, or GoToKey of the row's key (:43-:58 `TryCallDelayed`, :60-:67 `TryOkPlain`);
    /// - the page is a List the test opened with OpenEdit (:75 `Pg.OpenEdit()`), with no trigger, no property but the plain ones
    ///   and DelayedInsert, no extension and no subscriber (pages 62387 and 62388 of ActionReportInsertsRowsOnEmptyList.al: no page,
    ///   row or control trigger, only the actions');
    /// - the page still stands on the new-row line it went back to, typed into nothing (:79 `row after action` `[P1 0 ]`), and its
    ///   view holds that one row (:101 `stored after action` `[P1 10000 by report 1]`).
    pub(super) fn one_row_behind_call_pinned(
        &mut self,
        st: &St,
        name: PageMethod,
        args: &[Value],
    ) -> Result<bool, Signal> {
        use PageMethod as M;
        if !matches!(name, M::OK | M::Next | M::Previous | M::Last | M::GoToKey) {
            return Ok(false);
        }
        let shape = {
            let s = st.borrow();
            s.open && s.test_open_edit() && s.cursor == Cursor::NewLine && !s.dirty
        };
        let page_type = self.engine().object_facts[st.borrow().object as usize]
            .page_type
            .clone();
        let page_ok = page_type == Some(ir::PageType::List) && self.page_declares_within(st, &[], &["delayedinsert"]);
        if !shape || !page_ok || self.view_row_count(st)? != 1 {
            return Ok(false);
        }
        if name != M::GoToKey {
            return Ok(true);
        }
        // GoToKey of that row's key (:58 `Pg.GoToKey('P1', 10000)`); another key is not pinned.
        let rec = self.page_rec(st);
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            return Ok(false);
        }
        let key = self.row_key(&probe)?;
        Ok(Self::same_key(&key, args))
    }
}
