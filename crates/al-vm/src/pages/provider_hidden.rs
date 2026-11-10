//! Parts of a Provider chain that a Visible expression hid as the host opened (testpage/parts/ProviderLinkedPartsMatrix.al).
//! What the runner reads for them is derived from that file's expectations; the shapes it does not pin are refused as the page
//! opens, before the test observed anything.

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
}
