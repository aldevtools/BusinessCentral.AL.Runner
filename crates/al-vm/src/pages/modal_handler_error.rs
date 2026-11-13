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
    /// unsaved row, is refused. `caught_before`: the session's count of caught errors before the handler ran (caught_errors.rs).
    pub(super) fn close_modal_after_handler_error(
        &mut self,
        st: &St,
        e: &Signal,
        caught_before: u64,
    ) -> Result<(), Signal> {
        if self.relation_closed_in_handler(st) {
            return self.close_relation_lookup_after_handler_error(st);
        }
        if self.document_close_pinned(st)? {
            return self.close_document_after_handler_error(st, e, caught_before);
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
        let close_three_drafts = self.three_draft_parts_close_pinned(st, &parts, e);
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

    // al-runner issue 7891, corpus PR #1353: pages/close-errors/TestModalThreeDraftParts.al.
    fn three_draft_parts_close_pinned(&self, st: &St, parts: &[St], error: &Signal) -> bool {
        use super::pinned_shape::{TestCall, TreeCall, table_code};
        use al_builtins::page_op::PageMethod;
        let Signal::Error(error) = error else { return false };
        if error.code != "TestValidation" {
            return false;
        }
        let host = st.borrow();
        let engine = self.engine();
        let object = host.object as usize;
        if host.lookup
            || host.run_static != Some(false)
            || host.tree_calls_unknown
            || engine.object_facts[object].page_type != Some(ir::PageType::Document)
        {
            return false;
        }
        let [first, second, third] = parts else { return false };
        let part_object = first.borrow().object;
        let shape = |state: &St| {
            let object = state.borrow().object as usize;
            let page = page_of(&engine, state);
            page.triggers.len() == 1
                && page.triggers[0].0 == "onqueryclosepage"
                && page.extensions.is_empty()
                && page.ext_triggers.is_empty()
                && !engine.events.any_subscriber_of(object)
                && table_code(&engine, self.page_rec(state).borrow().table as usize) == Some((Vec::new(), false))
        };
        drop(host);
        if !shape(st)
            || !shape(first)
            || !engine.object_facts[part_object as usize]
                .page_type
                .as_ref()
                .is_some_and(|p| p.lower() == "listpart")
            || ![first, second, third].iter().all(|p| {
                let p = p.borrow();
                p.object == part_object && p.draft && !p.dirty
            })
        {
            return false;
        }
        let host = st.borrow();
        let page = page_of(&engine, st);
        let indices: Vec<_> = host.parts.iter().map(|(i, _)| *i).collect();
        let [a, b, c] = indices.as_slice() else { return false };
        if !indices.iter().all(|&i| {
            page.controls[i].part_link.as_ref().is_some_and(|link| {
                link.provider.is_none()
                    && link.view.is_none()
                    && matches!(link.entries.as_slice(), [ir::LinkEntry::Field(..)])
            })
        }) {
            return false;
        }
        let [
            new_a,
            new_b,
            new_c,
            TreeCall {
                part: Some(part),
                call: TestCall::Write(field),
                failed: true,
                ..
            },
        ] = host.tree_calls.as_slice()
        else {
            return false;
        };
        let new_on = |step: &TreeCall, index| {
            step.part == Some(index) && step.call == TestCall::Page(PageMethod::New) && !step.failed && !step.no
        };
        if !new_on(new_a, *a) || !new_on(new_b, *b) || !new_on(new_c, *c) || *part != *a {
            return false;
        }
        let control = &page_of(&engine, first).controls[*field];
        control.triggers.iter().any(|(name, _)| name == "onvalidate")
            && first.borrow().trigger_errors.iter().any(|(index, message)| {
                *index == *field
                    && error.message
                        == format!("Validation error for Field: {},  Message = '{}'", control.name, message)
            })
    }
}
