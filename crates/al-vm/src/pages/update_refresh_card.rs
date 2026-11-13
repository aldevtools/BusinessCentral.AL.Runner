//! The assumption a CurrPage.Update refresh of a card with OnFindRecord or OnNextRecord and a row trigger runs under.
//!
//! A card with only OnNextRecord (no OnFindRecord) stays on its row as the OnFindRecord card does
//! (pages/currpage/UpdateOnFindRecord.al:357 CurrPage_Update_CardWithOnlyOnNextRecord_KeepsCurrentRow, corpus pull request 1199,
//! al-runner issue 7268): the default Find('=') reads the row again. A list with only OnNextRecord, or such a card with unsaved
//! edits, is not pinned and stays refused in `actions.rs`.

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

impl Session {
    /// The client's refresh after a field's DrillDown, AssistEdit or Lookup trigger, with the two measured shapes that change
    /// it first: a CardPart drilldown that defers its host's reread (`defer_host_refresh_after_part_drilldown`), and an
    /// AssistEdit on a dirty Card that stored the edited row itself (`settle_next_only_card_assist_edit`).
    pub(super) fn refresh_after_field_trigger(
        &mut self,
        st: &St,
        method: PageMethod,
        rec_before: Option<&(Vec<Value>, bool)>,
    ) -> Result<(), Signal> {
        if method == PageMethod::DrillDown {
            self.defer_host_refresh_after_part_drilldown(st)?;
        }
        if let Some((before, true)) = rec_before {
            self.settle_next_only_card_assist_edit(st, before)?;
        }
        self.run_refresh(st)
    }

    /// NextOnlyCardRefresh.al: AssistEdit stores and reloads the edited row before Update(false).
    /// The old client edit is no longer pending. BC shows the returned Rec and refreshes the linked
    /// part without raising the card's current-record trigger.
    pub(super) fn settle_next_only_card_assist_edit(&mut self, st: &St, before: &[Value]) -> Result<(), Signal> {
        {
            let s = st.borrow();
            if !s.open
                || !s.dirty
                || !s.refresh
                || !s.refresh_by_update
                || !s.refresh_update_false
                || s.draft
                || s.mode != PageMode::Edit
                || s.modal
                || s.host.is_some()
                || s.cursor != Cursor::Row
                || self.engine.object_facts[s.object as usize].page_type != Some(ir::PageType::Card)
            {
                return Ok(());
            }
        }
        if !self.unextended_unsubscribed(st)
            || !self.assist_edit_has_linked_list_part(st)
            || self.declares(st, "onfindrecord")
            || !self.declares(st, "onnextrecord")
            || !self.declares(st, "onaftergetcurrrecord")
        {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if !Self::same_key(&table.primary_key(before), &self.row_key(&rec)?) {
            return Ok(());
        }
        let Some(stored) = self.stored_row(&rec)? else {
            return Ok(());
        };
        let current = rec.borrow();
        let already_stored =
            table
                .fields
                .iter()
                .zip(current.fields.iter().zip(&stored))
                .all(|(field, (shown, stored))| {
                    !field.stored
                        || Self::same_but_unwritten_blobs(std::slice::from_ref(shown), std::slice::from_ref(stored))
                });
        drop(current);
        if already_stored {
            assume(Assumption::NextOnlyCardAssistEditRefresh);
            {
                let mut s = st.borrow_mut();
                s.dirty = false;
                s.set_value_held = false;
                s.refresh = false;
            }
            self.set_xrec_fields(st, stored);
            self.sync_parts(st)?;
        }
        Ok(())
    }

    fn assist_edit_has_linked_list_part(&self, st: &St) -> bool {
        let engine = self.engine();
        let page = st.borrow();
        let [(control, part)] = page.parts.as_slice() else {
            return false;
        };
        let control = *control;
        let part = part.clone();
        drop(page);
        engine.object_facts[part.borrow().object as usize].page_type_lower() == "listpart"
            && self.unextended_unsubscribed(&part)
            && page_of(&engine, st).controls[control]
                .part_link
                .as_ref()
                .is_some_and(|link| {
                    link.provider.is_none()
                        && link.view.is_none()
                        && matches!(link.entries.as_slice(), [ir::LinkEntry::Field(_, _)])
                })
    }

    /// Marks the assumption of the refresh: update-refresh-onfindrecord (the trigger order of the refresh) unless a pin covers
    /// this page's refresh. A card with only OnNextRecord staying on its row is pinned (UpdateOnFindRecord.al:357); the trigger
    /// order of its refresh is not, so it is marked the same way.
    pub(super) fn assume_card_refresh(&self, st: &St) {
        if !st.borrow().refresh_pinned {
            assume(Assumption::UpdateRefreshOnFindRecord);
        }
    }
}
