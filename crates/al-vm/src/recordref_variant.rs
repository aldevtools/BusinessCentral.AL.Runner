//! A RecordRef held by a Variant (al-runner issue 6924).
//!
//! Pinned: `V := RecRef` gives a Variant that holds a RecordRef (types/variant/VariantRecordReassign.al:201-203), and the
//! RecordRef taken back out shares the record buffer with the original while both are open: a field written through one is
//! read through the other (types/variant/CxVariants.al:194-198).
//!
//! Not pinned, assumed (`variant-keeps-recordref-after-close`): the Variant holds a RecordRef of its own on that buffer, so
//! when one of the two leaves the buffer (Close, or Open or GetTable again) the other still holds it open. An outside
//! reporter's test passes on real BC with `CheckRecord := CheckRecRef; CheckRecRef.Close();` followed by
//! `Format(CheckRecord) <> ''` and Data Type Management.GetRecordRef(CheckRecord, R) giving the table. Probe:
//! tests/al-language/recordref-fieldref/recordref/RecordRefInVariantAfterClose.al. The assumption is marked only where it
//! shows: when a RecordRef leaves a buffer that the other side of such an assignment still holds.

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::library_kind::LibraryKind;
use al_builtins::value::{LibraryValue, RecordState, Value};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Lib = Rc<RefCell<LibraryValue>>;
type Rec = Rc<RefCell<RecordState>>;
type WeakLib = Weak<RefCell<LibraryValue>>;

/// The assignments of an open RecordRef into a Variant that are still alive: the source and the Variant's own RecordRef.
#[derive(Default)]
pub(crate) struct VariantRefs {
    pairs: Vec<(WeakLib, WeakLib)>,
    /// The `RecordRef.GetTable` that runs was given a Variant (refs.rs `ref_op`).
    pub(crate) get_table: bool,
}

/// The record buffer an open RecordRef holds.
fn buffer(h: &Lib) -> Option<Rec> {
    match &*h.borrow() {
        LibraryValue::RecordRef(rec) => Some(rec.clone()),
        _ => None,
    }
}

impl VariantRefs {
    /// The value a Variant holds after it is assigned the RecordRef `h`: for an open one, a RecordRef of its own on the same
    /// buffer. A RecordRef that is not open is handed on as it is (nothing pins or needs another answer).
    pub(crate) fn hold(&mut self, h: Lib) -> Value {
        let Some(rec) = buffer(&h) else {
            return Value::Library(LibraryKind::RecordRef, h);
        };
        self.pairs.retain(|(a, b)| a.strong_count() > 0 && b.strong_count() > 0);
        let own = Rc::new(RefCell::new(LibraryValue::RecordRef(rec)));
        self.pairs.push((Rc::downgrade(&h), Rc::downgrade(&own)));
        Value::Library(LibraryKind::RecordRef, own)
    }

    /// The buffer `h` holds before one of its methods runs, when there is anything to watch.
    pub(crate) fn before(&self, h: &Lib) -> Option<Rec> {
        if self.pairs.is_empty() { None } else { buffer(h) }
    }

    /// After a method of `h` ran: marks the assumption when `h` left the buffer `was` and the other side of an assignment
    /// into a Variant still holds it.
    pub(crate) fn after(&self, h: &Lib, was: Option<Rec>) {
        let Some(was) = was else { return };
        if buffer(h).is_some_and(|now| Rc::ptr_eq(&now, &was)) {
            return;
        }
        let holds = |w: &WeakLib| {
            w.upgrade()
                .and_then(|o| buffer(&o))
                .is_some_and(|b| Rc::ptr_eq(&b, &was))
        };
        let me = Rc::as_ptr(h);
        if self
            .pairs
            .iter()
            .any(|(a, b)| (a.as_ptr() == me && holds(b)) || (b.as_ptr() == me && holds(a)))
        {
            assume(Assumption::VariantKeepsRecordrefAfterClose);
        }
    }
}
