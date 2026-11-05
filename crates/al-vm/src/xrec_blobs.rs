//! Which Blob buffers an xRec handle holds. An xRec is a copy of Rec's field list, so it shares Rec's Blob buffers
//! (`records.rs` `xrec_handle`). `Clear(Rec.BlobField)` empties a buffer in place only when no live xRec holds it
//! (al-builtins `native::values::clear`): what xRec shows after such a Clear is not pinned by the corpus, so it stays
//! what it was before al-runner issue 6791 (xRec keeps its content).

use std::rc::Rc;

use al_builtins::stream::BlobRef;
use al_builtins::value::Value;

use crate::Session;

type Rec = Rc<std::cell::RefCell<al_builtins::value::RecordState>>;

impl Session {
    /// Records that `x` is given to AL code as an xRec (a trigger or a table event). Dead handles are dropped here.
    pub(crate) fn note_xrec(&mut self, x: &Rec) {
        self.live_xrecs.retain(|w| w.strong_count() > 0);
        if !self.live_xrecs.iter().any(|w| w.as_ptr() == Rc::as_ptr(x)) {
            self.live_xrecs.push(Rc::downgrade(x));
        }
    }

    /// Whether a live xRec handle holds `b` as one of its fields.
    pub(crate) fn xrec_holds_blob(&self, b: &BlobRef) -> bool {
        self.live_xrecs.iter().filter_map(|w| w.upgrade()).any(|x| {
            x.borrow()
                .fields
                .iter()
                .any(|f| matches!(f, Value::Blob(o) if std::sync::Arc::ptr_eq(o, b)))
        })
    }
}
