//! The conversion of a value whose (source, target) cell the run matrix covers.
//!
//! Numbers (Integer, BigInteger, Decimal, Char, Byte, an Option's or a Duration's number) convert into each other as a
//! checked .NET cast: a value outside the target's range, and a Decimal with a fraction into anything but a Decimal, raise
//! ConversionCast `Overflow under conversion of <source> value <v> to <target>.`, no rounding (Y31ConvCv.al, LT Y31 Cv
//! Integer/BigInteger/Decimal/Option/Duration: `Integer_To_Char` -1 and 65536, `Decimal_To_Option` 1.5 and 2147483648.0,
//! `Option_To_Byte` -1, `Duration_To_Option` 2147483648). A Char and a Byte convert through code page 437 (`Byte_To_Char`
//! 255 is U+00A0, `Char_To_Byte` 256 is `A`): only ASCII and the values the table holds (`rows::POINTS`).

use super::{Family, Kind, Verdict};
use crate::native::{Signal, raise};
use crate::value::{RtType, Value};
use rust_decimal::Decimal;

/// The kind of a value, for the cells of the table.
pub(super) fn kind_of_value(v: &Value) -> Option<Kind> {
    Some(match v {
        Value::Integer(_) => Kind::Integer,
        Value::BigInteger(_) => Kind::BigInteger,
        Value::Decimal(_) => Kind::Decimal,
        Value::Char(_) => Kind::Char,
        Value::Byte(_) => Kind::Byte,
        Value::Option(..) => Kind::Option,
        Value::Enum(..) => Kind::Enum,
        Value::Text(_) => Kind::Text,
        Value::Code(_) => Kind::Code,
        Value::Guid(_) => Kind::Guid,
        Value::Duration(_) => Kind::Duration,
        Value::Boolean(_) => Kind::Boolean,
        Value::Date(..) => Kind::Date,
        Value::Time(_) => Kind::Time,
        Value::DateTime(_) => Kind::DateTime,
        _ => return None,
    })
}

/// The kind of a slot type, for the cells of the table.
pub(super) fn kind_of_type(t: &RtType) -> Option<Kind> {
    Some(match t {
        RtType::Integer => Kind::Integer,
        RtType::BigInteger => Kind::BigInteger,
        RtType::Decimal => Kind::Decimal,
        RtType::Char => Kind::Char,
        RtType::Byte => Kind::Byte,
        RtType::Option(_) => Kind::Option,
        RtType::Enum(_) => Kind::Enum,
        RtType::Text(_) => Kind::Text,
        RtType::Code(_) => Kind::Code,
        RtType::Guid => Kind::Guid,
        RtType::Duration => Kind::Duration,
        RtType::Boolean => Kind::Boolean,
        RtType::Date => Kind::Date,
        RtType::Time => Kind::Time,
        RtType::DateTime => Kind::DateTime,
        _ => return None,
    })
}

/// The number a value of a number kind stands for, with the .NET type BC names for it in a cast error.
fn number(v: &Value) -> Option<(Decimal, &'static str)> {
    Some(match v {
        Value::Integer(n) => (Decimal::from(*n), "System.Int32"),
        Value::BigInteger(n) => (Decimal::from(*n), "System.Int64"),
        Value::Decimal(d) => (*d, "Microsoft.Dynamics.Nav.Runtime.Decimal18"),
        Value::Option(n, _) => (Decimal::from(*n), "System.Int32"),
        Value::Duration(n) => (Decimal::from(*n), "System.Int64"),
        Value::Char(c) => (Decimal::from(*c), "System.Char"),
        Value::Byte(b) => (Decimal::from(*b), "System.Byte"),
        _ => return None,
    })
}

/// The .NET type of a number target in a cast error, and its range.
fn target(t: &RtType) -> Option<(&'static str, i128, i128)> {
    Some(match t {
        RtType::Integer | RtType::Option(_) => ("System.Int32", i128::from(i32::MIN), i128::from(i32::MAX)),
        RtType::BigInteger | RtType::Duration => ("System.Int64", i128::from(i64::MIN), i128::from(i64::MAX)),
        RtType::Char => ("System.Char", 0, i128::from(u16::MAX)),
        RtType::Byte => ("System.Byte", 0, i128::from(u8::MAX)),
        _ => return None,
    })
}

