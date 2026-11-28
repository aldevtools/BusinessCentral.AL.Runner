//! The Blob fields System Application's "Temp Blob Impl." opened a stream on: the field `Blob` of its temporary record
//! `TempBlob` (TempBlobImpl.Codeunit.al: `TempBlob.Blob.CreateInStream(InStream)`, `TempBlob.Blob.CreateOutStream(OutStream)`).
//! `Base64Convert.ToBase64(InStream, true ...)` asks whether a stream's buffer is such a field now
//! (`al_builtins::stream_line_breaks`, tests/al-language/streams/Base64LineBreaks.al:53-61): a record that went away (a
//! `Clear` of the Temp Blob) or a field given another buffer since does not count.
//!
//! The same question for the Blob field of a record of a `TableType = Temporary` table in a local variable of a `[Test]`
//! method, both of whose streams that method opened ([`TestLocalBlobFields`]): the repro of al-runner issue 9322, assumption
//! `base-convert-line-breaks-temporary-table-blob`.

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
        self.0.iter().any(|(w, pos)| holds_field(w, *pos, b))
    }
}

/// The Blob fields of records of a `TableType = Temporary` table in local variables of `[Test]` methods, with whether that
/// method called `CreateOutStream` and `CreateInStream` on the field. A record no longer alive is dropped at the next note.
/// No session image carries it: the image requires it empty (`session_fields`), and install code too.
#[derive(Default)]
pub(crate) struct TestLocalBlobFields(Vec<(Weak<RefCell<RecordState>>, usize, Opened)>);

/// Which streams a `[Test]` method opened on a field of [`TestLocalBlobFields`].
#[derive(Default, Clone, Copy, PartialEq, Eq)]
struct Opened {
    out_stream: bool,
    in_stream: bool,
}

impl TestLocalBlobFields {
    /// Holds what a new session holds: no field noted.
    pub(crate) fn as_new(&self) -> bool {
        self.0.is_empty()
    }

    fn note(&mut self, rec: &Rc<RefCell<RecordState>>, pos: usize, id: BuiltinId) {
        self.0.retain(|(w, _, _)| w.strong_count() > 0);
        let i = match self
            .0
            .iter()
            .position(|(w, p, _)| *p == pos && w.as_ptr() == Rc::as_ptr(rec))
        {
            Some(i) => i,
            None => {
                self.0.push((Rc::downgrade(rec), pos, Opened::default()));
                self.0.len() - 1
            }
        };
        let opened = &mut self.0[i].2;
        match id {
            ids::BLOB_CREATEOUTSTREAM_1 => opened.out_stream = true,
            _ => opened.in_stream = true,
        }
    }

    /// Whether `b` is now the field of a noted live record on which both streams were opened.
    pub(crate) fn holds(&self, b: &BlobRef) -> bool {
        let both = Opened {
            out_stream: true,
            in_stream: true,
        };
        (self.0.iter()).any(|(w, pos, opened)| *opened == both && holds_field(w, *pos, b))
    }
}

/// Whether the record `w` still lives, is temporary and its field `pos` is now the buffer `b`.
fn holds_field(w: &Weak<RefCell<RecordState>>, pos: usize, b: &BlobRef) -> bool {
    w.upgrade().is_some_and(|r| {
        r.borrow().temporary
            && matches!(r.borrow().fields.get(pos), Some(Value::Blob(f)) if std::sync::Arc::ptr_eq(f, b))
    })
}

impl Session {
    /// After a `Blob.CreateInStream` or `Blob.CreateOutStream` call ran: notes the field when the call is one of
    /// "Temp Blob Impl." on a field of a temporary record, or one of a `[Test]` method on a field of a local record of a
    /// `TableType = Temporary` table.
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
        if in_impl {
            if let Ok(Value::Record(rc)) = self.read(base, f)
                && rc.borrow().temporary
            {
                self.temp_blob_impl_fields.note(&rc, *pos);
            }
            return;
        }
        if matches!(**base, ir::Place::Local(_))
            && self.is_test_method(f.object, f.routine)
            && let Ok(Value::Record(rc)) = self.read(base, f)
            && rc.borrow().temporary
            && self.engine.object_facts[rc.borrow().table as usize].table_type == Some(ir::TableType::Temporary)
        {
            self.test_local_blob_fields.note(&rc, *pos, id);
        }
    }
}
