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
//! Pinned pages (`validate_xrec_pinned_page`), matched by the shape the probes have: a List or a Card opened on its own with no
//! page trigger (its own or an extension's), no part, no DelayedInsert and a stored source table, and a ListPart with AutoSplitKey
//! and DelayedInsert, no page trigger and a stored source table, opened on its own or inside a Document page opened on its own
//! that has neither (UpdatePropagation not Both). On those a row stored after `CurrPage.Update(false)` in the same step is
//! refused, and `CurrPage.Update(false)` on a row that held an unsaved edit before the step keeps the edit as before: no probe
//! shows what stays then (a probe is queued). A save inside the step runs with the xRec the page had before this rule (the row it
//! read, or the row a save stored): what the table's modify triggers see then is not pinned. After an error in the step the page
//! holds its former xRec again, so an earlier unsaved edit is still saved.
//! On every other page the step keeps the former behavior: the page triggers see the xRec the page holds, a save inside the step
//! makes the stored row xRec at once, and `CurrPage.Update(false)` keeps the edit. The passing corpus cases of those pages run
//! through it (pages/parts/QjPartPropagation.al QJ_*, QL_*, QN_*, QT_*: a part's Update(false) whose edit stays;
//! pages/actions/Y45SaveBeforeAction.al NextField_*, pages/modify-denied/Y80ModifyDenied.al and pages/extensions/
//! Y65CodeChangeNextField.al: a second value on a row of a Worksheet or a Document). Only a row stored inside the step
//! whose xRec a page extension's OnAfterValidate reads is refused there, the shape of al-runner issue 7403.

use super::*;

/// What the control validation step that runs now asked of the page's xRec.
#[derive(Debug, Clone)]
pub(crate) struct ValidateStep {
    /// The page is one the corpus pins (see the module comment).
    pinned: bool,
    /// The page extensions' OnAfterValidate of the control, as (extension object, routine).
    ext_after: Vec<(usize, usize)>,
    /// The row held an unsaved edit when the step started.
    was_dirty: bool,
    /// The xRec the page held when the step started.
    held: Vec<Value>,
    /// Rec when the step started, before the table's validation.
    before: Vec<Value>,
    /// The xRec a save inside the step runs with: the held xRec, then the row the last save stored (as before this rule).
    save_xrec: Vec<Value>,
    /// `CurrPage.Update(false)` ran in the step.
    update_false: bool,
}