fn cast_error(d: Decimal, from: &str, to: &str) -> Result<Value, Signal> {
    raise(
        format!("Overflow under conversion of {from} value {d} to {to}."),
        "ConversionCast",
    )
}

/// Converts `v` into `t` when the matrix covers the cell as a conversion between numbers; `None` leaves it to
/// `ops::convert`.
pub fn convert_cell(v: &Value, t: &RtType) -> Option<Result<Value, Signal>> {
    target(t)?;
    let (src, dst) = (kind_of_value(v)?, kind_of_type(t)?);
    // A number into its own type is the number (`ops::convert`).
    if src == dst {
        return None;
    }
    super::verdict(Family::Cv, src, dst)?;
    match (v, t) {
        (Value::Byte(b), RtType::Char) => Some(byte_to_char(*b)),
        (Value::Char(c), RtType::Byte) => Some(char_to_byte(*c)),
        _ => {
            let (d, from) = number(v)?;
            let (to, min, max) = target(t)?;
            Some(cast(d, from, to, min, max, t))
        }
    }
}

fn cast(d: Decimal, from: &str, to: &str, min: i128, max: i128, t: &RtType) -> Result<Value, Signal> {
    if d.fract() != Decimal::ZERO {
        return cast_error(d, from, to);
    }
    let n = d.mantissa() / 10i128.pow(d.scale());
    if n < min || n > max {
        return cast_error(d, from, to);
    }
    Ok(match t {
        RtType::Integer => Value::Integer(n as i32),
        RtType::BigInteger => Value::BigInteger(n as i64),
        RtType::Duration => Value::Duration(n as i64),
        RtType::Char => Value::Char(n as u16),
        RtType::Byte => Value::Byte(n as u8),
        RtType::Option(m) => Value::Option(n as i32, m.clone().into()),
        _ => unreachable!("a number target"),
    })
}

/// The value BC showed for the source `input` (as AL writes it) of a code page cell.
fn point(src: Kind, dst: Kind, input: u32) -> Option<&'static str> {
    super::points(src, dst)
        .find(|(a, _)| a.parse::<u32>().ok() == Some(input))
        .map(|(_, b)| b)
}

/// A Byte into a Char: the character of the fixed table (`super::byte_char`).
fn byte_to_char(b: u8) -> Result<Value, Signal> {
    byte_as_char(b).map(Value::Char)
}

/// The character a Byte converts to (types/lt-byte-char-high/ByteCharHigh.al, see `super::byte_char`).
pub(super) fn byte_as_char(b: u8) -> Result<u16, Signal> {
    Ok(super::byte_char::byte_to_char(b))
}

/// A Char into a Byte: the byte of the fixed table (ByteCharHigh.al, `super::byte_char`); another character is the byte of
/// code page 437 that BC's Format shows as the character the conversion matrix holds (`Char_To_Byte` 256 shows `A`, 65535 `?`).
fn char_to_byte(c: u16) -> Result<Value, Signal> {
    if let Some(b) = super::byte_char::char_to_byte(c) {
        return Ok(Value::Byte(b));
    }
    let shown = point(Kind::Char, Kind::Byte, u32::from(c)).filter(|s| s.chars().count() == 1);
    match shown.map(|s| crate::stream::Encoding::MsDos.encode(s)) {
        Some(b) if b.len() == 1 => Ok(Value::Byte(b[0])),
        _ => Err(Signal::Unsupported(format!(
            "Char {c} into a Byte: the byte of a character from 128 up is pinned only for the characters the conversion matrix tries"
        ))),
    }
}

/// The error BC raises for every value of the kind `v` holds when a Variant holding it is converted into `t`
/// (`Verdict::Error` of a `CvVariant` cell, LT Y31 Cv Variant: `Variant_To_Date` from an Integer, `Variant_To_Guid` from a
/// Boolean), `None` when the cell is not covered or gives values.
pub fn variant_error(v: &Value, t: &RtType) -> Option<Signal> {
    let (src, dst) = (kind_of_value(v)?, kind_of_type(t)?);
    match super::verdict(Family::CvVariant, src, dst)? {
        Verdict::Error(code, text) => raise::<()>(text, code).err(),
        _ => None,
    }
}

