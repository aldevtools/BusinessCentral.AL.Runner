//! A Variant given to `FieldRef.Validate` or `FieldRef.Value :=` of a field that is no Text or Code field (those are
//! `fieldref_value_text`): an Integer, Decimal, BigInteger, Boolean, Option, Enum, Guid, Duration, Date, Time or DateTime field.
//!
//! Pinned by fieldref/lt-y67-fieldref-validate-variant/Y67FieldRefValidateVariant.al:288-1180 (writer Y67, real BC 27.5, 28.5 and
//! 29.0; Validate and the assignment give the same value or error, Validate runs OnValidate once and the assignment does not):
//! a Variant holding 42, -3, 2.5, 5000000000, true, 20260115D, 120000T, a DateTime, '42', 'abc', 'C1', '', an Option, an Enum, a
//! Guid or a Duration of 1500 into each field type. A value converts where BC converts it (a Date, Time and DateTime as the
//! numbers `conv::variant_scalars` gives, true as 1); where it does not, from BC 28 the Variant's Format text is evaluated into
//! the field's type and its Evaluate error raises, before BC 28 a conversion error of the .NET types. `None`: a value the probe
//! does not pin, converted as before.

use crate::format::FormatCtx;
use crate::native::{AlError, Signal};
use crate::value::{RtType, Value};
use rust_decimal::Decimal;

/// What a cell gives: `None` for one this module does not pin.
type Cell = Option<Result<Value, Signal>>;

const MS_PER_DAY: i64 = 86_400_000;
const NS: &str = "Microsoft.Dynamics.Nav.Runtime.";

fn err(message: String, code: &str) -> Option<Result<Value, Signal>> {
    Some(Err(Signal::Error(Box::new(AlError {
        message,
        code: code.into(),
        ..Default::default()
    }))))
}

/// The target types this module answers for, with the name an Evaluate error and a `Nav<Type>` text give them.
fn target_names(t: &RtType) -> Option<(&'static str, &'static str)> {
    Some(match t {
        RtType::Integer => ("Integer", "Integer"),
        RtType::Decimal => ("Decimal", "Decimal"),
        RtType::BigInteger => ("BigInteger", "BigInteger"),
        RtType::Boolean => ("Boolean", "Boolean"),
        RtType::Option(_) | RtType::Enum(_) => ("Option", "Option"),
        RtType::Duration => ("Duration", "Duration"),
        RtType::Guid => ("GUID", "Guid"),
        RtType::Date => ("Date", "Date"),
        RtType::Time => ("Time", "Time"),
        RtType::DateTime => ("DateTime", "DateTime"),
        _ => return None,
    })
}

/// The `Nav<Type>` of a source value in the BC 27 conversion errors (an Enum is a NavOption there).
fn nav_of(v: &Value) -> Option<&'static str> {
    Some(match v {
        Value::Integer(_) => "NavInteger",
        Value::Decimal(_) => "NavDecimal",
        Value::BigInteger(_) => "NavBigInteger",
        Value::Boolean(_) => "NavBoolean",
        Value::Text(_) => "NavText",
        Value::Code(_) => "NavCode",
        Value::Option(..) | Value::Enum(..) => "NavOption",
        Value::Guid(_) => "NavGuid",
        Value::Duration(_) => "NavDuration",
        _ => return None,
    })
}

/// A number into a number target: an Integer beyond its range raises the overflow (`biginteger` into Integer, Boolean, Option and
/// Enum fields); a Boolean takes any number but 0 as true (0 is not pinned).
fn number_into(n: i64, t: &RtType) -> Option<Result<Value, Signal>> {
    let small = || i32::try_from(n).ok();
    let overflow = || err("Arithmetic operation resulted in an overflow.".into(), "AL:Overflow");
    Some(Ok(match t {
        RtType::Integer => Value::Integer(match small() {
            Some(i) => i,
            None => return overflow(),
        }),
        RtType::Option(m) => match small() {
            Some(i) => Value::Option(i, m.clone().into()),
            None => return overflow(),
        },
        RtType::Enum(id) => match small() {
            Some(i) => Value::Enum(*id, i),
            None => return overflow(),
        },
        RtType::Boolean if small().is_none() => return overflow(),
        RtType::Boolean if n != 0 => Value::Boolean(true),
        RtType::Decimal => Value::Decimal(Decimal::from(n)),
        RtType::BigInteger => Value::BigInteger(n),
        RtType::Duration => Value::Duration(n),
        _ => return None,
    }))
}

/// The number a value stands for in a number field, `None` for a value that is no number there.
fn number_of(v: &Value) -> Option<i64> {
    Some(match v {
        Value::Integer(n) | Value::Option(n, _) | Value::Enum(_, n) => i64::from(*n),
        Value::BigInteger(n) | Value::Duration(n) => *n,
        Value::Boolean(true) => 1,
        Value::Decimal(d) if d.fract().is_zero() => i64::try_from(*d).ok()?,
        Value::Date(n, false) if *n > 0 => 2 * i64::from(*n) + 731,
        Value::Time(x) if *x > 0 => *x,
        Value::Text(s) | Value::Code(s) if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) => s.parse().ok()?,
        _ => return None,
    })
}

/// A text that is no number: empty, or letters and digits with a letter (`abc`, `C1`, ``).
fn no_number(s: &str) -> bool {
    s.is_empty() || (s.bytes().all(|b| b.is_ascii_alphanumeric()) && s.bytes().any(|b| b.is_ascii_alphabetic()))
}

