//! Parts a Visible expression hid as the host opened. Inside a Provider chain (testpage/parts/ProviderLinkedPartsMatrix.al) what
//! the runner reads is derived from that file's expectations; the shapes it does not pin are refused as the page opens. Outside
//! every chain: the turns to visible (`read_hidden_parts_the_action_showed`, which runs one pinned shape and refuses the
//! rest, and `note_hidden_part_found_visible`).

use super::*;

/// The page before an action's body ran, for the turn of a hidden part to visible (`Session::before_action_body`).
#[derive(Default)]
pub(super) struct BeforeActionBody {
    /// The Visible value of each part that runs record triggers, shown or hidden at open, as (control, visible). Empty on a
    /// page with a Provider link.
    visible: Vec<(usize, bool)>,
    /// Rec's fields when the host is the one of a pinned turn (`Session::first_turn_as_pinned`), else the first fact of
    /// the pinned Document host it does not have; `None` when no part was hidden.
    pinned_host_rec: Option<Result<Vec<Value>, &'static str>>,
    /// The host is the one of the journal shape before its measured turn (`Session::journal_turn_host_miss`), not the pinned
    /// Document: the session's commits before the action's body.
    journal: Option<u64>,
}

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

    /// A Provider part declared in the FactBoxes area of the host page itself, and a chain part a page extension added outside
    /// FactBoxes, on a new host. On a stored host these chains read as the pinned ones: the same rows and part triggers after
    /// OpenView, OpenEdit, GoToRecord, GoToKey, host Next and the Provider's Next and First (testpage/parts/
    /// ProviderPartOutsideFactBoxes.al ExtContent_*, BaseFactBoxes_*, BothExt_* `open`, `h1`, `h2`, `next`, `first`, `next-host`,
    /// BC 27.5, 28.5 and 29.0; corpus pull request 1324, al-runner issue 7748). On OpenNew the blank line reads follow the FactBoxes
    /// area and a Content chain does not insert on the typed key (`new`, `keyed`, `inserted`), which the runner does not model for
    /// these shapes, so a new host is refused as the page opens.
    pub(super) fn refuse_factbox_vs_extension_chain(&self, st: &St) -> Result<(), Signal> {
        if st.borrow().draft && self.chain_outside_pinned_areas(st) {
            return unsupported(
                "a new host with a Provider-chain part that is a Provider declared in the base page's FactBoxes, or added by a page extension outside FactBoxes (its blank line reads on OpenNew are not modelled)",
            );
        }
        Ok(())
    }

    /// Whether host `st` has a Provider chain of the shapes ProviderPartOutsideFactBoxes.al measures beyond
    /// ProviderLinkedPartsMatrix.al: a Provider part declared in the FactBoxes area of the host page itself (host `LT PX Host Base
    /// FB`), or a chain part a page extension added outside FactBoxes (hosts `LT PX Host Ext Content`, `LT PX Host Both Ext`).
    pub(super) fn chain_outside_pinned_areas(&self, st: &St) -> bool {
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
        parts.into_iter().any(|i| {
            let c = &page.controls[i];
            let extension = c.owner.is_some_and(|o| o != object);
            self.part_in_provider_chain(st, i)
                && ((c.factbox && !extension && self.part_is_provider(st, i)) || (extension && !c.factbox))
        })
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

    /// What `read_hidden_parts_the_action_showed` compares with after the body of an action of page `st`. `no_edit`: the page
    /// and its parts held no unsaved edit when the action was invoked (`holds_no_edit`, before the client saved anything).
    pub(super) fn before_action_body(&mut self, st: &St, no_edit: bool) -> Result<BeforeActionBody, Signal> {
        let mut before = BeforeActionBody::default();
        if self.page_has_provider_link(st) {
            return Ok(before);
        }
        for (i, c) in Self::open_parts_with_hidden(st) {
            if !self.part_eliminated(st, i) && self.runs_record_triggers(&c) {
                let visible = self.control_visible(st, i)?;
                c.borrow_mut().seen_visible |= visible;
                before.visible.push((i, visible));
            }
        }
        if before.visible.iter().any(|(_, visible)| !visible) {
            let mismatch = {
                let s = st.borrow();
                let document = self.engine.object_facts[s.object as usize].page_type == Some(al_ir::PageType::Document);
                [
                    (document, "the host page is not a Document"),
                    (s.opened_by_test, "the test did not open the host page itself"),
                    (s.mode == PageMode::Edit, "the host page was not opened with OpenEdit"),
                    (s.host.is_none(), "the host page is itself a part"),
                    (
                        s.cursor == Cursor::Row && !s.draft,
                        "the host page stands on no stored row",
                    ),
                    (no_edit, "the host page or a part held an unsaved edit"),
                ]
                .into_iter()
                .find_map(|(holds, reason)| (!holds).then_some(reason))
            };
            // A host outside the pinned Document may be the journal shape's (`journal_turn_host_miss`). A refusal names the
            // journal host's first missing fact on a Worksheet or a List, the Document's on any other page, as before.
            let journal_miss = match mismatch {
                Some(_) => self.journal_turn_host_miss(st)?,
                None => None,
            };
            before.journal = (mismatch.is_some() && journal_miss.is_none()).then_some(self.commits);
            before.pinned_host_rec = Some(match (mismatch, journal_miss) {
                (Some(_), Some(journal)) if self.journal_host_type(st) => Err(journal),
                (Some(reason), Some(_)) => Err(reason),
                _ => Ok(self.page_rec(st).borrow().fields.clone()),
            });
        }
        Ok(before)
    }

    /// A part that the action's body made visible (its Visible expression false before the body, true after it). One shape is
    /// pinned, and only that shape runs (`first_turn_as_pinned`): the part reads its row once more before the host's refresh
    /// (testpage/parts/HiddenPartExtensionVariable.al:632 Hidden_OpenEdit_FlipTheVariable_ThenActions `flip` `act-flip(Yes)
    /// fb-agr(H1) fb-agcr(H1) host-agcr(H1) ext-agcr(H1) fb-agr(H1) fb-agcr(H1)`). Every other turn by an action body is refused.
    /// A Visible value the host's own triggers change is `note_hidden_part_found_visible`.
    pub(super) fn read_hidden_parts_the_action_showed(
        &mut self,
        st: &St,
        before: &BeforeActionBody,
    ) -> Result<(), Signal> {
        for &(i, was_visible) in &before.visible {
            if was_visible || !self.control_visible(st, i)? {
                continue;
            }
            let child = match self.first_turn_as_pinned(st, i, before) {
                Ok(child) => child,
                Err(reason) => {
                    return unsupported(format!(
                        "an action that makes a part visible, other than the pinned shape (a Document page opened with OpenEdit, on a stored row without an unsaved edit, whose action body only sets the variable, shows a linked part without a repeater that was hidden since the page opened): {reason} (not pinned)"
                    ));
                }
            };
            {
                let mut c = child.borrow_mut();
                c.seen_visible = true;
                c.visible_at_sync = true;
            }
            match before.journal {
                Some(commits) => self.journal_turn(st, i, &child, commits)?,
                None => self.sync_part(st, i, &child)?,
            }
        }
        Ok(())
    }

    /// The part of control `i` when its turn to visible has every condition of the pin (HiddenPartExtensionVariable.al:632), else
    /// `None`:
    /// - the host is a top-level Document page the test opened with OpenEdit, on a stored row, and neither it nor a part held
    ///   an unsaved edit when the action was invoked (`before_action_body`);
    /// - the body did what the pinned one does, set the variable: the page stands on the same row with the same Rec, and
    ///   neither it nor a part was asked for CurrPage.Update or CurrPage.SaveRecord;
    /// - the part has no repeater, takes its rows through a SubPageLink of `field` terms, and a Visible expression hid it from
    ///   the open on (`PageState::seen_visible`).
    ///
    /// On the journal shape's host (`BeforeActionBody::journal`) the host facts are `journal_host_miss`, and the part is
    /// `journal_part_miss`'s (unlinked, with a repeater) instead of the linked part without one (HiddenPartJournalShape.al:537).
    ///
    /// `Err` names the first condition that does not hold, for the refusal.
    fn first_turn_as_pinned(&self, st: &St, i: usize, before: &BeforeActionBody) -> Result<St, &'static str> {
        let rec_before = match &before.pinned_host_rec {
            Some(Ok(rec)) => rec,
            Some(Err(reason)) => return Err(reason),
            None => return Err("no part was hidden before the action"),
        };
        let body_only_set_the_variable = {
            let s = st.borrow();
            s.open
                && s.cursor == Cursor::Row
                && !s.draft
                && !s.refresh
                && !s.save_record_called
                && Self::same_values(&self.page_rec(st).borrow().fields, rec_before)
                && Self::open_parts_with_hidden(st)
                    .iter()
                    .all(|(_, c)| !c.borrow().refresh && !c.borrow().save_record_called)
        };
        let linked_by_fields = before.journal.is_some() || {
            let engine = self.engine();
            page_of(&engine, st).controls[i]
                .part_link
                .as_ref()
                .is_some_and(|l| l.provider.is_none() && !l.entries.is_empty())
                && self.link_by_fields_only(st, i)
        };
        let Some(hidden) = st
            .borrow()
            .hidden_parts
            .iter()
            .find(|(h, _)| *h == i)
            .map(|(_, c)| c.clone())
        else {
            return Err("the part was not hidden as the page opened");
        };
        let never_shown = hidden.borrow().open && !hidden.borrow().seen_visible;
        if before.journal.is_some() {
            // The journal shape's part: unlinked, with a repeater (`journal_part_miss`).
            self.journal_part_miss(st, i, &hidden)?;
        }
        [
            (
                body_only_set_the_variable,
                "the action body did more than set the variable (moved, changed Rec, CurrPage.Update or SaveRecord)",
            ),
            (
                linked_by_fields,
                "the part has no SubPageLink of field terms, or takes its link from a Provider",
            ),
            (never_shown, "the part showed before"),
            (
                before.journal.is_some() || !self.has_repeater(&hidden),
                "the part has a repeater",
            ),
        ]
        .into_iter()
        .find_map(|(holds, reason)| (!holds).then_some(reason))
        .map_or(Ok(hidden), Err)
    }

    /// Whether page `st` and its parts hold no unsaved edit.
    pub(super) fn holds_no_edit(st: &St) -> bool {
        let s = st.borrow();
        !s.dirty && s.parts.iter().chain(&s.hidden_parts).all(|(_, c)| !c.borrow().dirty)
    }

    /// A part a Visible expression hid at open, on a page without a Provider link, that runs record triggers and was hidden at
    /// the host's last refill but is visible now, without the action's read: the host's own trigger or a field's OnValidate set
    /// the expression. Microsoft's tests need both turns: the Customer Card's Pending Approval FactBox (`Visible =
    /// OpenApprovalEntriesExistCurrUser`, set by the card's OnAfterGetCurrRecord; Tests-Workflow WF Demo Customer Approval,
    /// 134205) and a Name OnValidate that a workflow reverts (Tests-Workflow Workflow Additional Scenarios
    /// CustomerCardNameFieldRevertedByWorkflow, VendorCardNameFieldRevertedByWorkflow; WF Demo General Journal Batch
    /// RequestorCancelsPaymentRequestToDirectApprover). The part reads like a shown part, which no probe pins for these turns:
    /// marks the assumption `hidden-part-shown-outside-action`.
    /// The refill as the page opens (`opening`) only records that the part showed (`PageState::seen_visible`), so that a later
    /// turn by an action body is not taken for the first one; the assumption is marked at the next refill, as before.
    pub(super) fn note_hidden_part_found_visible(
        &mut self,
        st: &St,
        parts: &[(usize, St)],
        opening: bool,
    ) -> Result<(), Signal> {
        if st.borrow().hidden_parts.is_empty() || self.page_has_provider_link(st) {
            return Ok(());
        }
        for (i, c) in parts {
            let hidden = st.borrow().hidden_parts.iter().any(|(_, h)| Rc::ptr_eq(h, c));
            if !hidden || !(opening || c.borrow().open) || !self.runs_record_triggers(c) {
                continue;
            }
            let visible = self.control_visible(st, *i)?;
            let mut c = c.borrow_mut();
            c.seen_visible |= visible;
            if opening {
                continue;
            }
            if visible && !c.visible_at_sync {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::HiddenPartShownOutsideAction);
            }
            c.visible_at_sync = visible;
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
