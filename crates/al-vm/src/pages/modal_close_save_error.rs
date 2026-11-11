//! The platform's close of a modal page a ModalPageHandler returned from without closing it, when the save of the edited row
//! raises an AL error (al-runner issue 7400).
//!
//! Pinned by pages/close-errors/CemModalHandlerTests.al (BC 27.5, 28.5, 29.0): on a Card and a List with no parts, run by a
//! static `Page.RunModal` or a Page variable's `RunModal`, whose handler edited an existing row and returned, the save at close
//! raises the error of the page's OnModifyRecord (:900 CardStaticRunModal_HandlerEditReturns_PageModifyRecordError, :1743
//! ListStaticRunModal_HandlerEditReturns_PageModifyRecordError, :983 and :1829 _HandlerTableRefusesReturns_PageModifyRecordError)
//! or of the table's OnModify (:250 CardStaticRunModal_HandlerTableRefusesReturns_NoError, :1295 the List, and the
//! CardVariableRunModal and ListVariableRunModal cells of the same name): RunModal fails with the error's own text and the code
//! TestWrapped; OnQueryClosePage and OnClosePage do not run.
//!
//! A TestField error (the reporter's, from a codeunit OnModifyRecord runs) keeps its text and its code, and the save runs once
//! more first, as real BC showed in the queued probe; under the assumption `modal-close-save-error-code` until it is pinned.
//! Other codes are refused. A new row's insert at close, a Document or any page with parts (pages/close-errors/CemDocumentTests.al pins another
//! error there), a Page variable in LookupMode, and every other page type keep the refusal.

use super::*;

impl Session {
    /// Closes the modal page `st` its handler left open with the unattended `action` (`answer_page`). In the shape this module
    /// names, an error of the save at close is RunModal's error; elsewhere such an error is refused.
    pub(super) fn close_modal_left_open(&mut self, st: &St, action: &str) -> Result<(), Signal> {
        let pinned = self.modal_close_save_pinned(st);
        let errors_before = st.borrow().errors.len();
        self.pages.close_save_error = None;
        self.close_with(st, action, true, false)?;
        let saved = std::mem::take(&mut self.pages.close_save_error);
        // The save of the edited row raised (OnModifyRecord, OnInsertRecord or the table's trigger), so the page stayed open.
        let raised = st.borrow().open && st.borrow().errors.len() > errors_before;
        if !raised {
            return Ok(());
        }
        let Some(e) = saved.filter(|_| pinned) else {
            return unsupported(
                "an error raised while the page saved its edited row as the platform closed it after the handler returned, on a page other than a Card or List without parts saving an existing row (not pinned)",
            );
        };
        let mut wrapped = e.clone();
        if e.code == "Dialog" {
            wrapped.code = "TestWrapped".into();
        } else if e.code == "TestField" {
            // Observed on real BC (corpus PR 1323 ModalCloseSaveErrorShapes.al, not merged yet): a TestField error keeps its
            // code and the save runs once more before RunModal fails (`log` M-list NEW twice). Assumption until the pin lands.
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ModalCloseSaveErrorCode);
            match self.save_row(st) {
                Err(Signal::Error(again)) if again.code == "TestField" => wrapped = (*again).clone(),
                Err(Signal::Error(_)) | Ok(_) => {
                    return unsupported(
                        "the second save at a modal page's close after a TestField error, which did not raise the TestField error again (not pinned)",
                    );
                }
                Err(other) => return Err(other),
            }
        } else {
            return unsupported(format!(
                "an error with the code {} raised while a modal page saved its edited row as the platform closed it after the handler returned (only Dialog is pinned)",
                e.code
            ));
        }
        {
            let mut s = st.borrow_mut();
            s.dirty = false;
            s.errors.truncate(errors_before);
        }
        self.mark_closed(st);
        Err(Signal::Error(Box::new(wrapped)))
    }

    /// The shape whose save error is pinned: a modal Card or List with no parts, in lookup mode only as a static
    /// `Page.RunModal(Id, Rec)` puts it there, holding an edited row that exists (no new row, no pending insert).
    fn modal_close_save_pinned(&self, st: &St) -> bool {
        let s = st.borrow();
        s.modal
            && matches!(
                self.engine.object_facts[s.object as usize].page_type,
                Some(al_ir::PageType::Card | al_ir::PageType::List)
            )
            && s.parts.is_empty()
            && s.hidden_parts.is_empty()
            && s.host.is_none()
            && (!s.lookup || s.lookup_kind == LookupKind::StaticRunModal)
            && s.dirty
            && !s.draft
            && s.pending.is_none()
            && s.cursor == Cursor::Row
            && s.mode != PageMode::New
            && s.errors.is_empty()
    }
}
