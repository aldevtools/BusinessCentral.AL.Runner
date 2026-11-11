//! A ListPart that serves its rows from OnFindRecord and OnNextRecord, after a TestPart filter (al-runner issue 7476): the
//! client reads the rows again from the first row of the view, and the function controls' first repeated read of each window of a part. The function controls of such a
//! read are `fn_controls.rs`.

use super::*;

impl Session {
    /// A ListPart with OnFindRecord, OnNextRecord, OnAfterGetRecord and OnAfterGetCurrRecord reads its rows again after a
    /// TestPart filter with OnFindRecord(`-`), then OnNextRecord(1) to the end of the view or of the window, then the tail of an
    /// open read without the step back of OnNextRecord(-1) (testpage/parts/PartFilterExpressionControl.al:356
    /// Periods_FilterOneDay_First_Read `filter log` `[P.find(-)=1 P.agr=2 calc=5 P.next=1 P.find(=)=1 P.agcr=1 ]`, :414
    /// Periods_FilterThreeDays `row 1 log` `P.next=3`, :493 Periods_FilterThenClearFilter_Read `cleared log` a full window
    /// `P.next=19`, :676 EntriesFind_FilterOneDay_First_Read, :732 EntriesFind_FilterTwoDays_FirstNextNext). A top-level list
    /// and a part without one of the four triggers are not pinned and keep `=><` (`page_first_row`).
    /// The same part after its host moved reads `=><` and steps back with OnNextRecord(-1), which answers 0 on the first row
    /// of the view (:758 EntriesFind_FilterOneDay_HostMovesToNextItem `I3 no move log` `F.find(=><)=1 F.agr=2 F.next=2`): that window
    /// starts at the view's start too.
    ///
    /// Every fact the corpus pins is required (the `TP PF Host` page, PartFilterExpressionControl.al:251): the host is a ListPlus
    /// opened for view (`OpenHost`, :334), the part a ListPart that declares the four triggers, neither page has a page extension
    /// or an event subscriber, and no `Filter.SetCurrentKey` or `Ascending` changed the order since it opened. Any other host,
    /// open mode or part keeps `=><` and the step back.
    pub(super) fn find_list_part(&self, st: &St) -> bool {
        self.part_on_list_plus_view_host(st)
            && self.is_list(st)
            && !st.borrow().test_view_reordered
            && [
                "onfindrecord",
                "onnextrecord",
                "onaftergetrecord",
                "onaftergetcurrrecord",
            ]
            .iter()
            .all(|t| self.declares(st, t))
    }

    /// A ListPart on a ListPlus host opened for view, neither page with a page extension or an event subscriber (the host and the
    /// parts of PartFilterExpressionControl.al:251).
    fn part_on_list_plus_view_host(&self, st: &St) -> bool {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return false;
        };
        let engine = self.engine();
        let page_type = |p: &St| engine.object_facts[p.borrow().object as usize].page_type.clone();
        page_type(&host) == Some(ir::PageType::ListPlus)
            && host.borrow().mode == PageMode::View
            && page_type(st) == Some(ir::PageType::Other("listpart".into()))
            && self.unextended_unsubscribed(&host)
            && self.unextended_unsubscribed(st)
    }

    /// `find_list_part` with a TestPart filter on the part (filter group 0, `filter.rs`): the open read after a host move then
    /// starts at the view's start (:758 EntriesFind_FilterOneDay_HostMovesToNextItem, the filter `03/16/25` still set). Without a
    /// filter the open read is not pinned to do so and keeps the step back.
    fn find_list_part_filtered(&self, st: &St) -> bool {
        self.find_list_part(st) && self.page_rec(st).borrow().filters.iter().any(|f| f.group == 0)
    }

    /// A ListPart that a TestPart filter left on no row (pinned for the ListPlus host opened for view of `find_list_part`, where the part shows no blank line; for edit it is not pinned) runs its function
    /// controls once after OnAfterGetCurrRecord there, with OnFindRecord or without (PartFilterExpressionControl.al:629 Entries_FilterAdmitsNoRow_First `filter log`
    /// `[E.agcr=1 calc=1 ]`, :785 EntriesFind_FilterAdmitsNoRow_First `[F.find(-)=1 F.agcr=1 calc=1 ]`). Nothing reads the
    /// control there, so what it shows is not noted.
    pub(super) fn part_filter_runs_fn_on_new_line(&self, st: &St) -> bool {
        st.borrow().cursor != Cursor::Row && self.part_on_list_plus_view_host(st)
    }

    /// A part starts to read a window. Each window a part reads runs no function control at its first repeated
    /// OnAfterGetRecord, not only the open's (`fn_controls.rs`; PartFilterExpressionControl.al:732
    /// EntriesFind_FilterTwoDays_FirstNextNext `F.agr=4 calc=4` after a TestPart filter, :576 Entries_FilterTwoDays `E.agr=4
    /// calc=4`, :450 Periods_FilterOneDay_HostMovesToNextItem `I3 no move log` `E.agr=4 calc=8 ... F.agr=4` after the host moved).
    pub(super) fn part_window_read_starts(&self, st: &St) {
        let mut s = st.borrow_mut();
        if s.host.is_some() {
            s.fn_open_skipped = false;
        }
        s.find_window_at_view_start = s.landed_from_view_start;
    }

    /// The step back of the open read of a list that serves its rows from OnNextRecord: OnNextRecord(-1) after the first row
    /// is put back; whether it moved. A window found with OnFindRecord(`-`) after a TestPart filter starts at the first row of
    /// the view and asks nothing (`find_list_part`; :356 `P.next=1` for a view of one row).
    pub(super) fn open_read_steps_back(&mut self, st: &St, moved_open: bool) -> Result<bool, Signal> {
        let from_start = st.borrow().landed_from_view_start;
        let stepped_back = !from_start && self.declares(st, "onnextrecord") && self.nav_next(st, -1)?;
        if !stepped_back && !moved_open && self.find_list_part_filtered(st) {
            st.borrow_mut().find_window_at_view_start = true;
        }
        Ok(stepped_back)
    }
}
