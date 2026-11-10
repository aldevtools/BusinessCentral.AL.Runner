//! Shapes of Provider-linked parts that testpage/parts/ProviderLinkedPartsMatrix.al pins but that are not implemented yet:
//! the runner refuses them where its reads would differ from BC's, instead of running them with other trigger counts.

use super::*;

impl Session {
    /// Whether part `i` of the host page takes its link from a Provider or is the Provider of another part.
    fn part_in_provider_chain(&self, st: &St, i: usize) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        page.controls[i]
            .part_link
            .as_ref()
            .is_some_and(|l| l.provider.is_some())
            || page
                .controls
                .iter()
                .any(|c| c.part_link.as_ref().is_some_and(|l| l.provider == Some(i)))
    }

    /// A part of a Provider chain that a Visible expression hid as the host opened and that shows after the host's first row: BC reads
    /// the linked parts once more before the rows, and in edit mode the Provider part twice on its blank line
    /// (ProviderLinkedPartsMatrix.al:1213 Provider_BothVisible_OpenViewAndGoToKey `host-row(H1) | detail-row() | lines-row(10)`,
    /// Provider_BothVisible_OpenEditNextPreviousGoToRecord `lines-row(0) | lines-row(0) | detail-row()`). Not implemented.
    pub(super) fn refuse_provider_part_turning_visible(&mut self, st: &St) -> Result<(), Signal> {
        // A new host record: the reads of its Provider chain (blank line reads of the Provider and of the linked parts, the
        // snapshots of ProviderLinkedPartsMatrix.al Provider_*_OpenNewSetKeyClose, Provider_TwoLinkedParts_OpenNew) are pinned but
        // not implemented. Refused as the page opens, before the test observed anything.
        let (new_host, parts) = {
            let s = st.borrow();
            (
                s.draft && s.host.is_none(),
                s.parts
                    .iter()
                    .chain(s.hidden_parts.iter())
                    .map(|(i, _)| *i)
                    .collect::<Vec<_>>(),
            )
        };
        if new_host && parts.into_iter().any(|i| self.part_in_provider_chain(st, i)) {
            return unsupported(
                "a new host record with a Provider chain of parts (BC's blank reads of the chain are pinned but not implemented)",
            );
        }
        let hidden = st.borrow().hidden_parts.clone();
        let card = self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Card);
        for (i, c) in hidden {
            // On a Card host the parts that stay hidden are refused too: a later GoToKey back to a row reads the parts in an order
            // that is pinned (Provider_BothHidden_OpenViewAndGoToKey `h1_again`) but not implemented, and a refusal that late
            // would leave the observations of the test behind. Refused as the page opens.
            if c.borrow().open
                && !self.part_eliminated(st, i)
                && self.part_in_provider_chain(st, i)
                && (card || self.control_visible(st, i)?)
            {
                return unsupported(
                    "a part of a Provider chain that a Visible expression hid as the host opened and that shows after its first row (BC's extra reads are pinned but not implemented)",
                );
            }
        }
        Ok(())
    }

    /// A part of a Provider chain hidden by an expression that shows no row and is read again after a host move: BC reads its rows
    /// last to first and then from the first (ProviderLinkedPartsMatrix.al Provider_BothHidden_OpenViewAndGoToKey `h1_again`
    /// `lines-row(20) | detail-row(twenty) | lines-row(10)`). Not implemented.
    pub(super) fn refuse_provider_part_back_walk(&self, st: &St, i: usize, child: &St) -> Result<(), Signal> {
        let hidden = st.borrow().hidden_parts.iter().any(|(h, _)| *h == i);
        if hidden && child.borrow().open && child.borrow().cursor == Cursor::None && self.part_in_provider_chain(st, i)
        {
            return unsupported(
                "a Provider-chain part hidden by a Visible expression that shows no row, read again after a host move (BC reads its rows last to first; pinned but not implemented)",
            );
        }
        Ok(())
    }
}
