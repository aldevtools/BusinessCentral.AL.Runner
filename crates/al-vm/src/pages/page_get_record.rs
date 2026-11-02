//! `Page.GetRecord`, and the case of it after a TestPage handler cancelled a lookup list that showed no row of a table that has rows: the record
//! holds the values the page's view filters its primary key fields to (pages/lt-y68-lookup-getrecord/Y68LookupGetRecord.al:446
//! Q3_TestPageHandler_Cancel_NoRow_NonEditableList: `SetRange("No.", 'ZZ')` gives `no=[ZZ] name=[] rank=0`, corpus master
//! 0d01b0cc). A filter on another field is not carried (errors/lt-ms-refusals-129/P129PageMisc.al:749
//! TV_GetRecord_AfterLookupCancel_FilteredToNoRow, a filter on Grp gives `grp=`). A Page-typed handler, which opens no page,
//! hands back the blank record (Y68LookupGetRecord.al:253-335).

use super::*;

impl Session {
    /// `Page.GetRecord(Rec)`: the page's Rec, or for a page that closed on no row the record the cases below pin.
    pub(super) fn page_get_record(&mut self, st: &St, args: &[Value]) -> Result<Value, Signal> {
        let (Some(Value::Record(dst)), Some(inst)) = (args.first(), st.borrow().instance.clone()) else {
            return unsupported("Page.GetRecord of a page that never ran (not pinned)");
        };
        let object = st.borrow().object as usize;
        let Some(slot) = self.engine.program.objects[object].rec_slot else {
            return unsupported("Page.GetRecord of a page without a source table (not pinned)");
        };
        let Value::Record(src) = inst.borrow().globals[slot].clone() else {
            return unsupported("Page.GetRecord: the page's Rec is no record");
        };
        if src.borrow().table != dst.borrow().table {
            return unsupported("Page.GetRecord into a record of another table (not pinned)");
        }
        if st.borrow().cursor != Cursor::Row {
            // A page that ran over a table with no row at all hands back the blank record, whatever the record held
            // (errors/lt-ms-refusals-129/P129PageMisc.al:P129_GetRecord_AfterRunModalCancelledOnEmptyTable,
            // _AfterRunModalOkOnEmptyTable, _PageTypedHandlerOnEmptyTable: `id=0|grp=|qty=0`). So does a list with no
            // Editable property that closed on no row of a table that has rows, after OK, LookupOK, LookupCancel and
            // SetRecord of a row (P129PageMisc.al:675-818 on corpus master, TV_GetRecord_*_FilteredToNoRow). Other pages
            // on no row of a table that has rows, and a page on its new-row line, are not pinned.
            // A page that never ran hands back the record SetRecord gave it (pages/lt-part-never-ran/NeverRanSetRecord.al:
            // P182b_HostNeverRan_PartSetRecord, `id=7 name=seven`).
            // Only a part is pinned: a Page variable's SetRecord before a typed handler ran it is not (PageVariable_LookupMode_HandlerSetRecord_Cancel).
            let given = st
                .borrow()
                .set_record
                .clone()
                .filter(|_| !st.borrow().ran && st.borrow().host.is_some());
            if let Some(given) = given.filter(|g| g.borrow().table == dst.borrow().table) {
                let fields = given.borrow().fields.clone();
                dst.borrow_mut().fields = fields;
                return Ok(Value::Null);
            }
            let initial = self.initial_fields(dst.borrow().table as usize);
            let src_blank = self.is_blank_row(&src);
            // After a Page-typed handler called SetRecord, GetRecord hands back that record, for LookupOK and Cancel
            // (pages/lt-modal-handler-typed-page/PageTypedHandlerRec.al:214-266 PageVariable_LookupMode_HandlerSetRecord_*;
            // a SetTableView view does not bound the page's Rec, :116 SettableView_ThenAssignRecOutsideView).
            // So does an empty temporary Card or NavigatePage handed a record by SetRecord (`empty_temp_card_by_set_record`).
            if st.borrow().handler_set_record || self.empty_temp_card_by_set_record(st, &src)? {
                let fields = src.borrow().fields.clone();
                dst.borrow_mut().fields = fields;
                return Ok(Value::Null);
            }
            if let Some(fields) = self.cancelled_lookup_key_fields(st, &src)? {
                dst.borrow_mut().fields = fields;
                return Ok(Value::Null);
            }
            let pinned_no_row = st.borrow().cursor == Cursor::None && self.plain_list_with_rows(st);
            if !src_blank && !pinned_no_row {
                return unsupported(
                    "Page.GetRecord of a page that closed on no row of a table that has rows (not pinned)",
                );
            }
            dst.borrow_mut().fields = initial;
            return Ok(Value::Null);
        }
        let fields = src.borrow().fields.clone();
        dst.borrow_mut().fields = fields;
        Ok(Value::Null)
    }

    /// The fields GetRecord hands back in that case, or `None` when the case does not apply: a Page variable (not the static
    /// Page.RunModal) of a list page that opened, closed with LookupCancel, whose Rec is blank and whose filters are all
    /// single values on primary key fields.
    pub(super) fn cancelled_lookup_key_fields(&mut self, st: &St, src: &Rec) -> Result<Option<Vec<Value>>, Signal> {
        {
            let s = st.borrow();
            if !s.ran || s.static_lookup || s.result.as_deref() != Some("LookupCancel") {
                return Ok(None);
            }
        }
        if !self.is_list_page(st) || !self.is_blank_row(src) {
            return Ok(None);
        }
        let table = src.borrow().table as usize;
        let t = self.table(src)?;
        let filters = src.borrow().filters.clone();
        if filters.is_empty() {
            return Ok(None);
        }
        let mut fields = self.initial_fields(table);
        for f in &filters {
            let value = crate::platform::mat::single_value_of(&f.expr);
            match value {
                Some(v) if f.group == 0 && t.keys[0].contains(&f.field) => fields[f.field] = v,
                _ => return Ok(None),
            }
        }
        Ok(Some(fields))
    }
}
