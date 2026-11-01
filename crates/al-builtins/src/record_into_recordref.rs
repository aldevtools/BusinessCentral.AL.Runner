//! A Record converted to a RecordRef: assigned to a RecordRef variable or passed to a by-value RecordRef parameter. The RecordRef
//! holds a copy of the record's table, buffer, filters, key and current row, not shared afterwards: a change through the ref does
//! not reach the record and one to the record does not reach the ref (misc/lt-cx-odd-leads/CxVariants.al:174-210
//! RecordToRecordRef_Implicit: `assigned` and `param` `85482|2|2`, `assigned_filters` `Id: 2..3`, `record_after_ref_change` `two`,
//! `ref_after_record_change` `changed`). A temporary record, or one whose buffer carries an unpinned state, is not pinned and refused.

use crate::describe::Describe;
use crate::library_kind::LibraryKind;
use crate::native::Signal;
use crate::value::{LibraryValue, RecordState, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// The RecordRef a Record `v` converts to.
#[cold]
pub fn record_into_recordref(v: &Value) -> Result<Value, Signal> {
    let Value::Record(rec) = v else {
        return Err(Signal::Unsupported(format!(
            "conversion of {} to RecordRef",
            v.describe()
        )));
    };
    let rec = rec.borrow();
    if rec.temporary || rec.unpinned_buffer.is_some() {
        return Err(Signal::Unsupported(format!(
            "conversion of a temporary Record, or one in an unpinned state, of table object {} to RecordRef (not pinned)",
            rec.table
        )));
    }
    let mut copy = rec.clone();
    own_unloaded_blobs(&mut copy);
    Ok(Value::Library(
        LibraryKind::RecordRef,
        Rc::new(RefCell::new(LibraryValue::RecordRef(Rc::new(RefCell::new(copy))))),
    ))
}

/// A Blob of a stored row whose bytes were not read yet gets its own copy, so the copy and the record do not share it (as
/// RecordRef.GetTable does).
pub fn own_unloaded_blobs(copy: &mut RecordState) {
    for v in &mut copy.fields {
        if let Value::Blob(b) = v {
            let unloaded = {
                let b = b.borrow();
                b.db_row && !b.loaded
            };
            if unloaded {
                *b = crate::stream::copy_blob_db_row(b);
            }
        }
    }
}
