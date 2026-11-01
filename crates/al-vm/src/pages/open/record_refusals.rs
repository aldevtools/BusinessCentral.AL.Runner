//! The records a page refuses to open on, before it takes their view.

use super::*;

impl Session {
    /// - A stored record handed to a card whose source table is temporary: BC shows the row (OnAfterGetRecord reads it,
    ///   pages/lt-be-card-onfindrecord/BeCardOnFindRecord.al:374 CardTemp_PageRun_StoredRec `O:1/stored/;R:1/stored/`), but
    ///   whether the row is copied into the page's temporary table or read from the database (which decides what the page
    ///   writes, counts and steps to) is not pinned.
    /// - A record keyed on a FlowField whose value depends on the row (`records::FLOWFIELD_KEY`): the record's own finds
    ///   order its rows by the FlowField (records/flowfield_key.rs), the order a page shows them in is not pinned.
    pub(super) fn page_record_refusal(
        &mut self,
        st: &St,
        record: Option<&Rec>,
        has_source: bool,
    ) -> Result<(), Signal> {
        let Some(r) = record.filter(|_| has_source) else {
            return Ok(());
        };
        if r.borrow().unpinned_buffer == Some(crate::records::FLOWFIELD_KEY) {
            self.mark_closed(st);
            return unsupported(
                "a page opened on a record keyed on a FlowField whose value depends on the row (the order the page shows is not pinned)",
            );
        }
        let page_temp = self.page_rec(st).borrow().temporary;
        if page_temp && !r.borrow().temporary && !self.is_list(st) && self.stored_row(r)?.is_some() {
            self.mark_closed(st);
            return unsupported(
                "a stored record handed to a card whose source table is temporary (whether the row is copied into the temporary table is not pinned)",
            );
        }
        Ok(())
    }
}
