//! The page's xRec while a TestPage SetValue validates a control bound to a field of Rec, and after it (al-runner issue 7403).
//!
//! One validation step is the table's validation, the page's OnBeforeValidate triggers, the control's OnValidate and the page
//! extensions' OnAfterValidate. Real BC, on the pinned pages:
//! - Every page trigger of the step sees one xRec, taken when the step starts: Rec as it was before the new value when the value
//!   changes the field, else the xRec the page held. A `CurrPage.Update()`, `Update(true)` or `SaveRecord()` inside the step
//!   stores the row but leaves that xRec as it is, so a page extension's OnAfterValidate sees the same xRec as the base OnValidate
//!   (pages/currpage/Y7403XRecAfterCurrPageUpdate.al:355-526, list and card, F0-F5; XRecAfterUpdateInPart.al:432-482 a stored
//!   line of a ListPart, :484-534 and :536-586 new lines; XRecAfterUpdateNewRow.al:454-683 new rows of a list and a card).
//! - After the step the page holds the row the step stored as xRec, or the xRec it held when nothing was stored. A second value
//!   then sees Rec before it as xRec (Y7403XRecAfterCurrPageUpdate.al:529-543 F0 `second set 5 -> 6` `xrec=5`), and the same value
//!   again the held xRec (:542 `set to the held value 6` `xrec=1`).
//! - `CurrPage.Update(false)` stores nothing and the edit of the step does not stay: once the step ends Rec is the stored row
//!   again, or the row before the step on a new row nothing stored (Y7403XRecAfterCurrPageUpdate.al:590-604 F3 `xrec=1` and
//!   `equal=Yes`; XRecAfterUpdateInPart.al:624-636 `Part_EmptyDocument_F3_Then_F2`: the 7 of F3 is not in `10000: 0 0 5 0 0`;
//!   XRecAfterUpdateNewRow.al:493-501 and :649-657).
//!
//! Pinned pages (`validate_xrec_pinned_page`): a List or a Card opened on its own, and a ListPart with AutoSplitKey and DelayedInsert
//! inside a Document page (UpdatePropagation not Both) or opened on its own. On those a row stored after `CurrPage.Update(false)` in the same step is refused,
//! and `CurrPage.Update(false)` on a row that held an unsaved edit before the step keeps the edit as before: no probe shows what
//! stays then (a probe is queued).
//! On every other page the step keeps the former behavior: the page triggers see the xRec the page holds, a save inside the step
//! makes the stored row xRec at once, and `CurrPage.Update(false)` keeps the edit. The passing corpus cases of those pages run
//! through it (pages/parts/QjPartPropagation.al QJ_*, QL_*, QN_*, QT_*: a part's Update(false) whose edit stays;
//! pages/actions/Y45SaveBeforeAction.al NextField_*, pages/modify-denied/Y80ModifyDenied.al and pages/extensions/
//! Y65CodeChangeNextField.al: a second value on a row of a Worksheet or a Document). Only a row stored inside the step
//! that a page extension's OnAfterValidate would read is refused there, the shape of al-runner issue 7403.

use super::*;

/// What the control validation step that runs now asked of the page's xRec.
#[derive(Debug, Clone)]
pub(crate) struct ValidateStep {
    /// The page is one the corpus pins (see the module comment).
    pinned: bool,
    /// The control has a page extension's OnAfterValidate.
    ext_after: bool,
    /// The row held an unsaved edit when the step started.
    was_dirty: bool,
    /// The xRec the page held when the step started.
    held: Vec<Value>,
    /// Rec when the step started, before the table's validation.
    before: Vec<Value>,
    /// `CurrPage.Update(false)` ran in the step.
    update_false: bool,
}

