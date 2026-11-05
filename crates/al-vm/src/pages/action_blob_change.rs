//! Whether an action left changes in Rec for the page to save. `same_values` gives no answer for two Blobs, so a Blob never
//! compares equal; a Blob no stream wrote is no change for Modify (corpus record/PnASystemFields.al:110
//! Modify_StampsOnlyWhenStoredFieldDiffers), and an action that changes nothing else in Rec saves nothing on a List with
//! Editable = false and ModifyAllowed = false (testpage/RmReadOnlyBlob.al:316-470). Those pages keep the paths of `save_row`.
//! An editable page (Editable and ModifyAllowed left true) is not pinned: it saves nothing there under the assumption
//! page-action-unwritten-blob-no-change (al-runner issue 7114); when the action's code committed, it is refused until the
//! probe lands.

use super::*;

impl Session {
    /// Whether Rec of page `st` differs from `before` (Rec as the action found it) after the action. `commits_before` is the
    /// transaction's commit counter when the action started, `deleted` whether the action deleted the row.
    pub(super) fn rec_changed_by_action(
        &mut self,
        st: &St,
        before: &[Value],
        deleted: bool,
        was_dirty: bool,
        commits_before: u64,
    ) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        if Self::same_values(&rec.borrow().fields, before) {
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
        if self.tx.commit_counter != commits_before {
            return unsupported(
                "an action whose code committed on an editable page over a table with a Blob field (whether the page saves its row then is not pinned)",
            );
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageActionUnwrittenBlobNoChange);
        Ok(false)
    }
}
