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
//! Pinned by pages/close-errors/ModalCloseSaveErrorShapes.al (BC 27.5, 28.5, 29.0) on pages without OnQueryClosePage: on a List
//! and a Card run by a static `Page.RunModal(Id, Rec)` whose return value is used (on a List also called as a statement), a TestField error raised in the page's
//! OnModifyRecord (by a codeunit it runs, the al-runner issue 7400 report, or by the page's own code) keeps its text and the code
//! TestField, and the save runs once more first (:250, :271, :314, :336, `log` twice). A TestField error raised in a table trigger
//! (the table's OnModify at the save, or reached by `Modify(true)` in OnModifyRecord), after a Page variable's RunModal or on a
//! page with OnQueryClosePage is not measured and refused, and so is every error after a static `Page.RunModal(Id)` without a record (every pinned static call passes one). An Error()
//! gets the code TestWrapped (:292, :356), also on a Worksheet (:376) and on a List run by a Page variable in LookupMode (:419),
//! where only Error() is measured.
//! A new row's insert at close (:398 measures one, but the runtime inserts the row at SetValue and no corpus case shows when a
//! List without DelayedInsert inserts it in a TestPage), a Document or any page with parts (pages/close-errors/CemDocumentTests.al
//! pins another error there), another error code and every other page type keep the refusal.

use super::*;

