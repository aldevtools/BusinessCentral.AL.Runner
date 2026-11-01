//! The edit a TestPage holds is saved before the next step (corpus probe pages/lt-y45-save-before-action/
//! Y45SaveBeforeAction.al, file header lines 7-26, measured on BC 27.5, 28.5 and 29.0): an action, a RunObject codeunit or
//! page, Next, Previous, GoToRecord, GoToKey, OK and Close each save the dirty row first (OnModifyRecord, OnBeforeModifyEvent,
//! OnModify, OnAfterModifyEvent), then run.

use super::*;
use al_builtins::page_op::{PageMethod, PageOp};

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
            if host.borrow().open && host.borrow().dirty && !host.borrow().draft && host.borrow().cursor == Cursor::Row
            {
                // A host whose save fails: the part action still runs (:1473 Doc_ModifyError_LineAction `pM boom/old;...;tM
                // boom/old db=old/1;lact 10000 rec=a db=a head=old/1`).
                let depth = self.frames.len();
                host.borrow_mut().save_failure_depth = None;
                let saved = self.save_row(&host);
                self.hold_failed_save(&host, depth, saved)?;
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
        // A host with a part linked by field(filter(...)) saves its edit before the action, and the part reads its rows once, at
        // the action's CurrPage.Update (pages/lt-y37-link-filter/Y37LinkFilterTests.al:418 LT_C311_Link_ValueChangedOnPage
        // `update log` `LP.c B2 ...` once, `rows after update` B1,B2, and after a reopen).
        let engine = self.engine();
        let filter_linked = page_of(&engine, st).controls.iter().any(|c| {
            c.part_link
                .as_ref()
                .is_some_and(|l| l.entries.iter().any(|e| matches!(e, ir::LinkEntry::FieldFilter(..))))
        });
        let edited = {
            let s = st.borrow();
            s.open && s.dirty && !s.draft && s.cursor == Cursor::Row
        };
        if filter_linked && edited && self.has_source(st) {
            self.save_row(st)?;
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

    /// The page the client saves for: the page itself, or the host of a part.
    fn top_page(st: &St) -> St {
        st.borrow()
            .host
            .as_ref()
            .and_then(|(h, _)| h.upgrade())
            .unwrap_or_else(|| st.clone())
    }

    /// A save the client made before a step, at call depth `depth`, that failed with an AL error of the table's OnModify: no
    /// error reaches the test, GetLastErrorText holds it and the page counts one validation error on no control
    /// (Y45SaveBeforeAction.al:1808 Card_ModifyError_Action_PageState `last error` `Modify refused for boom.`, `page errors` 1,
    /// :1190 Card_ModifyError_Action `errors name` 0, `db after step` old/1, `page name` boom). Another error passes through.
    fn hold_failed_save(&mut self, st: &St, depth: usize, r: Result<(), Signal>) -> Result<(), Signal> {
        match r {
            Err(Signal::Error(e)) if e.code == "Dialog" && st.borrow().save_failure_depth == Some(depth) => {
                let mut s = st.borrow_mut();
                s.save_failed = true;
                s.errors.push((usize::MAX, e.message.clone()));
                drop(s);
                self.last_error = *e;
                Ok(())
            }
            other => other,
        }
    }

    /// A top-level Card or List whose table's OnValidate refused the last entry (SetValue raised TestValidation): an action
    /// and Next run nothing and raise nothing, Next answers No (Y45SaveBeforeAction.al:1296 Card_ValidateError_Action and :1427
    /// List_ValidateError_Action `step` empty, :1342 Card_ValidateError_Next `next` No).
    fn refused_entry_held(&self, st: &St) -> bool {
        let s = st.borrow();
        s.host.is_none() && s.dirty && !s.trigger_errors.is_empty() && !s.errors.is_empty()
    }

    /// `TestPage.*` with the rules above in front.
    pub(super) fn test_page_op_held(&mut self, method: PageOp, st: &St, args: &mut [Value]) -> Result<Value, Signal> {
        let name = method.method();
        let top = Self::top_page(st);
        if top.borrow().save_failed && top.borrow().open {
            // Next answers No, Close closes and runs nothing (:1228 Card_ModifyError_Next `next` No, `close` empty).
            return match name {
                PageMethod::Next if Rc::ptr_eq(&top, st) => Ok(Value::Boolean(false)),
                PageMethod::Close if Rc::ptr_eq(&top, st) => {
                    self.mark_closed(st);
                    Ok(Value::Null)
                }
                PageMethod::ValidationErrorCount => self.test_page_op(method, st, args),
                _ => unsupported(format!(
                    "TestPage.{name} on a page whose save before a step failed (not pinned)"
                )),
            };
        }
        // New() on a part of a host opened with OpenNew whose row a SetValue already inserted: the host's edit is saved first (Y45SaveBeforeAction.al:1684
        // Doc_New_HeadThenLine_Action `set line` `pM nn/ db=/0;eBM nn t=Yes db=/0;tM nn/ db=/0;eAM db=nn/0;lpN;lC 0;vDesc nl/`).
        if name == PageMethod::New && !Rc::ptr_eq(&top, st) {
            let stored_new = {
                let h = top.borrow();
                h.open && h.mode == PageMode::New && h.dirty && !h.draft && h.cursor == Cursor::Row
            };
            if stored_new {
                self.save_row(&top)?;
            }
        }
        if name == PageMethod::Next && self.is_card(st) && self.refused_entry_held(st) {
            return Ok(Value::Boolean(false));
        }
        // Close then runs no close trigger either (:1296 Card_ValidateError_Action `close` empty, the page's OnQueryClosePage
        // logs). It stores nothing and leaves the error as the last error, as `close_with` does after such a refusal
        // (pages/lt-ah-pages/AhPageTests.al:155-159).
        if name == PageMethod::Close && (self.is_card(st) || self.is_list(st)) && self.refused_entry_held(st) {
            let trigger_errors = st.borrow().trigger_errors.clone();
            self.last_error = match trigger_errors.as_slice() {
                [(_, message)] if !self.is_list(st) => al_builtins::native::AlError {
                    message: message.clone(),
                    code: "ClientValidateField:NSValidateField".into(),
                    ..Default::default()
                },
                _ => al_builtins::native::unpinned_error(),
            };
            self.mark_closed(st);
            return Ok(Value::Null);
        }
        let card_next = name == PageMethod::Next && self.is_card(st) && st.borrow().host.is_none() && st.borrow().dirty;
        if !card_next {
            return self.test_page_op(method, st, args);
        }
        let depth = self.frames.len();
        st.borrow_mut().save_failure_depth = None;
        let r = self.test_page_op(method, st, args);
        let failed = matches!(&r, Err(Signal::Error(_))) && st.borrow().save_failure_depth == Some(depth);
        if !failed {
            return r;
        }
        self.hold_failed_save(st, depth, r.map(|_| ()))?;
        Ok(Value::Boolean(false))
    }

    /// `TestField.*` and `TestAction.*` with the rules above in front.
    pub(super) fn test_control_op_held(
        &mut self,
        method: PageOp,
        c: &ControlRef,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        let m = method.method();
        let st = &c.page;
        let top = Self::top_page(st);
        if top.borrow().save_failed && top.borrow().open {
            let own = Rc::ptr_eq(&top, st);
            return match m {
                // :1808 Card_ModifyError_Action_PageState `set again error` (with the Refresh hint on a Card and a Document page),
                // :1907 List_ModifyError_Action_PageState (without it on a List); nothing runs (`set again` empty).
                PageMethod::SetValue if own => {
                    let ControlTarget::Control(idx) = c.target else {
                        return unsupported("SetValue on a page whose save before a step failed (not pinned)");
                    };
                    let engine = self.engine();
                    let name = page_of(&engine, st).controls[idx].name.clone();
                    let hint = if self.is_list(st) {
                        ""
                    } else {
                        " (Select Refresh to discard errors)"
                    };
                    Err(error_code(
                        format!(
                            "Validation error for Field: {name},  Message = 'This field cannot be validated since the key fields are not valid.{hint}'"
                        ),
                        "TestValidation",
                    ))
                }
                // `second action` empty, no error.
                PageMethod::Invoke if own => Ok(Value::Null),
                PageMethod::Value if args.is_empty() => self.test_control_op(method, c, args),
                PageMethod::ValidationErrorCount => self.test_control_op(method, c, args),
                _ => unsupported(format!("{m} on a page whose save before a step failed (not pinned)")),
            };
        }
        if m != PageMethod::Invoke || !matches!(c.target, ControlTarget::Control(_)) || st.borrow().host.is_some() {
            return self.test_control_op(method, c, args);
        }
        if (self.is_card(st) || self.is_list(st)) && self.refused_entry_held(st) {
            return Ok(Value::Null);
        }
        let depth = self.frames.len();
        st.borrow_mut().save_failure_depth = None;
        let r = self.test_control_op(method, c, args);
        if !st.borrow().dirty || st.borrow().save_failure_depth != Some(depth) {
            return r;
        }
        match r {
            Err(Signal::Error(e)) => self
                .hold_failed_save(st, depth, Err(Signal::Error(e)))
                .map(|()| Value::Null),
            other => other,
        }
    }
}
