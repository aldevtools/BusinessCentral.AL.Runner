//! The host's Next as the first call after a TestField.Lookup that left the refresh of `CurrPage.Update(false)` for later, on a
//! host with function controls and one linked ListPart (al-runner issue 6639). The Next moves as it does without a pending
//! refresh and runs the refresh with it: the host reads a window that starts at the row it moved to, then the row above, then
//! its current row again, before the part follows (pages/lt-y15/Y15LookupThreeFieldLink.al:337 Y26_ThreeLinkCurr_LookupRefresh
//! `next log`, :402 Y26_TwoLinkCurr_LookupRefresh, :467 Y26_ThreeLinkPlain_LookupRefresh, :532 Y26_TwoLinkPlain_LookupRefresh,
//! :597 Y26_ThreeLinkCurr_RegionsSet, :662 Y26_ThreeLinkCurr_LookupTwice). This is the read of a list that opens on a row
//! other than its first (`open_reads_above`).

use super::*;

impl Session {
    /// A Next on `root` may run the pending refresh within the move: the shapes the Y26 tests pin (one ListPart, function controls).
    pub(super) fn lookup_next_refresh_shape(&mut self, root: &St) -> bool {
        root.borrow().lookup_refresh_deferred
            && root.borrow().parts.len() == 1
            && !self.fn_log_controls(root).is_empty()
    }

    /// After the host's move, when the move carries the refresh a Lookup left (`lookup_next_refresh`): the window and the rows above
    /// the row the host moved to.
    pub(super) fn lookup_next_refresh(&mut self, st: &St) -> Result<(), Signal> {
        if !std::mem::take(&mut st.borrow_mut().lookup_next_refresh) {
            return Ok(());
        }
        let plain = !self.declares(st, "onaftergetcurrrecord");
        if plain {
            // The move itself reads the row, with no function control (Y26_ThreeLinkPlain_LookupRefresh `next log`).
            self.page_trigger_skipping_fn(st)?;
        }
        {
            let mut s = st.borrow_mut();
            s.window.clear();
            s.fn_open_skipped = plain;
            s.lookup_next_plain = plain;
            s.open_read = true;
            s.landing_first = true;
            s.open_reads_above = true;
        }
        let r = self.arrive(st);
        {
            let mut s = st.borrow_mut();
            s.open_read = false;
            s.landing_first = false;
            s.open_reads_above = false;
        }
        r
    }

    /// OnAfterGetRecord of `st` for the row it stands on, without its function controls.
    pub(super) fn page_trigger_skipping_fn(&mut self, st: &St) -> Result<(), Signal> {
        if !self.fn_log_controls(st).is_empty() {
            st.borrow_mut().fn_skip_agr = true;
        }
        let r = self.page_trigger(st, "onaftergetrecord", vec![]);
        st.borrow_mut().fn_skip_agr = false;
        r.map(|_| ())
    }

    /// The second TestField.Lookup while the first one's refresh is pending: the host reads its rows as when the page opens, the
    /// parts read nothing (pages/lt-y15/Y15LookupThreeFieldLink.al:662 Y26_ThreeLinkCurr_LookupTwice `second lookup log`).
    pub(super) fn replay_open_reads_host(&mut self, root: &St) -> Result<(), Signal> {
        {
            let mut s = root.borrow_mut();
            s.window.clear();
            s.fn_rows.clear();
            s.fn_open_skipped = false;
        }
        let mode = root.borrow().mode;
        root.borrow_mut().open_read = true;
        let loaded = self.load_first_row(root, mode, false);
        root.borrow_mut().open_read = false;
        loaded?;
        self.shown_opened(root);
        Ok(())
    }
}
