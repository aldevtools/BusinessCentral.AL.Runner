//! What the part control that shows a page says about `Visible()` of a control of that page.
//!
//! Pinned (corpus master pages/parts/PartVisibleBoundToRecField.al, al-runner issue 7455, same on BC 27.5, 28.5 and 29.0): a part
//! whose `Visible` is an expression over a Boolean field of the host's Rec, declared on the host page or added by a page extension
//! to the FactBoxes, reports Visible() No for a control inside it when the expression is false; true leaves the control's own
//! answer. A part hidden any other way (a literal `Visible = false`, a hidden group around the part control, a part further up a
//! chain of parts) is not pinned for the controls inside it and is refused (hard rule 4).

use super::*;

impl Session {
    /// `Visible()` of control `idx` of `st`, a control of a part answering what its part control says too.
    pub(super) fn control_visible_in_part(&mut self, st: &St, idx: usize) -> Result<bool, Signal> {
        match self.part_visible_gate(st)? {
            Some(b) => Ok(b),
            None => self.control_visible(st, idx),
        }
    }

    /// `Some(false)` when the part control that shows `st` hides its controls; `None` when it does not (or `st` is no part).
    fn part_visible_gate(&mut self, st: &St) -> Result<Option<bool>, Signal> {
        let Some((host, idx)) = st.borrow().host.clone() else {
            return Ok(None);
        };
        let Some(host) = host.upgrade() else {
            return Ok(None);
        };
        if self.control_visible(&host, idx)? {
            return Ok(None);
        }
        let engine = self.engine();
        let expression = matches!(
            page_of(&engine, &host).controls[idx].prop("visible"),
            Some(ir::PropVal::Routine(_) | ir::PropVal::Ext(..))
        );
        if expression && !self.prop_bool_now(&host, idx, "visible", true)? {
            return Ok(Some(false));
        }
        unsupported(
            "Visible() of a control inside a part hidden by other than a false Visible expression of its control (not pinned)",
        )
    }
}
