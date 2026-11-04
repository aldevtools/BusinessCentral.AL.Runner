//! Application areas on the test client: a control whose `ApplicationArea` the session does not
//! enable is not on the page at all, so a TestPage cannot reach it (corpus
//! pages/application-area/TestPageApplicationAreaControlRemoval.al, ...ActionRemoval.al,
//! ...PartRemoval.al, ...ExtensionControlRemoval.al, ...ExtensionActionPartRemoval.al).
//!
//! The areas are read when the page opens (`PageState::areas`): the page is built then. Every
//! corpus test restores the session's areas only after it is done with the page.

use super::*;

/// The areas the text `areas` (`#Basic,#Suite`) enables; `None` for the empty text, which enables all.
fn enabled(areas: &str) -> Option<Vec<String>> {
    if areas.trim().is_empty() {
        return None;
    }
    Some(
        areas
            .split(',')
            .map(|a| a.trim().trim_start_matches('#').to_ascii_lowercase())
            .filter(|a| !a.is_empty())
            .collect(),
    )
}

/// Whether a control that states `ApplicationArea = <value>` is on a page whose session enables
/// `areas`. `All` is on every page.
fn states_enabled(value: &str, areas: &[String]) -> bool {
    value
        .split(',')
        .map(|a| a.trim().trim_start_matches('#').to_ascii_lowercase())
        .any(|a| a == "all" || areas.contains(&a))
}

impl Session {
    /// The `ApplicationArea` control `idx` answers to: its own (the `modify` of an extension
    /// replaces it), else the page's for a control of the page itself; a control an extension
    /// adds does not take the page's (corpus ...ExtensionControlRemoval.al: ExtNoArea_DoesNotInheritPageArea_IsNotFound).
    /// An actionref that states none answers to its target's (...ExtensionActionPartRemoval.al: ExtBasicRef).
    fn control_area(&self, st: &St, idx: usize) -> Option<String> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let ctl = &page.controls[idx];
        if let Some(ir::PropVal::Lit(v)) = ctl.prop("applicationarea") {
            return Some(v.clone());
        }
        if let Some(ir::ActionRef::Target(t)) = ctl.action_ref {
            return self.control_area(st, t);
        }
        if ctl.owner.is_some() {
            return None;
        }
        let object = st.borrow().object as usize;
        engine.program.objects[object]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("ApplicationArea"))
            .map(|(_, v)| v.clone())
    }

    /// Whether application-area removal takes control `idx` off the page: a field, action, part or
    /// actionref whose area the session does not enable, or that has none while the session
    /// restricts the areas.
    pub(super) fn area_removed(&self, st: &St, idx: usize) -> bool {
        if self.flowfield_hidden(st, idx) {
            return true;
        }
        let Some(areas) = enabled(&st.borrow().areas) else {
            return false;
        };
        let engine = self.engine();
        let kind = &page_of(&engine, st).controls[idx].kind;
        if !matches!(
            kind,
            ir::ControlKind::Field | ir::ControlKind::Action | ir::ControlKind::Part
        ) {
            return false;
        }
        match self.control_area(st, idx) {
            Some(v) => !states_enabled(&v, &areas),
            None => true,
        }
    }

    /// Refuses what the corpus does not pin: a group around the control that states an area of its own that is not enabled.
    pub(super) fn check_areas_pinned(&self, st: &St, idx: usize) -> Result<(), Signal> {
        // A change of the session's areas while the page is open does not reach it: the page keeps the controls it opened with
        // (the areas it read then, `st.areas`) until it is closed and opened again (corpus branch al-runtime-rs-staging-r6962,
        // pages/application-area/AreasChangedWhilePageOpen.al AreasSetInPageTrigger_FewerTags_FieldsRead `after` `a;b;s;`,
        // `reopened` `a;b;ERR(...)`, _OtherTags_, _FromEmpty_, AreasSetInTest_FewerTags_; issue 7001).
        let Some(areas) = enabled(&st.borrow().areas) else {
            return Ok(());
        };
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut parent = page.controls[idx].parent;
        while let Some(p) = parent {
            if let Some(ir::PropVal::Lit(v)) = page.controls[p].prop("applicationarea") {
                if !states_enabled(v, &areas) {
                    return unsupported("a control inside a group whose ApplicationArea the session does not enable");
                }
            }
            parent = page.controls[p].parent;
        }
        Ok(())
    }
}