impl Session {
    /// Whether `st` is a page the corpus pins the validation step's xRec on (see the module comment). The pinned pages declare
    /// no page trigger and no SourceTableTemporary, and the List and Card pages no part and no DelayedInsert (Y7403XRecAfterCurrPageUpdate.al:51-316, XRecAfterUpdateNewRow.al:57-323,
    /// XRecAfterUpdateInPart.al:105-279); a page with one keeps the former behavior.
    fn validate_xrec_pinned_page(&self, st: &St) -> Result<bool, Signal> {
        if !self.has_source(st) || self.is_request_page(st) {
            return Ok(false);
        }
        // A temporary Worksheet whose rows a procedure set before the open: Update(false) in the OnValidate drops the edit
        // (pages/run-modal/PageVarOpenRow.al:946 TempWs_SecondaryKeyView_FilledBeforeRun_Run `row edited` `B|name B`, :974
        // run modally, :1002; `open_temp_ws_preloaded` holds the page's shape).
        if st.borrow().temp_ws_preloaded {
            return Ok(true);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page_type = |o: usize| engine.object_facts[o].page_type.clone();
        let on = |o: usize, name: &str| -> Result<bool, Signal> {
            Ok(effective_prop(&engine, o, name)?.is_some_and(|v| v.trim().eq_ignore_ascii_case("true")))
        };
        // No page trigger of its own or of an extension, and a stored source table.
        let quiet = |p: &ir::Page, o: usize| -> Result<bool, Signal> {
            Ok(p.triggers.is_empty() && p.ext_triggers.is_empty() && !on(o, "SourceTableTemporary")?)
        };
        let page = page_of(&engine, st);
        if !quiet(page, object)? {
            return Ok(false);
        }
        let host = st.borrow().host.as_ref().map(|(h, c)| h.upgrade().map(|h| (h, *c)));
        match (page_type(object), host) {
            (Some(al_ir::PageType::List | al_ir::PageType::Card), None) => {
                Ok(!on(object, "DelayedInsert")? && !page.controls.iter().any(|c| c.kind == ir::ControlKind::Part))
            }
            (Some(al_ir::PageType::Other(o)), host) if o == "listpart" => {
                if !(on(object, "AutoSplitKey")? && on(object, "DelayedInsert")?) {
                    return Ok(false);
                }
                Ok(match host {
                    None => true,
                    // With UpdatePropagation = Both the typed new row stays after Update(false) (pages/parts/QjPartPropagation.al
                    // QL_Both_NewPartRow_PartModifiesHost `part-qty-after-part` 5.00): the probe's part has the default.
                    Some(Some((h, c))) => {
                        let host_page = page_of(&engine, &h);
                        let host_object = h.borrow().object as usize;
                        let both = matches!(host_page.controls[c].prop("updatepropagation"),
                            Some(ir::PropVal::Lit(v)) if v.eq_ignore_ascii_case("both"));
                        !both
                            && quiet(host_page, host_object)?
                            && h.borrow().host.is_none()
                            && page_type(host_object) == Some(al_ir::PageType::Document)
                    }
                    Some(None) => false,
                })
            }
            _ => Ok(false),
        }
    }

    /// The step starts its page triggers: the table's validation of field `pos` ran on Rec, which was `before`. On a pinned page
    /// the page's xRec becomes `before` when the value changed the field; elsewhere the page keeps the xRec it holds.
    pub(super) fn validate_step_begin(
        &mut self,
        st: &St,
        pos: usize,
        before: &[Value],
        was_dirty: bool,
        ext_after: &[(usize, usize)],
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
            ext_after: ext_after.to_vec(),
            was_dirty,
            save_xrec: held.clone(),
            held,
            before: before.to_vec(),
            update_false: false,
        });
        Ok(())
    }

    /// A save of `CurrPage.Update()`, `Update(true)` or `SaveRecord()` inside the step: the xRec the save runs with when the
    /// page keeps its xRec until the step ends (the stored row becomes xRec then, `after_validate_rows`), else `None`. Refuses
    /// the shapes no probe shows.
    pub(super) fn validate_step_save_xrec(&self, st: &St) -> Result<Option<Vec<Value>>, Signal> {
        let s = st.borrow();
        let Some(step) = s.validate_step.as_ref() else {
            return Ok(None);
        };
        if step.update_false {
            return unsupported(
                "a row stored after CurrPage.Update(false) in the same control validation (what stays of the edit is not pinned)",
            );
        }
        if step.pinned {
            return Ok(Some(step.save_xrec.clone()));
        }
        let engine = self.engine();
        let base = s.object as usize;
        if step
            .ext_after
            .iter()
            .any(|&(e, r)| reads_xrec(&engine.program, base, e, r))
        {
            return unsupported(
                "a row stored inside a control validation whose xRec a page extension's OnAfterValidate reads, outside a List, a \
                 Card and a ListPart of a Document without page triggers (which xRec it sees is not pinned)",
            );
        }
        Ok(None)
    }

    /// A save inside the step stored `stored`: the next save of the step runs with it as xRec (`validate_step_save_xrec`).
    pub(super) fn validate_step_saved(&self, st: &St, stored: &[Value]) {
        if let Some(step) = st.borrow_mut().validate_step.as_mut() {
            step.save_xrec = stored.to_vec();
        }
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

    /// The step ended with `result`: on success see `validate_step_end`; after an error nothing of the step stays, and the page
    /// holds its xRec again (so a later save or Close still writes an earlier unsaved edit, as before this rule; a row a save
    /// of the step stored becomes xRec in `after_validate_rows`).
    pub(super) fn validate_step_close(
        &mut self,
        st: &St,
        result: Result<(), Signal>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        match result {
            Ok(()) => self.validate_step_end(st),
            Err(e) => {
                let step = st.borrow_mut().validate_step.take();
                if let Some(step) = step.filter(|s| s.pinned) {
                    self.set_xrec_fields(st, step.held);
                }
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

/// Whether routine `r` of page extension `e` reads the page's xRec: it names xRec, passes it to a `var` parameter or calls a
/// procedure of its own object or of the base page `base` that does.
fn reads_xrec(program: &ir::Program, base: usize, e: usize, r: usize) -> bool {
    let objects = &program.objects;
    let base_slot = objects[base].xrec_slot;
    let mut todo = vec![(e, r)];
    let mut seen = std::collections::BTreeSet::new();
    while let Some((o, r)) = todo.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        let Some(routine) = objects.get(o).and_then(|x| x.routines.get(r)) else {
            continue;
        };
        let own_slot = objects[o].xrec_slot;
        let names = |p: &ir::Place| place_names_xrec(p, own_slot, base_slot);
        let refs = |args: &[ir::Arg]| args.iter().any(|a| matches!(a, ir::Arg::Ref(p) if names(p)));
        let mut found = false;
        ir::types::for_each_expr_post(routine.body.stmts(), &mut |x| match x {
            ir::Expr::Load(p) => found |= names(p),
            ir::Expr::Call {
                object,
                routine,
                instance,
                args,
                ..
            } => {
                found |= refs(args);
                let same_page = matches!(
                    instance.as_deref(),
                    None | Some(ir::Expr::ThisInstance | ir::Expr::BaseInstance)
                );
                if same_page && (*object == o || *object == base) {
                    todo.push((*object, *routine));
                }
            }
            ir::Expr::Builtin { receiver, args, .. } => {
                found |= refs(args) || receiver.as_deref().is_some_and(|a| refs(std::slice::from_ref(a)));
            }
            ir::Expr::InterfaceCall { args, .. } | ir::Expr::DotNet { args, .. } => found |= refs(args),
            _ => {}
        });
        if found {
            return true;
        }
    }
    false
}

/// Whether place `p` is xRec of the running object (`own`) or of the base page (`base`), or a part of it.
fn place_names_xrec(p: &ir::Place, own: Option<usize>, base: Option<usize>) -> bool {
    match p {
        ir::Place::Global(g) => Some(*g) == own,
        ir::Place::BaseGlobal(g) => Some(*g) == base,
        ir::Place::Field(inner, _) | ir::Place::Index(inner, _) | ir::Place::RecGlobal(inner, _, _) => {
            place_names_xrec(inner, own, base)
        }
        ir::Place::Local(_) | ir::Place::ExtGlobal(..) => false,
    }
}
