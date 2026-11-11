//! A key typed into a stored row of a temporary List that `CurrPage.Run()` shows (al-runner issue 7443; corpus
//! pages/run-modal/PageVarOpenRow.al).

use super::*;

impl Session {
    /// The page's save of a row whose key the test typed found no stored row of the new key. On a temporary List that
    /// `CurrPage.Run()` shows, BC writes nothing, raises nothing and the page goes on from the row it held: the handler types `T1`
    /// into the key of the preloaded row P2, `Next()` answers Yes onto the new-row line, and the page's Rec still holds P1 and P2
    /// as they were (PageVarOpenRow.al:788 TempPick_TwoRowsPreloaded_CurrPageRun_HandlerTypesRow `typed, next` Yes,
    /// `log after typing` `N C: `, `page rec after` `on : P1=preloaded;P2=preloaded;`). Pinned for that page's shape (page
    /// 63425, :221): a List over a temporary source, run by `CurrPage.Run()` (not modal), on its own, no OnModifyRecord. True
    /// when the save is dropped: Rec is back on the row the page held, with nothing to save.
    pub(super) fn typed_key_dropped_on_currpage_run(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let shape = {
            let s = st.borrow();
            s.run_by_currpage && !s.modal && s.host.is_none()
        } && self.is_list_page(st)
            && rec.borrow().temporary
            && !self.declares_or_ext_declares(st, "onmodifyrecord");
        if !shape {
            return Ok(false);
        }
        let held = self.page_xrec_fields(st);
        let table = self.table(&rec)?;
        if Self::same_key(&table.primary_key(&held), &table.primary_key(&rec.borrow().fields)) {
            return Ok(false);
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        probe.borrow_mut().fields = held.clone();
        if self.stored_row(&probe)?.is_none() {
            return Ok(false);
        }
        rec.borrow_mut().fields = held;
        st.borrow_mut().dirty = false;
        Ok(true)
    }
}
