//! `DeleteAll`: the rows it deletes, row by row (with delete events, database triggers, OnDelete for DeleteAll(true), the
//! User table, MarkedOnly) or in one step in the store; a filter on a FlowField goes to `delete_all_flow`.

use super::*;

impl Session {
    /// `DeleteAll(run)` of `rec` on `t` (the arm of `record_op_inner`; `Session::note_bulk_write` notes it around this call).
    pub(super) fn delete_all(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        cx: &al_data::AccessContext,
        run: bool,
    ) -> Result<Value, Signal> {
        // The platform takes no delete on Tenant License State (record/TestPlatformTablesWritableContracts.al:96-101).
        self.tenant_license_state_refuses(t, "delete")?;
        self.document_service_trigger_refused(t, rec, run, "DeleteAll")?;
        let (filters, flow_field_filters) = self.split_filters(rec);
        if !flow_field_filters.is_empty() {
            return self.delete_all_flow_filtered(t, rec, run);
        }
        let temporary = rec.borrow().temporary;
        self.coverage_cleared(t, temporary, &filters);
        // The platform asks for the table's Global Triggers setup, as the first write to the table, before it looks
        // for rows: a row the setup subscriber inserts is deleted (events/W4GlobalTriggers.al:327, :341), and a table
        // with no row asks too (session-system/TestEnvironmentUserTenantContracts.al:289-290, `GtTable.DeleteAll()` on an
        // empty table). A temporary record is not pinned and stays as it was.
        if !temporary {
            self.global_setup_asked(t)?;
        }
        let n = self
            .store_for(rec)
            .count(cx, t, &filters)
            .map_err(|e| Signal::unsupported(format!("{e}")))?;
        // A platform table whose fresh-tenant rows nothing pins (`UNSEEDED_ROWS`): a DeleteAll without filters removes
        // them in BC too, so the store marks the table emptied (`Session::is_unseeded`), even when it holds no row
        // itself. With delete events or database triggers BC would run them for those rows: not known, refused.
        if !temporary
            && filters.is_empty()
            && !rec.borrow().marked_only
            && self.engine.platform_unseeded.contains(&(t.object as usize))
            && self.is_unseeded(t)
        {
            let per_row = self.listened(t, "onbeforedeleteevent", None)?
                || self.listened(t, "onafterdeleteevent", None)?
                || self.global_enabled(t, GlobalOp::Delete)?;
            if per_row {
                return unsupported(format!(
                    "DeleteAll on {} with delete events or database triggers: BC runs them for the rows a fresh tenant holds there, which nothing pins",
                    t.name
                ));
            }
            self.store_for(rec)
                .delete_all(cx, t, &filters)
                .map_err(|e| Signal::unsupported(format!("{e}")))?;
            self.deletes += 1;
            return Ok(Value::Null);
        }
        // No row, no trigger and no event.
        if n == 0 {
            return Ok(Value::Null);
        }
        let events = self.listened(t, "onbeforedeleteevent", None)? || self.listened(t, "onafterdeleteevent", None)?;
        let mut trigger = self.table_trigger(t, "OnDelete").is_some();
        for n in ["OnBeforeDelete", "OnDelete", "OnAfterDelete"] {
            trigger |= !self.ext_triggers(t, n).is_empty();
        }
        let global = !temporary && self.global_enabled(t, GlobalOp::Delete)?;
        // With delete events or database triggers a DeleteAll deletes row by row: the events
        // and the database trigger per row, the table's and extensions' triggers only for
        // DeleteAll(true). Pinned on real BC 27.5 by the probe codeunit 68212
        // (record/TestTableExtensionTriggerOrderContracts.al:102
        // Delete_And_DeleteAll_WithAndWithoutRunTrigger).
        // The User table has no bulk delete: the platform deletes it row by row.
        let user_rows = self.is_user_row(t, rec.borrow().temporary);
        // MarkedOnly limits the rows to the marked ones, which the store's bulk delete does not know: delete row
        // by row what the scan finds (Post Inventory Cost to G/L ends its data item with
        // `MarkedOnly(true); DeleteAll()` and keeps the capacity entries it has not posted yet). Only the marked
        // rows that pass the filters go, with OnDelete per row for DeleteAll(true)
        // (record/AsMarkedDeleteAll.al:27-94).
        let marked_only = rec.borrow().marked_only;
        if events || global || user_rows || (run && trigger) || marked_only {
            // One Delete per row (record/TestTriggerContracts.al:DeleteAll_WithTrigger_FiresForEachFilteredRecord).
            let key = rec.borrow().key;
            let mut rows = Vec::new();
            let mut after = None;
            while let Some(row) = self.scan(rec, t, true, after.take())? {
                after = Some(position(t, key, &row));
                rows.push(row);
            }
            // One fresh set of table globals for all rows, not the caller's
            // (ADR-021 decision 6; corpus probe TableGlobals_DeleteAll_TriggersRunOnOneInstanceForAllRows).
            let shared = al_builtins::value::TableGlobals::new_shared();
            for row in rows {
                // A row an earlier row's OnDelete deleted is not visited: BC finds the next row of the table as
                // it is then. Not pinned; Requisition Line's OnDelete deletes the lines that follow it
                // (Tests-SCM-Manufacturing B42538_UserIDOnReqLine; assumption delete-all-skips-rows-deleted-meanwhile).
                let pk: Vec<Value> = t.keys[0].iter().map(|&f| row[f].clone()).collect();
                let still_there = self
                    .store_for(rec)
                    .get(cx, t, &pk)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?
                    .is_some();
                if !still_there {
                    // record/crud/DeleteAllSkipsDeleted.al:DeleteAllTrue_FirstDeletesAnEarlierSibling_NoneEarlier.
                    continue;
                }
                let mut st = state_with_fields(rec, row);
                st.filters.clear();
                st.globals = shared.share();
                self.delete_row(t, &Rc::new(RefCell::new(st)), run)?;
            }
            return Ok(Value::Null);
        }
        if self.is_company_table(t) && !temporary {
            return unsupported(
                "DeleteAll on the Company table without delete events (a deleted company's rows go with it; not pinned)",
            );
        }
        self.store_for(rec)
            .delete_all(cx, t, &filters)
            .map_err(|e| Signal::unsupported(format!("{e}")))?;
        self.deletes += 1;
        Ok(Value::Null)
    }
}
