//! A key typed into a stored row of a temporary List that `CurrPage.Run()` shows (al-runner issue 7443; corpus
//! pages/run-modal/PageVarOpenRow.al).

use super::*;
use al_builtins::record_op::RecordOp;

/// The page triggers of the pinned page (PageVarOpenRow.al:221 page 63425), and nothing else may declare one.
const PIN_TRIGGERS: [&str; 5] = [
    "onopenpage",
    "onnewrecord",
    "onaftergetcurrrecord",
    "oninsertrecord",
    "onclosepage",
];

impl Session {
    /// The page's save of a row whose key the test typed found no stored row of the new key. BC writes nothing, raises nothing
    /// and the page goes on from the row it held: the handler types `T1` into the key of the preloaded row P2, `Next()` answers
    /// Yes onto the new-row line, and the page's Rec still holds P1 and P2 as they were (PageVarOpenRow.al:788
    /// TempPick_TwoRowsPreloaded_CurrPageRun_HandlerTypesRow `typed, next` Yes, `log after typing` `N C: `, `page rec after`
    /// `on : P1=preloaded;P2=preloaded;`). True when the save is dropped: Rec is back on the row the page held, with nothing to
    /// save. Only on the pinned shape (`typed_key_drop_pinned`); every other shape refuses as before.
    pub(super) fn typed_key_dropped_on_currpage_run(&mut self, st: &St) -> Result<bool, Signal> {
        if !self.typed_key_drop_pinned(st)? {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        rec.borrow_mut().fields = self.page_xrec_fields(st);
        st.borrow_mut().dirty = false;
        Ok(true)
    }

    /// The entry condition, all of it measured by PageVarOpenRow.al:788 (handler `TryTypeTemp` at :703: the key typed, then
    /// `Next()`):
    /// - the page: a List over a temporary source, run by `CurrPage.Run()` (not modal), on its own; its page triggers exactly
    ///   those of page 63425 (`PIN_TRIGGERS`), no page extension trigger, no subscriber of its record events;
    /// - the call: the save is the one `TestPage.Next()` asks for;
    /// - the row: only key fields differ from the row the page held, that row is still stored, and it is the last row of the
    ///   view (the `Next` goes on to the new-row line).
    fn typed_key_drop_pinned(&mut self, st: &St) -> Result<bool, Signal> {
        const EVENTS: [&str; 5] = [
            "onaftergetrecordevent",
            "onaftergetcurrrecordevent",
            "onnewrecordevent",
            "oninsertrecordevent",
            "onmodifyrecordevent",
        ];
        let rec = self.page_rec(st);
        let page_shape = {
            let s = st.borrow();
            s.run_by_currpage && !s.modal && s.host.is_none() && s.next_running
        } && self.is_list_page(st)
            && rec.borrow().temporary
            && {
                let engine = self.engine();
                let page = super::page_of(&engine, st);
                let object = st.borrow().object as usize;
                page.triggers.len() == PIN_TRIGGERS.len()
                    && page.triggers.iter().all(|(n, _)| PIN_TRIGGERS.contains(&n.as_str()))
                    && page.ext_triggers.is_empty()
                    && EVENTS.iter().all(|e| !engine.events.is_subscribed(object, e))
            };
        if !page_shape {
            return Ok(false);
        }
        let held = self.page_xrec_fields(st);
        let table = self.table(&rec)?;
        let pk = &table.keys[0];
        let now = rec.borrow().fields.clone();
        let only_key_typed = held.len() == now.len()
            && !Self::same_key(&table.primary_key(&held), &table.primary_key(&now))
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
