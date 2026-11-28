//! `DeleteAll` with row code over two or more rows of a record sorted descending (`Ascending(false)` or
//! `SetAscending(Field, false)`): the rows are deleted one by one in the record's scan order, each with its row code,
//! then the next row (al-runner issue 9401; assumption `delete-all-row-code-scan-order`, corpus PR 1659).
//!
//! `scan_order_measured` is the one predicate for the record and table facts the probe measured; everything else keeps
//! main's refusal. While it runs, every row must be the row the scan finds at that moment, unchanged, and no row the
//! filters see may come up after the last one: then a delete that follows a live cursor and one that follows the rows
//! found at the start give the same rows, so row code that writes other rows of the table changes nothing the probe did
//! not measure, or the run refuses. An error raised by the row code refuses too (not measured).
//!
//! Entry point: `Session::delete_all_in_scan_order`, called by `delete_all_selected` (delete_all.rs).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// Main's refusal, kept for every shape the probe did not measure.
const UNMEASURED: &str = "DeleteAll with delete events, database triggers or OnDelete over two or more rows of a record sorted \
                          descending: the order of the row code is not pinned";

impl Session {
    /// The record and table facts of corpus PR 1659 (tests/al-language/record/crud/DeleteAllRowCodeOrder.al), each by
    /// equality:
    /// - a table of an app, not a platform table (tables 90660 to 90663, :28-111);
    /// - a key of one field over a primary key of one field, no ad hoc key (`PK; Id` and `ByGrp; Grp`, :38-41);
    /// - `Ascending(false)` with no field set by `SetAscending` (:276, :326), or `SetAscending` of the one field of a
    ///   secondary key to false on an ascending record (:332-333, `SetCurrentKey(Grp)`);
    /// - every table extension delete trigger in the table's own app (tableextension 90664, :113-135).
    fn scan_order_measured(&self, t: &TableDef, rec: &Rec) -> bool {
        let table_of_an_app = self.engine.program.objects[t.object as usize]
            .id
            .is_some_and(|id| id < 2_000_000_000);
        let r = rec.borrow();
        let one_field_keys = r.adhoc.is_none() && t.keys.get(r.key).map(Vec::len) == Some(1) && t.keys[0].len() == 1;
        let direction =
            (!r.ascending && r.field_descending == 0) || (r.ascending && r.field_descending == 1 && r.key > 0);
        let app = self.engine.program.objects[t.object as usize].app;
        let own_app_extensions = ["OnBeforeDelete", "OnDelete", "OnAfterDelete"]
            .iter()
            .flat_map(|n| self.ext_triggers(t, n))
            .all(|(e, _)| self.engine.program.objects[e].app == app);
        table_of_an_app && one_field_keys && direction && own_app_extensions
    }

    /// `DeleteAll(run)` of `rec` on `t` with row code (delete events, the OnDatabaseDelete global trigger, or OnDelete with
    /// `run`), sorted descending, over `found` rows the filters see (`n` counts the filtered rows without marks).
    /// Per row it runs `delete_row`: OnBeforeDeleteEvent, the table extension's OnBeforeDelete, OnDelete, the extension's
    /// OnDelete, OnDatabaseDelete (not for a temporary record), the extension's OnAfterDelete, OnAfterDeleteEvent; with
    /// `run` false the events (RunTrigger No) and OnDatabaseDelete only (DeleteAllRowCodeOrder.al:543-547, :610-613).
    /// The rows go in the scan order: `Ascending(false)` reverses the primary key inside equal key values too, the
    /// `SetAscending` of the key field only that field (:345-349); with a filter, MarkedOnly, a temporary record and a
    /// RecordRef alike (:301-307, :394-399).
    pub(super) fn delete_all_in_scan_order(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        cx: &al_data::AccessContext,
        run: bool,
        filters: &[FieldFilter],
        n: usize,
    ) -> Result<Value, Signal> {
        if !self.scan_order_measured(t, rec) {
            return unsupported(UNMEASURED);
        }
        assume(Assumption::DeleteAllRowCodeScanOrder);
        let (key, ascending) = (rec.borrow().key, rec.borrow().ascending);
        let mut rows = Vec::new();
        let mut after = None;
        while let Some(row) = self.scan(rec, t, ascending, after.take())? {
            after = Some(position(t, key, &row));
            rows.push(row);
        }
        let found = rows.len();
        // One fresh set of table globals for all rows (ADR-021 decision 6), as the other row-by-row DeleteAll.
        let shared = al_builtins::value::TableGlobals::new_shared();
        let mut cursor: Option<Vec<Value>> = None;
        for row in rows {
            let live = self.scan(rec, t, ascending, cursor.clone())?;
            if !live.is_some_and(|l| crate::query_cache::same_rows(&l, &row)) {
                return unsupported(
                    "DeleteAll sorted descending whose row code deleted, changed or inserted a row it had still to visit: not pinned",
                );
            }
            cursor = Some(position(t, key, &row));
            match self.delete_row(t, &super::delete_all::row_record(rec, row, &shared), run) {
                Err(Signal::Error(_)) => {
                    return unsupported("an error raised by the row code of a DeleteAll sorted descending: not pinned");
                }
                other => other?,
            };
        }
        let left = self
            .store_for(rec)?
            .count(cx, t, filters)
            .map_err(|e| Signal::unsupported(format!("{e}")))?;
        if self.scan(rec, t, ascending, cursor)?.is_some() || left + found != n {
            return unsupported(
                "DeleteAll sorted descending whose row code inserted or changed rows its filters see: not pinned",
            );
        }
        Ok(Value::Null)
    }
}
