//! `RecordRef.Copy(Record)`: a Record as the source of the copy. Pinned for a non-temporary record into an opened non-temporary
//! RecordRef: the target takes the record's filters, key, view and first row as from a RecordRef
//! (misc/lt-probes-needed-6/PnF27RecordRefCopy.al:117-136 Copy_FromRecord, :138-158 Copy_FromRecord_WithView). A temporary target or
//! source is not pinned (gap G5-15) and is refused.

use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, Value};
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// The record to copy from, for the Copy argument `arg` into the RecordRef's record `dst`.
pub(crate) fn copy_source_record(arg: Option<&Value>, dst: &Rec) -> Result<Rec, Signal> {
    match arg {
        Some(Value::Record(src)) if !src.borrow().temporary && !dst.borrow().temporary => Ok(src.clone()),
        Some(Value::Record(_)) => Err(Signal::Unsupported(
            "RecordRef.Copy from a Record where the record or the RecordRef is temporary (not pinned)".into(),
        )),
        other => Err(Signal::Unsupported(format!(
            "RecordRef.Copy from {} (not pinned)",
            other.describe()
        ))),
    }
}
