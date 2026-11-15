//! A RecordRef that is not open: one never opened (`LibraryValue::Empty`, also after `Clear`) or one closed
//! (`LibraryValue::ClosedRecordRef`, set by `RecordRef.Close` in al-vm `refs.rs`). Every place that answers a RecordRef that is
//! not open answers both alike, except the members the corpus measured only on a RecordRef never opened (al-vm
//! `recordref_not_open.rs`).

use crate::value::LibraryValue;

/// Whether a RecordRef handle holds no open record, never opened or closed.
pub fn not_open(l: &LibraryValue) -> bool {
    matches!(l, LibraryValue::Empty | LibraryValue::ClosedRecordRef)
}
