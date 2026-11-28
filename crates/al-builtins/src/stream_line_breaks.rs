//! `Base64Convert.ToBase64(InStream, true[, OutStream])` on BC 28 and 29: the streams and targets
//! tests/al-language/streams/Base64LineBreaks.al measured, and the refusal of every other one.

use crate::native::unsupported;
use crate::native::{Ctx, Signal};
use crate::stream::{BlobBuf, Encoding, InState, out_buf, with_in_unknown_ok};
use crate::value::{LibraryValue, Value};

/// The bytes `ToBase64(InStream, true[, OutStream])` converts: every byte of the Blob buffer the stream reads, whatever its
/// read position (`out_form` false: the InStream form, true: the OutStream form, which is measured only at position 0). The
/// stream is then marked: where it stands is not measured, so every later use of it refuses (a second call too: the stream
/// was used since, :195-198 is not reached). `Ok(None)`
/// when the stream does not read a Blob buffer, or reads a file's; `Err` for a stream or a use of it the pin does not cover
/// (see [`measured_line_break_stream`]). Its one caller cites what it pins.
pub fn line_break_source(ctx: &dyn Ctx, v: &mut Value, out_form: bool) -> Result<Option<Vec<u8>>, Signal> {
    with_in_unknown_ok(Some(v), |s| {
        let reads_blob = s
            .live_blob()
            .is_some_and(|b| matches!(&*b.borrow(), BlobBuf { file: false, .. }));
        if !reads_blob {
            return Ok(None);
        }
        let Some(holder) = measured_line_break_stream(ctx, s, out_form) else {
            return unsupported(
                "Base64Convert.ToBase64(InStream, true ...) of a stream other than the one Blob.CreateInStream made with the default encoding on the Blob a Temp Blob holds, after one WriteText of the default encoding, used only by ReadText with a length, never assigned (not measured)",
            );
        };
        if out_form && s.pos != 0 {
            return unsupported(
                "Base64Convert.ToBase64(InStream, true, OutStream) on a stream not at position 0 (not measured)",
            );
        }
        if holder == MeasuredHolder::TemporaryTableTestLocal {
            crate::assumptions::assume(crate::assumptions::Assumption::BaseConvertLineBreaksTemporaryTableBlob);
        }
        s.after_line_break_call = true;
        Ok(Some(s.with_data(<[u8]>::to_vec)))
    })
}

/// The uses of the stream "Base64 Convert Impl." makes before it calls the native converter: `Length`, `EOS` and `Length`
/// (System Application 28 and 29, Base64ConvertImpl.Codeunit.al:75-78 for the Text form, :84-87 for the OutStream form),
/// and the converter's own.
const IMPL_USES: u32 = 4;

/// Who holds the Blob a measured stream reads ([`measured_line_break_stream`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeasuredHolder {
    /// A live "Temp Blob Impl." (Base64LineBreaks.al:53-61): pinned.
    TempBlob,
    /// A record of a `TableType = Temporary` table in a local variable of a `[Test]` method, never inserted: the repro of
    /// al-runner issue 9322 (BC 28.4), assumption `base-convert-line-breaks-temporary-table-blob`.
    TemporaryTableTestLocal,
}

/// The stream Base64LineBreaks.al measured (`FillStream`, :53-61): `Blob.CreateInStream` without an encoding, the one InStream
/// created on the Blob the temporary record of a live "Temp Blob Impl." holds now, after one `WriteText(Text)` through the one
/// OutStream of the default encoding opened on that Blob since it was made or cleared, and not opened for writing since; used
/// by nothing but `ReadText(Text, Length)` calls (:188 reads 10 bytes; the file states that BC 28 and 29 convert the whole
/// stream whatever was read before, :13-14) before "Base64 Convert Impl." takes it ([`IMPL_USES`]), never given to another
/// variable by `:=`.
///
/// The same history on the Blob field of a never inserted record of a `TableType = Temporary` table in a local variable of a
/// `[Test]` method, the stream unread (position 0, no `ReadText`), in the InStream form, on BC 28, holding 100 ASCII bytes: the
/// repro of al-runner issue 9322, measured on BC 28.4 with 100 'A' (a CR LF at position 77, 138 characters).
fn measured_line_break_stream(ctx: &dyn Ctx, s: &InState, out_form: bool) -> Option<MeasuredHolder> {
    let b = s.live_blob()?;
    let buf = b.borrow();
    let stream = (
        s.on_blob_field,
        s.enc,
        s.created_edits,
        s.pos,
        s.uses,
        s.shared_by_assign,
    );
    let measured = (
        true,
        Encoding::MsDos,
        buf.content_edits,
        s.text_len_pos,
        s.text_len_reads + IMPL_USES,
        false,
    );
    let history = (
        buf.line_breaks.text_writes(buf.content_edits),
        buf.line_breaks.in_creates,
    );
    if (stream, history) != (measured, (Some(1), 1)) || !never_stored_buffer(&buf) {
        return None;
    }
    if ctx.held_by_temp_blob_impl(b) {
        return Some(MeasuredHolder::TempBlob);
    }
    let issue_9322 = (
        out_form,
        s.pos,
        s.text_len_reads,
        buf.bytes.len(),
        buf.bytes.is_ascii(),
        ctx.temporary_table_blob_line_breaks_measured(),
        ctx.held_by_temporary_table_test_local(b),
    );
    (issue_9322 == (false, 0, 0, 100, true, true, true)).then_some(MeasuredHolder::TemporaryTableTestLocal)
}

