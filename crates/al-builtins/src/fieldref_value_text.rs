//! A `FieldRef.Value()` converted into a Text or Code variable or by-value parameter (al-vm `Op::ConvertFieldValue`).

use crate::native::Signal;
use crate::value::{RtType, Value};

/// A number is its Format text and an Enum its caption ([`crate::ops::number_into_text_field`]); an Option is its caption too,
/// upper-cased into a Code (text/lt-y49-fieldref-into-text/Y49FieldRefIntoText.al:290-327 `opt text250` `[Green Cap]`, `opt code20`
/// `[GREEN CAP]`; into an unbounded Text no conversion runs, misc/lt-cx-odd-leads/CxScalars.al:378-417). `None`: convert as an
/// assignment does.
pub fn fieldref_value_into_text(
    v: &Value,
    t: &RtType,
    ctx: &dyn crate::format::FormatCtx,
) -> Option<Result<Value, Signal>> {
    if let (Value::Option(o, names), RtType::Text(_) | RtType::Code(_)) = (v, t) {
        // The Option a FieldRef hands out carries its captions; an ordinal past them is not pinned.
        let Some(caption) = usize::try_from(*o).ok().and_then(|i| names.get(i)) else {
            return Some(Err(Signal::Unsupported(format!(
                "FieldRef.Value of Option ordinal {o} without a caption into a {}",
                crate::describe::Describe::describe(t)
            ))));
        };
        return Some(crate::ops::convert(Value::Text(caption.as_str().into()), t));
    }
    crate::ops::number_into_text_field(v, t, ctx)
}

/// A Variant given to `FieldRef.Validate` or `FieldRef.Value :=` of a Text or Code field: a number, an Enum, an Option, a Date
/// or a Boolean goes in as its Format text (`42`, `12.5`, `1,234.5`, `3000000000`, `01/15/24`, `Yes`, `B`, `Second`), upper-cased
/// into a Code (`YES`), and a text longer than the field raises StringLengthExceeded (recordref/lt-y47-validate-fieldref/
/// Y47ValidateFieldRef.al:415-570, Y57_*). `None`: convert as an assignment does.
pub fn variant_into_text_field(
    v: &Value,
    t: &RtType,
    ctx: &dyn crate::format::FormatCtx,
) -> Option<Result<Value, Signal>> {
    if !matches!(t, RtType::Text(_) | RtType::Code(_)) {
        return None;
    }
    match v {
        Value::Option(..) => fieldref_value_into_text(v, t, ctx),
        Value::Date(..) | Value::Boolean(_) => {
            let text = match crate::format::try_format(v, 0, 0, ctx) {
                Ok(text) => text,
                Err(e) => return Some(Err(Signal::Unsupported(e))),
            };
            Some(crate::ops::convert(Value::Text(text.into()), t))
        }
        _ => crate::ops::number_into_text_field(v, t, ctx),
    }
}

/// A `FieldRef.Value()` of a field that is no Blob, assigned to a Blob: from BC 28 `The value "<Format>" can't be evaluated into
/// type BLOB.` (Evaluate:Argument), in BC 27 `Invalid Conversion. Source: Nav<Type>, Destination: NavBLOB.` (Argument), for an
/// Integer, Text, Date and Guid field; the Blob keeps no value (types/lt-y71-value-conversions/Y71ValueConversions.al:440-547).
/// Other types are refused. `None` when the target is no Blob or the value is one.
pub fn fieldref_value_into_blob(
    v: &Value,
    t: &RtType,
    ctx: &dyn crate::format::FormatCtx,
    major: u16,
) -> Option<Result<Value, Signal>> {
    if !matches!(t, RtType::Library(n) if n.eq_ignore_ascii_case("Blob")) || matches!(v, Value::Blob(_)) {
        return None;
    }
    let nav = match v {
        Value::Integer(_) => "NavInteger",
        Value::Text(_) => "NavText",
        Value::Date(..) => "NavDate",
        Value::Guid(_) => "NavGuid",
        other => {
            return Some(Err(Signal::Unsupported(format!(
                "FieldRef.Value of {} into a Blob (not pinned)",
                crate::describe::Describe::describe(other)
            ))));
        }
    };
    if major < 28 {
        return Some(crate::native::raise(
            format!("Invalid Conversion. Source: {nav}, Destination: NavBLOB."),
            "Argument",
        ));
    }
    Some(match crate::format::try_format(v, 0, 0, ctx) {
        Ok(text) => crate::native::raise(
            format!("The value \"{text}\" can't be evaluated into type BLOB."),
            "Evaluate:Argument",
        ),
        Err(e) => Err(Signal::Unsupported(e)),
    })
}
