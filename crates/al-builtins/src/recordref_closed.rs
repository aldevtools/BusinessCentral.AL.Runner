//! A RecordRef that is not open: one never opened (`LibraryValue::Empty`, also after `Clear`) or one closed
//! (`LibraryValue::ClosedRecordRef`, set by `RecordRef.Close` in al-vm `refs.rs`, and by an `Open` that raised or a `Get` that left a
//! ref never opened not open, al-vm `unopened_refs.rs` `after_open_attempt`). Every place that answers a RecordRef that is
//! not open answers both alike, except the members the corpus measured only on a RecordRef never opened (al-vm
//! `unopened_refs.rs`).

use crate::value::LibraryValue;

/// Whether a RecordRef handle holds no open record, never opened or closed.
pub fn not_open(l: &LibraryValue) -> bool {
    matches!(l, LibraryValue::Empty | LibraryValue::ClosedRecordRef)
}