impl Session {
    /// Whether `st` is a page the corpus pins the validation step's xRec on (see the module comment).
    fn validate_xrec_pinned_page(&self, st: &St) -> Result<bool, Signal> {
        if !self.has_source(st) || self.is_request_page(st) {
            return Ok(false);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page_type = |o: usize| engine.object_facts[o].page_type.clone();
        let host = st.borrow().host.as_ref().map(|(h, c)| h.upgrade().map(|h| (h, *c)));
        match (page_type(object), host) {
            (Some(al_ir::PageType::List | al_ir::PageType::Card), None) => Ok(true),
            (Some(al_ir::PageType::Other(o)), host) if o == "listpart" => {
                let on = |name: &str| -> Result<bool, Signal> {
                    Ok(effective_prop(&engine, object, name)?.is_some_and(|v| v.trim().eq_ignore_ascii_case("true")))
                };
                if !(on("AutoSplitKey")? && on("DelayedInsert")?) {
                    return Ok(false);
                }
                Ok(match host {
                    None => true,
                    // With UpdatePropagation = Both the typed new row stays after Update(false) (pages/parts/QjPartPropagation.al
                    // QL_Both_NewPartRow_PartModifiesHost `part-qty-after-part` 5.00): the probe's part has the default.
                    Some(Some((h, c))) => {
                        let both = matches!(page_of(&engine, &h).controls[c].prop("updatepropagation"),
                            Some(ir::PropVal::Lit(v)) if v.eq_ignore_ascii_case("both"));
                        !both
                            && h.borrow().host.is_none()
                            && page_type(h.borrow().object as usize) == Some(al_ir::PageType::Document)
                    }
                    Some(None) => false,
                })
            }
            _ => Ok(false),
        }
    }

    /// The step starts its page triggers: the table's validation of field `pos` ran on Rec, which was `before`. On a pinned page
    /// the page's xRec becomes `before` when the value changed the field; elsewhere a step where that would differ from the xRec
    /// the page holds is refused.
    pub(super) fn validate_step_begin(
        &mut self,
        st: &St,
        pos: usize,
        before: &[Value],
        was_dirty: bool,
        ext_after: bool,
    ) -> Result<(), Signal> {
        let held = self.page_xrec_fields(st);
        let changed = {
            let rec = self.page_rec(st);
            let now = rec.borrow().fields[pos..=pos].to_vec();
            !Self::same_values(&now, &before[pos..=pos])
        };
        let pinned = self.validate_xrec_pinned_page(st)?;
        if pinned && changed {
            self.set_xrec_fields(st, before.to_vec());
        }
        st.borrow_mut().validate_step = Some(ValidateStep {
            pinned,
            ext_after,
            was_dirty,
            held,
            before: before.to_vec(),
            update_false: false,
        });
        Ok(())
    }

    /// A save of `CurrPage.Update()`, `Update(true)` or `SaveRecord()` inside the step: whether the page keeps its xRec until
    /// the step ends (the stored row becomes xRec then, `after_validate_rows`). Refuses the shapes no probe shows.
    pub(super) fn validate_step_keeps_xrec(&self, st: &St) -> Result<bool, Signal> {
        let s = st.borrow();
        let Some(step) = s.validate_step.as_ref() else {
            return Ok(false);
        };
        if step.update_false {
            return unsupported(
                "a row stored after CurrPage.Update(false) in the same control validation (what stays of the edit is not pinned)",
            );
        }
        if !step.pinned && step.ext_after {
            return unsupported(
                "a row stored inside a control validation that a page extension's OnAfterValidate reads, outside a List, a Card and \
                 a ListPart of a Document (which xRec it sees is not pinned)",
            );
        }
        Ok(step.pinned)
    }

    /// `CurrPage.Update(false)` inside the step: the edit does not stay once the step ends (`validate_step_end`).
    pub(super) fn validate_step_update_false(&mut self, st: &St) {
        let mut s = st.borrow_mut();
        if let Some(step) = s.validate_step.as_mut() {
            // On a row that held an unsaved edit before the step the edit stays, as before this rule: what real BC keeps of the
            // earlier edit is not pinned (a probe is queued), and the passing case that reaches it observes neither
            // (pages/currpage/TestPageUpdateGoneRow.al:538 UnsavedKeyedRow_OnValidateUpdateFalse_Trace).
            step.update_false = step.pinned && !step.was_dirty;
        }
    }

    /// The step ended with `result`: on success see `validate_step_end`; after an error nothing of the step stays.
    pub(super) fn validate_step_close(
        &mut self,
        st: &St,
        result: Result<(), Signal>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        match result {
            Ok(()) => self.validate_step_end(st),
            Err(e) => {
                st.borrow_mut().validate_step = None;
                Err(e)
            }
        }
    }

    /// After a SetValue validated without an error: the row as typed is the current row the next OnAfterGetCurrRecord sees as
    /// xRec (corpus bump 41 pages/parts/QjPartPropagation.al QJ_Both_PartModifiesHost_ThenHostSave `L.Qty.V(5 x=1) ...
    /// L.AGCR(10000 q=1 x.q=5)`); then Rec goes back to `discard` after `CurrPage.Update(false)` (`validate_step_discard`).
    pub(super) fn validated_row_typed(
        &mut self,
        st: &St,
        rec_bound: bool,
        discard: Option<Vec<Value>>,
    ) -> Result<(), Signal> {
        if rec_bound && !st.borrow().draft && st.borrow().cursor == Cursor::Row {
            let typed = self.page_rec(st).borrow().fields.clone();
            st.borrow_mut().agcr_xrec = Some(typed);
        }
        match discard {
            Some(row) => self.validate_step_discard(st, row),
            None => Ok(()),
        }
    }

    /// The step ended without an error: the page holds its xRec again. Returns the row Rec goes back to after
    /// `CurrPage.Update(false)` (`validate_step_discard`): the stored row, or the row before the step when nothing is stored.
    fn validate_step_end(&mut self, st: &St) -> Result<Option<Vec<Value>>, Signal> {
        let Some(step) = st.borrow_mut().validate_step.take() else {
            return Ok(None);
        };
        if !step.pinned {
            return Ok(None);
        }
        self.set_xrec_fields(st, step.held);
        if !step.update_false {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        Ok(Some(self.stored_row(&rec)?.unwrap_or(step.before)))
    }

    /// After `CurrPage.Update(false)` in the step, once the typed row was taken as the next OnAfterGetCurrRecord's xRec
    /// (pages/parts/QjPartPropagation.al QJ_Default_PartModifiesHost_ThenHostSave `L.AGCR(10000 q=1 x.q=5)`): Rec is `row` again.
    /// A stored row holds no edit any more; a new row nothing stored keeps the page on it, as typed before
    /// (XRecAfterUpdateNewRow.al:571-579 `List_NewRow_FieldFirst_F3`, XRecAfterUpdateInPart.al:624-636
    /// `Part_EmptyDocument_F3_Then_F2`).
    fn validate_step_discard(&mut self, st: &St, row: Vec<Value>) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_some() {
            st.borrow_mut().dirty = false;
        } else if !self.refresh_shows_row_triggers(st) {
            // The refresh of a new row runs no trigger here, of the page or its host, so the client stays on the new row and
            // nothing reads again (XRecAfterUpdateInPart.al:624-636 and :536-586 `Part_NewLine_FieldFirst_F3`: the next
            // SetValue lands on that row, which holds no edit).
            let mut s = st.borrow_mut();
            s.dirty = false;
            s.refresh = false;
            s.refresh_by_update = false;
            s.refresh_update_false = false;
        }
        rec.borrow_mut().fields = row;
        Ok(())
    }

    /// Whether the page or its host declares a row trigger a refresh runs.
    fn refresh_shows_row_triggers(&self, st: &St) -> bool {
        let triggers = |p: &St| {
            ["onaftergetrecord", "onaftergetcurrrecord"]
                .iter()
                .any(|t| self.declares(p, t))
        };
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        triggers(st) || host.is_some_and(|h| triggers(&h))
    }
}
