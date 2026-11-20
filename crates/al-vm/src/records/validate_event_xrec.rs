//! The xRec of a procedure of a table or of one of its table extensions that a subscriber of the OnBeforeValidateEvent or
//! OnAfterValidateEvent of a Validate calls on its Rec (al-runner issue 8306): the xRec the Validate hands the event, shared
//! with the subscriber's xRec parameter. No trigger of the record runs at that moment, so `running_call_xrec` finds no call
//! whose xRec it could share. `validate_events` notes the event while its subscribers run (`validate_event`, by
//! `validate_hooks` in `records.rs`); `validate_event_xrec` takes it (by `table_proc_xrec` in `nested_validate.rs`).

use crate::records::{Rec, ValidateRun, ValidateSite};
use crate::{Frame, Session};
use al_builtins::native::Signal;
use al_builtins::value::Value;
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
        let field_measured = field.on_validate.is_some_and(|(o, _)| o == t.object as usize)
            && field.ext_before_validate.is_empty()
            && field.ext_after_validate.is_empty();
        self.validate_events.push(ValidateEvent {
            handle: Rc::as_ptr(rec) as usize,
            xrec: xrec.clone(),
            depth: self.call_stack.len(),
            field_measured,
        });
        let r = self.table_event(t, event, rec, Some(xrec), false, Some(&field.name));
        self.validate_events.pop();
        r
    }

    /// The xRec a procedure of table `object` (or of one of its table extensions) runs with when the subscriber in frame `f`
    /// of the validate event being raised calls it on `rc`: the Validate's xRec, shared, so an assignment to it shows in the
    /// subscriber's xRec parameter and in the trigger (record/validate/XRecTableProcThroughSubscriber.al:633
    /// OnAfterValidateEvent after Insert, :935 after Get, :952 on a fresh record, :1011 on a temporary record, :1025 after
    /// FindFirst, :969 the assignment, :983 OnBeforeValidateEvent, :1326 its assignment the trigger reads, :997 and :1312 a
    /// subscriber that does not declare xRec), pinned at corpus 8fcde9a6. Every fact by equality:
    /// - the innermost event being raised is a validate event of `rc`, and `f` is the subscriber's frame the event put right
    ///   above the code that called the Validate (table events put no frame of their own between them);
    /// - `f` runs an event subscriber of a codeunit that is neither bound manually nor SingleInstance; it declares Rec by `var`,
    ///   holding `rc`, alone or with xRec by `var` and CurrFieldNo, as the probe's subscribers do;
    /// - the validated field is one of the base table with its own OnValidate trigger (`ValidateEvent::field_measured`);
    /// - the Validate is the one Validate of the record that runs, called by the code the cells measured
    ///   (`measured_validate_caller`);
    /// - no record method ran on the record since the Validate started and its fields are as they were then
    ///   (`ValidateRun::touched`, `ValidateRun::entered`), and its buffer came from no TransferFields: the cells hold a record
    ///   after Insert (:633), Get (:935), FindFirst (:1025), its own Modify (:1294), fresh (:952) and temporary (:1011);
    /// - no trigger or procedure of the record's table or of its table extensions runs on the record (`running_xrecs`): the
    ///   Validate called from one of them was not measured.
    ///
    /// `None` for any other call; `running_call_xrec` then refuses a read of xRec while a validate event of `rc` is raised
    /// (`raised_validate_event_xrec`).
    pub(super) fn validate_event_xrec(&self, object: usize, rc: &Rec, f: &Frame) -> Option<Rec> {
        let ValidateEvent {
            handle,
            xrec,
            depth,
            field_measured,
        } = self.validate_events.last()?;
        let table = rc.borrow().table as usize;
        let n = self.frames.len();
        let frame = self.call_stack.last()?;
        let objects = &self.engine.program.objects;
        let sub = &objects[f.object].routines[f.routine];
        let declares = match sub.params {
            1 => true,
            3 => sub.by_var.get(1) == Some(&true),
            _ => false,
        };
        let measured = *handle == Rc::as_ptr(rc) as usize
            && n == depth + 1
            && self.frames[n - 1] == f.object
            && frame.routine == f.routine
            && !frame.event
            && objects[f.object].kind == ObjKind::Codeunit
            && !self.engine.object_facts[f.object].manual_subscriber
            && !self.engine.object_facts[f.object].single_instance
            && *field_measured
            && sub
                .attributes
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"))
            && declares
            && sub.by_var.first() == Some(&true)
            && matches!(f.locals.first(), Some(Value::Record(p)) if Rc::ptr_eq(p, rc))
            && self.table_or_its_extension(object, table)
            && self
                .sole_validate_run(*handle)
                .is_some_and(|run| self.untouched_since(run, rc))
            && self.measured_validate_caller(*handle, *depth, table)
            && self.running_xrecs(table, rc).is_empty();
        measured.then(|| xrec.clone())
    }

    /// Whether the record `rc` is as the Validate `run` found it: no record method ran on it since, its fields are those it had
    /// when the Validate's triggers started, and its buffer came from no TransferFields.
    fn untouched_since(&self, run: &ValidateRun, rc: &Rec) -> bool {
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