impl Session {
    /// Closes the modal page `st` its handler left open with the unattended `action` (`answer_page`). In the shape this module
    /// names, an error of the save at close is RunModal's error; elsewhere such an error is refused.
    pub(super) fn close_modal_left_open(&mut self, st: &St, action: &str) -> Result<(), Signal> {
        let pinned = self.modal_close_save_pinned(st);
        let errors_before = st.borrow().errors.len();
        self.pages.close_save_error = None;
        self.pages.failed_trigger = None;
        self.close_with(st, action, true, false)?;
        // The page trigger the save's error came from (the close keeps the error on the page, `PageState::errors`).
        let object = st.borrow().object;
        let origin = self
            .pages
            .failed_trigger
            .take()
            .filter(|(o, _)| *o == object)
            .map(|(_, name)| name);
        let saved = std::mem::take(&mut self.pages.close_save_error);
        // The TestField is pinned raised by the page's own code or by a codeunit it calls (ModalCloseSaveErrorShapes.al:34 the
        // TableNo codeunit's OnRun, :74 a codeunit procedure); one raised in a table trigger the save reaches is not.
        let raised_in_page_or_codeunit =
            self.pages.close_save_raised_in.take().is_some_and(|o| {
                o == object as usize || self.engine.program.objects[o].kind == al_ir::ObjKind::Codeunit
            });
        // The save of the edited row raised (OnModifyRecord, OnInsertRecord or the table's trigger), so the page stayed open.
        let raised = st.borrow().open && st.borrow().errors.len() > errors_before;
        if !raised {
            return Ok(());
        }
        let Some((e, codes)) = saved.zip(pinned) else {
            return unsupported(
                "an error raised while the page saved its edited row as the platform closed it after the handler returned, on a page other than a Card, List or Worksheet without parts saving an existing row (not pinned)",
            );
        };
        let mut wrapped = e.clone();
        if e.code == "Dialog" {
            wrapped.code = "TestWrapped".into();
        } else if e.code == "TestField"
            && codes == PinnedCodes::DialogAndTestField
            && origin.as_deref() == Some("onmodifyrecord")
            && raised_in_page_or_codeunit
            && !self.query_close_code(st)
        {
            // The save runs once more and raises the TestField error again (ModalCloseSaveErrorShapes.al:250 `log` M-list NEW twice).
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
                "an error with the code {} raised while a modal page saved its edited row as the platform closed it after the handler returned (not pinned for this page)",
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

    /// The shape whose save error is pinned, and the error codes pinned on it: a modal Card, List or Worksheet with no parts,
    /// holding an edited row that exists (no new row, no pending insert). A Card or List in lookup mode as a static
    /// `Page.RunModal(Id, Rec)` puts it there, and a List that call opened as a statement: Dialog and TestField (the TestField only from OnModifyRecord without
    /// OnQueryClosePage, `close_modal_left_open`). A Card or List not in lookup mode (CemModalHandlerTests.al CardVariableRunModal,
    /// ListVariableRunModal cells), a Worksheet in lookup mode as a static `Page.RunModal(Id, Rec)` puts it there, and a List in
    /// lookup mode by `LookupMode(true)` on its Page variable: Dialog.
    fn modal_close_save_pinned(&self, st: &St) -> Option<PinnedCodes> {
        let s = st.borrow();
        let row_shape = s.modal
            && s.parts.is_empty()
            && s.hidden_parts.is_empty()
            && s.host.is_none()
            && s.dirty
            && !s.draft
            && s.pending.is_none()
            && s.cursor == Cursor::Row
            && s.mode != PageMode::New
            && s.errors.is_empty();
        if !row_shape {
            return None;
        }
        let page_type = self.engine.object_facts[s.object as usize].page_type.clone();
        // How the page was run. Every pinned static `Page.RunModal` passes a record (ModalCloseSaveErrorShapes.al:258, :321,
        // :343, :384; CemModalHandlerTests.al: all 66 calls are `Page.RunModal(Id, Row)`); one without a record is no pinned
        // shape and keeps the refusal.
        let run = match (s.run_static, s.static_run_record, s.lookup.then_some(s.lookup_kind)) {
            (Some(true), true, Some(LookupKind::StaticRunModal)) => Run::StaticRecordUsed,
            (Some(true), true, None) => Run::StaticRecordStatement,
            (Some(true), ..) => return None,
            (_, _, None) => Run::NoLookup,
            (_, _, Some(LookupKind::Variable)) => Run::VariableLookup,
            _ => return None,
        };
        match (page_type?, run) {
            (al_ir::PageType::Card | al_ir::PageType::List, Run::StaticRecordUsed) => {
                Some(PinnedCodes::DialogAndTestField)
            }
            // ModalCloseSaveErrorShapes.al:314 List_CodeunitTestField_LastErrorCode: `asserterror Page.RunModal(Id, Rec)`.
            (al_ir::PageType::List, Run::StaticRecordStatement) => Some(PinnedCodes::DialogAndTestField),
            // CemModalHandlerTests.al:900, :1743 (`Page.RunModal(Id, Row)` as a statement) and the CardVariableRunModal and
            // ListVariableRunModal cells; ModalCloseSaveErrorShapes.al:376 the Worksheet, :419 the List in LookupMode.
            (al_ir::PageType::Card, Run::StaticRecordStatement)
            | (al_ir::PageType::Card | al_ir::PageType::List, Run::NoLookup)
            | (al_ir::PageType::Worksheet, Run::StaticRecordUsed)
            | (al_ir::PageType::List, Run::VariableLookup) => Some(PinnedCodes::Dialog),
            _ => None,
        }
    }
}

/// How a modal page whose save error at close is pinned was run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Run {
    /// A static `Page.RunModal(Id, Rec)` whose return value is used (the page is a lookup).
    StaticRecordUsed,
    /// A static `Page.RunModal(Id, Rec)` called as a statement.
    StaticRecordStatement,
    /// Not a static RunModal and not in lookup mode (a Page variable's RunModal).
    NoLookup,
    /// A Page variable in lookup mode by `LookupMode(true)`.
    VariableLookup,
}

impl Session {
    /// Whether the page, a page extension or a subscriber has OnQueryClosePage code.
    fn query_close_code(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.declares_or_ext_declares(st, "onqueryclosepage")
            || self.engine().events.is_subscribed(object, "onqueryclosepageevent")
    }
}

/// The error codes of a save at close that the corpus pins on a page shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PinnedCodes {
    Dialog,
    DialogAndTestField,
}
