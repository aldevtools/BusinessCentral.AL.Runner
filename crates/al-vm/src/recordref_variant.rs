//! A copy of a RecordRef (al-runner issue 6924).
//!
//! `V := RecRef` gives a Variant that holds a RecordRef (types/variant/VariantRecordReassign.al:201-203), and the RecordRef
//! taken back out shares the record buffer with the original while both are open: a field written through one is read through
//! the other (types/variant/CxVariants.al:194-198).
//!
//! Every copy of an open RecordRef is a RecordRef of its own on that buffer, so when one copy leaves the buffer (Close, Open
//! again) the others still hold it open, positioned as they were. Pinned by recordref-fieldref/recordref/
//! RecordRefInVariantAfterClose.al for `V := RecRef` (:125-206, Format of the Variant not empty, IsRecordRef, the row taken back
//! out), `R := V` then `R.Close()` (:207-230), the original opened again (:266-285), a Variant by value and by var (:329-400), a
//! RecordRef handed to a Variant parameter (:401), `RR2 := RR` with a Close of either side (:424-459), a move of either side
//! seen by the other (:460-482) and a RecordRef parameter by value that the callee closes (:484-500).

use al_builtins::library_kind::LibraryKind;
use al_builtins::value::{LibraryValue, RecordState, Value};
use std::cell::RefCell;
use std::rc::Rc;

type Lib = Rc<RefCell<LibraryValue>>;

/// What the `RecordRef.GetTable` that runs was given (refs.rs `ref_op`).
#[derive(Default)]
pub(crate) struct VariantRefs {
    /// The `RecordRef.GetTable` that runs was given a Variant (refs.rs `ref_op`).
    pub(crate) get_table: bool,
}

/// The value a copy of the RecordRef `h` holds: for an open one, a RecordRef of its own on the same buffer. A RecordRef that is
/// not open is handed on as it is: Format of a Variant holding one ends the test with an uncatchable CLR exception on BC
/// (RecordRefInVariantAfterClose.al:12-17), so nothing pins another answer.
pub(crate) fn own_copy(h: &Lib) -> Value {
    let copy = match &*h.borrow() {
        LibraryValue::RecordRef(rec) => Rc::new(RefCell::new(LibraryValue::RecordRef(rec.clone()))),
        _ => h.clone(),
    };
    Value::Library(LibraryKind::RecordRef, copy)
}

/// A GetTable from a typed record of the same table updates the open shared
/// buffer (RecordRefGetTable.al, by-value integration event subscriber).
pub(crate) fn install_get_table_buffer(lib: &Lib, copy: RecordState, typed_source: bool) {
    let shared = match &*lib.borrow() {
        LibraryValue::RecordRef(current) if typed_source && !copy.temporary && Rc::strong_count(current) > 1 => {
            let same_table = {
                let current = current.borrow();
                !current.temporary && current.table == copy.table
            };
            same_table.then(|| current.clone())
        }
        _ => None,
    };
    if let Some(current) = shared {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RecordRefGetTableSharedSameTable);
        *current.borrow_mut() = copy;
    } else {
        *lib.borrow_mut() = LibraryValue::RecordRef(Rc::new(RefCell::new(copy)));
    }
}
