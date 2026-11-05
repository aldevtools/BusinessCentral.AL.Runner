//! The page's Rec when a temporary record is handed to `Page.Run(Id, TempRec)` answered by a Page-typed PageHandler
//! (ui-handlers/Y72PageHandlerSetUpVariable.al header): the page's Rec shares the caller's temporary rows, takes its
//! filters and stands on its row (TypedHandler_TempRecord_PageRun_OnRowTwo `got=2/b pagecount=3 cur=2`, _Filtered `pagecount=2`,
//! _Empty `got=0/ pagecount=0`); the caller's record keeps its row and count.

use super::*;

impl Session {
    /// Puts the record `handed` to a run answered by a Page-typed handler into the page's Rec `rec`: a stored record's values
    /// (Y17PageHandlerRecord.al), or a temporary record shared, for a page over a temporary source of the same table (also for
    /// RunModal, Y72 TypedHandler_TempRecord_PageRunModal `pagecount=3 cur=2`), and for a page over a stored source of that table.
    pub(super) fn typed_handler_record(&mut self, rec: &Rec, handed: &Rec) -> Result<(), Signal> {
        let h = handed.borrow();
        let mut r = rec.borrow_mut();
        if !h.temporary {
            r.fields = h.fields.clone();
            return Ok(());
        }
        if r.table != h.table {
            return unsupported(
                "a Page-typed PageHandler for Page.Run with a temporary record of another table than the page's (not pinned)",
            );
        }
        // A page over a stored source takes the temporary record the same way: its Rec becomes temporary and shares the caller's
        // rows, filters and row; the stored rows stay out (corpus branch al-runtime-rs-staging-r6962,
        // ui-handlers/TypedHandlerTempRecordStoredSource.al TypedPageHandler_TempRecordStoredSource_PageRun_Filtered `got=2/b
        // pagecount=2 cur=2 temp=Yes`, _OnRowTwo, _Empty, TypedModalHandler_TempRecordStoredSource_PageRunModal_OnRowTwo; issue 7018).
        r.temporary = true;
        r.temp_id = h.temp_id.clone();
        r.fields = h.fields.clone();
        r.filters = h.filters.clone();
        r.key = h.key;
        r.ascending = h.ascending;
        r.position = h.position.clone();
        Ok(())
    }
}
