//! `Base64Convert.ToBase64(InStream, true[, OutStream])` on BC 28 and 29: the streams and targets
//! tests/al-language/streams/Base64LineBreaks.al measured, and the refusal of every other one.

use crate::native::Signal;
use crate::native::unsupported;
use crate::stream::{AfterLineBreakCall, BlobBuf, Encoding, InState, out_buf, with_in_unknown_ok};
use crate::value::{LibraryValue, Value};

/// The bytes `ToBase64(InStream, true[, OutStream])` converts: every byte of the Blob buffer the stream reads, whatever its
/// read position (`out_form` false: the InStream form, true: the OutStream form, which is measured only at position 0). The
/// stream is then marked: where it stands is not measured, and only the same InStream form may run on it again. `Ok(None)`
/// when the stream does not read a Blob buffer, or reads a file's; `Err` for a stream or a use of it the pin does not cover
/// (see [`measured_line_break_stream`]). Its one caller cites what it pins.
pub fn line_break_source(v: &mut Value, out_form: bool) -> Result<Option<Vec<u8>>, Signal> {
    with_in_unknown_ok(Some(v), |s| {
        let reads_blob = s
            .live_blob()
            .is_some_and(|b| matches!(&*b.borrow(), BlobBuf { file: false, .. }));
        if !reads_blob {
            return Ok(None);
        }
        if !measured_line_break_stream(s) {
            return unsupported(
                "Base64Convert.ToBase64(InStream, true ...) of a stream other than one Blob.CreateInStream made with the default encoding on a Temp Blob written before, moved only by reads (not measured)",
            );
        }
        match (s.after_line_break_call, out_form) {
            (AfterLineBreakCall::No, false) | (AfterLineBreakCall::InStreamForm, false) => {}
            (AfterLineBreakCall::No, true) if s.pos == 0 => {}
            (AfterLineBreakCall::No, true) => {
                return unsupported(
                    "Base64Convert.ToBase64(InStream, true, OutStream) on a stream not at position 0 (not measured)",
                );
            }
            _ => {
                return unsupported(
                    "Base64Convert.ToBase64(InStream, true ...) on a stream a ToBase64(InStream, true ...) call ran on, other than the InStream form again (not measured)",
                );
            }
        }
        s.after_line_break_call = if out_form {
            AfterLineBreakCall::OutStreamForm
        } else {
            AfterLineBreakCall::InStreamForm
        };
        Ok(Some(s.with_data(<[u8]>::to_vec)))
    })
}

/// The stream Base64LineBreaks.al measured (`FillStream`, :53-61): `Blob.CreateInStream` without an encoding on the Blob of
/// a Temp Blob, written before the stream was created, and moved only by `ReadText` (:186-187).
fn measured_line_break_stream(s: &InState) -> bool {
    let Some(b) = s.live_blob() else {
        return false;
    };
    let buf = b.borrow();
    (s.on_blob_field, s.enc, s.position_set, s.created_edits) == (true, Encoding::MsDos, false, buf.content_edits)
        && temp_blob_buffer(&buf)
}

/// The Blob of a Temp Blob: the field of the temporary record inside "Temp Blob Impl.", which that codeunit never inserts,
/// reads or assigns, holding what streams wrote to it (Base64LineBreaks.al:53-61, :351-403).
fn temp_blob_buffer(b: &BlobBuf) -> bool {
    let row = (
        b.db_row,
        b.temp_row,
        b.refetch,
        b.persisted,
        b.inserted,
        b.temp_inserted,
    );
    let copy = (
        b.assigned_unsaved,
        b.unread_content,
        b.utf16_binary,
        b.file,
        b.dotnet_view,
    );
    (b.loaded, row, copy)
        == (
            true,
            (false, false, false, false, false, false),
            (false, false, false, false, 0),
        )
}

/// The target of `ToBase64(InStream, true, OutStream)` as Base64LineBreaks.al:351-403 measured it: an OutStream of the default
/// encoding on another Temp Blob, empty or after what that stream wrote (`AB`), so it writes at the end.
pub fn refuse_unmeasured_line_break_target(target: &mut Value, source: &Value) -> Result<(), Signal> {
    let (b, enc) = out_buf(Some(target))?;
    let source_blob = match source {
        Value::Library(_, h) => match &*h.borrow() {
            LibraryValue::InStream(s) => s.live_blob().cloned(),
            _ => None,
        },
        _ => None,
    };
    let other_blob = source_blob.is_some_and(|s| !std::sync::Arc::ptr_eq(&s, &b));
    let buf = b.borrow();
    if (enc, other_blob, buf.write_pos) == (Encoding::MsDos, true, buf.bytes.len()) && temp_blob_buffer(&buf) {
        return Ok(());
    }
    unsupported(
        "Base64Convert.ToBase64(InStream, true, OutStream) into an OutStream other than one of the default encoding at the end of another Temp Blob (not measured)",
    )
}

/// Where a `ToBase64(InStream, true ...)` call leaves the stream is not measured (Base64LineBreaks.al moves no stream after it).
pub(crate) fn refuse_move_after_call(s: &InState, what: &str) -> Result<(), Signal> {
    match s.after_line_break_call {
        AfterLineBreakCall::No => Ok(()),
        _ => unsupported(format!(
            "InStream.{what} after Base64Convert.ToBase64(InStream, true ...) (where it stands is not measured)"
        )),
    }
}
