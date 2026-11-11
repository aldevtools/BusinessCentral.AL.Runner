//! The xRec a Validate hands its triggers (`RecordOp::Validate`): nested in a Validate of the same record, in a plain table
//! procedure call, in a page's OnModifyRecord, else the record before the call.

use crate::Session;
use crate::records::Rec;
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_ir::ObjKind;

impl Session {
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
