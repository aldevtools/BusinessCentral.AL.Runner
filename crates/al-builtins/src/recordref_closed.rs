//! A RecordRef that is not open: one never opened (`LibraryValue::Empty`), one closed (`LibraryValue::ClosedRecordRef`, set by
//! `RecordRef.Close` in al-vm `refs.rs`, and by an `Open` that raised or a `Get` that left a ref never opened not open, al-vm
//! `unopened_refs.rs` `after_open_attempt`) or one emptied by `Clear` or `Open(0)` (`LibraryValue::EmptiedRef`, [`cleared`] and
//! `after_open_attempt`). Every place that answers a RecordRef that is not open answers all of them alike, except the members the
//! corpus measured only in some of these states (al-vm `unopened_refs.rs`).
//!
//! a RecordRef that is not open, never opened (`LibraryValue::Empty`) or closed
//! (`LibraryValue::ClosedRecordRef`, set by `RecordRef.Close`).

use crate::value::{EmptiedRef, LibraryValue};

/// Whether a RecordRef handle holds no open record: never opened, closed, or emptied.
pub fn not_open(l: &LibraryValue) -> bool {
    matches!(
        l,
        LibraryValue::Empty
            | LibraryValue::ClosedRecordRef
            | LibraryValue::EmptiedRef(
                EmptiedRef::ClearedOpenRecordRef | EmptiedRef::ZeroOpenedRecordRef | EmptiedRef::OtherRecordRef
            )
    )
}

/// What `Clear` leaves in a new handle for the RecordRef (`record_ref`) or KeyRef that held `old`: a RecordRef that was open and
/// a KeyRef that held a key are the states corpus PR 1411 measured (UhUntouchedStreamsAfter.al, codeunit 66996); any other is
/// kept apart from them.
pub fn cleared(record_ref: bool, old: &LibraryValue) -> LibraryValue {
    LibraryValue::EmptiedRef(match (record_ref, old) {
        (true, LibraryValue::RecordRef(_)) => EmptiedRef::ClearedOpenRecordRef,
        (true, _) => EmptiedRef::OtherRecordRef,
        (false, LibraryValue::KeyRef(..)) => EmptiedRef::ClearedAssignedKeyRef,
        (false, _) => EmptiedRef::OtherKeyRef,
    })
}

/// The handle a variable assigned from the KeyRef handle `h` gets: a KeyRef cleared after it held a key is the state corpus PR 1411
/// measured only on the cleared variable itself (UhUntouchedStreamsAfter.al:285-299 `TryKeyRef`), so a copy gets a handle of its
/// own that holds a KeyRef emptied by another sequence. `None`: any other handle is assigned as it is.
pub fn keyref_assigned_copy(h: &LibraryValue) -> Option<LibraryValue> {
    matches!(h, LibraryValue::EmptiedRef(EmptiedRef::ClearedAssignedKeyRef))
        .then_some(LibraryValue::EmptiedRef(EmptiedRef::OtherKeyRef))
}
