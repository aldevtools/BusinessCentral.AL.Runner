//! `=`, `<>` and `<` of the comparison cells of the run matrix whose operand converts in a way the general comparison does
//! not know (LT Y31 Cmp, Y31ConvCmp.al).

use super::{Family, Kind};
use crate::native::Signal;
use crate::value::Value;
use std::cmp::Ordering;

/// `l` against `r` when the matrix pins a conversion of an operand the general comparison of `ops::compare_operator` lacks;
/// `None` leaves it there. `ordered` is `<` and its kin, else `=` and `<>`.
pub fn compare_cell(l: &Value, r: &Value, ordered: bool) -> Option<Result<Ordering, Signal>> {
    let family = if ordered { Family::CmpLt } else { Family::CmpEq };
    let text_kind = |v: &Value| match v {
        Value::Text(_) => Some(Kind::Text),
        Value::Code(_) => Some(Kind::Code),
        _ => None,
    };
    match (l, r) {
        // A Char next to a Text or Code is a text of that one character, compared as texts are: 'a' < 'A' and 'A' < 'abc' in
        // the culture order, Code 'A' = 'a' false (Text_Char, Char_Text, Code10_Char).
        (Value::Text(_) | Value::Code(_), Value::Char(c)) => {
            super::verdict(family, text_kind(l)?, Kind::Char)?;
            Some(crate::ops::compare_operator(l, &char_text(*c)?, ordered))
        }
        (Value::Char(c), Value::Text(_) | Value::Code(_)) => {
            super::verdict(family, Kind::Char, text_kind(r)?)?;
            Some(crate::ops::compare_operator(&char_text(*c)?, r, ordered))
        }
        // `<` of a Text or Code and a Guid reads neither: a Text or Code first is never less, a Guid first always is, a text that
        // is no GUID included (Text_Guid, Code10_Guid, Label_Guid, Guid_Text, Guid_Code10: `lt` for 'a', 'abc', a GUID text).
        (Value::Text(_) | Value::Code(_), Value::Guid(_)) if ordered => {
            super::verdict(family, text_kind(l)?, Kind::Guid)?;
            Some(Ok(Ordering::Greater))
        }
        (Value::Guid(_), Value::Text(_) | Value::Code(_)) if ordered => {
            super::verdict(family, Kind::Guid, text_kind(r)?)?;
            Some(Ok(Ordering::Less))
        }
        // A Char next to a Guid is a text of that one character (Char_Guid, Guid_Char: `=` reads the text as a GUID and raises,
        // `<` as for a Text).
        (Value::Char(c), Value::Guid(_)) | (Value::Guid(_), Value::Char(c)) => {
            let k = |v: &Value| {
                if matches!(v, Value::Char(_)) {
                    Kind::Char
                } else {
                    Kind::Guid
                }
            };
            super::verdict(family, k(l), k(r))?;
            let t = char_text(*c)?;
            let (a, b) = if matches!(l, Value::Char(_)) { (&t, r) } else { (l, &t) };
            Some(crate::ops::compare_operator(a, b, ordered))
        }
        _ => numbers(family, l, r),
    }
}

/// A Char, Byte, Enum or Option compares as its number, a Duration as its milliseconds (Option_Char 1 < 'A', Duration_Char
/// 65 ms = 'A', Enum_Byte B < 65, Integer_Enum 1 < ord5, Decimal_Duration 1.5 < 65 ms).
fn numbers(family: Family, l: &Value, r: &Value) -> Option<Result<Ordering, Signal>> {
    let n = |v: &Value| -> Option<rust_decimal::Decimal> {
        Some(match v {
            Value::Char(c) => (*c).into(),
            Value::Byte(b) => (*b).into(),
            Value::Enum(_, n) | Value::Option(n, _) | Value::Integer(n) => (*n).into(),
            Value::BigInteger(n) | Value::Duration(n) => (*n).into(),
            Value::Decimal(d) => *d,
            _ => return None,
        })
    };
    let special = |v: &Value| {
        matches!(
            v,
            Value::Char(_) | Value::Byte(_) | Value::Enum(..) | Value::Duration(_)
        )
    };
    if !special(l) && !special(r) {
        return None;
    }
    let (kl, kr) = (super::engine::kind_of_value(l)?, super::engine::kind_of_value(r)?);
    super::verdict(family, kl, kr)?;
    Some(Ok(n(l)?.cmp(&n(r)?)))
}

fn char_text(c: u16) -> Option<Value> {
    char::from_u32(u32::from(c)).map(|ch| Value::Text(ch.to_string().into()))
}
