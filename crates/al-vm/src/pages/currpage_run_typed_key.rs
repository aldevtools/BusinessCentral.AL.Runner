//! A key typed into a stored row of a temporary List that `CurrPage.Run()` shows (al-runner issue 7443; corpus
//! pages/run-modal/PageVarOpenRow.al).

use super::pinned_shape::{PinnedCode, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

/// The code of the pinned page (PageVarOpenRow.al:221 page 63425) and its table (:22, no code).
const CODE: PinnedCode = PinnedCode {
    triggers: &[
        "onopenpage",
        "onnewrecord",
        "onaftergetcurrrecord",
        "oninsertrecord",
        "onclosepage",
    ],
    control_triggers: &[],
    properties: &["sourcetabletemporary"],
    action_props: None,
    part_props: None,
    table_triggers: &[],
};

impl Session {
    /// The page's save of a row whose key the test typed found no stored row of the new key. BC writes nothing, raises nothing
    /// and the page goes on from the row it held: the handler types `T1` into the key of the preloaded row P2, `Next()` answers
    /// Yes onto the new-row line, and the page's Rec still holds P1 and P2 as they were (PageVarOpenRow.al:788
    /// TempPick_TwoRowsPreloaded_CurrPageRun_HandlerTypesRow `typed, next` Yes, `log after typing` `N C: `, `page rec after`
    /// `on : P1=preloaded;P2=preloaded;`). True when the save is dropped: Rec is back on the row the page held, with nothing to
    /// save. Only on the pinned scenario (`typed_key_drop_pinned`); every other shape refuses as before.
    pub(super) fn typed_key_dropped_on_currpage_run(&mut self, st: &St) -> Result<bool, Signal> {
        if !self.typed_key_drop_pinned(st)? {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        rec.borrow_mut().fields = self.page_xrec_fields(st);
        let mut s = st.borrow_mut();
        s.dirty = false;
        s.typed_key_dropped = true;
        Ok(true)
    }

    /// The pin ends with the `Next` that dropped the key (the handler returns, :841): a later call of the test on that page,
    /// other than a read of a control's value, is not pinned.
    pub(super) fn refuse_call_after_typed_key_drop(st: &St) -> Result<(), Signal> {
        let s = st.borrow();
        if s.typed_key_dropped && s.test_calls.last() != Some(&TestCall::Read) {
            return unsupported(
                "a call of the test on a list run by CurrPage.Run() after the page dropped a key typed into a stored row (not pinned)",
            );
        }
        Ok(())
    }

    /// The entry condition. Every fact is what PageVarOpenRow.al:788 has (handler `TempPickHandler` at :832, `TryTypeTemp` at
    /// :703), and all of them are required:
    /// - the page (page 63425, :221): PageType List, a temporary source without code, in its primary key (one field),
    ///   ascending, no filter, no marks; its only code the five page triggers of `CODE`; a repeater of fields of Rec; no part,
    ///   no action, no page extension, no subscriber (`page_code_is`); on its own;
    /// - how it opened: `CurrPage.Run()` in a procedure of the page (and no other page of its object lives, so the caller is
    ///   this page), not modal, not in lookup mode; a procedure of the page
    ///   inserted its 2 rows before, and OnOpenPage left Rec on the one inserted last (`onopenpage_kept_rec`);
    /// - what the test did since the open: read control values, typed one value, into the control of the key field, and
    ///   called `Next`, which asks for this save;
    /// - the row: the key typed is not blank, only the key differs from the row the page held, that row is still stored and
    ///   it is the last of the view (the `Next` goes on to the new-row line).
    fn typed_key_drop_pinned(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let pk = &table.keys[0];
        let calls = match Self::test_calls(st).as_deref() {
            Some([before @ .., TestCall::Write(control), TestCall::Page(PageMethod::Next)]) => {
                let engine = self.engine();
                before.iter().all(|c| *c == TestCall::Read)
                    && pk.len() == 1
                    && super::page_of(&engine, st).controls[*control].binding == ir::Binding::Field(pk[0])
            }
            _ => false,
        };
        let opened = {
            let s = st.borrow();
            s.run_by_currpage
                && !s.modal
                && !s.lookup
                && s.host.is_none()
                && s.onopenpage_kept_rec
                && !s.typed_key_dropped
        };
        let shape = calls
            && opened
            && self.is_list_page(st)
            && self.only_live_page_of_its_object(st)
            && rec.borrow().temporary
            && self.rec_view_is(st, 0)
            && self.page_code_is(st, &CODE)?
            && self.view_row_count(st)? == 2;
        if !shape {
            return Ok(false);
        }
        let held = self.page_xrec_fields(st);
        let now = rec.borrow().fields.clone();
        let blank = self.initial_fields(rec.borrow().table as usize);
        let only_key_typed = held.len() == now.len()
            && !Self::same_key(&table.primary_key(&held), &table.primary_key(&now))
            && !Self::same_key(&table.primary_key(&blank), &table.primary_key(&now))
            && (0..now.len()).all(|i| pk.contains(&i) || Self::same_values(&now[i..=i], &held[i..=i]));
        if !only_key_typed {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        probe.borrow_mut().fields = held;
        if self.stored_row(&probe)?.is_none() {
            return Ok(false);
        }
        self.ret_used = true;
        let steps = self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?;
        Ok(matches!(steps, Value::Integer(0)))
    }
}
