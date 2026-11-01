//! The edit a TestPage holds is saved before the next step (corpus probe pages/lt-y45-save-before-action/
//! Y45SaveBeforeAction.al, file header lines 7-26, measured on BC 27.5, 28.5 and 29.0): an action, a RunObject codeunit or
//! page, Next, Previous, GoToRecord, GoToKey, OK and Close each save the dirty row first (OnModifyRecord, OnBeforeModifyEvent,
//! OnModify, OnAfterModifyEvent), then run.

use super::*;

impl Session {
    /// A RunObject action on a top-level page whose row holds an edit: the row is saved first and nothing is read before the
    /// target runs (Y45SaveBeforeAction.al:98 Card_SetName_RunObjectCodeunit `step` `pM ...;eAM db=new/1;cu H1 rec=new/1
    /// db=new/1`, :128 Card_SetName_RunObjectPage `peek open f=H1 db=new/1`, :431 and :461 the same on a List, :670 on a
    /// Document page, :1596 Card_New_CodeThenName_RunObjectCodeunit on the row a SetValue inserted).
    /// What is saved before action control `idx` of page `st` runs, besides the page's own edit (which the action path saves
    /// in the shape each page needs). A part saves its own row first, as it always did (corpus
    /// handlers/TestPageAutoSplitKey_Tests.al:TestPage_ActionInvoke_OnANewSubpageRow_*), so nothing is read again before the
    /// action (Y45SaveBeforeAction.al:847 Doc_SetLine_LineAction `lpM...;lM ...;lact 10000 rec=new`); a part action saves the
    /// host's edit before that (:700 Doc_SetName_LineAction, :1015 CardPart_SetName_LineAction `pM ...;eAM db=new/1;lact 10000
    /// ... head=new/1`). A host action saves the edit or new line of each part first (:817 Doc_SetLine_Action, :1045
    /// CardPart_SetLine_Action `lpM ...;lM ...;R H1;act H1 ... l1=new`, :1748 Doc_Existing_NewLine_Action, :1778
    /// CardPart_Existing_NewLine_Action `lpI 30000 nl;eBIL nl;lI ...;R H1;act`).
    pub(super) fn save_before_action(&mut self, st: &St, idx: usize) -> Result<(), Signal> {
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        if let Some(host) = host {
            if host.borrow().open && host.borrow().dirty && !host.borrow().draft && host.borrow().cursor == Cursor::Row {
                self.save_row(&host)?;
            }
            return self.save_row(st);
        }
        let parts: Vec<St> = st.borrow().parts.iter().map(|(_, c)| c.clone()).collect();
        for part in parts {
            if part.borrow().open && part.borrow().dirty {
                let new_line = part.borrow().draft;
                self.save_row(&part)?;
                let inserted = new_line && !part.borrow().draft && part.borrow().cursor == Cursor::Row;
                part.borrow_mut().inserted_before_host_action = inserted;
            }
        }
        self.save_before_run_object(st, idx)
    }

    fn save_before_run_object(&mut self, st: &St, idx: usize) -> Result<(), Signal> {
        let engine = self.engine();
        let run_object = page_of(&engine, st).controls[idx].run_object.as_ref().map(|r| r.kind);
        if !matches!(run_object, Some(ir::RunKind::Codeunit | ir::RunKind::Page)) {
            return Ok(());
        }
        let edited = {
            let s = st.borrow();
            s.host.is_none() && s.open && s.dirty && !s.draft && s.cursor == Cursor::Row
        };
        if !edited || !self.has_source(st) {
            return Ok(());
        }
        self.save_row(st)?;
        st.borrow_mut().saved_before_run_object = run_object == Some(ir::RunKind::Codeunit);
        Ok(())
    }
}
