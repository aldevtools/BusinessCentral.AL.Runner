//! A modal page its ModalPageHandler left open by raising an error, and a modal close the page's OnQueryClosePage refused
//! (moved out of `rows.rs`). Owns the close the platform still asks after the handler's error (`close_modal_after_handler_error`).

use super::*;

impl Session {
    /// A modal page whose close was refused by OnQueryClosePage: the corpus pins the page's own trigger answering false,
    /// on a page with no extension that declares OnQueryClosePage and no subscriber to OnQueryClosePageEvent (a modal page with
    /// open parts is refused in `close_with`).
    pub(super) fn refuse_unpinned_modal_veto(&self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let ext_query = page_of(&engine, st)
            .ext_triggers
            .iter()
            .any(|(n, _, _)| n == "onqueryclosepage");
        if ext_query || engine.events.is_subscribed(object, "onqueryclosepageevent") {
            return unsupported(
                "a modal page whose close was refused, with an extension's OnQueryClosePage or a subscriber to its close event (not pinned)",
            );
        }
        Ok(())
    }

    /// A ModalPageHandler that raised an error left its page open: the platform still asks the close, the parts first with
    /// None, then the page with its own action, and RunModal raises the handler's error (pages/triggers/
    /// PageModalQueryCloseVeto.al:478 RP_ListWithFactBox_HandlerCancel_HostVetoes, where the page refuses the close). Pinned only
    /// for a refusal by a page with open parts, and for a close allowed in lookup mode (WbModalHandlerCloseTests.al); a page without
    /// parts, or one where no close trigger is declared, closes as before without asking; another allowed close, or one with an
    /// unsaved row, is refused.
    pub(super) fn close_modal_after_handler_error(&mut self, st: &St, e: &Signal) -> Result<(), Signal> {
        if self.relation_closed_in_handler(st) {
            return self.close_relation_lookup_after_handler_error(st);
        }
        if self.document_close_pinned(st)? {
            return self.close_document_after_handler_error(st, e);
        }
        let parts: Vec<St> = st
            .borrow()
            .parts
            .iter()
            .map(|(_, c)| c.clone())
            .filter(|c| c.borrow().open)
            .collect();
        // Kept, not pinned: a page without open parts closes without its close triggers, as before (handlers/
        // TestPageModalQueryClose_Tests.al PlainModal_HasNoBuiltInCancelAction, codeunit 60338's dialog cases,
        // TestPageQueryCloseError_Tests.al ErrorInQueryClosePage_*, P106RunInstance.al P106_Inst_RunModal_List pass through it).
        if parts.is_empty() && self.no_source_handler_error_close_pinned(st) {
            return self.close_no_source_modal_after_handler_error(st);
        }
        let asks = self.declares(st, "onqueryclosepage") || parts.iter().any(|p| self.declares(p, "onqueryclosepage"));
        if !asks || parts.is_empty() {
            return Ok(());
        }
        if st.borrow().dirty || st.borrow().draft || !st.borrow().hidden_parts.is_empty() {
            return unsupported(
                "a page left open by a ModalPageHandler that raised an error, with an unsaved row or a hidden part (not pinned)",
            );
        }
        // ModalThreeParts.al: after a ListPart validation error, the three clean drafts
        // receive None and the instance-run Document receives OK before the handler error returns.
        let close_three_drafts = matches!(e, Signal::Error(error) if error.code == "TestValidation")
            && !st.borrow().lookup
            && st.borrow().run_static == Some(false)
            && self.engine.object_facts[st.borrow().object as usize].page_type == Some(ir::PageType::Document)
            && parts.len() == 3
            && parts.iter().any(|p| !p.borrow().trigger_errors.is_empty())
            && parts.iter().all(|p| {
                let state = p.borrow();
                state.draft
                    && !state.dirty
                    && self.engine.object_facts[state.object as usize]
                        .page_type
                        .as_ref()
                        .is_some_and(|kind| kind.lower() == "listpart")
            });
        let a = self.unattended_action(st);
        let lookup = st.borrow().lookup;
        match self.close_with(st, a, false, false) {
            Ok(_) if st.borrow().open && st.borrow().close_vetoed => self.refuse_unpinned_modal_veto(st),
            // Allowed in lookup mode: the part answers None, the page LookupCancel, the page closes and RunModal raises the
            // handler's error, also an error of a page action the handler invoked (ui-handlers/WbModalHandlerCloseTests.al:9-39
            // `Part:None|Page:LookupCancel`, al-runner issue 6291). Outside lookup mode it is not pinned.
            Ok(_) if lookup && !st.borrow().open => Ok(()),
            Ok(_) if close_three_drafts && !st.borrow().open => {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::ModalValidationThreeDraftPartsClose,
                );
                Ok(())
            }
            Err(Signal::Error(_)) | Ok(_) => unsupported(
                "a page left open by a ModalPageHandler that raised an error, whose close is allowed (not pinned)",
            ),
            Err(e) => Err(e),
        }
    }
}
