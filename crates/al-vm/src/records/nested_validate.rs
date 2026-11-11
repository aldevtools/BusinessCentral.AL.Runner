//! The xRec of table code. What a Validate hands its triggers (`RecordOp::Validate`): nested in a Validate of the same record,
//! in a plain table procedure call, in a page's OnModifyRecord, else the record before the call. What a table procedure called
//! on a record value (`Rec.Proc()`) runs with (`table_proc_xrec`, by `call_site` in `interp.rs`).

use crate::records::Rec;
use crate::{Frame, Session};
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
    /// - A field OnValidate of the table, or a field OnValidate or OnAfterValidate of one of its table extensions, calls it on
    ///   Rec (`Rec.Proc()`, `this.Proc()` or unqualified): the trigger's xRec, on a stored and on a fresh record; what the
    ///   procedure assigns to xRec shows in the trigger after the call (record/validate/XRecTableProcCalls.al:186-339, al-runner
    ///   issues 7785 and 7792; the unqualified call from the table: record/TestXRecContracts.al:52, :255). Every other call from
    ///   a trigger keeps the blank xRec of a plain call, unprobed (OnInsert, OnModify, OnDelete, OnRename, OnBeforeValidate,
    ///   another record variable).
    pub(crate) fn table_proc_xrec(&mut self, object: usize, rc: &Rec, f: &Frame) -> Option<Rec> {
        let caller_xrec = {
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
            // The table itself or one of its table extensions.
            let owner = |x: usize| {
                x == object
                    || (objects[x].kind == ObjKind::TableExtension && objects[x].extends_id == objects[object].id)
            };
            let field_validate = owner(f.object)
                && owner(fi.object as usize)
                && objects[f.object].routines.get(f.routine).is_some_and(|r| {
                    let modify = r.container.as_ref().is_some_and(|(sec, _)| sec == "modify");
                    r.is_trigger
                        && ((r.field.is_some() && r.name.eq_ignore_ascii_case("OnValidate"))
                            || (modify && r.name.eq_ignore_ascii_case("OnAfterValidate")))
                });
            match (o.kind == ObjKind::Page, field_validate) {
                (true, _) | (false, true) => caller_xrec.clone(),
                (false, false) => return None,
            }
        };
        Some(caller_xrec)
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
