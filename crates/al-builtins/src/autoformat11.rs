//! `Format`, StrSubstNo, Message and Error `%n` in code of a Decimal table field with AutoFormatType 11
//! (format-evaluate/Y43AfType11.al, corpus PR 966, en-US, BC 27.5, 28.5, 29.0 and SaaS): the
//! AutoFormatExpression, evaluated at each call, is the format string of `Format(Value, 0, Expression)`. Pinned:
//! - format number 0 without a length: a literal (`Lit11`), a procedure of a global record of a table extension (`Ext11`,
//!   once per call: `Ext11_CallCount`), a procedure of the row (`Own11`) and a string without a Precision code (`Ext11Int`);
//! - a blank expression formats as a plain Decimal (`Ext11Blank`);
//! - format numbers 1, 2 and 9 of `<Precision,a:b><Standard Format,0>`: the same text without the thousands separator
//!   (`Ext11_Format`);
//! - format number 9 of `<precision, a:b><standard format, 0>` the same way (`7.00` for 7 with 2:2, `8` for 7.5 with 0:0), and of a
//!   blank expression as a plain Decimal (`7`) (pages/controls/AutoFormatProcedureControl.al:260, :279, :298, the `close-edit`
//!   step that formats the stored field with `Format(Amount, 0, 9)`, :215).
//!
//! A length, any other format number and the format numbers 1, 2 and 9 of any other expression are refused.
//!
//! `Format` in code of an AutoFormatType 11 field with the format string its expression gave.

use crate::native::{Ctx, Signal, lookup};
use crate::value::Value;

fn refuse<T>(why: String) -> Result<T, Signal> {
    Err(Signal::unsupported(why))
}

/// `args`: the value, length, format number (the arguments of `System.FormatAutoFormatField#1`); `expr`: the format
/// string the field's AutoFormatExpression gave.
pub(crate) fn format(ctx: &mut dyn Ctx, args: &[Value], expr: &str) -> Result<Value, Signal> {
    let int = |i: usize| match args.get(i) {
        None => Some(0),
        Some(Value::Integer(n)) => Some(*n),
        Some(_) => None,
    };
    let (Some(0), Some(number)) = (int(1), int(2)) else {
        return refuse(format!(
            "Format of an AutoFormatType 11 field with a length or a format that is no number (the format string `{expr}`)"
        ));
    };
    let run = |ctx: &mut dyn Ctx, id: &str, mut a: Vec<Value>| {
        let f = lookup(id).ok_or_else(|| Signal::unsupported(format!("{id} not registered")))?;
        f(ctx, None, &mut a)
    };
    if expr.trim().is_empty() {
        // Format number 9 of a blank expression: AutoFormatProcedureControl.al:279 `close-edit` `7`.
        if number == 9 {
            return run(
                ctx,
                "System.Format#1",
                vec![args[0].clone(), Value::Integer(0), Value::Integer(9)],
            );
        }
        if number != 0 {
            return refuse(format!(
                "Format of an AutoFormatType 11 field whose expression is blank with the format number {number}"
            ));
        }
        return run(ctx, "System.Format#1", vec![args[0].clone()]);
    }
    let text = run(
        ctx,
        "System.Format#2",
        vec![args[0].clone(), Value::Integer(0), Value::Text(expr.to_string().into())],
    )?;
    // Numbers 1, 2 and 9 drop the thousands separator of the measured `<Precision,a:b><Standard Format,0>` forms
    // (format-evaluate/Y43AfType11.al:187-206; the lower-case spaced spelling for 9, format-evaluate/P144FormatCodes.al:111-113).
    match number {
        0 => Ok(text),
        1 | 2 | 9 if precision_standard_0(expr) || (number == 9 && lower_spaced_precision_standard_0(expr)) => {
            match text {
                Value::Text(t) => Ok(Value::Text(t.as_str().replace(',', "").into())),
                other => Ok(other),
            }
        }
        _ => refuse(format!(
            "Format of an AutoFormatType 11 field with the format number {number} and the format string `{expr}`"
        )),
    }
}

/// `<Precision,a:b><Standard Format,0>` with digits `a` and `b`.
fn precision_standard_0(expr: &str) -> bool {
    digits_pair(
        expr.strip_prefix("<Precision,")
            .and_then(|r| r.strip_suffix("><Standard Format,0>")),
    )
}

/// `<precision, a:b><standard format, 0>` with digits `a` and `b`, the spelling AutoFormatProcedureControl.al:260 and :298 measure.
fn lower_spaced_precision_standard_0(expr: &str) -> bool {
    digits_pair(
        expr.strip_prefix("<precision, ")
            .and_then(|r| r.strip_suffix("><standard format, 0>")),
    )
}

/// `a:b` with digits `a` and `b`.
fn digits_pair(p: Option<&str>) -> bool {
    p.and_then(|p| p.split_once(':')).is_some_and(|(a, b)| {
        [a, b]
            .iter()
            .all(|x| !x.is_empty() && x.bytes().all(|c| c.is_ascii_digit()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_spelling_is_matched_only_as_measured() {
        assert!(precision_standard_0("<Precision,2:2><Standard Format,0>"));
        assert!(!precision_standard_0("<precision, 2:2><standard format, 0>"));
        assert!(lower_spaced_precision_standard_0(
            "<precision, 0:0><standard format, 0>"
        ));
        assert!(!lower_spaced_precision_standard_0("<precision,2:2><standard format,0>"));
        assert!(!lower_spaced_precision_standard_0(
            "<precision, 2:2><standard format, 1>"
        ));
    }
}
