//! xRec in the insert and delete triggers of a table extension (`OnBeforeInsert`, `OnInsert`, `OnAfterInsert`,
//! `OnBeforeDelete`, `OnDelete`, `OnAfterDelete`): the corpus does not measure what it holds, so every read of it refuses.
//!
//! What is pinned nearby: xRec in the table's own `OnInsert`, `OnModify`, `OnDelete` and `OnRename` (record/TestXRecContracts.al:77-161,
//! record/TestXRecStampFields.al:38-73) and in a table extension's `OnAfterModify` (table-objects/extensions/
//! TestTableExtensionTableTriggers_Tests.al:183). The table extension fixture that logs its insert and delete triggers
//! (_fixtures/tables/ALTTriggeredOrder.TableExt.al:52-89, :139-176) does not read xRec there. Before this rule `OnAfterInsert` got the
//! blank xRec of a plain call, and an outside report (al-runner issue 8986) shows BC code that needs xRec to equal Rec there.
//!
//! The trigger gets a blank xRec marked [`XREC_OF_EXT_INSERT_DELETE_TRIGGER`] (`RecordState::unpinned_buffer`). Neither its
//! buffer nor its view (filters, key, temporary table) is measured, so a field read, `Format`, a copy, a field write and every
//! record method on it refuse, `Get` and the methods that read only the view among them (`records.rs` `buffer_read_refused`,
//! `copy_refused_mark`, `held_part_field_write`, `Session::unpinned_buffer_op`). A procedure of the table or of a table
//! extension called on Rec while the trigger runs gets a blank xRec with the same mark (`running_call_xrec.rs`).
//!
//! Entry points: `Session::run_ext_trigger_unmeasured_xrec`, `Session::ext_trigger_xrec_running`.

use crate::Session;
use crate::records::Rec;
use al_builtins::native::Signal;
use al_data::TableDef;

/// The `RecordState::unpinned_buffer` reason of xRec in a table extension's insert or delete trigger.
pub(crate) const XREC_OF_EXT_INSERT_DELETE_TRIGGER: &str = "xRec in a table extension's insert or delete trigger (OnBeforeInsert, OnInsert, OnAfterInsert, OnBeforeDelete, OnDelete, OnAfterDelete): the corpus does not measure what it holds";

impl Session {
    /// Runs the table extension trigger `name` (an insert or delete trigger) of every extension of `t` with an xRec whose reads
    /// refuse (see the module comment).
    pub(crate) fn run_ext_trigger_unmeasured_xrec(
        &mut self,
        t: &TableDef,
        name: &str,
        rec: &Rec,
    ) -> Result<(), Signal> {
        let extended = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .is_some_and(|s| !s.extensions.is_empty());
        if !extended {
            return Ok(());
        }
        let x = Self::xrec_handle(rec, self.initial_fields(t.object as usize).to_vec());
        x.borrow_mut().unpinned_buffer = Some(XREC_OF_EXT_INSERT_DELETE_TRIGGER);
        self.run_ext_trigger(t, name, rec, Some(&x))
    }

    /// Whether a table extension's insert or delete trigger runs on `rc` (a record of table `table`): its xRec carries
    /// [`XREC_OF_EXT_INSERT_DELETE_TRIGGER`], and so does the xRec of a procedure called on `rc` (`running_call_xrec.rs`).
    pub(super) fn ext_trigger_xrec_running(&self, table: usize, rc: &Rec) -> bool {
        self.running_xrecs(table, rc)
            .iter()
            .any(|(_, x)| x.borrow().unpinned_buffer == Some(XREC_OF_EXT_INSERT_DELETE_TRIGGER))
    }
}
