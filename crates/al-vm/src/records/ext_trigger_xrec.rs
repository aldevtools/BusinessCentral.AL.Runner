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
//! The methods that read neither its buffer nor its view (`TableName`, `TableCaption`, `FieldNo`, `FieldCaption`) run
//! (`Session::unpinned_buffer_op`).
//!
//! The table's own `OnInsert` and `OnDelete` xRec when a table extension declares `OnBeforeInsert` or `OnBeforeDelete`
//! (assumption `xrec-after-ext-before-trigger`, corpus PR 1611, record/events/XebXRecAfterExtBeforeTrigger.al:292-874): its
//! fields are Rec at the `Insert(true)` or `Delete(true)` call, before the `OnBefore...Event` subscribers and the extension's
//! trigger; its filters and key are Rec's after the extension's trigger. Only the shape the probe measures runs
//! ([`AtCall`]); every other shape gets that xRec marked [`XREC_AFTER_EXT_BEFORE_UNMEASURED`], read like the mark above.
//!
//! Entry points: `Session::run_ext_trigger_unmeasured_xrec`, `Session::ext_trigger_xrec_running`,
//! `Session::rec_at_ext_before_call`, `Session::xrec_after_ext_before`.

use crate::Session;
use crate::records::Rec;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_data::TableDef;
use std::rc::Rc;

/// The `RecordState::unpinned_buffer` reason of xRec in a table extension's insert or delete trigger.
pub(crate) const XREC_OF_EXT_INSERT_DELETE_TRIGGER: &str = "xRec in a table extension's insert or delete trigger (OnBeforeInsert, OnInsert, OnAfterInsert, OnBeforeDelete, OnDelete, OnAfterDelete): the corpus does not measure what it holds";

/// The `RecordState::unpinned_buffer` reason of xRec in a table's own `OnInsert` or `OnDelete` after a table extension's
/// `OnBeforeInsert` or `OnBeforeDelete`, in a shape corpus PR 1611 does not measure.
pub(crate) const XREC_AFTER_EXT_BEFORE_UNMEASURED: &str = "xRec in a table's OnInsert or OnDelete after a table extension's OnBeforeInsert or OnBeforeDelete, in a shape the corpus does not measure";

/// Whether `why` is a mark of this module.
pub(crate) fn unmeasured_trigger_xrec(why: &str) -> bool {
    why == XREC_OF_EXT_INSERT_DELETE_TRIGGER || why == XREC_AFTER_EXT_BEFORE_UNMEASURED
}

/// Rec at an `Insert(true)` or `Delete(true)` call on a table whose extension declares the `OnBefore...` trigger.
pub(crate) struct AtCall {
    fields: Vec<Value>,
    position: Option<Rc<[Value]>>,
    /// The call is of the shape XebXRecAfterExtBeforeTrigger.al measures (`Start`, `RunInsert`, `RunDelete`, :229-290).
    measured: bool,
}

impl Session {
    fn table_extended(&self, t: &TableDef) -> bool {
        self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .is_some_and(|s| !s.extensions.is_empty())
    }

    /// Rec at the call, taken before the `OnBefore...Event` subscribers run, when exactly the table's own trigger reads it:
    /// `None` keeps main's xRec (no extension declares `before`, or the table declares no `own` trigger).
    pub(crate) fn rec_at_ext_before_call(&self, t: &TableDef, before: &str, own: &str, rec: &Rec) -> Option<AtCall> {
        let declared = self.ext_triggers(t, before).len();
        if declared == 0 || self.table_trigger(t, own).is_none() {
            return None;
        }
        let r = rec.borrow();
        // XebXRecAfterExtBeforeTrigger.al:22-103, :229-290: one table extension declaring the trigger, a normal record of the
        // session company, no filters, the primary key, filter group 0, no marks.
        let extensions = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .map_or(0, |s| s.extensions.len());
        let measured = extensions == 1
            && declared == 1
            && r.temporary == false
            && r.company.is_none()
            && r.filters.is_empty()
            && r.key == 0
            && r.adhoc.is_none()
            && r.filter_group == 0
            && r.marks.is_empty()
            && r.marked_only == false;
        Some(AtCall {
            fields: r.fields.to_vec(),
            position: r.position.clone(),
            measured,
        })
    }

    /// xRec of the table's own trigger after the extension's `OnBefore...` trigger ran: the fields of `at`, the view of Rec now
    /// (XebXRecAfterExtBeforeTrigger.al:590 SetRange, :627 SetCurrentKey, :839). Measured when the trigger left filter group 0
    /// and did not move Rec to another row; else the mark [`XREC_AFTER_EXT_BEFORE_UNMEASURED`].
    pub(crate) fn xrec_after_ext_before(at: AtCall, rec: &Rec) -> Rec {
        let measured = at.measured && {
            let r = rec.borrow();
            r.filter_group == 0 && r.position == at.position
        };
        let x = Self::xrec_handle(rec, at.fields);
        if measured {
            assume(Assumption::XRecAfterExtBeforeTrigger);
        } else {
            x.borrow_mut().unpinned_buffer = Some(XREC_AFTER_EXT_BEFORE_UNMEASURED);
        }
        x
    }

    /// Runs the table extension trigger `name` (an insert or delete trigger) of every extension of `t` with an xRec whose reads
    /// refuse (see the module comment).
    pub(crate) fn run_ext_trigger_unmeasured_xrec(
        &mut self,
        t: &TableDef,
        name: &str,
        rec: &Rec,
    ) -> Result<(), Signal> {
        if !self.table_extended(t) {
            return Ok(());
        }
        let x = Self::xrec_handle(rec, self.initial_fields(t.object as usize).to_vec());
        x.borrow_mut().unpinned_buffer = Some(XREC_OF_EXT_INSERT_DELETE_TRIGGER);
        self.ext_xrec_triggers += 1;
        let r = self.run_ext_trigger(t, name, rec, Some(&x));
        self.ext_xrec_triggers -= 1;
        r
    }

    /// Whether a table extension's insert or delete trigger runs on `rc` (a record of table `table`): its xRec carries
    /// [`XREC_OF_EXT_INSERT_DELETE_TRIGGER`], and so does the xRec of a procedure called on `rc` (`running_call_xrec.rs`).
    /// A counter (`Session::ext_xrec_triggers`) keeps the scan off the calls made while no such trigger runs.
    pub(super) fn ext_trigger_xrec_running(&self, table: usize, rc: &Rec) -> bool {
        self.ext_xrec_triggers > 0
            && self
                .running_xrecs(table, rc)
                .iter()
                .any(|(_, x)| x.borrow().unpinned_buffer == Some(XREC_OF_EXT_INSERT_DELETE_TRIGGER))
    }
}