/// A Variant holding a value of a kind whose conversion into `t` the matrix pins only from a Variant (a `CvVariant` cell that
/// gives values): `None` when the cell is not covered or not one of these (LT Y31 Cv Variant, Y31ConvCv.al).
pub fn variant_convert(v: &Value, t: &RtType) -> Option<Result<Value, Signal>> {
    let (src, dst) = (kind_of_value(v)?, kind_of_type(t)?);
    if matches!(super::verdict(Family::CvVariant, src, dst)?, Verdict::Error(..)) {
        return None;
    }
    match (v, t) {
        // false and true are 0 and 1 in a number (`Variant_To_Integer` boolF, `Variant_To_Duration` boolT, `Variant_To_Option`).
        (Value::Boolean(b), RtType::Decimal) => Some(Ok(Value::Decimal(Decimal::from(u8::from(*b))))),
        (Value::Boolean(b), _) => {
            let (to, min, max) = target(t)?;
            Some(cast(Decimal::from(u8::from(*b)), "System.Int32", to, min, max, t))
        }
        // A number into a Boolean: one that is not zero is true, a Decimal with a fraction raises as into an Integer
        // (`Variant_To_Boolean`: 3, 300, 2.0, Char 'A', Byte 65, option B, enum C, Duration 65; 1.5). Zero is not tried.
        (_, RtType::Boolean) if !matches!(v, Value::Text(_) | Value::Code(_)) => {
            let (d, _) = number(v).or_else(|| match v {
                Value::Enum(_, n) => Some((Decimal::from(*n), "System.Int32")),
                _ => None,
            })?;
            if d.fract() != Decimal::ZERO {
                return Some(cast_error(
                    d,
                    "Microsoft.Dynamics.Nav.Runtime.Decimal18",
                    "System.Int32",
                ));
            }
            (!d.is_zero()).then_some(Ok(Value::Boolean(true)))
        }
        // An Enum is its ordinal (`Variant_To_BigInteger` enumC 2, `Variant_To_Char`, `Variant_To_Duration`).
        (Value::Enum(_, n), _) => {
            let (to, min, max) = target(t)?;
            Some(cast(Decimal::from(*n), "System.Int32", to, min, max, t))
        }
        // A Text or Code of digits into a number is the number, one of letters raises (`Variant_To_Integer`, `_BigInteger`,
        // `_Char`, `_Byte`, `_Option`, `_Boolean`, `_Duration`: '12' and 'abc', 'abcdefghijk', Code 'ABC'). Other texts are not tried.
        (Value::Text(s) | Value::Code(s), _) => text_into_number(s.as_str(), t),
        // A Byte into a Text or Code is its character (`Variant_To_Text10` byte65 `A`).
        (Value::Byte(b), RtType::Text(_) | RtType::Code(_)) => {
            Some(byte_as_char(*b).and_then(|c| crate::ops::convert(Value::Char(c), t)))
        }
        // A DateTime into a Date is its date (`Variant_To_Date_2` dt: CreateDateTime(20260131D, 120000T) is 2026-01-31), as
        // DT2Date gives it; the undefined DateTime is not tried.
        (Value::DateTime(n), RtType::Date) if *n > 0 => {
            Some(Ok(Value::Date(((n - 1).div_euclid(86_400_000) + 1) as i32, false)))
        }
        _ => None,
    }
}

/// A text a Variant holds into a number target (see `variant_convert`).
fn text_into_number(s: &str, t: &RtType) -> Option<Result<Value, Signal>> {
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        let n: i64 = s.parse().ok()?;
        return match t {
            RtType::Boolean if n != 0 => Some(Ok(Value::Boolean(true))),
            RtType::Boolean => None,
            RtType::Decimal => Some(Ok(Value::Decimal(n.into()))),
            _ => {
                let (to, min, max) = target(t)?;
                // A number beyond the target is not tried.
                (i128::from(n) >= min && i128::from(n) <= max)
                    .then(|| cast(Decimal::from(n), "System.Int64", to, min, max, t))
            }
        };
    }
    if s.len() < 2 || !s.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let name = match t {
        RtType::Integer | RtType::Option(_) => "Integer",
        RtType::BigInteger => "BigInteger",
        RtType::Char => "Char",
        RtType::Byte => "Byte",
        RtType::Boolean => "Boolean",
        RtType::Duration => {
            return Some(raise(
                format!("The input string '{s}' was not in a correct format."),
                "AL:Format",
            ));
        }
        _ => return None,
    };
    Some(raise(
        format!("The expression Variant cannot be type-converted to a {name} value."),
        "TypeConversion:Format",
    ))
}
