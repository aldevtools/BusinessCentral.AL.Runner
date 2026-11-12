//! A new ListPart row after a page-variable control's OnValidate ran `CurrPage.Update()` on a Document host (al-runner issue
//! 7481): the part reads the row and the row before it, nothing more, and reads its row once more when the host closes, before it
//! saves (pages/parts/PartNewRowPageVariable.al:1218 Split_NewRow_RecordFieldsThenVariableWithUpdate `ActUpd` `... AGR 20000 |
//! AGR 10000`, `closed` `[AGR 20000]`; :1248 Split_NewRow_VariableWithUpdateFirst, a SetValue between the update and the close,
//! `closed` `[AGR 20000 | MOD 20000 1 | T-M 20000 1]`; :1470, :1500, :1722, :1752 with DelayedInsert).
//!
//! This module owns the scenario (`part_update_unsettled`), the mark it leaves (`PageState::reread_at_close`) and the read at
//! the close (`reread_part_at_close`, asked again there with the calls up to the close: `unsettled_close_pinned`). Every other
//! part row keeps the walk of ThPartUpdateCursor.al (`read_part_rows_around`) and reads nothing at the close.

use super::pinned_shape::{PartStep, PinnedPart, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;

/// The pages of the pin (PartNewRowPageVariable.al pages 62635..62637 on hosts 62640..62642), see `part_pages_are`: a Document
/// host with no trigger whose controls are fields of Rec and this part (:419); a ListPart with AutoSplitKey, with or without
/// DelayedInsert and PopulateAllFields (:335, :449, :564), whose triggers are OnAfterGetRecord, OnNewRecord, OnInsertRecord and
/// OnModifyRecord (:386..:410; no OnAfterGetCurrRecord); controls over fields of Rec without a trigger and over page variables,
/// those with or without OnValidate (:346, :359, :371); `SubPageLink = "Doc No." = field("No.")` (:437), no view, no filter but
/// the link's; no page extension, table extension or subscriber.
const PAGES: PinnedPart = PinnedPart {
    host_type: ir::PageType::Document,
    host_triggers: &[],
    part_triggers: &["onaftergetrecord", "onnewrecord", "oninsertrecord", "onmodifyrecord"],
    part_properties: &["autosplitkey", "delayedinsert", "populateallfields"],
    part_variable_controls: true,
    part_variable_validate: true,
    part_field_validate_update: false,
    part_expression_controls: false,
    unlinked: false,
    sibling_parts: false,
    user_filter: false,
};

impl Session {
    /// The pinned scenario, every fact required; anything else keeps `read_part_rows_around`:
    /// - the pages are `PAGES`; the part has AutoSplitKey and reads in its primary key, ascending, whose fields are the link's
    ///   and an Integer line number (:63); the test opened the host with OpenEdit (:1224);
    /// - `CurrPage.Update()` (it saves, not `Update(false)`); the view holds two rows and the part stands on the second, the
    ///   new row its `New()` started below the first (`PageState::after_new`);
    /// - the test's calls since the host opened (`unsettled_calls`), the running one the SetValue on the variable control.
    ///
    /// `rows` is the part's view from its start.
    pub(super) fn part_update_unsettled(&self, st: &St, rows: &[Vec<Value>], at: usize) -> Result<bool, Signal> {
        let shape = !st.borrow().refresh_update_false
            && rows.len() == 2
            && at == 1
            && self.unsettled_pages_pinned(st)?
            && matches!(self.unsettled_calls(st), Some((_, 0, false)));
        if !shape {
            return Ok(false);
        }
        let Some(anchor) = st.borrow().after_new.clone() else {
            return Ok(false);
        };
        let table = self.table(&self.page_rec(st))?;
        Ok(Self::same_key(
            &table.primary_key(&anchor),
            &table.primary_key(&rows[0]),
        ))
    }

    fn unsettled_pages_pinned(&self, st: &St) -> Result<bool, Signal> {
        Ok(Self::part_host_opened(st) == Some((true, PageMode::Edit, false, false))
            && self.has_auto_split_key(st)
            && !Self::reads_in_view_order(&self.page_rec(st))
            && self.part_pages_are(st, &PAGES)?
            && self.part_key_is_link_then_integer(st)?)
    }

    /// The calls of the pinned tests on the host and the part (reads of the part's fields stand between them): `GoToKey`,
    /// `New`, values typed into controls over fields of Rec, then a value typed into a control over a page variable that has an
    /// OnValidate trigger (the one that runs `CurrPage.Update()`), then values typed into fields of Rec, then the host's
    /// `Close`. Answers how many values were typed before the variable control, how many after it, and whether the host closes.
    /// Pinned: two before and none after (:1218 Type, Code, ActUpd; the existing test of this rule types one), none before and
    /// one after (:1248 ActUpd, Code). `None` for any other call, a call on the host between them, a second value in the
    /// variable control, or a write to the store between two calls (the test deleted the row).
    fn unsettled_calls(&self, st: &St) -> Option<(usize, usize, bool)> {
        use PageMethod as M;
        use PartStep::{Host, Own};
        use TestCall::{Page, Write};
        let steps = Self::part_steps(st)?;
        let [Host(Page(M::GoToKey)), Own(Page(M::New)), typed @ ..] = &steps[..] else {
            return None;
        };
        let (typed, closes) = match typed {
            [typed @ .., Host(Page(M::Close))] => (typed, true),
            _ => (typed, false),
        };
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut variable_at = None;
        for (n, step) in typed.iter().enumerate() {
            let Own(Write(control)) = step else {
                return None;
            };
            let c = &page.controls[*control];
            match c.binding {
                ir::Binding::Field(_) => {}
                ir::Binding::Var(_) if variable_at.is_none() && c.triggers.iter().any(|(t, _)| t == "onvalidate") => {
                    variable_at = Some(n)
                }
                _ => return None,
            }
        }
        let before = variable_at?;
        let after = typed.len() - before - 1;
        let pinned = match before {
            0 => after <= 1,
            1 | 2 => after == 0,
            _ => false,
        };
        pinned.then_some((before, after, closes))
    }

    /// The pages of PartNewRowPageVariable.al: a ListPart on a Document host, neither with a page extension or an event
    /// subscriber. The behaviors this file measured on a new or untouched part row of every host ask this (`dirty_step.rs`,
    /// `save_record_now`); other hosts and parts keep the rules before it. The reads after the update have their own, narrower
    /// scenario (`part_update_unsettled`).
    pub(super) fn new_row_part_pinned(&self, st: &St) -> bool {
        self.list_part_on_document_host(st)
            && self.unextended_unsubscribed(st)
            && st
                .borrow()
                .host
                .as_ref()
                .and_then(|(h, _)| h.upgrade())
                .is_some_and(|h| self.unextended_unsubscribed(&h))
    }

    /// The part read as `part_update_unsettled` pins: it reads its row once more at the close (`reread_part_at_close`).
    pub(super) fn read_part_rows_unsettled(&mut self, st: &St, rows: &[Vec<Value>], at: usize) -> Result<(), Signal> {
        self.read_part_rows_unsettled_walk(st, rows, at)?;
        st.borrow_mut().reread_at_close = true;
        Ok(())
    }

    /// At the host's close, before the part saves: a part that holds the mark, and whose host got no call since the update
    /// but the pinned ones (`unsettled_calls`), reads its row once more; the read keeps what was typed on the row since, and
    /// the save after it stores it (:1248 `closed`).
    pub(super) fn reread_part_at_close(&mut self, child: &St) -> Result<(), Signal> {
        if !std::mem::take(&mut child.borrow_mut().reread_at_close)
            || !self.unsettled_pages_pinned(child)?
            || !matches!(self.unsettled_calls(child), Some((_, _, true)))
        {
            return Ok(());
        }
        let rec = self.page_rec(child);
        let (typed, dirty, x) = (
            rec.borrow().fields.clone(),
            child.borrow().dirty,
            self.page_xrec_fields(child),
        );
        self.populate_current(child)?;
        rec.borrow_mut().fields = typed;
        child.borrow_mut().dirty = dirty;
        self.set_xrec_fields(child, x);
        Ok(())
    }
}
