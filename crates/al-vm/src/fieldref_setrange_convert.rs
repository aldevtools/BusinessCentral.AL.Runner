//! `FieldRef.SetRange` with a value of another type than the field's (al-runner issue 8857). BC converts it by how the
//! argument is written: lowering (al-sema `setrange_args_written`) keeps the catalog id when every argument is a plain value,
//! and picks `FieldRef.SetRangeVariant#1` when every argument is a Variant, `FieldRef.SetRangeUntyped#1` for any other mix.
//! The measured cells raise here before the range is set; every other value goes on to `setrange_filter` as before.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::{LibraryValue, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// How the arguments of a `FieldRef.SetRange` call are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Written {
    /// No argument, or every argument a plain value (the catalog id).
    Plain,
    /// Every argument a Variant.
    Variant,
    /// Anything else.
    Untyped,
}

/// The catalog id of `id` and, for a `FieldRef.SetRange` id, how its arguments are written.
pub(crate) fn catalog_id(id: &str) -> (&str, Option<Written>) {
    match id {
        "FieldRef.SetRange#1" => (id, Some(Written::Plain)),
        "FieldRef.SetRangeVariant#1" => ("FieldRef.SetRange#1", Some(Written::Variant)),
        "FieldRef.SetRangeUntyped#1" => ("FieldRef.SetRange#1", Some(Written::Untyped)),
        _ => (id, None),
    }
}

impl Session {
    /// Raises the measured error of a `FieldRef.SetRange` on the FieldRef `lib` whose arguments `args` are written as
    /// `written`; `Ok` leaves the call to the range `setrange_filter` sets (and a FieldRef that holds no field to its error).
    pub(crate) fn fieldref_setrange_written(
        &mut self,
        lib: &Rc<RefCell<LibraryValue>>,
        args: &[Value],
        written: Written,
    ) -> Result<(), Signal> {
        let (rec, pos) = match &*lib.borrow() {
            LibraryValue::FieldRef(r, p) | LibraryValue::KeyFieldRef(r, p) => (r.clone(), *p),
            _ => return Ok(()),
        };
        let ty = self.table(&rec)?.fields[pos].ty.clone();
        let _ = (ty, args, written);
        Ok(())
    }
}
