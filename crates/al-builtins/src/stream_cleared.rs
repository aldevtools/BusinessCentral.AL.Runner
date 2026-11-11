//! An `InStream` or `OutStream` variable that holds no stream: one never created (`LibraryValue::Empty`) or one cleared
//! (`LibraryValue::ClearedStream`, `Clear` in native/values.rs). The corpus measured more uses of the first than of the second.

use crate::native::Signal;
use crate::value::{LibraryValue, Value};

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
