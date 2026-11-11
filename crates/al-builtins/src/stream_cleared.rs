//! An `InStream` variable that holds no stream: a cleared one, which is one that was never created again
//! (`Clear` in native.rs).

use crate::native::Signal;
use crate::value::{LibraryValue, Value};

/// `Length` and `ReadText` of an InStream that holds no stream raise NotInitialized
/// (streams/instream-outstream/Y62GoneInStream.al:GoneStream_Clear `after clear`, `readtext after clear`, writer Y62), and so do
/// `EOS`, `Position` and `Read(Text)` of one that was never created (StreamUncreatedUse.al:UncreatedInStream_EveryMethod
/// `in_eos`, `in_position`, `in_read`, and `in_length`, `in_readtext` again). The other methods of such a stream stay refused
/// where they run.
pub(crate) fn refuse_cleared(r: Option<&Value>) -> Result<(), Signal> {
    match r {
        Some(Value::Library(_, h)) if matches!(&*h.borrow(), LibraryValue::Empty) => {
            Err(crate::xml::uninitialized("InStream"))
        }
        _ => Ok(()),
    }
}

/// Whether `v` is a stream variable that holds no stream (never created, or cleared).
pub(crate) fn holds_none(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Library(_, h)) if matches!(&*h.borrow(), LibraryValue::Empty))
}

/// A write to, or a `CopyStream` from or into, a stream that was never created (`nav` names the stream that holds none:
/// `InStream` or `OutStream`). Used, `WriteText` and `Write` answer 0 and `CopyStream` false, without writing
/// (streams/instream-outstream/StreamUncreatedUse.al:UncreatedOutStream_EveryMethod `out_writetext`, `out_write`,
/// `out_copystream`; UncreatedInStream_EveryMethod `in_copystream`); as a statement they raise NotInitialized
/// (UncreatedStreams_StatementForms).
pub(crate) fn uncreated_result(used: bool, nav: &str, answer: Value) -> Result<Value, Signal> {
    if used {
        Ok(answer)
    } else {
        Err(crate::xml::uninitialized(nav))
    }
}
