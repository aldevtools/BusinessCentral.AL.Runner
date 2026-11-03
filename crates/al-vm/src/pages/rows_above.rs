//! The rows above the row a list opened on (`read_rows_above_the_moved_rec`).

use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// The open read of a plain list (no OnFindRecord or OnNextRecord) that opens on the row its OnOpenPage left Rec on, after the
    /// rows from it on are read: the client steps back from that row and reads each row above it up to the first of the view,
    /// then reads the row again; the rows it holds then start at the first row of the view (corpus
    /// pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:292 QR_List_OpenView_Get `O:C;R:C;R:D;R:C;R:B;R:A;R:C;R:C;C:C;`, :305
    /// _FindLast `O:D;R:D;R:C;R:B;R:A;R:D;R:D;C:D;`, :318 _FindFirstNext `O:B;R:B;R:C;R:D;R:B;R:A;R:B;R:B;C:B;`, :331
    /// _SetRangeFindLast `O:D;R:D;R:C;R:B;R:D;R:D;C:D;`, :370 _GetThenFilterExcludes `O:C;R:B;R:A;R:B;R:B;C:B;`): the row is read
    /// again before the step back only when rows followed it. Pinned for a view that ends inside the window, with rows above
    /// the row, on a page of its own; more rows than a window holds are refused.
    ///
    /// With more rows the client reads at most a window of 20 rows from the row on, and steps back over at most 20 rows above
    /// it (pages/lt-y37-long-list-open/Y37LongListTests.al:101 Get(25) of 30 `R:25..R:30;R:25;R:24..R:5;R:25;R:25;C:25`, :141
    /// FindLast, :180 a temporary list, :241 Get(5) `R:5..R:24;R:5;R:4..R:1;...`, :281 and :304 21 rows); `First` and `Last`
    /// then read on from the edge of the rows it holds (`PageState::moved_rec_open_partial`).
    pub(super) fn read_rows_above_the_moved_rec(
        &mut self,
        st: &St,
        start: Vec<Value>,
        end_reached: bool,
    ) -> Result<(), Signal> {
        if st.borrow().host.is_some() && !st.borrow().part_on_host_row {
            return unsupported("the open read of a part on the row its OnOpenPage left Rec on (not pinned)");
        }
        let rec = self.page_rec(st);
        if st.borrow().lookup_next_plain {
            // The first row of the window once more, with no function control (Y26_ThreeLinkPlain_LookupRefresh `next log`).
            self.page_trigger_skipping_fn(st)?;
        }
        let mut above = Vec::new();
        let mut top_unread = false;
        while self.nav_next(st, -1)? {
            if above.len() == Self::WINDOW {
                top_unread = true;
                break;
            }
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            above.push(al_builtins::value::WindowRow {
                key,
                stored,
                shown: rec.borrow().fields.clone(),
            });
        }
        if above.is_empty() {
            return unsupported(
                "the open read of a list on the row its OnOpenPage left Rec on, which is the first of the view (not pinned)",
            );
        }
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut start.clone())?;
        let stored = rec.borrow().fields.clone();
        // The move back onto the row is the client's two reads of a row it holds: the first runs no function control
        // (Y15LookupThreeFieldLink.al:337 Y26_ThreeLinkCurr_LookupRefresh `next log`). A host without OnAfterGetCurrRecord
        // reads nothing here (:467 Y26_ThreeLinkPlain_LookupRefresh).
        if st.borrow().lookup_next_plain {
            self.record_op(RecordOp::Get, &rec, &mut start.clone())?;
        } else {
            self.populate_current_pair_first(st)?;
        }
        let shown = rec.borrow().fields.clone();
        above.reverse();
        let mut s = st.borrow_mut();
        if let Some(row) = s.window.first_mut() {
            row.stored = stored;
            row.shown = shown;
        }
        let forward = std::mem::take(&mut s.window);
        above.extend(forward);
        s.window = above;
        s.moved_rec_open_partial = (s.window.len() > Self::WINDOW).then_some((top_unread, !end_reached));
        drop(s);
        Ok(())
    }
}
