//! A `CurrPage.Close()` that the page's own OnQueryClosePage refused: once the page's code returns to the client, BC asks
//! OnQueryClosePage a second time, with the same action, and the page stays open when it refuses again.
//!
//! Pinned on a Card not run modally, for a close from OnAfterGetCurrRecord while the TestPage opens with OpenView
//! (pages/parts/Y37PartAfterClose.al:660 CLP_AGCR_CloseVeto_PartAccess_Observe, :861 `operation.trace`
//! `PARENT.AGCR;BEFORE.CLOSE;PARENT.QUERY:OK;AFTER.CLOSE;BEFORE.PART;...;PARENT.QUERY:OK;`) and for a close from an action
//! (:676 CLP_Action_CloseVeto_PartAccess_Observe, :913 the same trace after `PARENT.ACTION`; pages/deleted-rows-navigation/
//! Y89GoneSingleMove.al:3321 Y89_CA_False_OpenEdit_1Row_closeact `action log` `/Q:OK:10;Q:OK:10`, OpenEdit and OpenView). On a
//! Card run modally through a static RunModal with a record, the action's close is asked again too, with the same LookupCancel
//! (Y89GoneSingleMove.al:4079 Y89_CA_Modal_False_CloseAct `Q:LookupCancel:10` three times: the close, the return of the
//! action, the platform's close after the handler).
//!
//! In every pinned case the second query refuses again. A second query that allows the close, an error in it, a vetoed close
//! from any other trigger or outside a TestPage call, on another page type or a part, with an extension's OnQueryClosePage,
//! a subscriber to OnQueryClosePageEvent or a part that declares OnQueryClosePage are not pinned and are refused.

use super::*;
use al_builtins::page_op::{PageMethod, PageOp, PageRecv};

/// The kind of page trigger that runs now (only the ones the corpus pins are told apart).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TriggerKind {
    AfterGetCurrRecord,
    Action,
    /// OnOpenPage: a report's `UseRequestPage()` read there is pinned (`reports/running.rs`, `use_request_page_getter`).
    OpenPage,
    Other,
}

/// A vetoed `CurrPage.Close()` waiting for the end of the TestPage call it ran in.
pub(crate) struct CodeCloseVeto {
    page: St,
    action: String,
    kind: TriggerKind,
    depth: u32,
}

impl Session {
    /// Runs page trigger `name` of `st` with the trigger kind noted, for `note_code_close_veto`.
    pub(super) fn in_page_trigger<T>(
        &mut self,
        st: &St,
        name: &str,
        f: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let kind = match name {
            "onaftergetcurrrecord" => TriggerKind::AfterGetCurrRecord,
            "onaction" => TriggerKind::Action,
            "onopenpage" => TriggerKind::OpenPage,
            _ => TriggerKind::Other,
        };
        self.pages.trigger_kinds.push((Rc::as_ptr(st), kind));
        let r = f(self);
        self.pages.trigger_kinds.pop();
        r
    }

