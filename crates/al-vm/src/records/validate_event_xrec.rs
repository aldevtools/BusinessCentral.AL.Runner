//! The xRec of a procedure of a table or of one of its table extensions that a subscriber of the OnBeforeValidateEvent or
//! OnAfterValidateEvent of a Validate calls on its Rec (al-runner issue 8306): the xRec the Validate hands the event, shared
//! with the subscriber's xRec parameter. No trigger of the record runs at that moment, so `running_call_xrec` finds no call
//! whose xRec it could share. `validate_events` notes the event while its subscribers run (`validate_event`, by
//! `validate_hooks` in `records.rs`); `validate_event_xrec` takes it (by `table_proc_xrec` in `nested_validate.rs`).

use crate::records::Rec;
use crate::{Frame, Session};
use al_builtins::assumptions::{self, Assumption};
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
    /// subscriber's xRec parameter (record/validate/XRecTableProcThroughSubscriber.al:489 OnAfterValidateEvent after Insert,
    /// :791 after Get, :808 on a fresh record, :867 on a temporary record, :881 after FindFirst, :825 the assignment, :839
    /// OnBeforeValidateEvent, :853 a subscriber that does not declare xRec), on assumption
    /// `xrec-table-proc-from-validate-event`. Every fact by equality:
    /// - the innermost event being raised is a validate event of `rc`, and `f` is the subscriber's frame the event put right
    ///   above the code that called the Validate (table events put no frame of their own between them);
    /// - `f` runs an event subscriber of a codeunit that is not bound manually; it declares Rec by `var`, holding `rc`, alone or
    ///   with xRec by `var` and CurrFieldNo, as the probe's subscribers do;
    /// - the Validate is the one `Record.Validate` AL code called on the record (`sole_validate_from_al`).
    ///
    /// `None` for any other call, which keeps main's answer. The probe found the same xRec on every buffer it tried, so the
    /// buffer is not a fact of the rule.
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
            && self.sole_validate_from_al(*handle);
        (measured && assumptions::hit(Assumption::XrecTableProcFromValidateEvent)).then(|| xrec.clone())
    }

    /// Whether, while a validate event of `rc` is raised, the only xRec holding other values than a blank record among the calls
    /// running on `rc` is the one `validate_event_xrec` shared. A procedure called on `rc` from there through other code (a
    /// codeunit the shared procedure calls) then gets main's plain call, as it did before the share made that xRec visible
    /// (`running_call_xrec` would refuse it); not measured either way.
    pub(super) fn only_validate_event_xrec_runs(&mut self, rc: &Rec) -> bool {
        let Some((handle, xrec, _)) = self.validate_events.last().cloned() else {
            return false;
        };
        if handle != Rc::as_ptr(rc) as usize {
            return false;
        }
        let table = rc.borrow().table as usize;
        let blank = self.initial_fields(table);
        let running = self.running_xrecs(table, rc);
        running.iter().any(|(_, x)| Rc::ptr_eq(x, &xrec))
            && running
                .iter()
                .all(|(_, x)| Rc::ptr_eq(x, &xrec) || Self::same_values(&x.borrow().fields, &blank))
    }
}
