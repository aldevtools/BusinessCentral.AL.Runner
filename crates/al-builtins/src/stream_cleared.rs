//! An `InStream` variable that holds no stream: a cleared one, which is one that was never created again
//! (`Clear` in native.rs).

use crate::native::Signal;
use crate::value::{LibraryValue, Value};

/// `Length` and `ReadText` of an InStream that holds no stream raise NotInitialized
/// (streams/lt-y62-gone-instream/Y62GoneInStream.al:GoneStream_Clear `after clear`, `readtext after clear`, writer Y62). The other
/// methods of such a stream stay refused where they run.
pub(crate) fn refuse_cleared(r: Option<&Value>) -> Result<(), Signal> {
    match r {
        Some(Value::Library(_, h)) if matches!(&*h.borrow(), LibraryValue::Empty) => {
            Err(crate::xml::uninitialized("InStream"))
        }
        _ => Ok(()),
    }
}
