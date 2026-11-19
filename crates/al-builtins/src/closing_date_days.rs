//! A closing date moved by a number of days (`ops::arith` sends a closing date plus or minus a whole number here).

use crate::native::{Signal, closing_date_operation};
use crate::value::Value;

/// `closing date + n` and `closing date - n`, the closing date on the left (`day` its day). Only the shape the probe measured
/// runs (assumption `closing-date-plus-minus-one-raises`, corpus PR 1428 GlobalLanguageClosingDate.al:349-350 at 04e1fa0a:
/// `CD + 1` and `CD - 1` with the Integer 1 under GlobalLanguage 1033): it raises ClosingDateOperation. Every other number,
/// number type and language refuses.
pub(crate) fn closing_date_moved(day: i32, n: &Value) -> Result<Value, Signal> {
    if day > 0 && matches!(n, Value::Integer(1)) && crate::filter::filter_language() == 1033 {
        crate::assumptions::assume(crate::assumptions::Assumption::ClosingDatePlusMinusOneRaises);
        return closing_date_operation();
    }
    Err(Signal::unsupported(
        "a closing date plus or minus a number other than the Integer 1 under GlobalLanguage 1033 (not measured)",
    ))
}
