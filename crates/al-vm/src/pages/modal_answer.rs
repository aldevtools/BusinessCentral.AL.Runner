//! The `Action` a modal page reports: the built-in action a close asks for, mapped for lookup mode, and what a page reports when its
//! handler invoked nothing (moved out of `rows.rs`).

use super::*;

impl Session {
    /// The built-in action name `close_with` is asked for, as the `Action` the page reports. A
    /// lookup-mode modal reads OK and Cancel as LookupOK and LookupCancel; a page closing itself
    /// (`CurrPage.Close()`) reads as OK in a plain modal and as LookupCancel in lookup mode (corpus
    /// ui-handlers/TestPageModalQueryClose_Tests.al, TestPageModalSelfClose_Tests.al).
    pub(super) fn close_action(&self, st: &St, requested: &str) -> String {
        let (modal, lookup, object) = {
            let s = st.borrow();
            (s.modal, s.lookup, s.object as usize)
        };
        // A confirmation dialog stays Cancel even where the lookup form reads Cancel as LookupCancel (misc/lt-probes-needed-7/
        // PnGActionOrdinals.al:ModalResultsOfAConfirmationDialog `nothing-invoked`).
        if let Some(a) = self.relation_close_action(st) {
            return a.into();
        }
        if requested == "Cancel" && self.engine.object_facts[object].page_type_lower() == "confirmationdialog" {
            return "Cancel".into();
        }
        match (requested, modal, lookup) {
            ("OK", true, true) if self.lookup_reads_plain_ok(st) => "OK".into(),
            ("OK", true, true) => "LookupOK".into(),
            ("Cancel", true, true) => "LookupCancel".into(),
            ("Close", true, true) => "LookupCancel".into(),
            ("Close", true, false) => "OK".into(),
            ("Close", false, _) => "OK".into(),
            (other, _, _) => other.into(),
        }
    }

    /// What a modal page reports when its handler invoked nothing: the page type decides (corpus
    /// ui-handlers/TestPageBuiltInActions_Tests.al (i)-(k), TestPageModalQueryClose_Tests.al (b) and (e)).
    pub(super) fn unattended_action(&self, st: &St) -> &'static str {
        let (lookup, object) = {
            let s = st.borrow();
            (s.lookup, s.object as usize)
        };
        let page_type = self.engine.object_facts[object].page_type_lower().to_string();
        match page_type.as_str() {
            "standarddialog" | "promptdialog" | "confirmationdialog" => "Cancel",
            _ if lookup && self.lookup_reads_plain_ok(st) => "OK",
            _ if lookup => "LookupCancel",
            _ => "OK",
        }
    }
}
