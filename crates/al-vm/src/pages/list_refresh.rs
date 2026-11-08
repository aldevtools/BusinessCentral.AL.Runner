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
        // al-runtime-rs-staging-y28, pages/extensions/Y28CollapsedTreeTests.al:170 Y28_ActionResetAndUpdate
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
    /// runs OnAfterGetRecord 25 times when 20 rows or more follow it and 24 times with 19 (testpage/Y15ActionWidensView.al:365
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

    /// Whether the refresh after an action reads the list again from row `key` (`read_list_again_from_row_in_view`): a list whose
    /// window starts at the first row of its view, or one walked past its first window whose action left another row in Rec
    /// (`rec_moved_other`) that lies in the first window of the view with a full window from it on. From R3, R20, R21, R25 and R30
    /// of thirty rows an action that leaves R2 is followed by the same reads, and Next then steps from R2 (testpage/PnGRefresh.al:1204
    /// BW_GetRow2_FromRow3_InsideFirstWindow, :1240 _FromRow20_LastRowOfFirstWindow, :1276 _FromRow21_FirstRowPastFirstWindow,
    /// :1312 _FromRow25_BeyondFirstWindow, :1348 _FromRow30_LastRowOfView; al-runner issue 7288). A row nearer the end of the view
    /// is read back 20 rows only (:1384 BW_GetRow24_FromRow25_RowAboveInWindow, :1420 BW_GetRow28_FromRow25_RowBelowInView),
    /// which the runtime does not build: those stay refused.
    pub(super) fn reads_again_from_row_held(&mut self, st: &St, rec: &Rec, key: &[Value]) -> Result<bool, Signal> {
        if st.borrow().window_from_start {
            return Ok(true);
        }
        if !st.borrow().rec_moved_other {
            return Ok(false);
        }
        let rows = self.view_rows(rec, 2 * Self::WINDOW)?;
        let table = self.table(rec)?;
        let at = rows.iter().position(|r| Self::same_key(&table.primary_key(r), key));
        let held = at.is_some_and(|i| i < Self::WINDOW && rows.len() - i >= Self::WINDOW);
        if held {
            // The client holds the rows it reads now, back to the first row of the view, so a move after it steps from the row the
            // action left as from inside the first window (BW_GetRow2_FromRow25_BeyondFirstWindow `log-next` `G:R3;G:R3;C:R3/x=R2`,
            // `log-previous` `G:R2;G:R2;C:R2/x=R3`); the refresh keeps those rows as the window.
            st.borrow_mut().action_landed_on_new_row = false;
        }
        Ok(held)
    }
}
