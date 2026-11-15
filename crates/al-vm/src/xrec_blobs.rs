//! Which Blob buffers an xRec handle holds. An xRec is a copy of Rec's field list, so it shares Rec's Blob buffers
//! (`records.rs` `xrec_handle`). `Clear(Rec.BlobField)` empties a buffer in place only when no live xRec holds it
//! (al-builtins `native::values::clear`): what xRec shows after such a Clear is not pinned by the corpus, so it stays
//! what it was before al-runner issue 6791 (xRec keeps its content). A stream write to xRec's Blob while Rec shares the
//! buffer is refused ([`Session::xrec_blob_write_check`]).

use std::rc::Rc;

use al_builtins::native::Signal;
use al_builtins::stream::BlobRef;
use al_builtins::value::Value;
use al_ir as ir;

use crate::{Frame, Session};

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

    /// Before a stream write (`CreateOutStream`, `Import`) to the Blob field `place` whose buffer is `b` (`operand`: the
    /// call's own references to it): refused when the record is a live xRec and another holder shares the buffer (Rec,
    /// [`Session::xrec_handle`]). BC keeps the two apart: a write to xRec does not show in Rec
    /// (streams/BlobCopyStates.al:2419-2453, OnModify_Mode2_*).
    pub(crate) fn xrec_blob_write_check(
        &mut self,
        place: &ir::Place,
        b: &BlobRef,
        operand: usize,
        f: &mut Frame,
    ) -> Result<(), Signal> {
        let ir::Place::Field(base, _) = place else {
            return Ok(());
        };
        if self.live_xrecs.is_empty() || std::sync::Arc::strong_count(b) <= 1 + operand {
            return Ok(());
        }
        let Value::Record(rc) = self.read(base, f)? else {
            return Ok(());
        };
        let is_xrec = (self.live_xrecs.iter()).any(|w| w.strong_count() > 0 && w.as_ptr() == Rc::as_ptr(&rc));
        if is_xrec {
            return Err(Signal::Unsupported(
                "a stream write to a Blob field of xRec, whose buffer Rec shares: on BC the write does not show in Rec \
                 (streams/BlobCopyStates.al:2419-2453); the runtime shares the buffer"
                    .into(),
            ));
        }
        Ok(())
    }
}
