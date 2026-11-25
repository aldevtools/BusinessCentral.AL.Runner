//! The Blob fields System Application's "Temp Blob Impl." opened a stream on: the field `Blob` of its temporary record
//! `TempBlob` (TempBlobImpl.Codeunit.al: `TempBlob.Blob.CreateInStream(InStream)`, `TempBlob.Blob.CreateOutStream(OutStream)`).
//! `Base64Convert.ToBase64(InStream, true ...)` asks whether a stream's buffer is such a field now
//! (`al_builtins::stream_line_breaks`, tests/al-language/streams/Base64LineBreaks.al:53-61): a record that went away (a
//! `Clear` of the Temp Blob) or a field given another buffer since does not count.

use al_builtins::builtin_id::{BuiltinId, ids};
use al_builtins::stream::BlobRef;
use al_builtins::value::{RecordState, Value};
use al_ir as ir;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::{Frame, Session};

/// The records and field positions noted so far; a record no longer alive is dropped at the next note.
#[derive(Default)]
pub(crate) struct TempBlobImplFields(Vec<(Weak<RefCell<RecordState>>, usize)>);

impl TempBlobImplFields {
    /// Holds what a new session holds (`install_left.rs`): no field noted.
    pub(crate) fn as_new(&self) -> bool {
        self.0.is_empty()
    }

    /// The records noted that still live, with their field positions, in the order noted: what code can tell of this
    /// list (the reader upgrades the records and passes over a gone one, `holds`; each note drops them first, `note`). A
    /// session image (al-vm `login_image`) keeps them.
    pub(crate) fn live(&self) -> Vec<(Rc<RefCell<RecordState>>, usize)> {
        (self.0.iter())
            .filter_map(|(w, pos)| w.upgrade().map(|r| (r, *pos)))
            .collect()
    }

    /// Notes `records` (from [`TempBlobImplFields::live`]) as this list's own, after any noted so far.
    pub(crate) fn note_all(&mut self, records: &[(Rc<RefCell<RecordState>>, usize)]) {
        for (r, pos) in records {
            self.note(r, *pos);
        }
    }

    fn note(&mut self, rec: &Rc<RefCell<RecordState>>, pos: usize) {
        self.0.retain(|(w, _)| w.strong_count() > 0);
        if !self.0.iter().any(|(w, p)| *p == pos && w.as_ptr() == Rc::as_ptr(rec)) {
            self.0.push((Rc::downgrade(rec), pos));
        }
    }

    pub(crate) fn holds(&self, b: &BlobRef) -> bool {
        self.0.iter().any(|(w, pos)| {
            w.upgrade().is_some_and(|r| {
                r.borrow().temporary
                    && matches!(r.borrow().fields.get(*pos), Some(Value::Blob(f)) if std::sync::Arc::ptr_eq(f, b))
            })
        })
    }
}

impl Session {
    /// After a `Blob.CreateInStream` or `Blob.CreateOutStream` call ran: notes the field when the call is one of
    /// "Temp Blob Impl." on a field of a temporary record.
    pub(crate) fn note_temp_blob_impl_field(&mut self, id: BuiltinId, receiver: Option<&ir::Arg>, f: &mut Frame) {
        if !matches!(id, ids::BLOB_CREATEINSTREAM_1 | ids::BLOB_CREATEOUTSTREAM_1) {
            return;
        }
        let (Some(ir::Arg::Ref(ir::Place::Field(base, pos)))
        | Some(ir::Arg::Value(ir::Expr::Load(ir::Place::Field(base, pos))))) = receiver
        else {
            return;
        };
        let in_impl = al_builtins::native::Ctx::calling_object(&*self)
            .is_some_and(|(app, object)| app == "System Application" && object == "Temp Blob Impl.");
        if !in_impl {
            return;
        }
        if let Ok(Value::Record(rc)) = self.read(base, f)
            && rc.borrow().temporary
        {
            self.temp_blob_impl_fields.note(&rc, *pos);
        }
    }
}
