//! FilterGroup(-1) (the OR group) on a page's Rec (gap G5-21).

use super::*;

impl Session {
    /// A record handed to the page with filters in FilterGroup(-1) (Page.Run(Id, Rec), SetTableView) reaches OnOpenPage
    /// without them; its marks and MarkedOnly stay (pages/lt-y50-or-group-page-rec/Y50OrGroupPageRec.al:432
    /// G521_PageRunRec_OrMarks `open g0=Marked: Yes gm1=Marked: Yes`, :521 G521_SetTableView_GroupMinus1Current_Lookup `open g0=
    /// gm1= n=5`: the page shows every row).
    pub(super) fn drop_handed_or_group(&mut self, st: &St) {
        let rec = self.page_rec(st);
        let before = rec.borrow().filters.len();
        rec.borrow_mut()
            .filters
            .retain(|f| f.group != al_builtins::filter::OR_GROUP);
        if rec.borrow().filters.len() != before {
            st.borrow_mut().or_group_handed = true;
        }
    }

    /// A list handed a record on a stored row whose FilterGroup(-1) filters were dropped (`drop_handed_or_group`) opens on that
    /// row as a list opens on a row its OnOpenPage left Rec on (`open_on_rec_moved_by_onopenpage`): the row, the rows after it,
    /// the row again, the rows above it, the row twice more (Y50OrGroupPageRec.al:432 `R:4;R:2;R:1;R:4;R:4;C:4` on the marked
    /// rows 1, 2, 4; :521 run modally in lookup mode after SetRecord(2), `R:2;R:3;R:4;R:5;R:2;R:1;R:2;R:2;C:2`).
    pub(super) fn handed_or_group_list_on_row(&mut self, st: &St) -> Result<bool, Signal> {
        if !st.borrow().or_group_handed || !self.is_list(st) || st.borrow().host.is_some() {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        Ok(self.stored_row(&rec)?.is_some())
    }

    /// Whether the FilterGroup(-1) filters OnOpenPage left may be dropped on a page whose code runs later: pinned with marks
    /// and MarkedOnly (the page shows the marked rows; later code reads `Marked: Yes` in every group and MarkedOnly true,
    /// Y50OrGroupPageRec.al:395 G521_OnOpenPage_OrMarks_Walk, :413 _Action, :551 _Empty), and without marks and MarkedOnly
    /// (every row shows, later code reads no filter, :468 G521_OnOpenPage_OrOnly_LaterCode). MarkedOnly with no row marked is
    /// the empty page (:551 G521_OnOpenPage_OrMarks_Empty). Marks without MarkedOnly are not pinned.
    pub(super) fn or_group_drop_pinned(&self, st: &St) -> bool {
        let rec = self.page_rec(st);
        let r = rec.borrow();
        r.marked_only || r.marks.is_empty()
    }
}
