//! Json methods whose failure is pinned only as a statement: what they answer when the return value is used.

use crate::native::{Ctx, Signal};
use crate::value::Value;

/// A failed `JsonObject.Add` (a key that exists) or `JsonArray.Insert`, `Set` or `RemoveAt` (an index outside the array),
/// before anything was changed. As a statement it raises `statement_error`, which the corpus pins. With the return value
/// used it answers false, raises nothing and leaves the last error as it was (AL Runner issue 6924; json/json/
/// JsonObjectUsedReturn.al:37-73 ObjectAdd_DuplicateKey_UsedForms, JsonArrayTokenUsedReturn.al:37 Insert, :165 Set, :303 RemoveAt).
pub(super) fn failed(c: &dyn Ctx, statement_error: Signal) -> Result<Value, Signal> {
    if c.ret_used() {
        Ok(Value::Boolean(false))
    } else {
        Err(statement_error)
    }
}

/// `JsonArray.Insert` or `Set` that worked: true (json/json/JsonArrayTokenUsedReturn.al:842 Array_Success_UsedForms).
pub(super) fn done() -> Value {
    Value::Boolean(true)
}
