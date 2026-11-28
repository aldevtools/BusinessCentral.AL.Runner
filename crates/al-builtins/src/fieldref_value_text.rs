//! A `FieldRef.Value()` converted into a Text or Code variable or by-value parameter (al-vm `Op::ConvertFieldValue`).

use crate::native::Signal;
use crate::value::{RtType, Value};

/// A number is its Format text and an Enum its caption ([`crate::ops::number_into_text_field`]); an Option is its caption too,
/// upper-cased into a Code (text/text-methods/Y49FieldRefIntoText.al:290-327 `opt text250` `[Green Cap]`, `opt code20`
/// `[GREEN CAP]`; into an unbounded Text no conversion runs, types/CxScalars.al:378-417). `None`: convert as an
/// assignment does.
pub fn fieldref_value_into_text(
    v: &Value,
    t: &RtType,
    ctx: &dyn crate::format::FormatCtx,
) -> Option<Result<Value, Signal>> {
    if let (Value::Option(o, names), RtType::Text(_) | RtType::Code(_)) = (v, t) {
        // The Option a FieldRef hands out carries its captions; an ordinal past them is not pinned.
        let Some(caption) = usize::try_from(*o).ok().and_then(|i| names.get(i)) else {
            return Some(Err(Signal::unsupported(format!(
                "FieldRef.Value of Option ordinal {o} without a caption into a {}",
                crate::describe::Describe::describe(t)
            ))));
        };
        return Some(crate::ops::convert(Value::Text(caption.as_str().into()), t));
    }
    crate::ops::number_into_text_field(v, t, ctx)
}

/// `exit(FR.Value())` from a procedure that returns a Text or Code (al-vm `Op::ConvertFieldValue` at an exit): an
/// Integer, BigInteger (the timestamp too), Decimal, Enum, Option, Boolean, Date or Duration is the value's Format
/// text, as FieldRef.Validate converts it ([`variant_into_text_field`]): upper-cased into a Code, an Option's
/// OptionCaption, an Enum member's Caption or else its name, a Decimal with its thousands separator, and
/// StringLengthExceeded past the length of a Code or a bounded Text (issues 9285 and 9305). Corpus PR 1629, merged at
/// 778b8fc1, not yet pinned: recordref-fieldref/fieldref-value-convert/FvcTimestampField.al:443 ExitValue_NumberFields,
/// :466 ExitValue_EnumOptionFields, :485 ExitValue_OtherFields, :214 Field0_InsertedRow; and the follow-up corpus
/// probe, FvcExitValueMore.al:142-280 ExitValue_* (Decimals of 1000 or more, an OptionCaption, an Enum member without a
/// Caption, Text[3] and Text[30]). `unmeasured`: the fact of the session no cell covers (another language, a major the
/// corpus does not cover); there a number or an Enum refuses, since BC gives a text where the Variant conversion
/// raises. `None`: another value or target, which converts as every exit value does.
pub fn fieldref_value_at_exit(
    v: &Value,
    t: &RtType,
    ctx: &dyn crate::format::FormatCtx,
    unmeasured: Option<&str>,
) -> Option<Result<Value, Signal>> {
    if !matches!(t, RtType::Text(_) | RtType::Code(_)) {
        return None;
    }
    if let Some(fact) = unmeasured {
        let number = matches!(
            v,
            Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) | Value::Enum(..)
        );
        return number.then(|| {
            Err(Signal::unsupported(format!(
                "exit(FieldRef.Value()) of {} into a {} {fact} (not measured, issue 9285)",
                crate::describe::Describe::describe(v),
                crate::describe::Describe::describe(t)
            )))
        });
    }
    let measured = matches!(
        v,
        Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Enum(..)
            | Value::Option(..)
            | Value::Boolean(_)
            | Value::Date(..)
            | Value::Duration(_)
    );
    if !measured {
        return None;
    }
    crate::assumptions::assume(crate::assumptions::Assumption::FieldRefValueExitIntoText);
    variant_into_text_field(v, t, ctx)
}

/// A Variant given to `FieldRef.Validate` or `FieldRef.Value :=` of a Text or Code field: a number, an Enum, an Option, a Date
/// or a Boolean goes in as its Format text (`42`, `12.5`, `1,234.5`, `3000000000`, `01/15/24`, `Yes`, `B`, `Second`), upper-cased
/// into a Code (`YES`), and a text longer than the field raises StringLengthExceeded (recordref-fieldref/recordref/
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
        // A Duration too: `1 second 500 milliseconds`, which a Code[20] field cannot hold (recordref-fieldref/fieldref/
        // Y67FieldRefValidateVariant.al:303-336 Validate_Into_Txt_B, _Cod_B `duration 1500`).
        Value::Date(..) | Value::Boolean(_) | Value::Duration(_) => {
            let text = match crate::format::try_format(v, 0, 0, ctx) {
                Ok(text) => text,
                Err(e) => return Some(Err(Signal::unsupported(e))),
            };
            Some(crate::ops::convert(Value::Text(text.into()), t))
        }
        _ => crate::ops::number_into_text_field(v, t, ctx),
    }
}

/// A `FieldRef.Value()` of a field that is no Blob, assigned to a Blob: from BC 28 `The value "<Format>" can't be evaluated into
/// type BLOB.` (Evaluate:Argument), in BC 27 `Invalid Conversion. Source: Nav<Type>, Destination: NavBLOB.` (Argument), for an
/// Integer, Text, Date and Guid field; the Blob keeps no value (types/operators-expressions/Y71ValueConversions.al:440-547).
/// Other types are refused. `None` when the target is no Blob or the value is one.
pub fn fieldref_value_into_blob(
    v: &Value,
    t: &RtType,
    ctx: &dyn crate::format::FormatCtx,
    evaluate_error: bool,
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
            return Some(Err(Signal::unsupported(format!(
                "FieldRef.Value of {} into a Blob (not pinned)",
                crate::describe::Describe::describe(other)
            ))));
        }
    };
    // `evaluate_error`: the gate FieldRefTextNumberEvaluateErrorFrom28.
    if !evaluate_error {
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
        Err(e) => Err(Signal::unsupported(e)),
    })
}
