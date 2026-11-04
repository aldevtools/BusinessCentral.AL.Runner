//! The xRec of a Validate nested in a Validate of the same record (`RecordOp::Validate`).

use crate::Session;
use crate::records::Rec;
use al_builtins::native::Signal;
use al_builtins::value::Value;

impl Session {
    /// The before-image of the outer Validate of the record at `handle`, which a Validate nested in it sees as xRec
    /// (corpus record/TestXRecContracts.al:133, a Validate that the table's own OnValidate trigger nests); `None` when no
    /// Validate of the record is running.
    ///
    /// A Validate that an event subscriber nests, started inside the outer Validate (a subscriber of OnAfterValidateEvent
    /// that validates the field again), is not pinned: the outer before-image or the record as it is now. Where the two
    /// differ it is refused (al-runner issue 6924 item 3.9; probe events/table-events/ValidateInValidateSubscriber.al on
    /// corpus branch al-runtime-rs-staging-r6924b, docs/probes-needed.md).
    pub(crate) fn outer_validate_before(&self, handle: usize, rec: &Rec) -> Result<Option<Vec<Value>>, Signal> {
        let Some((_, before, subscribers)) = self.validating.iter().find(|(h, _, _)| *h == handle) else {
            return Ok(None);
        };
        if self.events.subscriber_stack.len() > *subscribers && !Self::same_values(before, &rec.borrow().fields) {
            return Err(Signal::Unsupported(
                "a Validate of a record that an event subscriber nests in a Validate of the same record: whether its xRec is the outer before-image or the record as it is now is not pinned (docs/probes-needed.md)".into(),
            ));
        }
        Ok(Some(before.clone()))
    }

    /// Marks the record at `handle` as validating, with the before-image its nested Validates see as xRec.
    pub(crate) fn push_validating(&mut self, handle: usize, before: Vec<Value>) {
        let subscribers = self.events.subscriber_stack.len();
        self.validating.push((handle, before, subscribers));
    }
}
