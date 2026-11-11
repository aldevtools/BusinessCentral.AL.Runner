//! The xRec of table code. What a Validate hands its triggers (`RecordOp::Validate`): nested in a Validate of the same record,
//! in a plain table procedure call, in a page's OnModifyRecord, else the record before the call. What a table procedure called
//! on a record value (`Rec.Proc()`) runs with (`table_proc_xrec`, by `call_site` in `interp.rs`).

use crate::records::Rec;
use crate::{Frame, Session};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_ir::ObjKind;
use std::rc::Rc;

impl Session {
    /// The caller's xRec that a procedure of table `object`, called on the record `rc` from the code running in frame `f`,
    /// shares: the caller's own xRec handle when the call is one of these, `None` otherwise (a plain call, whose xRec is the
    /// blank record `enter_record` makes: record/PWTableGlobals.al:196-260 XRecPlainProc_*).
    /// - Page code calls it on the page's Rec (`Rec.InitType()`): the page's xRec, as the page's triggers see it (Sales
    ///   Invoice Subform.OnNewRecord calls Rec.InitType, which copies xRec.Type; pages/xrec-in-table-procedure/
    ///   XRecTableProcedure.al `proc xRec=2/22/two`).
    /// - The table's own field OnValidate calls it on Rec (`Rec.ArchiveServiceCommitments()`): the trigger's xRec, from code
    ///   and from a TestPage SetValue alike. Not pinned for the `Rec.` call (the unqualified call is: record/TestXRecContracts.al:52,
    ///   :255 `internal xRec=A/stored`); Microsoft's Subscription Billing test Service Object Test.CheckChangeServiceObjectSN
    ///   needs it, so it is taken on assumption `xrec-table-proc-from-field-validate` (al-runner issue 7785), and only when the
    ///   trigger's xRec holds other values than the blank record a plain call gets (else the two read the same). Every other
    ///   call from a trigger keeps the blank xRec of a plain call until a probe measures it (the entry's probe).
    pub(crate) fn table_proc_xrec(&mut self, object: usize, rc: &Rec, f: &Frame) -> Option<Rec> {
        let (caller_xrec, page) = {
            let fi = f.instance.borrow();
            let objects = &self.engine.program.objects;
            let o = &objects[fi.object as usize];
            let (Value::Record(caller_rec), Value::Record(caller_xrec)) =
                (&fi.globals[o.rec_slot?], &fi.globals[o.xrec_slot?])
            else {
                return None;
            };
            if !Rc::ptr_eq(caller_rec, rc) {
                return None;
            }
            let own_field_validate = o.kind == ObjKind::Table
                && f.object == object
                && fi.object as usize == object
                && objects[object]
                    .routines
                    .get(f.routine)
                    .is_some_and(|r| r.is_trigger && r.field.is_some() && r.name.eq_ignore_ascii_case("OnValidate"));
            match (o.kind == ObjKind::Page, own_field_validate) {
                (true, _) => (caller_xrec.clone(), true),
                (false, true) => (caller_xrec.clone(), false),
                (false, false) => return None,
            }
        };
        if page {
            return Some(caller_xrec);
        }
        let blank = self.initial_fields(rc.borrow().table as usize);
        let differs = !Self::same_values(&caller_xrec.borrow().fields, &blank);
        (differs && assumptions::hit(Assumption::XrecTableProcFromFieldValidate)).then_some(caller_xrec)
    }

    /// The before-image of the innermost running Validate of the record at `handle`, when the Validate about to start is
    /// nested in it by the table's own code: it then sees that before-image as xRec (record/TestXRecContracts.al:133, a
    /// Validate that the table's OnValidate trigger nests). `None` when no Validate of the record runs, or when other code
    /// nests it (an event subscriber of the validate events, a codeunit that OnValidate calls): that Validate sees the
    /// record as it is when it starts, as an unnested one does (events/table-events/ValidateInValidateSubscriber.al:188-235
    /// AfterValidateSubscriber_* `B(rec=BLUE,x=BLUE)`, :236
    /// OnValidate_CodeunitValidatesSameFieldAgain `B(rec=INNER,x=BLUE)`; al-runner issue 6924 item 3.9).
    /// A part's link stamp is the platform's own Validate: it sees the blank line it pushed whatever code runs
    /// (pages/parts/QjPartPropagation.al:3954 QT_ValPartSplit_Empty_New_ThenType `T.Doc.V(H1 x= ln=0)`).
    pub(crate) fn outer_validate_before(&self, handle: usize, table_object: usize) -> Option<Vec<Value>> {
        let (_, before) = self.validating.iter().rev().find(|(h, _)| *h == handle)?;
        let objects = &self.engine.program.objects;
        let own = self.frames.last().is_some_and(|&o| {
            o == table_object
                || (objects[o].kind == ObjKind::TableExtension && objects[o].extends_id == objects[table_object].id)
        });
        (own || self.stamp_validating.contains(&handle)).then(|| before.clone())
    }

    /// The before-image a Validate of `rec` (at `handle`) hands its triggers as xRec, `outer` being `outer_validate_before` and
    /// `with_value` whether the Validate hands a new value.
    /// In a table procedure that no operation of the record called (a plain call, whose xRec is a blank record:
    /// `enter_record`), a Validate of the procedure's own Rec passes that xRec, not the record as it is
    /// (types/operators-expressions/UpThisField.al:175 UP_This_ValidateTestFieldCompound: `this.Validate(Qty, 5)` after
    /// Qty := 3 sees old=0, and `this.Qty := 7; this.Validate(this.Qty)` sees old=0). In the page's OnModifyRecord a Validate
    /// of the page's Rec passes the page's xRec or is refused (`pages/modify_record_xrec.rs`). Called from other code, the xRec is the record
    /// before the call (record/TestXRecContracts.al:190, record/TestTableEventSequenceContracts.al).
    pub(crate) fn validate_before_image(
        &self,
        handle: usize,
        outer: Option<Vec<Value>>,
        rec: &Rec,
        with_value: bool,
    ) -> Result<Vec<Value>, Signal> {
        if let Some(b) = outer {
            return Ok(b);
        }
        let depth = self.call_stack.len();
        if let Some(x) = self
            .plain_proc_xrec
            .last()
            .filter(|(h, d, _)| *h == handle && *d == depth)
        {
            return Ok(x.2.borrow().fields.clone());
        }
        let before = rec.borrow().fields.clone();
        Ok(self
            .modify_record_validate_xrec(handle, &before, with_value)?
            .unwrap_or(before))
    }
}
