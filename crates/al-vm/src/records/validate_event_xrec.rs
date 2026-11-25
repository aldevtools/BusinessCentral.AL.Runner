//! The xRec of a procedure of a table or of one of its table extensions that a subscriber of the OnBeforeValidateEvent or
//! OnAfterValidateEvent of a Validate calls on its Rec (al-runner issue 8306): the xRec the Validate hands the event, shared
//! with the subscriber's xRec parameter. No trigger of the record runs at that moment, so `running_call_xrec` finds no call
//! whose xRec it could share. `validate_events` notes the event while its subscribers run (`validate_event`, by
//! `validate_hooks` in `records.rs`); `validate_event_xrec` takes it (by `table_proc_xrec` in `nested_validate.rs`).
//!
//! The xRec of a procedure of a table or of its table extensions that a subscriber of the
//! OnBeforeValidateEvent or OnAfterValidateEvent of a Validate calls on its Rec: the xRec the
//! Validate hands the event, shared in the facts the pinned cells share (the subscriber, the field,
//! the Validate's caller, the record untouched since the Validate started: `ValidateRun::touched`,
//! noted by `record_op`) (al-runner issue 8306, record/validate/XRecTableProcThroughSubscriber.al);
//! any other procedure called on the record while the event is raised refuses a read of xRec. Entry
//! points: `validate_event` (by `validate_hooks` in `records.rs`, notes `Session::validate_events`),
//! `validate_event_xrec` (by `table_proc_xrec`), `raised_validate_event_xrec` (by
//! `running_call_xrec`).

use crate::records::{Rec, ValidateRun, ValidateSite};
use crate::{Frame, Session};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;
use al_ir::ObjKind;
use std::rc::Rc;

/// An OnBeforeValidateEvent or OnAfterValidateEvent being raised (`Session::validate_events`).
pub(crate) struct ValidateEvent {
    /// The record's handle.
    handle: usize,
    /// The xRec the Validate hands the event.
    xrec: Rec,
    /// The `call_stack` index of its subscribers' frames; the code that called the Validate runs right below.
    depth: usize,
    /// The field is one of the base table with its own OnValidate trigger and no OnBeforeValidate or OnAfterValidate of a
    /// table extension, as the probe's Mode field.
    field_measured: bool,
    /// The event is the OnAfterValidateEvent.
    after: bool,
}

impl Session {
    /// Raises the OnBeforeValidateEvent or OnAfterValidateEvent (`event`) of a Validate of `rec` that hands `xrec` as xRec,
    /// with the event noted in `validate_events` while its subscribers run.
    pub(super) fn validate_event(
        &mut self,
        t: &TableDef,
        event: &str,
        rec: &Rec,
        xrec: &Rec,
        field: &al_ir::Field,
    ) -> Result<(), Signal> {
        let before = event == "onbeforevalidateevent";
        let handle = Rc::as_ptr(rec) as usize;
        if !self.listened(t, event, Some(&field.name))? {
            if before {
                self.note_kept_by_before_event(handle, true);
            }
            return self.table_event(t, event, rec, Some(xrec), false, Some(&field.name));
        }
        let field_measured = field.on_validate.is_some_and(|(o, _)| o == t.object as usize)
            && field.ext_before_validate.is_empty()
            && field.ext_after_validate.is_empty();
        self.validate_events.push(ValidateEvent {
            handle,
            xrec: xrec.clone(),
            depth: self.call_stack.len(),
            field_measured,
            after: !before,
        });
        let r = self.table_event(t, event, rec, Some(xrec), false, Some(&field.name));
        self.validate_events.pop();
        if before && r.is_ok() {
            let kept = self
                .sole_validate_run(handle)
                .is_some_and(|run| Self::same_values(&rec.borrow().fields, &run.entered));
            self.note_kept_by_before_event(handle, kept);
        }
        r
    }

