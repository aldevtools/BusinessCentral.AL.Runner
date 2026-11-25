//! A closing date moved by a number of days (`ops::arith` sends a closing date plus or minus a whole number here).
//!
//! A closing date plus or minus a whole number (from `ops::arith`): the Integer 1 under 1033 raises
//! ClosingDateOperation (GlobalLanguageClosingDate.al:349-350); every other shape refuses.

use crate::native::{Signal, closing_date_operation};
use crate::value::Value;

/// `closing date + n` and `closing date - n`, the closing date on the left (`day` its day). Only the pinned shape runs
/// (localization/global-language/GlobalLanguageClosingDate.al:302, :304, asserted at :349-350: `CD + 1` and `CD - 1` with the
/// Integer 1 under GlobalLanguage 1033): it raises ClosingDateOperation. Every other number, number type and language refuses.
pub(crate) fn closing_date_moved(day: i32, n: &Value) -> Result<Value, Signal> {
    if day > 0 && matches!(n, Value::Integer(1)) && crate::filter::filter_language() == 1033 {
        return closing_date_operation();
    }
    Err(Signal::unsupported(
        "a closing date plus or minus a number other than the Integer 1 under GlobalLanguage 1033 (not measured)",
    ))
}