/// Whether the Format text of `v` cannot be a date, a time or a datetime, so that BC's Evaluate of it fails: a text with a
/// letter or empty, a Guid, a Boolean, a Duration, a caption, a negative or fractional number or one above 31.
fn never_temporal(v: &Value) -> bool {
    match v {
        Value::Text(s) | Value::Code(s) => {
            no_number(s) || (s.bytes().all(|b| b.is_ascii_digit()) && s.parse::<i64>().is_ok_and(|n| n > 31))
        }
        Value::Integer(n) => !(0..=31).contains(n),
        Value::BigInteger(n) => !(0..=31).contains(n),
        Value::Decimal(d) => !d.fract().is_zero() || *d < Decimal::ZERO || *d > Decimal::from(31),
        Value::Boolean(_) | Value::Guid(_) | Value::Duration(_) => true,
        // An Option or Enum is its caption, which the probe's members spell with letters.
        Value::Option(..) | Value::Enum(..) => true,
        _ => false,
    }
}

/// The conversion of `v` into a field of type `t`. `from28`: the target is BC 28 or later (Gate::FieldRefVariantEvaluateErrorFrom28).
pub fn variant_into_field(v: &Value, t: &RtType, from28: bool, ctx: &dyn FormatCtx) -> Option<Result<Value, Signal>> {
    let (eval_name, nav_name) = target_names(t)?;
    let text = || crate::format::try_format(v, 0, 0, ctx).ok();
    let evaluate_error = |kind: &str| -> Option<Result<Value, Signal>> {
        let f = text()?;
        if matches!(t, RtType::Date) {
            return err(
                format!(
                    "The format of the Date '{f}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
                ),
                "Evaluate:Conversion",
            );
        }
        err(
            format!("The value \"{f}\" can't be evaluated into type {eval_name}."),
            kind,
        )
    };
    let unable = |from: &str, to: &str| err(format!("Unable to convert from {NS}{from} to {to}."), "Conversion");
    match t {
        RtType::Guid => match v {
            Value::Guid(_) => None,
            _ if from28 => evaluate_error("Evaluate:Argument"),
            _ => err(
                "Invalid Conversion. Source: NavVariant, Destination: NavGuid.".into(),
                "Argument",
            ),
        },
        RtType::Date | RtType::Time | RtType::DateTime => temporal_into(v, t, from28, &evaluate_error, &unable),
        _ => match v {
            // A Decimal that is no whole number (`decimal 2.5`).
            Value::Decimal(d) if !d.fract().is_zero() => {
                let to = if matches!(t, RtType::BigInteger | RtType::Duration) {
                    "System.Int64"
                } else if matches!(t, RtType::Decimal) {
                    return None;
                } else {
                    "System.Int32"
                };
                err(
                    format!("Overflow under conversion of {NS}Decimal18 value {d} to {to}."),
                    "ConversionCast",
                )
            }
            // A DateTime is a number in a Decimal, BigInteger and Duration field only.
            Value::DateTime(x) if *x > 0 => match t {
                RtType::Decimal | RtType::BigInteger | RtType::Duration => number_into(*x + 366 * MS_PER_DAY, t),
                _ if from28 => evaluate_error("Evaluate:Conversion"),
                _ => unable("NavDateTime", "System.Int32"),
            },
            Value::Guid(_) if from28 => evaluate_error("Evaluate:Conversion"),
            Value::Guid(_) => unable("NavGuid", "System.Int32"),
            Value::Text(s) | Value::Code(s) if no_number(s.as_str()) => {
                if from28 {
                    evaluate_error("Evaluate:Format")
                } else {
                    err(format!("The type Nav{nav_name} is unknown."), "NotSupportedType")
                }
            }
            // A Date, Time or true into a Boolean field is true.
            Value::Date(n, false) | Value::Enum(_, n) | Value::Option(n, _)
                if matches!(t, RtType::Boolean) && *n != 0 =>
            {
                Some(Ok(Value::Boolean(true)))
            }
            Value::Time(x) if matches!(t, RtType::Boolean) && *x > 0 => Some(Ok(Value::Boolean(true))),
            v => number_into(number_of(v)?, t),
        },
    }
}

/// Into a Date, Time or DateTime field.
fn temporal_into(
    v: &Value,
    t: &RtType,
    from28: bool,
    evaluate_error: &dyn Fn(&str) -> Cell,
    unable: &dyn Fn(&str, &str) -> Cell,
) -> Option<Result<Value, Signal>> {
    match (v, t) {
        (Value::Date(n, false), RtType::DateTime) if *n > 0 => {
            Some(Ok(Value::DateTime((i64::from(*n) - 1) * MS_PER_DAY + 1)))
        }
        // A Time into a DateTime is that time on 0001-01-02 (`time 120000T` 0001-01-02T12:00:00Z).
        (Value::Time(x), RtType::DateTime) if *x > 0 => Some(Ok(Value::DateTime(*x + MS_PER_DAY))),
        (Value::Time(x), RtType::Date) if *x > 0 => err("The date is not valid.".into(), "DateInvalid"),
        (Value::DateTime(x), RtType::Date) if *x > 0 => err("The date is not valid.".into(), "DateInvalid"),
        (Value::Date(n, false), RtType::Time) if *n > 0 => err("The time is not valid.".into(), "InvalidTime"),
        (Value::DateTime(x), RtType::Time) if *x > 0 => err("The time is not valid.".into(), "InvalidTime"),
        (Value::Date(..) | Value::Time(_) | Value::DateTime(_), _) => None,
        (v, _) if from28 && never_temporal(v) => evaluate_error("Evaluate:Conversion"),
        (v, _) if from28 => {
            let _ = v;
            None
        }
        // Before BC 28 every value of another kind raises the conversion error.
        (v, _) => unable(nav_of(v)?, "System.DateTime"),
    }
}
