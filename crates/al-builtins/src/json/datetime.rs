use super::{NAV_JSON_VALUE, Scalar, Signal, Value, conversion, unsupported};
use crate::describe::Describe;

pub(super) fn as_datetime(s: &Scalar) -> Result<Value, Signal> {
    match s {
        Scalar::Val(Value::DateTime(d)) => Ok(Value::DateTime(*d)),
        // al-runner issue 7907 and corpus PR #1361, record/crud/TestMinimumUtcDateTimeStorage.al.
        Scalar::Val(Value::Text(t)) if t.as_ref() == "0001-01-01T00:00:00Z" => {
            crate::assumptions::assume(crate::assumptions::Assumption::JsonMinimumUtcDateTimeAsBlank);
            Ok(Value::DateTime(0))
        }
        Scalar::Val(Value::Text(t)) => match crate::iso::parse_datetime(t) {
            Some(d) => Ok(Value::DateTime(d)),
            None => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavDateTime")),
        },
        // A Date or Time held reads as no DateTime (P88DateTimeReadback.al:78-163, Date_Readback and Time_Readback `_asdatetime`).
        Scalar::Val(
            Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) | Value::Date(..) | Value::Time(_),
        ) => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavDateTime")),
        other => unsupported(format!(
            "reading {} as DateTime (conversion not pinned yet)",
            other.describe()
        )),
    }
}
