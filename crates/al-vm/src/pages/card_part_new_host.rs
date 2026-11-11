//! A part without a repeater (a CardPart) that shows no row while its host stands on a new row nobody stored yet.
//!
//! BC runs the part's OnAfterGetCurrRecord on the blank record, after the host's own (testpage/parts/HiddenPartOutsideChain.al:319
//! Chain_FbHidden_OpenNewTypeKey `new` `host-open | fb-open() | host-row() | fb-row()`, the same for the shown part and on a page
//! without a Provider link, :333, :347, :361; testpage/parts/HiddenPartExtensionVariable.al:430 `host-agcr() ext-agcr() fb-agcr()`;
//! pages/parts/HeldSenderPartAccess.al:750 HS07 `sub.curr;f4.curr(0)`). It runs no OnAfterGetRecord. Whether it runs OnNewRecord
//! is not measured there: a part that declares it or has a subscriber of its event is refused. Pinned for a top-level Card or
//! Document host; a hidden part on another host, or with a repeater, is refused as the new row opens
//! (`refuse_unpinned_hidden_part_on_new_host`).

use super::*;

impl Session {
    /// Whether `st` is a part of a top-level Card or Document page.
    pub(super) fn part_on_card_or_document_host(&self, st: &St) -> bool {
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        host.is_some_and(|h| self.top_card_or_document(&h))
    }

    /// Whether part `st` is linked to its host by a SubPageLink.
    pub(super) fn part_linked(&self, st: &St) -> bool {
        let Some((host, control)) = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)))
        else {
            return false;
        };
        page_of(&self.engine(), &host).controls[control]
            .part_link
            .as_ref()
            .is_some_and(|l| !l.entries.is_empty())
    }

    /// Whether page `st` has a repeater.
    pub(super) fn has_repeater(&self, st: &St) -> bool {
        page_of(&self.engine(), st)
            .controls
            .iter()
            .any(|c| c.kind == ir::ControlKind::Repeater)
    }

    /// Whether `st` is a part without a repeater, linked by a SubPageLink, whose host is a top-level Card or Document page on the
    /// new row TestPage.OpenNew started and nobody stored yet. Every pin is that shape; after a New() on an open host the part
    /// reads as before (not pinned).
    pub(super) fn card_part_of_new_host(&self, st: &St) -> bool {
        let new_host = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, _)| h.upgrade())
            .is_some_and(|h| {
                let h = h.borrow();
                h.draft && !h.draft_stored && h.draft_from_open
            });
        new_host
            && self.part_linked(st)
            && self.part_on_card_or_document_host(st)
            && self.has_source(st)
            && !self.has_repeater(st)
    }

    /// The blank record of such a part: Rec is blank, and its OnAfterGetCurrRecord and that event's subscribers run once
    /// (HiddenPartOutsideChain.al:320 `keyed` `fb-row()` after the key typed, a blank record again).
    pub(super) fn card_part_blank_on_new_host(&mut self, st: &St) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        if self.declares_or_ext_declares(st, "onnewrecord")
            || self.engine().events.is_subscribed(object, "onnewrecordevent")
        {
            return unsupported(
                "OnNewRecord of a part without a repeater that shows no row of a new host row (whether BC runs it is not pinned)",
            );
        }
        let rec = self.page_rec(st);
        let fresh = self.initial_fields(rec.borrow().table as usize);
        rec.borrow_mut().fields = fresh;
        self.populate_expressions(st)?;
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        Ok(())
    }

    /// A new row on a page without a Provider link, with a part a Visible expression hid that declares OnAfterGetCurrRecord,
    /// with a repeater, without a SubPageLink or on a host other than a top-level Card or Document page: what BC runs for it is not measured (a hidden
    /// ListPart reads its blank line fewer times than its shown twin inside a Provider chain, testpage/parts/
    /// ProviderLinkedPartsMatrix.al:1225 against :1244). Refused before the parts open. With a Provider link the hidden parts read
    /// as ProviderLinkedPartsMatrix.al pins.
    pub(super) fn refuse_unpinned_hidden_part_on_new_host(
        &self,
        st: &St,
        hidden: &[(usize, St)],
    ) -> Result<(), Signal> {
        if st.borrow().mode != PageMode::New || self.page_has_provider_link(st) {
            return Ok(());
        }
        for (_, c) in hidden {
            if self.declares_or_ext_declares(c, "onaftergetcurrrecord")
                && (self.has_repeater(c) || !self.part_on_card_or_document_host(c) || !self.part_linked(c))
            {
                return unsupported(
                    "a new row on a page with a part hidden by a Visible expression that declares OnAfterGetCurrRecord, a part with a repeater, without a SubPageLink or on a host other than a Card or Document page (not pinned)",
                );
            }
        }
        Ok(())
    }
}
