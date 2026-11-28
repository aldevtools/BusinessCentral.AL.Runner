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
//! (assumption `xrec-after-ext-before-trigger`, corpus PR 1611, merged and not yet pinned, record/events/XebXRecAfterExtBeforeTrigger.al:292-874): its
//! fields are Rec at the `Insert(true)` or `Delete(true)` call, before the `OnBefore...Event` subscribers and the extension's
//! trigger; its filters and key are Rec's after the extension's trigger. Only the call shape the probe measures runs
//! ([`AtCall`]); every other call shape keeps main's xRec (the buffer after the subscribers, before the extension's
//! trigger). In a measured call where the subscribers or the extension's trigger ran a record method on Rec other than
//! `SetRange` and `SetCurrentKey` (a `Get` or `Find` of the same row among them), wrote a whole record into it (`Rec := Other`),
//! changed a view fact the entry requires at the call, left another filter group or set a filter in another group, that xRec
//! is marked [`XREC_AFTER_EXT_BEFORE_UNMEASURED`], read like the mark above. A call runs through the assumption only when
//! code of the table or of its extensions names xRec (`table_code_names_xrec`), so a test that cannot read it stays clean.
//!
//! Entry points: `Session::run_ext_trigger_unmeasured_xrec`, `Session::ext_trigger_xrec_running`,
//! `Session::rec_at_ext_before_call`, `Session::xrec_after_ext_before`.

use crate::Session;
use crate::records::Rec;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The `touched` flags of the measured calls still in their subscribers or extension trigger, per record handle
/// (`Session::ext_before_watch`). An [`AtCall`] removes its own entry when it is dropped, so the list is empty outside
/// such a call.
pub(crate) type ExtBeforeWatch = Rc<RefCell<Vec<(usize, Rc<Cell<bool>>)>>>;

/// The `RecordState::unpinned_buffer` reason of xRec in a table extension's insert or delete trigger.
pub(crate) const XREC_OF_EXT_INSERT_DELETE_TRIGGER: &str = "xRec in a table extension's insert or delete trigger (OnBeforeInsert, OnInsert, OnAfterInsert, OnBeforeDelete, OnDelete, OnAfterDelete): the corpus does not measure what it holds";

/// The `RecordState::unpinned_buffer` reason of xRec in a table's own `OnInsert` or `OnDelete` after a table extension's
/// `OnBeforeInsert` or `OnBeforeDelete`, in a shape corpus PR 1611 does not measure.
pub(crate) const XREC_AFTER_EXT_BEFORE_UNMEASURED: &str = "xRec in a table's OnInsert or OnDelete after a table extension's OnBeforeInsert or OnBeforeDelete, in a shape the corpus does not measure";

/// Whether `why` is a mark of this module.
pub(crate) fn unmeasured_trigger_xrec(why: &str) -> bool {
    why == XREC_OF_EXT_INSERT_DELETE_TRIGGER || why == XREC_AFTER_EXT_BEFORE_UNMEASURED
}

/// Rec at an `Insert(true)` or `Delete(true)` call of the shape XebXRecAfterExtBeforeTrigger.al measures (`Start`,
/// `RunInsert`, `RunDelete`, :229-290) on a table whose extension declares the `OnBefore...` trigger.
pub(crate) struct AtCall {
    fields: Vec<Value>,
    position: Option<Rc<[Value]>>,
    /// Some code of the table or of one of its extensions names xRec ([`table_code_names_xrec`]): only then can the trigger
    /// read the served xRec, so only then does the call run through the assumption.
    xrec_named: bool,
    /// Set when a record method other than `SetRange` and `SetCurrentKey`, or a write of a whole record, ran on Rec between
    /// the call and the end of the extension's trigger ([`Session::note_ext_before_touch`]): the probe measures only field
    /// assignments, `SetRange` and `SetCurrentKey` there.
    touched: Rc<Cell<bool>>,
    /// The call reads the record first (`Delete`, [`Session::rec_at_ext_before_call`]).
    read_first: bool,
    /// The session's watch list, which holds `touched` while this call lives (`Session::ext_before_watch`).
    watch: ExtBeforeWatch,
}

/// The measured call ended, by its own trigger or by an error before it: its watch entry goes.
impl Drop for AtCall {
    fn drop(&mut self) {
        self.watch.borrow_mut().retain(|(_, t)| !Rc::ptr_eq(t, &self.touched));
    }
}

/// Whether a routine of table object `table` or of one of its table extensions names that object's xRec, or passes it to a
/// `var` parameter. xRec exists only in that code, so when none names it no read of it can happen.
fn table_code_names_xrec(program: &al_ir::Program, table: usize) -> bool {
    let objects = &program.objects;
    let base_slot = objects[table].xrec_slot;
    let exts = objects[table].table.as_ref().map_or(&[][..], |s| &s.extensions[..]);
    std::iter::once(table).chain(exts.iter().copied()).any(|o| {
        let own_slot = objects[o].xrec_slot;
        objects[o].routines.iter().any(|routine| {
            let mut found = false;
            al_ir::types::for_each_expr_post(routine.body.stmts(), &mut |x| {
                found |= crate::pages::validate_xrec::expr_names_xrec(x, own_slot, base_slot);
            });
            found
        })
    })
}

