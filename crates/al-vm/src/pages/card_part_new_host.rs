//! A part without a repeater (a CardPart) that shows no row while its host stands on a new row nobody stored yet.
//!
//! BC runs the part's OnAfterGetCurrRecord on the blank record, after the host's own (testpage/parts/HiddenPartOutsideChain.al:319
//! Chain_FbHidden_OpenNewTypeKey `new` `host-open | fb-open() | host-row() | fb-row()`, the same for the shown part and on a page
//! without a Provider link, :333, :347, :361; testpage/parts/HiddenPartExtensionVariable.al:430 `host-agcr() ext-agcr() fb-agcr()`;
//! pages/parts/HeldSenderPartAccess.al:750 HS07 `sub.curr;f4.curr(0)`). It runs no OnAfterGetRecord. Whether it runs OnNewRecord
//! is not measured there: a part that declares it or has a subscriber of its event is refused.

use super::*;

impl Session {
    /// Whether `st` is a part without a repeater whose host is a top-level Card or Document page on a new, unsaved row.
    pub(super) fn card_part_of_new_host(&self, st: &St) -> bool {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return false;
        };
        let new_host = {
            let h = host.borrow();
            h.host.is_none()
                && h.draft
                && !h.draft_stored
                && matches!(
                    self.engine.object_facts[h.object as usize].page_type,
                    Some(al_ir::PageType::Card | al_ir::PageType::Document)
                )
        };
        new_host
            && self.has_source(st)
            && !page_of(&self.engine(), st)
                .controls
                .iter()
                .any(|c| c.kind == ir::ControlKind::Repeater)
    }

    /// The blank record of such a part: its OnAfterGetCurrRecord and that event's subscribers run once.
    pub(super) fn card_part_blank_on_new_host(&mut self, st: &St) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        if self.declares_or_ext_declares(st, "onnewrecord")
            || self.engine().events.is_subscribed(object, "onnewrecordevent")
        {
            return unsupported(
                "OnNewRecord of a part without a repeater that shows no row of a new host row (whether BC runs it is not pinned)",
            );
        }
        self.populate_expressions(st)?;
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        Ok(())
    }
}
