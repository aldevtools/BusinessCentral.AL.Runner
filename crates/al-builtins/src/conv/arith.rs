//! `+` and `-` of the arithmetic cells of the run matrix whose operand converts in a way the general arithmetic does not
//! know (LT Y31 Ar, Y31ConvAr.al).

use super::{Family, Kind};
use crate::native::Signal;
use crate::ops::{Arith, int_arith};
use crate::value::Value;
use rust_decimal::Decimal;

/// `l op r` when the matrix pins a conversion of an operand the general arithmetic of `ops::arith` lacks; `None` leaves it
/// there.
pub fn arith_cell(op: Arith, l: &Value, r: &Value) -> Option<Result<Value, Signal>> {
    let family = match op {
        Arith::Add => Family::Add,
        Arith::Sub => Family::Sub,
        _ => return None,
    };
    match (l, r) {
        // A Byte next to a Char is the character of code page 437 (Char_Byte: 'A' + 255 is 225, 'A' - 255 is -95), an Integer.
        (Value::Char(c), Value::Byte(b)) => {
            super::verdict(family, Kind::Char, Kind::Byte)?;
            Some(
                super::engine::byte_as_char(*b)
                    .and_then(|b| int_arith(op, i32::from(*c), i32::from(b)).map(Value::Integer)),
            )
        }
        // `B += C` with a Byte target: the same sum (Char_Byte `compound_add`: Byte 255 += 'A' is 225, shown `ß`).
        (Value::Byte(b), Value::Char(c)) if op == Arith::Add => {
            super::verdict(Family::AddAssign, Kind::Char, Kind::Byte)?;
            Some(
                super::engine::byte_as_char(*b)
                    .and_then(|b| int_arith(op, i32::from(b), i32::from(*c)).map(Value::Integer)),
            )
        }
        // An Option or Enum with an Option or Enum gives an Option of the ordinals, with no member names (Option_Option_Same,
        // Option_Enum, Enum_Option, Enum_Enum_Same: `Option|2`, shown as its number).
        (Value::Option(a, _) | Value::Enum(_, a), Value::Option(b, _) | Value::Enum(_, b)) => {
            let kind = |v: &Value| {
                if matches!(v, Value::Option(..)) {
                    Kind::Option
                } else {
                    Kind::Enum
                }
            };
            super::verdict(family, kind(l), kind(r))?;
            Some(int_arith(op, *a, *b).map(|n| Value::Option(n, std::sync::Arc::from(Vec::<String>::new()))))
        }
        _ => general(op, family, l, r),
    }
}

/// The other covered cells: a Char, Byte, Enum or Option counts as its number, a Duration as its milliseconds (days next to a
/// Date: Date_Duration + 65 ms is 65 days; a Decimal next to it: Decimal_Duration 1.5 - 65 is the Decimal -63.5), and a Guid
/// joins a Char or a Guid as its text (Guid_Char, Guid_Guid_Same). Date_Enum, Date_Char, Time_Byte, DateTime_Char, Char_Enum,
/// Enum_Byte, Duration_Char, Duration_Option.
fn general(op: Arith, family: Family, l: &Value, r: &Value) -> Option<Result<Value, Signal>> {
    // A Char next to a text joins it as a character (`ops::arith`, Char_Text: 'A' + 'a' is 'Aa').
    if matches!(l, Value::Text(_) | Value::Code(_)) || matches!(r, Value::Text(_) | Value::Code(_)) {
        return None;
    }
    let as_number = |v: &Value, other: &Value| -> Option<Value> {
        Some(match v {
            Value::Char(c) => Value::Integer(i32::from(*c)),
            Value::Byte(b) => Value::Integer(i32::from(*b)),
            Value::Enum(_, n) | Value::Option(n, _) => Value::Integer(*n),
            Value::Duration(ms) if matches!(other, Value::Date(..)) => Value::Integer(i32::try_from(*ms).ok()?),
            Value::Duration(ms) if matches!(other, Value::Decimal(_)) => Value::Decimal((*ms).into()),
            Value::Guid(g) if op == Arith::Add => Value::Text(crate::format::format_guid(g).into()),
            _ => return None,
        })
    };
    let (nl, nr) = (as_number(l, r), as_number(r, l));
    if nl.is_none() && nr.is_none() {
        return None;
    }
    let (kl, kr) = (super::engine::kind_of_value(l)?, super::engine::kind_of_value(r)?);
    super::verdict(family, kl, kr)?;
    // A Char next to a Guid is a character of the text.
    let text = |v: Option<Value>, orig: &Value| match (v, orig) {
        (_, Value::Char(c)) if matches!(l, Value::Guid(_)) || matches!(r, Value::Guid(_)) => {
            char::from_u32(u32::from(*c)).map(|ch| Value::Text(ch.to_string().into()))
        }
        (Some(v), _) => Some(v),
        (None, o) => Some(o.clone()),
    };
    let (l2, r2) = (text(nl, l)?, text(nr, r)?);
    Some(crate::ops::arith(op, &l2, &r2))
}

/// DateTime +/- a Decimal with a fraction: the conversion error names the amount added, a subtraction the negated one
/// (types/lt-y31-conversion-matrix/Y31ConvAr.al, LT Y31 Ar DateTime `DateTime_Decimal`: DT - 1.5 names -1.5).
pub(crate) fn datetime_fraction_error(op: Arith, d: Decimal) -> Result<Value, Signal> {
    let shown = if op == Arith::Sub { -d } else { d };
    crate::native::raise(
        format!(
            "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value {} to System.Int64.",
            shown.normalize()
        ),
        "ConversionCast",
    )
}