    /// Notes on the innermost Validate of the record at `handle` whether its OnBeforeValidateEvent returned with the fields
    /// the Validate started with (`ValidateRun::kept_by_before_event`).
    fn note_kept_by_before_event(&mut self, handle: usize, kept: bool) {
        if let Some(run) = self.validate_sites.iter_mut().rev().find(|v| v.handle == handle) {
            run.kept_by_before_event = kept;
        }
    }

    /// The xRec a procedure of table `object` (or of one of its table extensions) runs with when the subscriber in frame `f`
    /// of the validate event being raised calls it on `rc`: the Validate's xRec, shared, so an assignment to it shows in the
    /// subscriber's xRec parameter and in the trigger (record/validate/XRecTableProcThroughSubscriber.al:633
    /// OnAfterValidateEvent after Insert, :935 after Get, :952 on a fresh record, :1011 on a temporary record, :1025 after
    /// FindFirst, :969 the assignment, :983 OnBeforeValidateEvent, :1326 its assignment the trigger reads, :997 and :1312 a
    /// subscriber that does not declare xRec), pinned at corpus 8fcde9a6. Every fact by equality:
    /// - the innermost event being raised is a validate event of `rc`, and the subscriber's frame is the one the event put
    ///   right above the code that called the Validate (table events put no frame of their own between them); `f` is that
    ///   frame, or, through assumption `xrec-table-proc-routes`, one helper procedure of its codeunit right above it
    ///   (`through_helper`; the subscriber then declares xRec);
    /// - the subscriber runs an event subscriber of a codeunit that is neither bound manually nor SingleInstance; it declares
    ///   Rec by `var`, holding `rc`, alone or with xRec by `var` and CurrFieldNo, as the probe's subscribers do;
    /// - the validated field is one of the base table with its own OnValidate trigger (`ValidateEvent::field_measured`);
    /// - the Validate is the one Validate of the record that runs, called by the code the cells measured
    ///   (`measured_validate_caller`);
    /// - no record method ran on the record since the Validate started and its fields are as they were then
    ///   (`ValidateRun::touched`, `ValidateRun::entered`), and its buffer came from no TransferFields: the cells hold a record
    ///   after Insert (:633), Get (:935), FindFirst (:1025), its own Modify (:1294), fresh (:952) and temporary (:1011). Or,
    ///   through assumption `xrec-table-proc-routes`, in an OnAfterValidateEvent, the fields changed only after the
    ///   OnBeforeValidateEvent returned, by no record method (`changed_after_before_event`);
    /// - no trigger or procedure of the record's table or of its table extensions runs on the record (`running_xrecs`): the
    ///   Validate called from one of them was not measured.
    ///
    /// `None` for any other call; `running_call_xrec` then refuses a read of xRec while a validate event of `rc` is raised
    /// (`raised_validate_event_xrec`).
    pub(super) fn validate_event_xrec(&self, object: usize, rc: &Rec, f: &Frame) -> Option<Rec> {
        let ev = self.validate_events.last()?;
        let ValidateEvent {
            handle,
            xrec,
            depth,
            field_measured,
            ..
        } = ev;
        let table = rc.borrow().table as usize;
        let n = self.frames.len();
        let frame = self.call_stack.last()?;
        let (sub_o, sub_frame) = (*self.frames.get(*depth)?, *self.call_stack.get(*depth)?);
        let objects = &self.engine.program.objects;
        let sub = &objects[sub_o].routines[sub_frame.routine];
        // Parameters bind by name: the second of three is xRec only when it is a record of the table (review of PR 8307,
        // 2026-11-22: `(var Rec; var CurrFieldNo: Integer; xRec)` is not measured).
        let declares_xrec = sub.params == 3
            && sub.by_var.get(1) == Some(&true)
            && matches!(sub.locals.get(1).map(|l| &l.ty), Some(RtType::Record(t, _)) if *t as usize == table);
        // The subscriber calls the procedure itself (`n == depth + 1`, frame `f` is the subscriber's), or, from an
        // OnAfterValidateEvent, through one helper procedure of its codeunit (`through_helper`, assumed).
        let (calls, helper) = match n.checked_sub(*depth) {
            Some(1) => (
                (sub.params == 1 || declares_xrec)
                    && matches!(f.locals.first(), Some(Value::Record(p)) if Rc::ptr_eq(p, rc)),
                false,
            ),
            Some(2) => (
                ev.after && declares_xrec && self.through_helper(sub_o, rc, xrec, table, f),
                true,
            ),
            _ => return None,
        };
        let run = self.sole_validate_run(*handle)?;
        let untouched = self.untouched_since(run, rc);
        let measured = *handle == Rc::as_ptr(rc) as usize
            && self.frames[n - 1] == f.object
            && frame.routine == f.routine
            && !sub_frame.event
            && objects[sub_o].kind == ObjKind::Codeunit
            && !self.engine.object_facts[sub_o].manual_subscriber
            && !self.engine.object_facts[sub_o].single_instance
            && *field_measured
            && sub
                .attributes
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"))
            && sub.by_var.first() == Some(&true)
            && calls
            && self.table_or_its_extension(object, table)
            && (untouched || (ev.after && Self::changed_after_before_event(run, rc)))
            && self.measured_validate_caller(*handle, *depth, table)
            && self.running_xrecs(table, rc).is_empty();
        if measured && (helper || !untouched) {
            assumptions::assume(Assumption::XrecTableProcRoutes);
        }
        measured.then(|| xrec.clone())
    }

