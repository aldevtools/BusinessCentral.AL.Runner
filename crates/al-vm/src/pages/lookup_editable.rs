//! What lookup mode makes read-only, by page type (al-runner issues 5012 and 5029). The one owner of that rule: `page_editable`,
//! `control_editable` and the `Editable()` reads ask here.

use super::*;

/// What a page in lookup mode locks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum LookupLock {
    /// Not in lookup mode, or a page type lookup mode leaves editable.
    Nothing,
    /// The page and its controls bound to Rec; a control bound to a page variable stays editable.
    RecControls,
    /// The page and every control, page-variable controls included.
    Everything,
}

impl Session {
    /// What lookup mode locks on this page. Pinned per page type, run modally from a Page variable with `LookupMode(true)`:
    /// a List locks everything (testpage/TestPageControlEditableRoutes.al:446 List_RunModal_LookupMode); a Card, a Worksheet, a
    /// Document and a ListPlus lock nothing (:460, :474, :488, :502); a ListPart locks everything, a CardPart the page and its Rec
    /// controls, a StandardDialog, a NavigatePage and a ConfirmationDialog nothing (testpage/LookupModeEditablePageTypes.al:174
    /// ListPart, :204 CardPart, :234 StandardDialog, :264 NavigatePage, :294 ConfirmationDialog). `None`: another page type,
    /// whose answer is not pinned (`refuse_unpinned_lookup_editable`).
    pub(super) fn lookup_lock(&self, st: &St) -> Option<LookupLock> {
        if !st.borrow().lookup {
            return Some(LookupLock::Nothing);
        }
        let object = st.borrow().object as usize;
        match self.engine.object_facts[object].page_type_lower() {
            "list" | "listpart" => Some(LookupLock::Everything),
            "cardpart" => Some(LookupLock::RecControls),
            "" | "card" | "worksheet" | "document" | "listplus" | "standarddialog" | "navigatepage" | "confirmationdialog" => {
                Some(LookupLock::Nothing)
            }
            _ => None,
        }
    }

    /// An `Editable()` read on a page in lookup mode whose page type no corpus case pins.
    pub(super) fn refuse_unpinned_lookup_editable(&self, st: &St) -> Result<(), Signal> {
        if self.lookup_lock(st).is_some() {
            return Ok(());
        }
        let object = st.borrow().object as usize;
        unsupported(format!(
            "Editable() on a page in lookup mode of PageType {} (not pinned)",
            self.engine.object_facts[object].page_type_lower()
        ))
    }

    /// A ListPart, a CardPart and a NavigatePage in lookup mode report OK, not LookupOK: when the handler invokes OK
    /// (events/page-events/NoSourcePageEvents.al:1327 NoSourceCardPart_RunModal_OK_EventLog, :1527 ListPart, :1127 NavigatePage,
    /// a static Page.RunModal) and when it invokes nothing (testpage/LookupModeEditablePageTypes.al:174 `lookup action` = OK,
    /// :204, :264, LookupMode(true) on a Page variable).
    pub(super) fn lookup_reads_plain_ok(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        matches!(
            self.engine.object_facts[object].page_type_lower(),
            "listpart" | "cardpart" | "navigatepage"
        )
    }

    /// A ListPart and a CardPart in lookup mode offer no built-in Cancel (NoSourcePageEvents.al:1349
    /// NoSourceCardPart_RunModal_Cancel_EventLog `The built-in action = Cancel is not found on the page.`, :1549 ListPart).
    pub(super) fn lookup_part_without_cancel(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        matches!(self.engine.object_facts[object].page_type_lower(), "listpart" | "cardpart")
    }
}
