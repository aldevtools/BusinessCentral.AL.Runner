//! A Page variable over a temporary source after its non-modal `Run()` (al-runner issue 7443, 7420).

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// After a Page variable's `Run()` of a page over a temporary source, the variable's Rec holds no row and is blank: the rows
    /// its procedure put there before the run and the row the handler typed are gone (pages/run-modal/PageVarOpenRow.al:738
    /// TempPick_Empty_VariableRun_HandlerTypesRow `page rec after` = `on : `, :929 TempWs_SecondaryKeyView_FilledBeforeRun_Run,
    /// :985 TempWs_SecondaryKeyView_LastInsertedIsFirstOfView_Run). `CurrPage.Run()` in the page's own procedure keeps them
    /// (:718 TempPick_Empty_CurrPageRun_HandlerTypesRow `on : T1=new;`), and so does `RunModal()` (:956). The variable's other
    /// globals are left as they are (not pinned).
    pub(crate) fn empty_temp_rec_after_variable_run(&mut self, st: &St) -> Result<(), Signal> {
        let shape = {
            let s = st.borrow();
            s.ran && !s.open && s.instance.is_some()
        } && self.has_source(st);
        if !shape {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if !rec.borrow().temporary {
            return Ok(());
        }
        let all = Rc::new(RefCell::new(rec.borrow().clone()));
        all.borrow_mut().filters.clear();
        self.record_op(RecordOp::DeleteAll, &all, &mut [Value::Boolean(false)])?;
        let table = rec.borrow().table as usize;
        let blank = self.initial_fields(table);
        rec.borrow_mut().fields = blank;
        Ok(())
    }
}
