//! `--dev-endpoint-ui` (al-runner issue 3697) for `Page.Run(Id)` and `Page.RunModal(Id)` with no handler bound.
//!
//! Measured through the dev endpoint (TestRunnerHub) on a BC 28.5 container by the corpus probe
//! handlers/lt-y53-dev-endpoint-runner/Y53DevRunnerProbe.al (codeunit 97530, page 97532: a card with no source table,
//! no OnInit and no part): PageRunModal_NoHandler returns `Action::None` and PageRun_NoHandler returns, and neither runs
//! a page trigger (OnOpenPage, OnQueryClosePage, OnClosePage). Only that shape is answered: a record argument, a source
//! table, an OnInit trigger (page or extension), a part, a pending Trap() or a handler of the kind bound for any page
//! leave the call as it is without the flag.
use super::{St, page_of};
use crate::Session;
use crate::ui::UiKind;
use al_builtins::value::Value;

impl Session {
    /// The dev-endpoint answer to the static run of `st`'s page, `None` to run it as without the flag.
    pub(super) fn dev_endpoint_page_run(&mut self, st: &St, modal: bool, with_record: bool) -> Option<Value> {
        let kind = if modal { UiKind::ModalPage } else { UiKind::Page };
        if with_record || !self.pages.traps.is_empty() || !self.dev_endpoint_unhandled(kind) || self.has_source(st) {
            return None;
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let on_init = page.triggers.iter().any(|(n, _)| n == "oninit") || page.ext_triggers.iter().any(|(n, _, _)| n == "oninit");
        if on_init || page.controls.iter().any(|c| c.kind == al_ir::ControlKind::Part) {
            return None;
        }
        let shown = self.object_id_of(st.borrow().object).to_string();
        if !modal {
            self.note_dev_endpoint_answer(kind, &shown, "no answer");
            return Some(Value::Null);
        }
        self.note_dev_endpoint_answer(kind, &shown, "Action::None");
        let members: std::sync::Arc<[String]> =
            al_builtins::value::ACTION_MEMBERS.iter().map(|m| m.to_string()).collect();
        let none = members.iter().position(|m| m == "None").unwrap_or(0) as i32;
        Some(Value::Option(none, members))
    }
}
