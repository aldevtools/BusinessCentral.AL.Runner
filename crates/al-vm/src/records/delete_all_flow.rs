//! `DeleteAll` on a record with a filter on a FlowField (al-runner issue 6961). Measured on BC 29 by corpus branch
//! al-runtime-rs-staging-r6961, record/crud/R6961DeleteAllFlowFilter.al:
//! - the FlowField filters select the rows as FindSet does (Exist, Sum with a FlowFilter, Count, Lookup with `<>` matching a
//!   blank lookup), on a temporary record too (its FlowFields read the real tables), and through RecordRef;
//! - `DeleteAll()` runs no OnDelete and deletes the rows chosen before the first delete: an Exist over the same table deletes
//!   heads 1 and 5 only (DeleteAll_SelfReferenceExist_NoTrigger);
//! - `DeleteAll(true)` runs OnDelete per row in the order of the current key and tests each row against the filters when it
//!   reaches it, after the earlier deletes: the same Exist deletes the whole chain (DeleteAll_SelfReferenceExist_RunTrigger),
//!   Exist = true deletes heads 2 and 4 (DeleteAll_SelfReferenceExistTrue_RunTrigger).
//!
//! Only a table with an OnDelete trigger is measured for `DeleteAll(true)`, and none of the probes has delete events, database
//! triggers, table extension delete triggers or MarkedOnly: those are refused.

use super::*;

impl Session {
    /// `DeleteAll(run)` of `rec`, whose filters include one on a FlowField (see the module).
    pub(super) fn delete_all_flow_filtered(&mut self, t: &TableDef, rec: &Rec, run: bool) -> Result<Value, Signal> {
        let temporary = rec.borrow().temporary;
        let events = self.listened(t, "onbeforedeleteevent", None)? || self.listened(t, "onafterdeleteevent", None)?;
        let ext_trigger = ["OnBeforeDelete", "OnDelete", "OnAfterDelete"]
            .iter()
            .any(|n| !self.ext_triggers(t, n).is_empty());
        let global = !temporary && self.global_enabled(t, GlobalOp::Delete)?;
        let trigger = self.table_trigger(t, "OnDelete").is_some();
        if events
            || global
            || ext_trigger
            || rec.borrow().marked_only
            || self.is_user_row(t, temporary)
            || (run && !trigger)
            || (!temporary && self.engine.platform_unseeded.contains(&(t.object as usize)))
        {
            return Err(Signal::Unsupported(
                "DeleteAll with a filter on a FlowField and delete events, database triggers, extension delete triggers, \
                 MarkedOnly, or DeleteAll(true) without an OnDelete trigger (not pinned)"
                    .into(),
            ));
        }
        let (filters, _) = self.split_filters(rec);
        self.coverage_cleared(t, temporary, &filters);
        if !temporary {
            self.global_setup_asked(t)?;
        }
        // The rows go in the order of the current key and direction (DeleteAll_ExistTrue_RunTrigger_Descending: 3, then 1).
        let (key, ascending) = (rec.borrow().key, rec.borrow().ascending);
        // One fresh set of table globals for all rows (ADR-021 decision 6), as the other row-by-row DeleteAll.
        let shared = al_builtins::value::TableGlobals::new_shared();
        if run {
            // Each row is found after the previous one is deleted, so the filters see the earlier deletes.
            let mut after = None;
            while let Some(row) = self.scan(rec, t, ascending, after.take())? {
                after = Some(position(t, key, &row));
                self.delete_flow_row(t, rec, row, &shared, true)?;
            }
            return Ok(Value::Null);
        }
        let mut rows = Vec::new();
        let mut after = None;
        while let Some(row) = self.scan(rec, t, ascending, after.take())? {
            after = Some(position(t, key, &row));
            rows.push(row);
        }
        for row in rows {
            self.delete_flow_row(t, rec, row, &shared, false)?;
        }
        Ok(Value::Null)
    }

    fn delete_flow_row(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        row: Vec<Value>,
        shared: &al_builtins::value::TableGlobals,
        run: bool,
    ) -> Result<(), Signal> {
        let mut st = state_with_fields(rec, row);
        st.filters.clear();
        st.globals = shared.share();
        self.delete_row(t, &Rc::new(RefCell::new(st)), run)?;
        Ok(())
    }
}
