//! The xRec of a Validate nested in a Validate of the same record (`RecordOp::Validate`).

use crate::Session;
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
}
