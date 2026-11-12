//! The Visible of a repeater column that a page extension's `modify()` binds to a variable of the extension: on a List opened by
//! OpenEdit or OpenView, the value counts as it stands after the extension's OnAfterGetRecord of the open read, and before
//! OnAfterGetCurrRecord (pages/extensions/PageExtModifyVisibleVariable.al:519-547: `ColRec` set in OnAfterGetRecord reads Yes,
//! `ColCurr` set in OnAfterGetCurrRecord reads No, `ColOpen` set in OnOpenPage reads Yes, also after a move). Every other
//! control keeps the value frozen at open (`frozen_visible`).

use super::*;

impl Session {
    /// After the page trigger `name` ran: on the pinned shape, the columns it governs take their Visible again.
    pub(super) fn refreeze_ext_columns_after_open_read(&mut self, st: &St, name: &str) -> Result<(), Signal> {
        if name != "onaftergetrecord" {
            return Ok(());
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        {
            let s = st.borrow();
            // Pinned facts: the open read (:522, :537), OpenEdit or OpenView (:519, :534), a page of type List (:84).
            if !s.open_read || !matches!(s.mode, PageMode::Edit | PageMode::View) || s.frozen_visible.is_empty() {
                return Ok(());
            }
        }
        if engine.object_facts[object].page_type != Some(al_ir::PageType::List) {
            return Ok(());
        }
        let page = page_of(&engine, st);
        // The base page declares no OnAfterGetRecord, one page extension does (:200-203).
        if page.triggers.iter().any(|(n, _)| n == name) {
            return Ok(());
        }
        let declaring: Vec<usize> = page
            .ext_triggers
            .iter()
            .filter(|(n, _, _)| n == name)
            .map(|(_, e, _)| *e)
            .collect();
        let [ext] = declaring.as_slice() else {
            return Ok(());
        };
        for idx in 0..page.controls.len() {
            let c = &page.controls[idx];
            // A field whose parent is the repeater (:94-101), Visible bound by that extension's modify() (:182-185).
            let in_repeater = c
                .parent
                .is_some_and(|p| page.controls[p].kind == ir::ControlKind::Repeater);
            let by_ext = matches!(c.prop("visible"), Some(ir::PropVal::Ext(e, _)) if e == ext);
            if c.kind != ir::ControlKind::Field || !in_repeater || !by_ext {
                continue;
            }
            let now = self.prop_bool_now(st, idx, "visible", true)?;
            if let Some(slot) = st.borrow_mut().frozen_visible.get_mut(idx) {
                *slot = now;
            }
        }
        Ok(())
    }
}