    /// Runs one TestPage call on page `st`; a `CurrPage.Close()` its code vetoed is asked again when it returns.
    pub(super) fn code_close_client_call(
        &mut self,
        method: PageOp,
        st: &St,
        f: impl FnOnce(&mut Self) -> Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        // A call of the page's own code (CurrPage, a Page variable) is no TestPage call.
        if method.recv() == PageRecv::Page {
            return f(self);
        }
        if self.blank_after_open_veto(st) {
            return self.call_after_open_veto(method, st, f);
        }
        self.pages.client_depth += 1;
        let depth = self.pages.client_depth;
        let r = f(self);
        self.pages.client_depth -= 1;
        if !self.pages.code_close_vetoes.iter().any(|v| v.depth == depth) {
            return r;
        }
        let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.pages.code_close_vetoes)
            .into_iter()
            .partition(|v| v.depth == depth);
        self.pages.code_close_vetoes = rest;
        let r = r?;
        for veto in mine {
            self.ask_vetoed_close_again(method, st, &veto)?;
        }
        Ok(r)
    }

    /// Whether `st` is a page whose close, requested by OnAfterGetCurrRecord while it opened, was refused twice.
    fn blank_after_open_veto(&mut self, st: &St) -> bool {
        self.pages
            .open_vetoed
            .retain(|w| w.upgrade().is_some_and(|p| p.borrow().open));
        self.pages
            .open_vetoed
            .iter()
            .any(|w| std::ptr::eq(w.as_ptr(), Rc::as_ptr(st)))
    }

    /// A TestPage call on a page whose close OnAfterGetCurrRecord requested at OpenView was refused twice. The page's own
    /// Integer control then reads 0, though Rec holds 1 (Y37PartAfterClose.al:870 `follow.parent.id` 0; after a close vetoed
    /// in an action it reads 1, :922), its part reads its own row (:874 `follow.part.id` 1), and Close asks OnQueryClosePage
    /// once more and leaves the TestPage closed (:871-876 `follow.close.ok` T, `follow.queries` 3). Everything else, and
    /// another read of a control, is not pinned.
    fn call_after_open_veto(
        &mut self,
        method: PageOp,
        st: &St,
        f: impl FnOnce(&mut Self) -> Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        match method.method() {
            PageMethod::Close if method.recv() == PageRecv::TestPage => {
                self.pages
                    .open_vetoed
                    .retain(|w| !std::ptr::eq(w.as_ptr(), Rc::as_ptr(st)));
                f(self)
            }
            PageMethod::AsInteger => match f(self)? {
                Value::Integer(_) => Ok(Value::Integer(0)),
                _ => unsupported(
                    "AsInteger of a control that is not an Integer, on a page whose close OnQueryClosePage refused twice at OpenView (not pinned)",
                ),
            },
            m => unsupported(format!(
                "TestPage {} on a page whose close OnQueryClosePage refused twice at OpenView (not pinned)",
                m.name()
            )),
        }
    }

    fn ask_vetoed_close_again(&mut self, method: PageOp, st: &St, veto: &CodeCloseVeto) -> Result<(), Signal> {
        let page = &veto.page;
        let pinned = Rc::ptr_eq(page, st)
            && page.borrow().open
            && match veto.kind {
                TriggerKind::Action => method.method() == PageMethod::Invoke,
                TriggerKind::AfterGetCurrRecord => method.method() == PageMethod::OpenView && !page.borrow().modal,
                TriggerKind::OpenPage | TriggerKind::Other => false,
            };
        if !pinned {
            return unsupported(format!(
                "a CurrPage.Close() that OnQueryClosePage refused, at the end of TestPage.{} (not pinned)",
                method.method().name()
            ));
        }
        let members: std::sync::Arc<[String]> = al_builtins::value::ACTION_MEMBERS
            .iter()
            .map(|m| m.to_string())
            .collect();
        let ordinal = al_builtins::value::ACTION_MEMBERS
            .iter()
            .position(|m| *m == veto.action)
            .unwrap_or(0) as i32;
        match self.query_close_answer(page, vec![Value::Option(ordinal, members)]) {
            Ok(Some(false)) => {
                if veto.kind == TriggerKind::AfterGetCurrRecord {
                    self.pages.open_vetoed.push(Rc::downgrade(page));
                }
                Ok(())
            }
            Err(Signal::Error(_)) => {
                unsupported("an error in the second OnQueryClosePage after a refused CurrPage.Close() (not pinned)")
            }
            Err(e) => Err(e),
            Ok(_) => unsupported(
                "a second OnQueryClosePage that allows the close a CurrPage.Close() was refused (not pinned)",
            ),
        }
    }

    /// After `CurrPage.Close()` on `st` with the close action `action`: a close the page's OnQueryClosePage refused is
    /// noted, to be asked again when the TestPage call returns (`code_close_client_call`); the shapes the corpus does not
    /// pin are refused here.
    pub(super) fn note_code_close_veto(&mut self, st: &St, action: &str) -> Result<(), Signal> {
        {
            let s = st.borrow();
            if !(s.open && s.close_vetoed && !s.close_error) {
                return Ok(());
            }
        }
        let depth = self.pages.client_depth;
        if depth == 0 {
            return unsupported(
                "a CurrPage.Close() that OnQueryClosePage refused, outside a TestPage call (not pinned)",
            );
        }
        let kind = match self.pages.trigger_kinds.last() {
            Some((p, k)) if *p == Rc::as_ptr(st) => *k,
            _ => TriggerKind::Other,
        };
        if matches!(kind, TriggerKind::Other | TriggerKind::OpenPage)
            || (kind == TriggerKind::AfterGetCurrRecord && st.borrow().modal)
        {
            return unsupported(
                "a CurrPage.Close() that OnQueryClosePage refused, from a trigger other than an action or OnAfterGetCurrRecord at OpenView (not pinned)",
            );
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let ext_query = page_of(&engine, st)
            .ext_triggers
            .iter()
            .any(|(n, _, _)| n == "onqueryclosepage");
        let parts: Vec<St> = st.borrow().parts.iter().map(|(_, c)| c.clone()).collect();
        let part_query = parts.iter().any(|p| self.declares(p, "onqueryclosepage"));
        if st.borrow().host.is_some()
            || engine.object_facts[object].page_type_lower() != "card"
            || ext_query
            || part_query
            || engine.events.is_subscribed(object, "onqueryclosepageevent")
        {
            return unsupported(
                "a CurrPage.Close() that OnQueryClosePage refused, on a part, a page other than a Card, or with an extension's or a part's OnQueryClosePage or a subscriber to its close event (not pinned)",
            );
        }
        self.pages.code_close_vetoes.push(CodeCloseVeto {
            page: st.clone(),
            action: action.to_string(),
            kind,
            depth,
        });
        Ok(())
    }
}
