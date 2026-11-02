//! Native builtin implementations, registered by catalog overload id (ADR-008).
//!
//! Split by responsibility: `error` (the error state and `Signal`), `ctx` (what builtins need from the session),
//! `registry` (the id tables and lookup), `unpinned` (the marks of error texts no test pins), `args` (argument
//! readers), and the builtins themselves by area (`format_fns`, `error_fns`, `messages`, `strings`, `session_fns`,
//! `enum_fns`, `values`). Every public item is re-exported here, so callers use `native::name`.

mod args;
mod ctx;
mod dialog;
mod enum_fns;
mod error;
mod error_fns;
mod format_fns;
mod messages;
mod registry;
mod session_fns;
mod strings;
#[cfg(test)]
pub(crate) mod tests;
mod unpinned;
mod values;

use crate::describe::Describe;
use crate::format::{FormatCtx, FormatFail, format_value, try_format};
use crate::record_op::RecordOp;
use crate::value::{LibraryValue, Value};
use dialog::{dialog_close, dialog_hide_subsequent, dialog_open, dialog_update};
use std::rc::Rc;

pub use args::*;
pub use ctx::*;
use enum_fns::*;
pub use error::*;
pub use error_fns::*;
use format_fns::*;
pub use messages::*;
pub use registry::*;
use session_fns::*;
pub use strings::*;
pub use unpinned::*;
pub(crate) use values::*;
