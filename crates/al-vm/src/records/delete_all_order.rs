//! `DeleteAll` with row code over two or more rows of a record sorted descending (`Ascending(false)` or
//! `SetAscending(Field, false)`): the rows are deleted one by one in the record's scan order, each with its row code,
//! then the next row (al-runner issue 9401; assumption `delete-all-row-code-scan-order`, corpus PR 1659).
//!
//! `scan_order_measured` is the one predicate for the record and table facts the probe measured; everything else keeps
//! main's refusal. While it runs, the row code of each row may read no row of any table (a record read, or a read
//! without a record operation: a FlowField, a query, a TableRelation, `Session::note_row_code_read`), may write no
//! row of the table but the row's own Delete, and may not commit, roll back or raise an error (none of that is measured): else
//! the run refuses with main's text, and the assumption is noted only for a run that got to its end. With no other
//! write to the table, the rows found at the start are the rows a live cursor would find.
//!
//! Entry point: `Session::delete_all_in_scan_order`, called by `delete_all_selected` (delete_all.rs).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// Per running row of a descending DeleteAll, innermost last, whether its row code read the rows of a table
/// (`Session::row_code_reads`). Empty outside such a row.
pub(crate) type RowCodeReads = RefCell<Vec<bool>>;

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
    /// - every table extension delete trigger in the table's own app (tableextension 90664, :113-135);
    /// - a table of the normal type (no `TableType`, :28-111), in the session's company (no `ChangeCompany`).
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
        let normal_table = matches!(
            self.engine.object_facts[t.object as usize].table_type,
            None | Some(al_ir::TableType::Normal)
        );
        let own_company = r.company.is_none();
        table_of_an_app && one_field_keys && direction && own_app_extensions && normal_table && own_company
    }

    /// `DeleteAll(run)` of `rec` on `t` with row code (delete events, the OnDatabaseDelete global trigger, or OnDelete with
    /// `run`), sorted descending, over two or more rows.
    /// Per row it runs `delete_row`: OnBeforeDeleteEvent, the table extension's OnBeforeDelete, OnDelete, the extension's
    /// OnDelete, OnDatabaseDelete (not for a temporary record), the extension's OnAfterDelete, OnAfterDeleteEvent; with
    /// `run` false the events (RunTrigger No) and OnDatabaseDelete only (DeleteAllRowCodeOrder.al:543-547, :610-613), the
    /// same sequence as one Delete (:566-567), then the next row. The rows go in the scan order: `Ascending(false)` reverses
    /// the primary key inside equal key values too, the `SetAscending` of the key field only that field (:345-349); with a
    /// filter, MarkedOnly, a temporary record and a RecordRef alike (:301-307, :394-399). The probe's row code reads no
    /// row of any table (it logs to a SingleInstance codeunit and reads the field values of its own row), writes no row of
    /// the table and does not commit; a write to another table is measured on corpus master
    /// (record/crud/RecordPathsUnpinned.al:41-47, :195-241). So once a row's code read a row of any table, wrote one of the table (the
    /// row itself, one it has still to visit or one outside the filter), committed or rolled back, or raised an error, the
    /// run refuses with main's text, and the assumption is noted only for a run that got to its end.
    /// `rows` are the rows the caller's forward scan found: the record's order for `SetAscending`, the reverse of it for
    /// `Ascending(false)`.
    pub(super) fn delete_all_in_scan_order(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        run: bool,
        mut rows: Vec<Vec<Value>>,
    ) -> Result<Value, Signal> {
        if !self.scan_order_measured(t, rec) {
            return unsupported(UNMEASURED);
        }
        if !rec.borrow().ascending {
            rows.reverse();
        }
        // One fresh set of table globals for all rows (ADR-021 decision 6), as the other row-by-row DeleteAll.
        let shared = al_builtins::value::TableGlobals::new_shared();
        for row in rows {
            let mark = self.tx.write_log_mark();
            let temp_writes = self.temp_table_writes(rec)?;
            self.row_code_reads.borrow_mut().push(false);
            let deleted = self.delete_row(t, &super::delete_all::row_record(rec, row, &shared), run);
            let read = self.row_code_reads.borrow_mut().pop().unwrap_or(false);
            match deleted {
                Err(Signal::Error(_)) => return unsupported(UNMEASURED),
                other => other?,
            };
            if read || !self.wrote_only_its_delete(t, rec, mark, temp_writes)? {
                return unsupported(UNMEASURED);
            }
        }
        assume(Assumption::DeleteAllRowCodeScanOrder);
        Ok(Value::Null)
    }

    /// Notes a read of the rows of a table (any table and company, temporary or not) for every running descending
    /// DeleteAll row: a record read (`record_op_inner`) or a read without a record operation (`note_indirect_read`).
    pub(crate) fn note_row_code_read(&self) {
        self.row_code_reads
            .borrow_mut()
            .iter_mut()
            .for_each(|read| *read = true);
    }

    /// The writes the temporary table of `rec` took so far (its own store); 0 for a database record.
    fn temp_table_writes(&mut self, rec: &Rec) -> Result<u64, Signal> {
        match rec.borrow().temporary {
            true => Ok(self.store_for(rec)?.writes()),
            false => Ok(0),
        }
    }

    /// Whether one row of the DeleteAll, since `mark` (and `temp_writes`, [`Self::temp_table_writes`]), wrote exactly one row
    /// of `t`, its own Delete, and the transaction went on (no Commit, no rollback: the write log of `mark` is still open).
    fn wrote_only_its_delete(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        mark: crate::state::WriteLogMark,
        temp_writes: u64,
    ) -> Result<bool, Signal> {
        let Some(writes) = self.tx.writes_since(mark) else {
            return Ok(false);
        };
        if rec.borrow().temporary {
            return Ok(self.store_for(rec)?.writes() == temp_writes + 1);
        }
        let company = rec.borrow().company.clone();
        let own = writes
            .iter()
            .filter(|w| w.table == t.object && w.company == company)
            .count();
        Ok(own == 1)
    }
}
