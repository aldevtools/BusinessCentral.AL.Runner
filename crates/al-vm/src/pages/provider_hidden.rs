//! Parts of a Provider chain that a Visible expression hid as the host opened (testpage/parts/ProviderLinkedPartsMatrix.al).
//! What the runner reads for them is derived from that file's expectations; the shapes it does not pin are refused as the page
//! opens, before the test observed anything.

use super::*;

/// The refusal of a part a Visible expression hid that a field's OnValidate turns visible.
const VALIDATE_TURN: &str = "an OnValidate that makes a part hidden by a Visible expression visible (not pinned)";

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

    /// Whether `st` is a part declared in the host's `area(FactBoxes)` that is in a Provider chain: its blank line reads once, not
    /// three times (testpage/parts/ProviderLinkedPartsMatrix.al Provider_PartsAddedByPageExtensionInFactBoxes_OpenNew
    /// `lines-row(0) | detail-row()`, against three for a part of the Content area, Provider_BothVisible_OpenNewSetKeyClose).
    pub(super) fn chain_part_in_factbox(&self, st: &St) -> bool {
        let Some((host, idx)) = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)))
        else {
            return false;
        };
        let engine = self.engine();
        page_of(&engine, &host).controls[idx].factbox && self.part_in_provider_chain(&host, idx)
    }

    /// The FactBox parts of a Provider chain that the refill after the host's insert skipped read their blank line now, as the key
    /// typed into the host reaches them (Provider_PartsAddedByPageExtensionInFactBoxes_OpenNew `keyed` `lines-row(0)`).
    pub(super) fn read_chain_parts_skipped_at_refill(&mut self, st: &St) -> Result<(), Signal> {
        let pending: Vec<(usize, St)> = Self::open_parts_with_hidden(st)
            .into_iter()
            .filter(|(_, c)| c.borrow().chain_refill == 2)
            .collect();
        for (i, c) in pending {
            c.borrow_mut().chain_refill = 0;
            let linked = {
                let engine = self.engine();
                page_of(&engine, st).controls[i]
                    .part_link
                    .as_ref()
                    .is_some_and(|l| l.provider.is_some())
                    && self.link_by_fields_only(st, i)
            };
            // A linked part whose Provider's row did not change keeps what it shows.
            self.part_sync_keeps_same_link = linked.then_some(true);
            let r = self.sync_part(st, i, &c);
            self.part_sync_keeps_same_link = None;
            r?;
        }
        Ok(())
    }

    /// Whether the SubPageLink of part `i` has only `field` terms: a `field(filter(...))` or `const` term with a blank value sets no
    /// filter, and the part reads again whatever the Provider's row was (pages/link-filter/Y37LinkFilterTests.al:316
    /// LT_C311_Provider_RecordIdFilter_Blank `EP.c 1` twice).
    pub(super) fn link_by_fields_only(&self, st: &St, i: usize) -> bool {
        let engine = self.engine();
        page_of(&engine, st).controls[i]
            .part_link
            .as_ref()
            .is_some_and(|l| l.entries.iter().all(|e| matches!(e, ir::LinkEntry::Field(..))))
    }

    /// The shapes where "a FactBox part of a Provider chain reads one blank row" and "a part a page extension added reads one blank
    /// row" disagree: a Provider part declared in the FactBoxes area of the host page itself, and a chain part a page extension
    /// added outside FactBoxes. The corpus pins only the case where both hold (Provider_PartsAddedByPageExtensionInFactBoxes_OpenNew),
    /// so these are refused as the page opens (probe spec: probe-provider-factbox-vs-extension).
    pub(super) fn refuse_factbox_vs_extension_chain(&self, st: &St) -> Result<(), Signal> {
        let (parts, object) = {
            let s = st.borrow();
            (
                s.parts
                    .iter()
                    .chain(s.hidden_parts.iter())
                    .map(|(i, _)| *i)
                    .collect::<Vec<_>>(),
                s.object as usize,
            )
        };
        let engine = self.engine();
        let page = page_of(&engine, st);
        for i in parts {
            let c = &page.controls[i];
            let extension = c.owner.is_some_and(|o| o != object);
            if self.part_in_provider_chain(st, i)
                && ((c.factbox && !extension && self.part_is_provider(st, i)) || (extension && !c.factbox))
            {
                return unsupported(
                    "a Provider-chain part that is a Provider declared in the base page's FactBoxes, or added by a page extension outside FactBoxes (open question: whether BC's one blank read follows the FactBoxes area or the page extension; probe pending)",
                );
            }
        }
        Ok(())
    }

    /// Whether part `i` is the Provider of another part.
    fn part_is_provider(&self, st: &St, i: usize) -> bool {
        let engine = self.engine();
        page_of(&engine, st)
            .controls
            .iter()
            .any(|c| c.part_link.as_ref().is_some_and(|l| l.provider == Some(i)))
    }

    /// Parts of a Provider chain that a Visible expression hid as the host opened and that show after its first row. When the
    /// Provider part turns visible too, BC reads blank before it reads the rows: the Provider part its blank line twice on an
    /// editable host (three times on a new host), then each linked part that turns visible one blank row
    /// (ProviderLinkedPartsMatrix.al Provider_BothVisible_OpenViewAndGoToKey `host-row(H1) | detail-row() | lines-row(10)`,
    /// Provider_BothVisible_OpenEditNextPreviousGoToRecord `lines-row(0) | lines-row(0) | detail-row() | lines-row(10)`,
    /// Provider_OnlyLinkedHidden_OpenEditNextPreviousGoToRecord `lines-row(0) | lines-row(0) | lines-row(10)`,
    /// Provider_TwoLinkedParts_OpenViewAndEmptyProvider `detail-row() | detail-row() | lines-row(10)`,
    /// Provider_PageRun_OnRowWithoutLines `lines-row(0) | lines-row(0) | detail-row() | lines-row(0) | lines-row(0) | lines-row(0)`,
    /// Provider_BothVisible_OpenNewSetKeyClose `lines-row(0) x3 | detail-row() | lines-row(0) x6`). A Provider part that stays hidden,
    /// or a linked part that turns visible without it, reads nothing extra (Provider_OnlyProviderHidden_OpenViewAndGoToKey).
    pub(super) fn read_blank_before_parts_turning_visible(&mut self, st: &St) -> Result<(), Signal> {
        let hidden = st.borrow().hidden_parts.clone();
        let mut turning: Vec<(usize, St)> = Vec::new();
        for (i, c) in hidden {
            if c.borrow().open
                && !self.part_eliminated(st, i)
                && self.part_in_provider_chain(st, i)
                && self.control_visible(st, i)?
            {
                turning.push((i, c));
            }
        }
        turning.sort_by_key(|(i, _)| *i);
        let providers: Vec<usize> = turning
            .iter()
            .map(|(i, _)| *i)
            .filter(|i| self.part_is_provider(st, *i))
            .collect();
        if providers.is_empty() {
            return Ok(());
        }
        let new_host = {
            let s = st.borrow();
            s.draft && s.host.is_none()
        };
        let engine = self.engine();
        let provider_of = |i: usize| {
            page_of(&engine, st).controls[i]
                .part_link
                .as_ref()
                .and_then(|l| l.provider)
        };
        for (i, child) in &turning {
            if providers.contains(i) {
                if child.borrow().new_line {
                    self.blank_runs_of_part(child, if new_host { 3 } else { 2 })?;
                }
            } else if provider_of(*i).is_some_and(|p| providers.contains(&p)) {
                self.sync_part(st, *i, child)?;
            }
        }
        Ok(())
    }

    /// Runs the blank line of `child` `runs` times and leaves it unloaded, as the refill of the part finds it.
    fn blank_runs_of_part(&mut self, child: &St, runs: usize) -> Result<(), Signal> {
        let saved = {
            let s = child.borrow();
            (
                s.placeholder_line,
                s.split_burned,
                s.typed_before_key,
                s.split_starts_low,
                s.empty_part_xrec.clone(),
            )
        };
        let r = self.land_on_part_open_line_runs(child, runs);
        let mut s = child.borrow_mut();
        s.cursor = Cursor::None;
        s.draft = false;
        s.dirty = false;
        s.window = Vec::new();
        (
            s.placeholder_line,
            s.split_burned,
            s.typed_before_key,
            s.split_starts_low,
            s.empty_part_xrec,
        ) = saved;
        drop(s);
        r
    }

    /// A Provider part that showed no row and is read again after the host moved onto a row: BC reads its rows last to first, so
    /// it stands on its last row, and the linked parts follow (ProviderLinkedPartsMatrix.al Provider_BothHidden_OpenViewAndGoToKey
    /// `h1_again` `lines-row(20) | detail-row(twenty) | lines-row(10)`, Provider_BothVisible_OpenEditNextPreviousGoToRecord
    /// `previous`).
    pub(super) fn chain_part_back_walk(&self, st: &St, i: usize, child: &St) -> bool {
        child.borrow().open
            && child.borrow().cursor == Cursor::None
            && self.part_is_provider(st, i)
            && !(self.declares(child, "onfindrecord") || self.declares(child, "onnextrecord"))
    }

    /// Whether part `st` takes its link from a Provider part that stands on no row.
    pub(super) fn provider_shows_no_row(&self, st: &St) -> bool {
        let Some((host, idx)) = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)))
        else {
            return false;
        };
        let engine = self.engine();
        let has_provider = page_of(&engine, &host).controls[idx]
            .part_link
            .as_ref()
            .is_some_and(|l| l.provider.is_some());
        has_provider
            && self
                .link_page(&host, idx)
                .is_some_and(|p| !matches!(p.borrow().cursor, Cursor::Row))
    }

    /// The Visible values of the parts that run record triggers, shown or hidden at open, on a page without a Provider link, as
    /// (control, visible), for `read_hidden_parts_the_action_showed`.
    pub(super) fn unchained_hidden_part_visibility(&mut self, st: &St) -> Result<Vec<(usize, bool)>, Signal> {
        if self.page_has_provider_link(st) {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for (i, c) in Self::open_parts_with_hidden(st) {
            if !self.part_eliminated(st, i) && self.runs_record_triggers(&c) {
                out.push((i, self.control_visible(st, i)?));
            }
        }
        Ok(out)
    }

    /// A part that the action's body made visible (its Visible expression false before the body, true after it). Pinned: the
    /// first time a part without a repeater that a Visible expression hid at open shows (`PageState::seen_visible`), on a
    /// top-level Document page that stands on a stored row: it reads its row once more before the host's refresh (testpage/parts/
    /// HiddenPartExtensionVariable.al:632 Hidden_OpenEdit_FlipTheVariable_ThenActions `flip` `act-flip(Yes) fb-agr(H1) fb-agcr(H1)
    /// host-agcr(H1) ext-agcr(H1) fb-agr(H1) fb-agcr(H1)`). Every other turn is refused: a part that showed before, a part shown
    /// at open, a part with a repeater, another host (a Card too: the pin is a Document page), a host on a new or no row. A
    /// Visible value the host's own triggers change is `note_hidden_part_found_visible`.
    pub(super) fn read_hidden_parts_the_action_showed(
        &mut self,
        st: &St,
        before: &[(usize, bool)],
    ) -> Result<(), Signal> {
        for &(i, was_visible) in before {
            if was_visible || !self.control_visible(st, i)? {
                continue;
            }
            let hidden = st
                .borrow()
                .hidden_parts
                .iter()
                .find(|(h, _)| *h == i)
                .map(|(_, c)| c.clone());
            let document_on_stored_row = {
                let s = st.borrow();
                s.cursor == Cursor::Row
                    && !s.draft
                    && s.host.is_none()
                    && self.engine.object_facts[s.object as usize].page_type == Some(al_ir::PageType::Document)
            };
            let pinned = hidden.filter(|c| {
                c.borrow().open && !c.borrow().seen_visible && document_on_stored_row && !self.has_repeater(c)
            });
            let Some(child) = pinned else {
                return unsupported(
                    "an action that makes a part visible that showed before, a part shown at open, with a repeater, on a host other than a Document page, or on a host without a stored row (not pinned)",
                );
            };
            {
                let mut c = child.borrow_mut();
                c.seen_visible = true;
                c.visible_at_sync = true;
            }
            self.sync_part(st, i, &child)?;
        }
        Ok(())
    }

    /// A part a Visible expression hid at open, on a page without a Provider link, that runs record triggers and was hidden at
    /// the host's last refill but is visible now, without the action's read: its host's own trigger set the expression (the
    /// Customer Card's Pending Approval FactBox, `Visible = OpenApprovalEntriesExistCurrUser`, set by the card's
    /// OnAfterGetCurrRecord: Tests-Workflow WF Demo Customer Approval, 134205). It reads like a shown part, which no probe pins
    /// for that turn: marks the assumption `hidden-part-shown-outside-action`. A turn made inside a SetValue (a field's
    /// OnValidate) is needed by no cited test: refused, here and after the SetValue (`refuse_hidden_part_shown_by_validate`).
    pub(super) fn note_hidden_part_found_visible(&mut self, st: &St, parts: &[(usize, St)]) -> Result<(), Signal> {
        if st.borrow().hidden_parts.is_empty() || self.page_has_provider_link(st) {
            return Ok(());
        }
        for (i, c) in parts {
            let hidden = st.borrow().hidden_parts.iter().any(|(_, h)| Rc::ptr_eq(h, c));
            if !hidden || !c.borrow().open || !self.runs_record_triggers(c) {
                continue;
            }
            let visible = self.control_visible(st, *i)?;
            if visible && !c.borrow().visible_at_sync {
                if st.borrow().in_set_value {
                    return unsupported(VALIDATE_TURN);
                }
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::HiddenPartShownOutsideAction);
            }
            let mut c = c.borrow_mut();
            c.visible_at_sync = visible;
            c.seen_visible |= visible;
        }
        Ok(())
    }

    /// After a TestPage SetValue on `st`: a part a Visible expression hid that the validation turned visible is refused
    /// (see `note_hidden_part_found_visible`).
    pub(super) fn refuse_hidden_part_shown_by_validate(&mut self, st: &St) -> Result<(), Signal> {
        if st.borrow().hidden_parts.is_empty() || self.page_has_provider_link(st) {
            return Ok(());
        }
        let hidden = st.borrow().hidden_parts.clone();
        for (i, c) in hidden {
            if c.borrow().open
                && !c.borrow().visible_at_sync
                && self.runs_record_triggers(&c)
                && self.control_visible(st, i)?
            {
                return unsupported(VALIDATE_TURN);
            }
        }
        Ok(())
    }

    /// Whether a part of `st`'s page is linked through a Provider.
    pub(super) fn page_has_provider_link(&self, st: &St) -> bool {
        let engine = self.engine();
        page_of(&engine, st)
            .controls
            .iter()
            .any(|c| c.part_link.as_ref().is_some_and(|l| l.provider.is_some()))
    }

    /// Whether part page `st` runs AL code when it reads a row: its or a page extension's OnAfterGetRecord or
    /// OnAfterGetCurrRecord, or a subscriber of their events.
    pub(super) fn runs_record_triggers(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        ["onaftergetrecord", "onaftergetcurrrecord"]
            .iter()
            .any(|t| self.declares_or_ext_declares(st, t))
            || ["onaftergetrecordevent", "onaftergetcurrrecordevent"]
                .iter()
                .any(|t| self.engine().events.is_subscribed(object, t))
    }
}