    /// Whether frame `f`, right above the subscriber's frame, runs one procedure of the subscriber's codeunit `sub_o` that is
    /// no trigger and has no attributes, called directly (not as an event, not through an interface), whose parameters are the
    /// record `rc` by `var`, then optionally the event's xRec `xrec` by `var` (a record of table `table`) and a Boolean by
    /// value: the reporter's `Helper(var Line; var xLine)` (al-runner issue 8974), the probe's `Helper(var Line; var xLine;
    /// Ext: Boolean)` and `HelperRecOnly(var Line)` (record/validate/XRecTableProcRoutes.al, corpus PR 1583, assumption
    /// `xrec-table-proc-routes`).
    fn through_helper(&self, sub_o: usize, rc: &Rec, xrec: &Rec, table: usize, f: &Frame) -> bool {
        let n = self.frames.len();
        let cf = self.call_stack[n - 1];
        let r = &self.engine.program.objects[sub_o].routines[cf.routine];
        let is_rec = |i: usize, of: &Rec| matches!(f.locals.get(i), Some(Value::Record(p)) if Rc::ptr_eq(p, of));
        let xrec_param = || {
            r.by_var.get(1) == Some(&true)
                && matches!(r.locals.get(1).map(|l| &l.ty), Some(RtType::Record(t, _)) if *t as usize == table)
                && is_rec(1, xrec)
        };
        let params_measured = match r.params {
            1 => true,
            2 => xrec_param(),
            3 => {
                xrec_param()
                    && r.by_var.get(2) == Some(&false)
                    && matches!(r.locals.get(2).map(|l| &l.ty), Some(RtType::Boolean))
            }
            _ => false,
        };
        self.frames[n - 1] == sub_o
            && f.object == sub_o
            && cf.routine == f.routine
            && !cf.event
            && !self.iface_frames.contains(&(n - 1))
            && !r.is_trigger
            && r.attributes.is_empty()
            && params_measured
            && r.by_var.first() == Some(&true)
            && is_rec(0, rc)
    }

    /// Whether the record `rc` may hold other fields than the Validate `run` started with, changed only after its
    /// OnBeforeValidateEvent returned: that event returned with the fields the Validate started with, no record method ran on
    /// the record since the Validate started, and its buffer came from no TransferFields. The probe changed it by an
    /// assignment in the field's OnValidate trigger, in a table procedure and in a codeunit that trigger calls, and in the
    /// subscriber before the call (assumption `xrec-table-proc-routes`: record/validate/XRecTableProcRoutes.al, corpus PR
    /// 1583; al-runner issue 8974). Also read by `cell_facts` in `running_call_xrec.rs`.
    pub(super) fn changed_after_before_event(run: &ValidateRun, rc: &Rec) -> bool {
        !run.touched && run.kept_by_before_event && rc.borrow().row_origin.is_none_or(|o| !o.lock_only)
    }

