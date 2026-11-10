//! Json methods whose failure is pinned only as a statement: what they answer when the return value is used.

use crate::assumptions::{Assumption, assume};
use crate::native::{Ctx, Signal};
use crate::value::Value;

/// A failed `JsonObject.Add` (a key that exists) or `JsonArray.Insert`, `Set` or `RemoveAt` (an index outside the array),
/// before anything was changed. As a statement it raises `statement_error`, which the corpus pins. With the return value
/// used it answers false and raises nothing: assumed (`json-failed-call-result-used`, AL Runner issue 6924: a user's
/// `exit(JsonObj.Add(Key, Value))` with a key that exists returns false on real BC), like the pinned used forms of
/// `Get` (json/JsonObjectBasics.al:31 `get_missing`) and `Replace` (json/TestJsonObjectReplaceKeysContracts.al).
pub(super) fn failed(c: &dyn Ctx, statement_error: Signal) -> Result<Value, Signal> {
    if c.ret_used() {
        assume(Assumption::JsonFailedCallResultUsed);
        Ok(Value::Boolean(false))
    } else {
        Err(statement_error)
    }
}

/// `JsonArray.Insert` or `Set` that worked. The corpus uses them as statements only; true with the return value used is
/// part of the same assumption.
pub(super) fn done(c: &dyn Ctx) -> Value {
    if c.ret_used() {
        assume(Assumption::JsonFailedCallResultUsed);
    }
    Value::Boolean(true)
}
