//! The refresh of a list after an action or a control's OnValidate reads its rows again from the row it stands on
//! (`actions.rs` `read_list_again_from_row_in_view`); this module holds its entry and the reads around a full window.

use super::*;

impl Session {
    /// [`Self::read_list_again_from_row`]; `read_first` false leaves out the read of the row before the others (a refresh after a
    /// control's OnValidate, `refresh_after_validate_filter`).
    pub(super) fn read_list_again_from_row_as(
        &mut self,
        st: &St,
        rec: &Rec,
        key: &[Value],
        read_first: bool,
    ) -> Result<(), Signal> {
        // A collapsed or expanded tree with row triggers reads the rows it hides too, as when it opens (corpus writer Y28, branch
        // al-runtime-rs-staging-y28, pageext/lt-page-ext-properties/Y28CollapsedTreeTests.al:170 Y28_ActionResetAndUpdate
        // `agr1;reset count=5;agr1;agr1;agr2;agr3;agr4;agr5;agr1;agr1;agcr1`, :130 Y28_ActionCountOnRoot).
        if self.tree_expand_all_with_row_triggers(st)
            && self.read_tree_rows_whole(st, |this| {
                this.read_list_again_from_row_in_view(st, rec, key, read_first)
            })?
        {
            return Ok(());
        }
        self.read_list_again_from_row_in_view(st, rec, key, read_first)
    }

    /// After a forward read that filled a window, a refresh by an action that changed the list's view reads one row more: an action that widens the view of a list standing on its second row
    /// runs OnAfterGetRecord 25 times when 20 rows or more follow it and 24 times with 19 (pages/lt-y15/Y15ActionWidensView.al:365
    /// Y15_WV_FromLast25 `A02 25:A02-A01`, :381 Y15_WV_FromLast20 `A02 24:A02-A01`, :397 Y15_WV_FromLast21, :413
    /// Y15_WV_FromLast40). Which row the extra read is for is not pinned (the first and the last read are): the runtime reads the last row of
    /// the window again (docs/probes-needed.md). Pinned for a page
    /// without OnFindRecord and OnAfterGetCurrRecord (`plain`); elsewhere the reads stay as they were.
    pub(super) fn read_start_after_full_window(&mut self, st: &St, read: usize, plain: bool) -> Result<(), Signal> {
        if plain && read == Self::WINDOW && self.view_changed_by_action(st) {
            self.populate_current(st)?;
        }
        Ok(())
    }
}