/// What happened to a Blob buffer since it was made or cleared, as far as `ToBase64(InStream, true[, OutStream])` asks
/// (Base64LineBreaks.al:53-61, :284-286, :364-367, :385-389) and the mark of an assigned copy asks
/// ([`LineBreakHistory::one_text_write`]). A copy of the buffer starts again ([`LineBreakHistory::cleared`]),
/// so it is not the measured buffer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineBreakHistory {
    /// [`BlobBuf::content_edits`] when the buffer was made or cleared.
    base_edits: u32,
    /// `CreateOutStream` calls on the buffer.
    out_opens: u32,
    /// Those of them without an encoding (or `TextEncoding::MSDos`) on an empty buffer.
    empty_default_opens: u32,
    /// `WriteText(Text)` calls through an OutStream of the default encoding on the buffer.
    default_text_writes: u32,
    /// `Blob.CreateInStream` calls on the buffer.
    pub in_creates: u32,
    /// Uses of an InStream that reads the buffer (`stream::with_in_core`).
    pub in_uses: u32,
}

impl LineBreakHistory {
    /// The history of a buffer cleared, or copied, when its edit count is `edits`.
    pub fn cleared(edits: u32) -> LineBreakHistory {
        LineBreakHistory {
            base_edits: edits,
            ..LineBreakHistory::default()
        }
    }

    pub(crate) fn note_out_open(&mut self, enc: Encoding, empty: bool) {
        self.out_opens += 1;
        if (enc, empty) == (Encoding::MsDos, true) {
            self.empty_default_opens += 1;
        }
    }

    pub(crate) fn note_text_write(&mut self, enc: Encoding) {
        if enc == Encoding::MsDos {
            self.default_text_writes += 1;
        }
    }

    /// Appends every field (al-builtins `blob_form`, the bytes form of a Blob buffer); exhaustive, so a new field does not
    /// compile until it is written.
    pub(crate) fn put(&self, out: &mut Vec<u8>) {
        let LineBreakHistory {
            base_edits,
            out_opens,
            empty_default_opens,
            default_text_writes,
            in_creates,
            in_uses,
        } = self;
        for n in [
            base_edits,
            out_opens,
            empty_default_opens,
            default_text_writes,
            in_creates,
            in_uses,
        ] {
            crate::value_codec::put_u32(out, *n);
        }
    }

    /// The history [`LineBreakHistory::put`] wrote; `None` for bytes it did not write.
    pub(crate) fn read(r: &mut crate::value_codec::Reader) -> Option<LineBreakHistory> {
        Some(LineBreakHistory {
            base_edits: r.u32()?,
            out_opens: r.u32()?,
            empty_default_opens: r.u32()?,
            default_text_writes: r.u32()?,
            in_creates: r.u32()?,
            in_uses: r.u32()?,
        })
    }

    /// Whether the buffer holds what one `WriteText(Text)` through the one `CreateOutStream` of the default encoding on it empty
    /// wrote, and nothing else edited it since it was made or cleared (al-vm `blob_transfer::assigned_write`).
    pub fn one_text_write(&self, content_edits: u32) -> bool {
        self.text_writes(content_edits) == Some(1)
    }

    /// How many `WriteText(Text)` calls (0 or 1) wrote the buffer when every edit of it since it was made or cleared is the one
    /// `CreateOutStream` of the default encoding on it empty and those writes; `None` otherwise.
    fn text_writes(&self, content_edits: u32) -> Option<u32> {
        let edits = content_edits.wrapping_sub(self.base_edits);
        let w = self.default_text_writes;
        ((edits, self.out_opens, self.empty_default_opens) == (1 + w, 1, 1) && w <= 1).then_some(w)
    }
}

/// The state of the Blob field of a temporary record that was never inserted, read or assigned, holding what streams wrote
/// to it: the record inside "Temp Blob Impl." (Base64LineBreaks.al:53-61, :351-403), and the record of the repro of al-runner
/// issue 9322. Who holds the buffer is [`Ctx::held_by_temp_blob_impl`] and [`Ctx::held_by_temporary_table_test_local`].
fn never_stored_buffer(b: &BlobBuf) -> bool {
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

/// The target of `ToBase64(InStream, true, OutStream)` as Base64LineBreaks.al:275-403 measured it: the one OutStream of the
/// default encoding on the Blob another live Temp Blob holds, empty or after one `WriteText` of it (`AB`), so it writes at the
/// end; an InStream created on it before is measured only on the empty Blob and unused (:364-367).
pub fn refuse_unmeasured_line_break_target(ctx: &dyn Ctx, target: &mut Value, source: &Value) -> Result<(), Signal> {
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
    let h = &buf.line_breaks;
    // Empty (:284-286, :380-382), holding `AB` (:385-389), or empty with an InStream created and not used (:364-367).
    let measured_history = matches!(
        (h.text_writes(buf.content_edits), h.in_creates, h.in_uses),
        (Some(0 | 1), 0, 0) | (Some(0), 1, 0)
    );
    if (enc, other_blob, buf.write_pos) == (Encoding::MsDos, true, buf.bytes.len())
        && measured_history
        && never_stored_buffer(&buf)
        && ctx.held_by_temp_blob_impl(&b)
    {
        return Ok(());
    }
    unsupported(
        "Base64Convert.ToBase64(InStream, true, OutStream) into an OutStream other than the one OutStream of the default encoding at the end of another Temp Blob, empty or after one WriteText, no InStream on it used (not measured)",
    )
}

/// Where a `ToBase64(InStream, true ...)` call leaves the stream is not measured (Base64LineBreaks.al moves no stream after it).
pub(crate) fn refuse_move_after_call(s: &InState, what: &str) -> Result<(), Signal> {
    if !s.after_line_break_call {
        return Ok(());
    }
    unsupported(format!(
        "InStream.{what} after Base64Convert.ToBase64(InStream, true ...) (where it stands is not measured)"
    ))
}
