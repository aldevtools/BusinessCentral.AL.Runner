//! `RecordRef.Copy(Record)`: a Record as the source of the copy. Pinned for a non-temporary record into an opened non-temporary
//! RecordRef: the target takes the record's filters, key, view and first row as from a RecordRef
//! (misc/lt-probes-needed-6/PnF27RecordRefCopy.al:117-136 Copy_FromRecord, :138-158 Copy_FromRecord_WithView). Into a RecordRef
//! opened as temporary, from a non-temporary or a temporary Record, the shared Copy runs as from a RecordRef: the ref stays temporary
//! and takes filters, marks and MarkedOnly; ShareTable true needs two temporary records
//! (record/lt-y51-recordref-copy-temp/Y51RecRefCopyTemp.al:RecRefCopy_RecordIntoTemporary_LoopInsert,
//! RecRefCopy_MarkedTemporaryRecordIntoTemporary, RecRefCopy_RecordIntoTemporary_NoArgument, RecRefCopy_RecordIntoTemporary_ShareTableTrue).
//! A temporary Record into a non-temporary RecordRef is not pinned and is refused.

use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, Value};
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// The record to copy from, for the Copy argument `arg` into the RecordRef's record `dst`.
pub(crate) fn copy_source_record(arg: Option<&Value>, dst: &Rec) -> Result<Rec, Signal> {
    match arg {
        Some(Value::Record(src)) if !src.borrow().temporary || dst.borrow().temporary => Ok(src.clone()),
        Some(Value::Record(_)) => Err(Signal::Unsupported(
            "RecordRef.Copy from a temporary Record into a non-temporary RecordRef (not pinned)".into(),
        )),
        other => Err(Signal::Unsupported(format!(
            "RecordRef.Copy from {} (not pinned)",
            other.describe()
        ))),
    }
}
