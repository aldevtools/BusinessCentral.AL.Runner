//! A List that stood on its new-row line over an empty view, after an action stored one row through another record variable (a
//! processing report the action ran): the list is back on its new-row line with the row behind it (`empty_page_after_action`,
//! `PageState::empty_list_row_behind`). The calls that follow (al-runner issue 7478;
//! pages/actions/ActionReportInsertsOneRowOnEmptyList.al, BC 27.5, 28.5 and 29.0):
//! - OK closes the page and inserts nothing; the row stays (:98 OneRow_DelayedInsertList_ReportWithRequestPage_Ok, :204
//!   OneRow_PlainList_ReportWithRequestPage_Ok);
//! - on the DelayedInsert list, Next answers No and the line stays (:128); Previous, Last and GoToKey of the row answer Yes and
//!   show the row (:147, :166, :185); the test then reads the controls and OK closes the page, inserting nothing (:87-:90).
//!
//! Those are the answers of the ordinary calls on such a page, so a pinned call goes on as usual; after it only the calls the
//! pin makes next are let through (`PinnedEnd`). Close stays as before (`pageops.rs`).

use super::pinned_shape::{NextEnd, PinnedEnd, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

const AFTER_CALL_REASON: &str = "a TestPage call after the call that followed an action that stored one row behind the new-row \
     line of an empty list, other than a control's value read and OK (not pinned)";

/// After Next, Previous, Last or GoToKey: control value reads, then OK (:88-:89 `ShownDelayed`, `TryCallDelayed(Pg, 'ok', ..)`).
const AFTER_MOVE: PinnedEnd = PinnedEnd {
    reason: AFTER_CALL_REASON,
    reads: true,
    caption: false,
    part_reads: false,
    one_action: false,
    first_ends: false,
    part_visible: false,
    part_moves: false,
    no_writes: false,
    next: NextEnd::Refused,
    close: false,
    ok: true,
};

impl Session {
    /// Whether `name` on `st`, right after the action that left one row behind the new-row line, is a pinned call, every fact
    /// required; a pinned call sets what may follow it (`PinnedEnd`):
    /// - the cell: OK, Next, Previous, Last, or GoToKey of the row's key on the List with DelayedInsert = true (page 62387 of
    ///   ActionReportInsertsRowsOnEmptyList.al:133-:199; :43-:58 `TryCallDelayed`), or OK on the List without DelayedInsert
    ///   (page 62388, :201; :62-:66 `TryOkPlain`);
    /// - the page: opened by the test with OpenEdit (:78 `Pg.OpenEdit()`), no trigger, no property but the plain ones (and
    ///   DelayedInsert), no extension and no subscriber; its controls are a repeater of fields of Rec without a trigger and
    ///   actions whose only trigger is OnAction (:144-:195); its table runs no code (:22-:37);
    /// - the calls the test made since the open, each answered Yes or nothing: the filter, the action's Invoke, then this call,
    ///   with only control value reads between (:79-:84);
    /// - the test's filter is the one filter of Rec: one value of letters and digits on the first key field (:79
    ///   `SetFilter("Parent No.", 'P1')`);
    /// - the page still stands on the new-row line it went back to, typed into nothing: Rec as the action left it (`rec_left`)
    ///   and as it is now holds the filter's value and the initial value of every other field (:83 `row after action`
    ///   `[P1 0 ]`), and its view holds that one row (:82 `stored after action` `[P1 10000 by report 1]`).
    pub(super) fn one_row_behind_call_pinned(
        &mut self,
        st: &St,
        name: PageMethod,
        args: &[Value],
        rec_left: &[Value],
    ) -> Result<bool, Signal> {
        use PageMethod as M;
        let delayed = self.page_property_true(st, "DelayedInsert");
        let cell = if delayed {
            matches!(name, M::OK | M::Next | M::Previous | M::Last | M::GoToKey)
                && self.page_declares_within(st, &[], &["delayedinsert"])
        } else {
            name == M::OK && self.page_declares_within(st, &[], &[])
        };
        let shape = {
            let s = st.borrow();
            s.open && s.test_open_edit() && s.cursor == Cursor::NewLine && !s.dirty
        };
        let page_type = self.engine().object_facts[st.borrow().object as usize]
            .page_type
            .clone();
        if !cell
            || !shape
            || page_type != Some(ir::PageType::List)
            || !self.has_source(st)
            || !self.controls_are_rec_fields(st, true)
        {
            return Ok(false);
        }
        let table = self.table(&self.page_rec(st))?.object as usize;
        if !self.table_code_is(table, &[])
            || !Self::one_row_calls(st, name)
            || !self.one_row_rec(st, table, rec_left)?
            || self.view_row_count(st)? != 1
        {
            return Ok(false);
        }
        if name == M::GoToKey && !self.go_to_key_of_the_row(st, args)? {
            return Ok(false);
        }
        st.borrow_mut().pinned_end = Some(if name == M::OK {
            PinnedEnd::after_ok(AFTER_CALL_REASON)
        } else {
            AFTER_MOVE
        });
        Ok(true)
    }

    /// The test's calls on `st` since the open, the running one (`name`) last: the filter (`Pg.Filter.SetFilter`), one action's
    /// Invoke, then `name`, each answered Yes or nothing, with only control value reads between.
    fn one_row_calls(st: &St, name: PageMethod) -> bool {
        let Some(calls) = Self::test_calls_all_succeeded(st) else {
            return false;
        };
        let steps: Vec<TestCall> = calls.into_iter().filter(|c| *c != TestCall::Read).collect();
        matches!(steps.as_slice(), [
            TestCall::Page(PageMethod::Filter),
            TestCall::Other(PageMethod::SetFilter),
            TestCall::Other(PageMethod::Invoke),
            TestCall::Page(last),
        ] if *last == name)
    }

    /// Rec of `st` (over table object `table`) has one filter, the test's own on the first key field, typed as one value of
    /// letters and digits; Rec as the action left it (`rec_left`) and as it is now hold that value in the field and every other
    /// field's initial value: the action wrote nothing into Rec.
    fn one_row_rec(&mut self, st: &St, table: usize, rec_left: &[Value]) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let key_field = self.table(&rec)?.keys[0][0];
        let typed = {
            let r = rec.borrow();
            let one_filter = matches!(r.filters.as_slice(), [f] if f.field == key_field && f.group == 0);
            let text = st.borrow().test_typed_filter(&r, key_field).map(str::to_string);
            text.filter(|t| one_filter && !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric()))
        };
        let Some(typed) = typed else {
            return Ok(false);
        };
        let initial = self.initial_fields(table);
        let new_line = |fields: &[Value]| {
            fields.len() == initial.len()
                && fields.iter().zip(&initial).enumerate().all(|(i, (have, init))| {
                    if i == key_field {
                        matches!(have, Value::Code(c) if c.as_str() == typed.to_uppercase())
                    } else {
                        have.identical(init)
                    }
                })
        };
        Ok(new_line(&rec.borrow().fields) && new_line(rec_left))
    }

    /// GoToKey of that row's key (:58 `Pg.GoToKey('P1', 10000)`); another key is not pinned.
    fn go_to_key_of_the_row(&mut self, st: &St, args: &[Value]) -> Result<bool, Signal> {
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
