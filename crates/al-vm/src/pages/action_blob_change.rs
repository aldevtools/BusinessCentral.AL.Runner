//! Whether an action left changes in Rec for the page to save. `same_values` gives no answer for two Blobs, so a Blob never
//! compares equal; a Blob no stream wrote is no change for Modify (corpus record/PnASystemFields.al:110
//! Modify_StampsOnlyWhenStoredFieldDiffers), and an action that changes nothing else in Rec saves nothing on a List with
//! Editable = false and ModifyAllowed = false (testpage/RmReadOnlyBlob.al:316-470). Those pages keep the paths of `save_row`.
//! An editable page (Editable and ModifyAllowed left true) saves nothing there either, with and without a Commit in the action's
//! code, on a List and a Card: the action's own store stays and no OnModifyRecord runs (corpus
//! pages/actions/R7114EditableBlobAction.al:272-345 PG7114_EditableList_ActionModifiesOwnVar_NoCommit and _Commit,
//! PG7114_EditableCard_ActionModifiesOwnVar_NoCommit and _Commit, BC 27.5, 28.5, 29.0; al-runner issue 7114). A Blob a stream wrote is
//! a change and is saved after the action (_ActionWritesBlobOfRec `P.MODR(Open/);T.MOD(Open/)`).

use super::*;

impl Session {
    /// Whether Rec of page `st` differs from `before` (Rec as the action found it) after the action. `deleted`: the action
    /// deleted the row.
    pub(super) fn rec_changed_by_action(
        &mut self,
        st: &St,
        before: &[Value],
        deleted: bool,
        was_dirty: bool,
    ) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        if Self::same_values(&rec.borrow().fields, before) {
            return Ok(false);
        }
        // Corpus PR 1346 (issue 7882): a view-mode Document action calculates a FlowField
        // and commits a change through a separate buffer. The page has no modify observers.
        if !was_dirty && !deleted && self.document_view_action_only_changed_flowfields(st, &rec, before)? {
            al_builtins::assumptions::assume(
                al_builtins::assumptions::Assumption::DocumentActionCalculatedFieldNotDirty,
            );
            return Ok(false);
        }
        // A row the action changed and then deleted (a codeunit that modifies and deletes Rec: Make Order, Post) is not
        // written again on a card: the card closes (`settle_after_action`) and nothing is re-inserted (corpus master 97c29f86,
        // pages/save-deleted-row/Y17SaveRowGone.al:632 PgSave_ActionDeletesEditedRow_Card `stored` `B/b/No;`, :638
        // without the edit). A list then moves on with reads the gone-row path does not model yet (:711
        // PgSave_ActionDeletesEditedRow_List_Next `P.AGR:B;P.AGR:B;P.AGCR:B/b;` twice, the row edited before the action):
        // refused. Without an edit before the action a list keeps the path it had (no corpus test pins its reads; Microsoft's
        // Sales and Purchase Batch Document Posting tests post a row of a list this way).
        if deleted && was_dirty && self.is_list(st) {
            return unsupported(
                "a list whose row an action changed and then deleted (the reads after the action are not pinned)",
            );
        }
        if deleted || !Self::same_but_unwritten_blobs(&rec.borrow().fields, before) {
            return Ok(true);
        }
        let object = st.borrow().object as usize;
        let engine = self.engine();
        if flag_denied(&engine, object, "ModifyAllowed")? || flag_denied(&engine, object, "Editable")? {
            // `save_row` decides these (RmReadOnlyBlob.al; page-unwritten-blob-no-change-not-editable).
            return Ok(true);
        }
        Ok(false)
    }

    fn document_view_action_only_changed_flowfields(
        &mut self,
        st: &St,
        rec: &Rec,
        before: &[Value],
    ) -> Result<bool, Signal> {
        let state = st.borrow();
        if state.mode != PageMode::View
            || state.host.is_some()
            || self.engine.object_facts[state.object as usize].page_type != Some(ir::PageType::Document)
            || !self.tx.writes.is_empty()
        {
            return Ok(false);
        }
        let object = state.object as usize;
        drop(state);
        if self.declares_or_ext_declares(st, "onmodifyrecord")
            || self.engine.events.is_subscribed(object, "onmodifyrecordevent")
        {
            return Ok(false);
        }
        let table = self.table(rec)?;
        let Some(stored) = self.stored_row(rec)? else {
            return Ok(false);
        };
        let current = rec.borrow();
        if current.temporary
            || current.assigned
            || current.fields.len() != before.len()
            || table.fields.len() != before.len()
        {
            return Ok(false);
        }
        let same = |a, b| Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b));
        let schema = self.engine.program.objects[table.object as usize]
            .table
            .as_ref()
            .expect("table");
        let mut changed_flowfield = false;
        let mut stored_changed = false;
        for (i, field) in table.fields.iter().enumerate() {
            if field.stored {
                if !same(&current.fields[i], &before[i]) {
                    return Ok(false);
                }
                stored_changed |= !same(&stored[i], &before[i]);
            } else if !same(&current.fields[i], &before[i]) {
                if schema.fields.get(i).is_none_or(|field| field.calc.is_none()) {
                    return Ok(false);
                }
                changed_flowfield = true;
            }
        }
        Ok(changed_flowfield && stored_changed)
    }
}
