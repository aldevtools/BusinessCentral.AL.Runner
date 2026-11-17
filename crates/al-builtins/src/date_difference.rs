//! The difference of two Dates or two DateTimes where a side is undefined or a closing date (`ops::arith` takes the others).

use crate::native::{Signal, closing_date_operation, date_invalid};
use crate::value::Value;

/// 0D - 0D is 0 (types/decimal-integer/Y65UndefinedArith.al Y65_UndefinedDate_Arithmetic); one undefined side or too wide a difference raises DateInvalid.
pub(crate) fn undefined_date_difference(a: i32, b: i32) -> Result<Value, Signal> {
    if a == 0 && b == 0 {
        return Ok(Value::Integer(0));
    }
    date_invalid()
}

/// DateTime - DateTime is a Duration in milliseconds (session-system/TestDateTimeContracts.al:52-90). 0DT - 0DT is a 0 Duration;
/// one undefined side raises DateInvalid (types/date-time/Y62UndefinedDate.al: DateTimeSub_BothUndefined,
/// DateTimeSub_OneUndefined_AndDefinedPair, :166-203).
pub(crate) fn datetime_difference(a: i64, b: i64) -> Result<Value, Signal> {
    if a > 0 && b > 0 {
        return Ok(Value::Duration(a - b));
    }
    if a == 0 && b == 0 {
        return Ok(Value::Duration(0));
    }
    date_invalid()
}

/// A subtraction of Dates with a closing date on either side. Only the shape the reporter of al-runner issue 8199 measured runs
/// (assumption `closing-date-minus-date-raises`): the closing date of a defined day minus the plain date of the same day, under
/// GlobalLanguage 1033, raises. Every other day, side, undefined date and language is refused.
pub(crate) fn closing_date_minus((a, a_closing): (i32, bool), (b, b_closing): (i32, bool)) -> Result<Value, Signal> {
    if a_closing && !b_closing && a > 0 && a == b && crate::filter::filter_language() == 1033 {
        crate::assumptions::assume(crate::assumptions::Assumption::ClosingDateMinusDateRaises);
        return closing_date_operation();
    }
    Err(Signal::unsupported(
        "a subtraction of Dates with a closing date other than the closing date of a day minus that day under 1033 (not measured)",
    ))
}
