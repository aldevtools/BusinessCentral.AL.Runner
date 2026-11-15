//! An `InStream` or `OutStream` variable that holds no stream: one never created (`LibraryValue::Empty`) or one cleared
//! (`LibraryValue::ClearedStream`, `Clear` in native/values.rs). The corpus measured more uses of the first than of the second.

use crate::native::Signal;
use crate::value::{LibraryValue, RecordState, Value};

/// Whether a stream handle holds no stream, never created or cleared.
pub(crate) fn holds_no_stream(l: &LibraryValue) -> bool {
    matches!(l, LibraryValue::Empty | LibraryValue::ClearedStream)
}

/// `Length` and `ReadText` of an InStream that holds no stream raise NotInitialized, cleared
/// (streams/instream-outstream/Y62GoneInStream.al:GoneStream_Clear `after clear`, `readtext after clear`, writer Y62) or never
/// created (StreamUncreatedUse.al:106-109 `in_length`, `in_readtext`).
pub(crate) fn refuse_cleared(r: Option<&Value>) -> Result<(), Signal> {
    match r {
        Some(Value::Library(_, h)) if holds_no_stream(&h.borrow()) => Err(crate::xml::uninitialized("InStream")),
        _ => Ok(()),
    }
}

/// `EOS`, `Position` and `Read(Text)` of an InStream that was never created raise NotInitialized
/// (StreamUncreatedUse.al:UncreatedInStream_EveryMethod, :100-110 `in_eos`, `in_position`, `in_read`); of a cleared one they are
/// not measured and refused. `method` names the call for the refusal.
pub(crate) fn refuse_uncreated(r: Option<&Value>, method: &str) -> Result<(), Signal> {
    match r {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::Empty => Err(crate::xml::uninitialized("InStream")),
            LibraryValue::ClearedStream => Err(Signal::unsupported(format!(
                "InStream.{method} of a cleared stream (only a stream never created is pinned)"
            ))),
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

/// Whether `v` is a stream variable that was never created (not a cleared one).
pub(crate) fn never_created(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Library(_, h)) if matches!(&*h.borrow(), LibraryValue::Empty))
}

/// A write to, or a `CopyStream` from or into, a stream that was never created (`nav` names the stream that holds none:
/// `InStream` or `OutStream`). Used, `WriteText(Text)` and `Write(Integer)` answer 0 and `CopyStream` false, without writing
/// (streams/instream-outstream/StreamUncreatedUse.al:57-60 and :123-124 `out_writetext`, `out_write`; :61-66 and :125
/// `out_copystream`; :41-45 and :111 `in_copystream`); as a statement they raise NotInitialized (UncreatedStreams_StatementForms,
/// :183-233).
pub(crate) fn uncreated_result(used: bool, nav: &str, answer: Value) -> Result<Value, Signal> {
    if used {
        Ok(answer)
    } else {
        Err(crate::xml::uninitialized(nav))
    }
}

/// `InStream.ResetPosition()` as a statement on an InStream never created raises NotInitialized naming NavInStream (corpus PR
/// 1408, merged and not yet pinned: tests/al-language/streams/instream-outstream/UhUntouchedStreams.al:44-48 `inreset`, test :88;
/// assumption `stream-untouched-reset-copy-media`). Other forms and a cleared stream go on (and refuse there).
pub(crate) fn reset_uncreated(r: Option<&Value>, used: bool) -> Result<(), Signal> {
    if !used && never_created(r) {
        crate::assumptions::assume(crate::assumptions::Assumption::StreamUntouchedResetCopyMedia);
        return Err(crate::xml::uninitialized("InStream"));
    }
    Ok(())
}

/// `CopyStream(OutStream, InStream)` with both streams never created: as a statement it raises NotInitialized naming NavInStream,
/// with the result used it answers false (corpus PR 1408, UhUntouchedStreams.al:51-56 `copy_stmt`, `copy_value`, tests :108,
/// :118; assumption `stream-untouched-reset-copy-media`).
pub(crate) fn copy_both_uncreated(used: bool) -> Result<Value, Signal> {
    crate::assumptions::assume(crate::assumptions::Assumption::StreamUntouchedResetCopyMedia);
    uncreated_result(used, "InStream", Value::Boolean(false))
}

/// The `RecordState::unpinned_buffer` reason of a record after [`import_from_uncreated`] answered: the probe did not read the
/// record after the import, so its field, a write of it to the store and a copy of it refuse (al-vm `records.rs`
/// `unpinned_buffer_op` and `copy_refused_mark`, `interp.rs` `read_record_field`, `refs.rs` `gettable_record_copy`, `recmeta.rs`
/// `format_composite_value`) until a read fills the buffer again. A `Clear` of the variable keeps it.
pub const UNCREATED_IMPORT_BUFFER: &str = "a record after Media or MediaSet ImportStream from an InStream never created: what its field holds afterwards is not measured (docs/probes-needed.md, stream-untouched-reset-copy-media)";

/// `Media.ImportStream` and `MediaSet.ImportStream` with the result used, from an InStream never created into a field that holds
/// no media, with the arguments the probe passed, on the record the probe used: `Some` with the empty Guid (corpus PR 1408,
/// UhUntouchedStreams.al:58-72 `media_import` (InS, 'icon'), `media_import2` (InS, 'icon', 'image/png'), `mediaset_import` (InS,
/// 'icon'); tests :128, :138, :148; assumption `stream-untouched-reset-copy-media`). The record (`receiver`, the record that holds
/// the field, from the call site in al-vm `exec/run.rs`) must be in the state of the probe's `Pic` (:37-40, :58-72), as
/// [`measured_receiver`] lists it (the key field the probe assigned is not checked). The record then
/// carries [`UNCREATED_IMPORT_BUFFER`]: the probe did not read it afterwards. `set`: a MediaSet. `None` to go on (the builtin
/// refuses an InStream never created).
pub fn import_from_uncreated(
    used: bool,
    set: bool,
    field_blank: bool,
    receiver: &mut RecordState,
    a: &[Value],
) -> Option<Value> {
    let text = |v: &Value, t: &str| matches!(v, Value::Text(s) if s.as_str() == t);
    let args_measured = match a {
        [s, d] => never_created(Some(s)) && text(d, "icon"),
        [s, d, m] if !set => never_created(Some(s)) && text(d, "icon") && text(m, "image/png"),
        _ => false,
    };
    if !(used && field_blank && args_measured && measured_receiver(receiver)) {
        return None;
    }
    crate::assumptions::assume(crate::assumptions::Assumption::StreamUntouchedResetCopyMedia);
    receiver.unpinned_buffer = Some(UNCREATED_IMPORT_BUFFER);
    Some(Value::Guid([0; 16]))
}

/// The probe's `Pic` (UhUntouchedStreams.al:37-40): a record of a stored table that no Insert, Get or Find reached, in the
/// session's company, with no other unpinned buffer.
fn measured_receiver(r: &RecordState) -> bool {
    !r.temporary
        && !r.inserted
        && r.row_origin.is_none()
        && r.position.is_none()
        && r.company.is_none()
        && r.unpinned_buffer.is_none()
}
