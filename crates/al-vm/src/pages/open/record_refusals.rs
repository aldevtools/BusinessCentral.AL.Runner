//! The records a page refuses to open on, before it takes their view.

use super::*;

impl Session {
    /// - A record that is no temporary one handed to a Card whose source table is temporary by `Page.Run(Id, Rec)` or
    ///   `Page.RunModal(Id, Rec)`: the page's Rec stops being temporary, OnOpenPage reads `temp=No` and counts the table, the page
    ///   shows the stored row, steps through the table and writes to it, also when the record is filtered to no row
    ///   (pages/lt-y61-page-probes/Y61StoredRecToTempCard.al:373 G617b_PageRunModalIdRec_Rename, :435
    ///   G617b_PageRunIdRec_TrapTestPage_CountAndStep, :499 G617b_RecordFilteredToNoRow). The same record handed by `SetRecord` of a
    ///   Page variable leaves a Card or NavigatePage temporary and empty: OnOpenPage reads the record's fields, the page shows a blank
    ///   row (:313 G617b_SetRecord_RunModal_Agree, :404 _StoredRow_Delete, :468 G617b_NavigatePage_SetRecord_RunModal_Agree). Other
    ///   page types handed a stored row are refused (not pinned).
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
        if !page_temp || r.borrow().temporary || self.is_list(st) {
            return Ok(());
        }
        let page_type = self.engine.object_facts[st.borrow().object as usize].page_type.clone();
        let by_set_record = st.borrow().set_record.as_ref().is_some_and(|s| Rc::ptr_eq(s, r));
        match (page_type, by_set_record) {
            (Some(al_ir::PageType::Card), false) => {
                let rec = self.page_rec(st);
                let mut rec = rec.borrow_mut();
                rec.temporary = false;
                rec.temp_id = None;
            }
            (Some(al_ir::PageType::Card | al_ir::PageType::NavigatePage), true) => {}
            _ if self.stored_row(r)?.is_some() => {
                self.mark_closed(st);
                return unsupported(
                    "a stored record handed to a page whose source table is temporary, other than a Card or a NavigatePage by SetRecord (not pinned)",
                );
            }
            _ => {}
        }
        Ok(())
    }
}
