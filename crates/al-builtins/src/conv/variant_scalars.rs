//! A Variant holding a defined Date, Time, DateTime, Duration, BigInteger, Decimal, Boolean, Integer or a text assigned to an
//! Integer, Decimal, BigInteger, Duration, Date, Time, DateTime or Boolean variable: the cells `ops::convert` refuses.
//!
//! Pinned by types/lt-y67-variant-numeric/Y67VariantNumeric.al:147-353 (writer Y67, real BC 29, the same on 27.5 and 28.5). A
//! Date is the number `2 * day + 731` (one more for a closing date; `day` counts from 0001-01-01 as 1), a Time its milliseconds
//! plus one, a DateTime its milliseconds since 0001-01-01 plus one, moved by 366 days; a Duration takes that number as
//! milliseconds. The undefined values are `conv_undefined`'s (Y62UndefinedDate.al).

use crate::native::{Signal, raise};
use crate::value::{RtType, Value};
use rust_decimal::Decimal;

const MS_PER_DAY: i64 = 86_400_000;

fn indirect(nav: &str) -> Result<Value, Signal> {
    raise(
        format!(
            "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.{nav} value."
        ),
        "TypeConversion",
    )
}

fn variant_error(target: &str, code: &str) -> Result<Value, Signal> {
    raise(
        format!("The expression Variant cannot be type-converted to a {target} value."),
        code,
    )
}

/// The number a Date, Time or DateTime stands for in a number slot, by the probe's values (Variant_DefinedDates_A..D,
/// Variant_Times_A, Variant_DateTimes_A).
fn temporal_number(v: &Value) -> Option<i64> {
    match v {
        Value::Date(n, closing) if *n > 0 => Some(2 * i64::from(*n) + 731 + i64::from(*closing)),
        Value::Time(x) if *x > 0 => Some(*x),
        Value::DateTime(x) if *x > 0 => Some(*x + 366 * MS_PER_DAY),
        _ => None,
    }
}

/// A number into one of the number targets.
fn number_into(n: i64, t: &RtType) -> Option<Value> {
    Some(match t {
        RtType::Integer => Value::Integer(i32::try_from(n).ok()?),
        RtType::Decimal => Value::Decimal(Decimal::from(n)),
        RtType::BigInteger => Value::BigInteger(n),
        RtType::Duration => Value::Duration(n),
        _ => return None,
    })
}

/// The conversion of `v`, held by a Variant, into `t`, `None` for a cell this probe does not pin.
pub(crate) fn convert(v: &Value, t: &RtType) -> Option<Result<Value, Signal>> {
    match v {
        Value::Date(n, closing) if *n > 0 => Some(match t {
            RtType::Time => indirect("NavTime"),
            // A closing date is that day at 23:59:59 (Variant_DefinedDates_D `closing 20260115D` 2026-01-15T23:59:59Z).
            RtType::DateTime => Ok(Value::DateTime(
                (i64::from(*n) - 1) * MS_PER_DAY + if *closing { 86_399_000 } else { 0 } + 1,
            )),
            RtType::Boolean => Ok(Value::Boolean(true)),
            t => number_into(temporal_number(v)?, t).map(Ok)?,
        }),
        Value::Time(x) if *x > 0 => Some(match t {
            RtType::Date => indirect("NavDate"),
            RtType::DateTime => indirect("NavDateTime"),
            RtType::Boolean => Ok(Value::Boolean(true)),
            t => number_into(*x, t).map(Ok)?,
        }),
        Value::DateTime(x) if *x > 0 => Some(match t {
            RtType::Integer | RtType::Boolean => raise(
                "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDateTime to System.Int32.",
                "Conversion",
            ),
            // The day of the UTC time (the probe ran in UTC; `DT2Date`).
            RtType::Date => Ok(Value::Date(((*x - 1).div_euclid(MS_PER_DAY) + 1) as i32, false)),
            RtType::Time => indirect("NavTime"),
            t => number_into(temporal_number(v)?, t).map(Ok)?,
        }),
        // A Duration, an Integer, a BigInteger, a Code '12' and a text of digits that are not zero are true (Variant_Numbers_A,
        // _B `duration 1500`, `integer -7`, `code 12`, Variant_TextNumbers_A `text 12`, `text 20260115`). Zero is not pinned.
        Value::Duration(n) | Value::BigInteger(n) if *n != 0 && matches!(t, RtType::Boolean) => Some(match v {
            Value::BigInteger(n) if i32::try_from(*n).is_err() => variant_error("Boolean", "TypeConversion:Overflow"),
            _ => Ok(Value::Boolean(true)),
        }),
        Value::Integer(n) if *n != 0 && matches!(t, RtType::Boolean) => Some(Ok(Value::Boolean(true))),
        Value::Duration(_) | Value::BigInteger(_) | Value::Integer(_) | Value::Decimal(_) | Value::Boolean(_) => {
            match t {
                RtType::Date => Some(indirect("NavDate")),
                RtType::Time => Some(indirect("NavTime")),
                RtType::DateTime => Some(indirect("NavDateTime")),
                // A Decimal that is no whole number into an Integer, a BigInteger, a Duration or a Boolean (Variant_Numbers_A
                // `decimal 2.5`).
                _ => match v {
                    Value::Decimal(d) if !d.fract().is_zero() => {
                        let to = match t {
                            RtType::Integer | RtType::Boolean => "System.Int32",
                            RtType::BigInteger | RtType::Duration => "System.Int64",
                            _ => return None,
                        };
                        Some(raise(
                            format!(
                                "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value {d} to {to}."
                            ),
                            "ConversionCast",
                        ))
                    }
                    // True is 1 (Variant_Numbers_B `boolean true` 1 ; 1 ; 1 ; P0DT0H0M0.001S).
                    Value::Boolean(true) => number_into(1, t).map(Ok),
                    _ => None,
                },
            }
        }
        Value::Text(s) | Value::Code(s) => text_into(s.as_str(), t),
        _ => None,
    }
}