    /// Whether the record `rc` is as the Validate `run` found it: no record method ran on it since, its fields are those it had
    /// when the Validate's triggers started, and its buffer came from no TransferFields. Also read by `cell_facts` in
    /// `running_call_xrec.rs`.
    pub(super) fn untouched_since(&self, run: &ValidateRun, rc: &Rec) -> bool {
        let r = rc.borrow();
        !run.touched && Self::same_values(&r.fields, &run.entered) && r.row_origin.is_none_or(|o| !o.lock_only)
    }

    /// Whether the one Validate of the record at `handle` that runs now came from a caller the cells measured; the code that
    /// called it runs in the frames below `depth`:
    /// - `Record.Validate` from a test codeunit's own procedures (:633 and the other cells, :1238 with one argument), from a
    ///   page action's OnAction (:1192), or from a subscriber of the record's table's OnAfterModifyEvent (:1294);
    /// - `FieldRef.Validate` (:1257) and the write of a TestPage control (:1215 SetValue; not on the line a trapped static
    ///   card's `New()` started) from a test codeunit's own procedures.
    fn measured_validate_caller(&self, handle: usize, depth: usize, table: usize) -> bool {
        let test_code = || depth > 0 && self.only_test_codeunit_frames(0, self.frames.len() - depth);
        match self.sole_validate_site(handle) {
            Some(ValidateSite::AlRecordValidate) => {
                test_code() || self.page_action_at(depth - 1) || self.after_modify_subscriber_at(depth - 1, table)
            }
            Some(
                ValidateSite::FieldRef
                | ValidateSite::TestPageControl {
                    new_line_of_trapped_static_card: false,
                },
            ) => test_code(),
            _ => false,
        }
    }

    /// Whether frame `i` runs the OnAction trigger of a page action.
    fn page_action_at(&self, i: usize) -> bool {
        let (o, cf) = (self.frames[i], self.call_stack[i]);
        let object = &self.engine.program.objects[o];
        let r = &object.routines[cf.routine];
        object.kind == ObjKind::Page
            && !cf.event
            && r.is_trigger
            && r.name.eq_ignore_ascii_case("OnAction")
            && r.container.as_ref().is_some_and(|(sec, _)| sec == "action")
    }

    /// Whether frame `i` runs a subscriber of the OnAfterModifyEvent of table `table`, and of nothing else, in a codeunit that is
    /// neither bound manually nor SingleInstance.
    fn after_modify_subscriber_at(&self, i: usize, table: usize) -> bool {
        let (o, cf) = (self.frames[i], self.call_stack[i]);
        let facts = &self.engine.object_facts[o];
        self.engine.program.objects[o].kind == ObjKind::Codeunit
            && !cf.event
            && !facts.manual_subscriber
            && !facts.single_instance
            && self.engine.events.subscriptions_of(o, cf.routine) == [(table, "onaftermodifyevent")]
    }

    /// The xRec the innermost validate event of `rc` being raised hands its subscribers, if one is. Every procedure called on
    /// `rc` while it is raised outside `validate_event_xrec`'s shape (through other code, from a subscriber bound manually or
    /// declaring xRec by value, a Validate called from the record's own code, at another call site) is not measured; the
    /// probe's neighbors read this xRec (record/validate/XRecTableProcThroughSubscriber.al:1280 through a table extension
    /// procedure and a codeunit), never the blank one of a plain call.
    pub(super) fn raised_validate_event_xrec(&self, rc: &Rec) -> Option<Rec> {
        let handle = Rc::as_ptr(rc) as usize;
        self.validate_events
            .iter()
            .rev()
            .find(|e| e.handle == handle)
            .map(|e| e.xrec.clone())
    }
}
