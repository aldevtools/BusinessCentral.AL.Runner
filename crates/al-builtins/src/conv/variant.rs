//! `Variant.Into#1`: the value a Variant holds, on its way into a slot of another type. The lowering puts it around every
//! conversion whose source is a Variant (`al-sema` `lower/conv_matrix.rs`), so the cells the matrix pins only for a Variant
//! (`Family::CvVariant`) apply there and nowhere else: a `FieldRef.Value` or a `TestField` value is no Variant here
//! (recordref-fieldref/fieldref/FieldRefTestFieldWrongType.al, types/CxScalars.al).

use super::Kind;
use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};

use crate::value::{RtType, Value};

pub const REGISTRY: &[(&str, NativeFn)] = &[("Variant.Into#1", into)];

/// The kinds a Variant converts into, by the number the lowering hands over (`kind_code`).
const TARGETS: &[Kind] = &[
    Kind::Integer,
    Kind::BigInteger,
    Kind::Decimal,
    Kind::Char,
    Kind::Byte,
    Kind::Option,
    Kind::Text,
    Kind::Code,
    Kind::Guid,
    Kind::Duration,
    Kind::Boolean,
    Kind::Date,
    Kind::Time,
    Kind::DateTime,
];

/// The number `Variant.Into#1` takes for a target kind, `None` for a kind no Variant cell has.
pub fn kind_code(k: Kind) -> Option<i32> {
    TARGETS.iter().position(|t| *t == k).map(|i| i as i32)
}

fn target_of(code: i32) -> Option<RtType> {
    Some(match TARGETS.get(usize::try_from(code).ok()?)? {
        Kind::Integer => RtType::Integer,
        Kind::BigInteger => RtType::BigInteger,
        Kind::Decimal => RtType::Decimal,
        Kind::Char => RtType::Char,
        Kind::Byte => RtType::Byte,
        Kind::Option => RtType::Option(std::sync::Arc::from(Vec::<String>::new())),
        Kind::Text => RtType::Text(None),
        Kind::Code => RtType::Code(None),
        Kind::Guid => RtType::Guid,
        Kind::Duration => RtType::Duration,
        Kind::Boolean => RtType::Boolean,
        Kind::Date => RtType::Date,
        Kind::Time => RtType::Time,
        Kind::DateTime => RtType::DateTime,
        _ => return None,
    })
}

fn into(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let v = a.first().cloned().unwrap_or(Value::Null);
    let code = match a.get(1) {
        Some(Value::Integer(c)) => *c,
        _ => return Ok(v),
    };
    match target_of(code) {
        Some(t) => variant_into(v, &t),
        None => super::variant_records::into_record_slot(v, code),
    }
}

/// The value `v` a Variant holds, readied for a slot of the kind of `t`: what `ops::convert` refuses and the matrix pins for
/// a Variant is converted here (`variant_convert`) or raises its error (`variant_error`); anything else is handed back for the
/// conversion that follows.
pub fn variant_into(v: Value, t: &RtType) -> Result<Value, Signal> {
    // A TextConst taken out of a Variant is a Text (types/Y81LabelVariant.al, codeunits 94256 to 94258).
    let v = match v {
        Value::TextConst(s) => Value::Text(s),
        other => other,
    };
    // An unassigned Variant into an Integer or a Boolean raises; into a Text it is the empty text
    // (types/variant/VariantRecordIdConversions.al:UnassignedVariant_Uses `integer`, `boolean`, `text`). Other slots: not pinned.
    if matches!(v, Value::Null) {
        return match t {
            RtType::Integer | RtType::Boolean => Err(crate::xml::uninitialized("Variant")),
            RtType::Text(_) => Ok(v),
            _ => Err(Signal::unsupported(format!(
                "an unassigned Variant into {} (not pinned)",
                t.describe()
            ))),
        };
    }
    // A RecordId, Record or RecordRef into a Code, Text, Integer or Guid (`variant_records`).
    if let Some(r) = super::variant_records::record_into_scalar(&v, t) {
        return r;
    }
    // A value of a kind the matrix does not try (a SQL counter, a record, a list) goes on as it is.
    if super::engine::kind_of_value(&v).is_none() {
        return Ok(v);
    }
    if let Some(r) = super::variant_scalars::before_convert(&v, t) {
        return r;
    }
    match crate::ops::convert(v.clone(), t) {
        Err(Signal::Unsupported(why)) => {
            match super::variant_convert(&v, t).or_else(|| super::variant_scalars::convert(&v, t)) {
                Some(r) => r,
                None => match super::variant_error(&v, t) {
                    Some(e) => Err(e),
                    None => Err(Signal::unsupported(why)),
                },
            }
        }
        _ => Ok(v),
    }
}
