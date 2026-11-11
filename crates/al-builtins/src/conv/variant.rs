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

/// Added by the lowering to the code of `Variant.Into#1` at an assignment `Target := V`: the rules measured only there (an
/// unassigned Variant, the record slots, the flag's text) apply to it and to no argument or `exit`.
pub const AT_ASSIGNMENT: i32 = 1 << 12;

/// The code of `Variant.Into#1` for a Variant used as an AL condition (`if V then`, `while`, `until`, `not V`).
pub const AS_CONDITION: i32 = 1 << 13;

fn into(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let v = a.first().cloned().unwrap_or(Value::Null);
    let code = match a.get(1) {
        Some(Value::Integer(c)) => *c,
        _ => return Ok(v),
    };
    if code == AS_CONDITION {
        return as_condition(v);
    }
    if code & AT_ASSIGNMENT != 0 {
        return assigned(v, code & !AT_ASSIGNMENT);
    }
    match target_of(code) {
        Some(t) => variant_into(v, &t),
        None => Ok(v),
    }
}

/// `Target := V`, the slot named by `code` (a matrix kind or a record slot of `variant_records`).
fn assigned(v: Value, code: i32) -> Result<Value, Signal> {
    if matches!(v, Value::Null) {
        return unassigned_into(code);
    }
    let Some(t) = target_of(code) else {
        return super::variant_records::into_record_slot(v, code);
    };
    // A Variant holding a text of letters into an Integer: GetLastErrorText(true) leaves the text
    // (errors/last-error/LastErrorFlagRuntimeCodes.al:233-234, :242). The empty text is not measured.
    if matches!(t, RtType::Integer)
        && matches!(&v, Value::Text(s) if !s.as_str().is_empty())
        && let Err(Signal::Error(e)) = crate::ops::convert(v.clone(), &t)
        && e.code == "TypeConversion:Format"
    {
        return crate::native::with_origin(Err(Signal::Error(e)), crate::native::ErrorOrigin::SameText);
    }
    variant_into(v, &t)
}

/// An unassigned Variant assigned to a slot (types/variant/VaTConvert.al:EmptyVariant_To_Types, :284-305;
/// VariantRecordIdConversions.al:UnassignedVariant_Uses, :267-319; recordref-fieldref/recordref/GetTableVariantKinds.al:176-191
/// for a Record): an Integer, Decimal, Boolean or Record raises
/// NotInitialized; a Text and a Date take it without an error (the empty text, VariantRecordIdConversions.al `text`; the Date
/// as before, its value not observed). Other slots: not measured.
fn unassigned_into(code: i32) -> Result<Value, Signal> {
    if code == super::variant_records::RECORD {
        return Err(crate::xml::uninitialized("Variant"));
    }
    match target_of(code) {
        Some(RtType::Integer | RtType::Decimal | RtType::Boolean) => Err(crate::xml::uninitialized("Variant")),
        Some(RtType::Text(_) | RtType::Date) => Ok(Value::Null),
        Some(t) => Err(Signal::unsupported(format!(
            "an unassigned Variant into {} (not pinned)",
            t.describe()
        ))),
        None => Err(Signal::unsupported(
            "an unassigned Variant into a RecordId or RecordRef (not pinned)",
        )),
    }
}

/// A Variant used as an AL condition (types/variant/VariantRecordIdConversions.al:VariantAsCondition, :238-265): unassigned
/// raises NotInitialized, a Boolean is itself, the Integer 1 or 0 is true or false, the text 'true' or '' raises what `B := V`
/// raises (`variant_into`; VaTConvert.al:69-71). Any other value is not measured.
fn as_condition(v: Value) -> Result<Value, Signal> {
    match v {
        Value::Null => Err(crate::xml::uninitialized("Variant")),
        Value::Boolean(_) => Ok(v),
        Value::Integer(n @ (0 | 1)) => Ok(Value::Boolean(n == 1)),
        Value::Text(ref t) if matches!(t.as_str(), "true" | "") => variant_into(v, &RtType::Boolean),
        other => Err(Signal::unsupported(format!(
            "a Variant holding {} used as a condition (not pinned)",
            other.describe()
        ))),
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
