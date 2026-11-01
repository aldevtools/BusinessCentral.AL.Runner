//! Undefined Date, Time and DateTime values (0D, 0T, 0DT): a Variant holding one assigned to a number, a Duration or a Text, and
//! the subtraction of two DateTimes.

use crate::native::{Signal, raise};
use crate::value::{RtType, Value};

/// The conversion of an undefined temporal value into `t`, `None` for any other pair (types/lt-y62-undefined-date/
/// Y62UndefinedDate.al:VariantHoldingUndefined_IntoNumbers, writer Y62): 0D and 0T are 0 as an Integer, a Decimal and a BigInteger
/// and a 0 Duration; 0DT raises Conversion into an Integer and is 0 as the others; all three raise TypeConversion into a Text.
/// A Code target is not pinned.
pub(crate) fn convert(v: &Value, t: &RtType) -> Option<Result<Value, Signal>> {
    let datetime = match v {
        Value::Date(0, false) | Value::Time(0) => false,
        Value::DateTime(0) => true,
        _ => return None,
    };
    Some(match t {
        RtType::Integer if datetime => raise(
            "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDateTime to System.Int32.",
            "Conversion",
        ),
        RtType::Integer => Ok(Value::Integer(0)),
        RtType::Decimal => Ok(Value::Decimal(Default::default())),
        RtType::BigInteger => Ok(Value::BigInteger(0)),
        RtType::Duration => Ok(Value::Duration(0)),
        RtType::Text(_) => raise(
            "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavText value.",
            "TypeConversion",
        ),
        _ => return None,
    })
}

/// `A - B` of two DateTimes: a Duration in milliseconds (types/TestDurationContracts.al:44-171, session/TestDateTimeContracts.al:52-90);
/// two undefined ones subtract to 0, one undefined one raises DateInvalid (types/lt-y62-undefined-date/Y62UndefinedDate.al:
/// DateTimeSub_BothUndefined, DateTimeSub_OneUndefined_AndDefinedPair; types/lt-datetime/Arithmetic.al:257-260).
pub(crate) fn datetime_sub(a: i64, b: i64) -> Result<Value, Signal> {
    if (a == 0) != (b == 0) {
        return raise("The date is not valid.", "DateInvalid");
    }
    Ok(Value::Duration(a - b))
}
