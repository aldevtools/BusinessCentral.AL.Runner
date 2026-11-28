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
use al_builtins::stream::{BlobCell, BlobRef};
use al_builtins::value::{RecordState, Value};
use al_ir as ir;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// A buffer a stream was opened on, without keeping it alive (compared by address), its
/// [`al_builtins::stream::BlobBuf::content_edits`] right after the call (a `Clear` of the field empties the same buffer in place,
/// al-builtins `native::values`, `blob_clears_in_place`, and moves that count on, as every write does), and whether the call
/// gave no encoding argument ([`opened_without_encoding`]).
type BufMark = (std::sync::Weak<BlobCell>, u32, bool);

/// Whether a `Blob.CreateOutStream(var OutStream [, TextEncoding])` or `Blob.CreateInStream(var InStream [, TextEncoding])`
/// call gave only its stream argument (`args`, the receiver apart). Both forms are one builtin and an explicit
/// `TextEncoding::MSDos` resolves to the default encoding, so the call site's arguments are the only place that tells them
/// apart; the repro of issue 9322 omits the encoding on both opens (tests/neighbors/9322/src/Repro.al).
fn opened_without_encoding(args: &[ir::Arg]) -> bool {
    args.len() == 1
}

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

/// The Blob fields of records of a `TableType = Temporary` table in local variables of `[Test]` methods, with the buffers
/// that method opened a `CreateOutStream` and a `CreateInStream` on. A record no longer alive is dropped at the next note.
/// No session image carries it: the image requires it empty (`session_fields`), and install code too.
#[derive(Default)]
pub(crate) struct TestLocalBlobFields(Vec<(Weak<RefCell<RecordState>>, usize, Opened)>);

/// The buffers a `[Test]` method opened streams on, per call, in the field of a [`TestLocalBlobFields`] record: the field's
/// buffer right after the call, with its edit count. A `Clear` of the record, an `Init` or a record assignment gives the
/// field another buffer; a `Clear` of the field, or a write, changes the count. Each list keeps the first two calls only:
/// [`TestLocalBlobFields::holds`] asks for exactly one, so a third changes no answer.
#[derive(Default)]
struct Opened {
    out_streams: Vec<BufMark>,
    in_streams: Vec<BufMark>,
}

/// The most calls an [`Opened`] list keeps.
const KEPT_CALLS: usize = 2;

impl Opened {
    /// Notes a `CreateOutStream` (`out`) or `CreateInStream` call that left the field holding `buf`, given no encoding
    /// argument or one (`no_encoding`).
    fn note_call(&mut self, out: bool, buf: &BlobRef, no_encoding: bool) {
        let calls = if out {
            &mut self.out_streams
        } else {
            &mut self.in_streams
        };
        if calls.len() < KEPT_CALLS {
            calls.push((Arc::downgrade(buf), buf.borrow().content_edits, no_encoding));
        }
    }
}

impl TestLocalBlobFields {
    /// Holds what a new session holds: no field noted.
    pub(crate) fn as_new(&self) -> bool {
        self.0.is_empty()
    }

    fn note(&mut self, rec: &Rc<RefCell<RecordState>>, pos: usize, id: BuiltinId, no_encoding: bool) {
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
        let Some(Value::Blob(buf)) = rec.borrow().fields.get(pos).cloned() else {
            return;
        };
        self.0[i]
            .2
            .note_call(id == ids::BLOB_CREATEOUTSTREAM_1, &buf, no_encoding);
    }

    /// Whether `b` is now the field of a noted live record on which the method opened one OutStream and one InStream,
    /// both on `b` itself and both without an encoding argument, and nothing edited `b` since but the one `WriteText` between the two calls (its edit count one
    /// past the OutStream's, the same as the InStream's; `stream_line_breaks::measured_line_break_stream` reads that the
    /// history since the buffer was made or cleared is that one CreateOutStream and that one WriteText).
    pub(crate) fn holds(&self, b: &BlobRef) -> bool {
        let edits = b.borrow().content_edits;
        let only_on_b =
            |calls: &[BufMark], at: u32| matches!(calls, [(w, e, true)] if w.as_ptr() == Arc::as_ptr(b) && *e == at);
        (self.0.iter()).any(|(w, pos, opened)| {
            only_on_b(&opened.out_streams, edits.wrapping_sub(1))
                && only_on_b(&opened.in_streams, edits)
                && holds_field(w, *pos, b)
        })
    }
}

/// Whether the record `w` still lives, is temporary and its field `pos` is now the buffer `b`.
fn holds_field(w: &Weak<RefCell<RecordState>>, pos: usize, b: &BlobRef) -> bool {
    w.upgrade().is_some_and(|r| {
        r.borrow().temporary && matches!(r.borrow().fields.get(pos), Some(Value::Blob(f)) if Arc::ptr_eq(f, b))
    })
}

impl Session {
    /// After a `Blob.CreateInStream` or `Blob.CreateOutStream` call ran: notes the field when the call is one of
    /// "Temp Blob Impl." on a field of a temporary record, or one of a `[Test]` method on a field of a local record of a
    /// `TableType = Temporary` table. `args` are the call's arguments, the receiver apart.
    pub(crate) fn note_temp_blob_impl_field(
        &mut self,
        id: BuiltinId,
        receiver: Option<&ir::Arg>,
        args: &[ir::Arg],
        f: &mut Frame,
    ) {
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
        // The repro of issue 9322 declares the record without `temporary` (tests/neighbors/9322/src/Repro.al).
        if let ir::Place::Local(slot) = **base
            && self.is_test_method(f.object, f.routine)
            && !self.engine.program.objects[f.object].routines[f.routine].locals[slot].declared_temporary
            && let Ok(Value::Record(rc)) = self.read(base, f)
            && rc.borrow().temporary
            && self.engine.object_facts[rc.borrow().table as usize].table_type == Some(ir::TableType::Temporary)
        {
            self.test_local_blob_fields
                .note(&rc, *pos, id, opened_without_encoding(args));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test that clears and refills the field in a loop keeps two marks per list, not one per call (al-runner issue 9346).
    #[test]
    fn opened_keeps_two_calls_per_list() {
        let mut opened = Opened::default();
        for _ in 0..5 {
            opened.note_call(true, &al_builtins::stream::new_blob(), true);
            opened.note_call(false, &al_builtins::stream::new_blob(), true);
        }
        assert_eq!(
            (opened.out_streams.len(), opened.in_streams.len()),
            (KEPT_CALLS, KEPT_CALLS)
        );
    }
}
