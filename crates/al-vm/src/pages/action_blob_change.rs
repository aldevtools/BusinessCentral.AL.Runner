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
        // ReworkSave.al on real BC extends the Card and List evidence in pages/actions/WfFlowFieldActionRec.al
        // to a Document: a changed CalcFields result alone does not make the action leave a page edit.
        if !was_dirty
            && self.top_card_or_document(st)
            && self.engine.object_facts[st.borrow().object as usize].page_type == Some(ir::PageType::Document)
        {
            let table = self.table(&rec)?;
            let current = rec.borrow();
            if current.fields.len() == before.len()
                && table.fields.len() == before.len()
                && table.fields.iter().zip(current.fields.iter().zip(before)).all(|(field, (after, before))| {
                    !field.stored || Self::same_values(std::slice::from_ref(after), std::slice::from_ref(before))
                })
            {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::DocumentActionCalculatedFieldNotDirty,
                );
                return Ok(false);
            }
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
}