/// The view facts XebXRecAfterExtBeforeTrigger.al:22-103, :229-290 hold at the call and that its extension and subscribers do
/// not change: a normal record of the session company, no marks, not MarkedOnly, no adhoc key, ascending order.
fn view_as_measured(r: &RecordState) -> bool {
    !r.temporary
        && r.company.is_none()
        && r.marks.is_empty()
        && !r.marked_only
        && r.adhoc.is_none()
        && r.ascending
        && r.field_descending == 0
}

impl Session {
    fn table_extended(&self, t: &TableDef) -> bool {
        self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .is_some_and(|s| !s.extensions.is_empty())
    }

    /// Rec at the call, taken before the `OnBefore...Event` subscribers run, when the call is of the measured shape: `None`
    /// keeps main's xRec (a call that runs no trigger (`run` false), no extension declares `before`, the table declares no
    /// `own` trigger, or another call shape). `read_first`: the probe measures the call only on a record that was read
    /// before it (`Delete`: RunDelete at XebXRecAfterExtBeforeTrigger.al:268-290 Gets the row first).
    pub(crate) fn rec_at_ext_before_call(
        &mut self,
        t: &TableDef,
        run: bool,
        before: &str,
        own: &str,
        read_first: bool,
        rec: &Rec,
    ) -> Option<AtCall> {
        let declared = self.ext_triggers(t, before).len();
        if !run || declared == 0 || self.table_trigger(t, own).is_none() {
            return None;
        }
        // XebXRecAfterExtBeforeTrigger.al:22-103, :229-290: one table extension declaring the trigger, no filters, the primary
        // key, filter group 0 and the view facts of `view_as_measured`.
        let extensions = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .map_or(0, |s| s.extensions.len());
        let r = rec.borrow();
        let measured = extensions == 1
            && declared == 1
            && view_as_measured(&r)
            && r.filters.is_empty()
            && r.key == 0
            && r.filter_group == 0
            && (r.position.is_some() || !read_first);
        if !measured {
            return None;
        }
        let program = &self.engine.program;
        let xrec_named = *self.engine.table_facts[t.object as usize]
            .xrec_named
            .get_or_init(|| table_code_names_xrec(program, t.object as usize));
        let touched = Rc::new(Cell::new(false));
        self.ext_before_watch
            .borrow_mut()
            .push((Rc::as_ptr(rec) as usize, touched.clone()));
        Some(AtCall {
            fields: r.fields.to_vec(),
            position: r.position.clone(),
            xrec_named,
            touched,
            read_first,
            watch: self.ext_before_watch.clone(),
        })
    }

    /// Notes that a record method `method` ran on `rec` (`Session::record_op`), or a whole record was written into it
    /// (`method` `None`: `Session::replace_record_fields`, or another record's `CopyFilter` into a field of `rec`, `recmeta.rs`): on a call of the measured shape still in its subscribers or the
    /// extension's trigger, every method but `SetRange` and `SetCurrentKey` (XebXRecAfterExtBeforeTrigger.al:590, :627)
    /// sets [`AtCall::touched`].
    pub(crate) fn note_ext_before_touch(&mut self, method: Option<al_builtins::record_op::RecordOp>, rec: &Rec) {
        use al_builtins::record_op::RecordOp;
        if matches!(method, Some(RecordOp::SetRange | RecordOp::SetCurrentKey)) {
            return;
        }
        let handle = Rc::as_ptr(rec) as usize;
        for (h, touched) in self.ext_before_watch.borrow().iter() {
            if *h == handle {
                touched.set(true);
            }
        }
    }

    /// xRec of the table's own trigger after the extension's `OnBefore...` trigger ran: the fields of `at`, the view of Rec now
    /// (XebXRecAfterExtBeforeTrigger.al:590 SetRange, :627 SetCurrentKey, :839). Measured when no record method but `SetRange` and
    /// `SetCurrentKey` and no whole-record write ran on Rec ([`AtCall::touched`]), the view facts of `view_as_measured` still
    /// hold, the trigger left filter group 0, every filter of Rec is in group 0 and Rec is on the row of the call; else the mark
    /// [`XREC_AFTER_EXT_BEFORE_UNMEASURED`].
    pub(crate) fn xrec_after_ext_before(mut at: AtCall, rec: &Rec) -> Rec {
        let measured = {
            let r = rec.borrow();
            !at.touched.get()
                && view_as_measured(&r)
                && r.filter_group == 0
                && r.filters.iter().all(|f| f.group == 0)
                && match (&r.position, &at.position) {
                    (None, None) => true,
                    // Delete only: Rec was on a row at the call and the runner's `SetRange` forgot it (op_inner.rs
                    // `RecordOp::SetRange`): Get, then the extension's SetRange, then Delete(true) is
                    // XebXRecAfterExtBeforeTrigger.al:268-290, :839. No cell measures Insert on a row with that SetRange. Every
                    // other record method and whole-record write on Rec sets `touched`.
                    (None, Some(_)) => at.read_first,
                    (Some(now), Some(then)) => Rc::ptr_eq(now, then) || Self::same_values(now, then),
                    _ => false,
                }
        };
        let x = Self::xrec_handle(rec, std::mem::take(&mut at.fields));
        if measured {
            if at.xrec_named {
                assume(Assumption::XRecAfterExtBeforeTrigger);
            }
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
