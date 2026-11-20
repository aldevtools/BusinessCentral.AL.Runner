//! The xRec of a procedure of a table or of one of its table extensions that a subscriber of the OnBeforeValidateEvent or
//! OnAfterValidateEvent of a Validate calls on its Rec (al-runner issue 8306): the xRec the Validate hands the event, shared
//! with the subscriber's xRec parameter. No trigger of the record runs at that moment, so `running_call_xrec` finds no call
//! whose xRec it could share. `validate_events` notes the event while its subscribers run (`validate_event`, by
//! `validate_hooks` in `records.rs`); `validate_event_xrec` takes it (by `table_proc_xrec` in `nested_validate.rs`).

use crate::records::{Rec, ValidateSite};
use crate::{Frame, Session};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_data::TableDef;
use al_ir::ObjKind;
use std::rc::Rc;

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
        let depth = self.call_stack.len();
        self.validate_events
            .push((Rc::as_ptr(rec) as usize, xrec.clone(), depth));
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
    /// - `f` runs an event subscriber of a codeunit that is not bound manually; it declares Rec by `var`, holding `rc`, alone or
    ///   with xRec by `var` and CurrFieldNo, as the probe's subscribers do;
    /// - the Validate is the one Validate of the record that runs, and AL code called it as `Record.Validate` (:1192 from a
    ///   page action, :1238 with one argument, :1294 from an OnAfterModifyEvent subscriber), as `FieldRef.Validate` (:1257) or
    ///   as the write of a TestPage control (:1215 SetValue; not on the line a trapped static card's `New()` started);
    /// - no trigger or procedure of the record's table or of its table extensions runs on the record (`running_xrecs`): the
    ///   Validate called from one of them was not measured.
    ///
    /// `None` for any other call; `running_call_xrec` then refuses a read of xRec while a validate event of `rc` is raised
    /// (`raised_validate_event_xrec`). The probe found the same xRec on every buffer it tried, so the buffer is not a fact of
    /// the rule.
    pub(super) fn validate_event_xrec(&self, object: usize, rc: &Rec, f: &Frame) -> Option<Rec> {
        let (handle, xrec, depth) = self.validate_events.last()?;
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
            && sub
                .attributes
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"))
            && declares
            && sub.by_var.first() == Some(&true)
            && matches!(f.locals.first(), Some(Value::Record(p)) if Rc::ptr_eq(p, rc))
            && self.table_or_its_extension(object, table)
            && self.sole_validate_measured_for_event(*handle)
            && self.running_xrecs(table, rc).is_empty();
        measured.then(|| xrec.clone())
    }

    /// Whether the one Validate of the record at `handle` that runs now has a call site the probe measured for a validate
    /// event's subscriber (`validate_event_xrec`).
    fn sole_validate_measured_for_event(&self, handle: usize) -> bool {
        matches!(
            self.sole_validate_site(handle),
            Some(
                ValidateSite::AlRecordValidate
                    | ValidateSite::FieldRef
                    | ValidateSite::TestPageControl {
                        new_line_of_trapped_static_card: false
                    }
            )
        )
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
            .find(|(h, _, _)| *h == handle)
            .map(|(_, x, _)| x.clone())
    }
}