/// A text held by a Variant (Variant_TextNumbers_A..C, Variant_Numbers_B `code 12`): digits read as a number into every number
/// target and as true into a Boolean; `12.5` reads into a Decimal only; `true`, the empty text and letters read into none.
fn text_into(s: &str, t: &RtType) -> Option<Result<Value, Signal>> {
    let digits = !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let pinned = digits || s == "12.5" || s == "true" || s.is_empty() || s.bytes().all(|b| b.is_ascii_alphabetic());
    if !pinned {
        return None;
    }
    let target = match t {
        RtType::Integer => "Integer",
        RtType::Decimal => "Decimal",
        RtType::BigInteger => "BigInteger",
        RtType::Boolean => "Boolean",
        RtType::Duration => {
            return Some(if digits {
                s.parse::<i64>()
                    .map(Value::Duration)
                    .map_err(|_| Signal::Unsupported(format!("text `{s}` into a Duration (out of range)")))
            } else {
                raise(
                    format!("The input string '{s}' was not in a correct format."),
                    "AL:Format",
                )
            });
        }
        RtType::Date => return Some(indirect("NavDate")),
        RtType::Time => return Some(indirect("NavTime")),
        RtType::DateTime => return Some(indirect("NavDateTime")),
        _ => return None,
    };
    if digits {
        let n: i64 = s.parse().ok()?;
        return match t {
            RtType::Boolean if n != 0 => Some(Ok(Value::Boolean(true))),
            RtType::Boolean => None,
            t => number_into(n, t).map(Ok),
        };
    }
    if s == "12.5" && matches!(t, RtType::Decimal) {
        return Some(Ok(Value::Decimal(Decimal::new(125, 1))));
    }
    Some(variant_error(target, "TypeConversion:Format"))
}

/// The cells where `ops::convert` converts or raises another error but a Variant raises its own: a BigInteger beyond the Integer
/// range into a Boolean (Variant_Numbers_A `biginteger` TypeConversion:Overflow; `ops::convert` reads any number that is not zero
/// as true), a number beyond a Char or a Byte.
pub(crate) fn before_convert(v: &Value, t: &RtType) -> Option<Result<Value, Signal>> {
    match (v, t) {
        (Value::BigInteger(n), RtType::Boolean) if i32::try_from(*n).is_err() => {
            Some(variant_error("Boolean", "TypeConversion:Overflow"))
        }
        // A number beyond the target into a Char or a Byte (types/lt-y31-conversion-matrix/Y31ConvCv.al:9942 Variant_To_Char
        // `big` 3000000000, :10138 Variant_To_Byte `int300` and `big`): the Variant's overflow, where `ops::convert` raises the cast.
        (Value::BigInteger(n), RtType::Char) if i32::try_from(*n).is_err() => {
            Some(variant_error("Char", "TypeConversion:Overflow"))
        }
        (Value::BigInteger(n), RtType::Byte) if u8::try_from(*n).is_err() => {
            Some(variant_error("Byte", "TypeConversion:Overflow"))
        }
        (Value::Integer(n), RtType::Byte) if u8::try_from(*n).is_err() => {
            Some(variant_error("Byte", "TypeConversion:Overflow"))
        }
        // A Decimal that is no whole number into a Char or a Byte casts through an Int32 (Variant_To_Char, Variant_To_Byte `dec1p5`).
        (Value::Decimal(d), RtType::Char | RtType::Byte) if !d.fract().is_zero() => Some(raise(
            format!("Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value {d} to System.Int32."),
            "ConversionCast",
        )),
        _ => None,
    }
}
