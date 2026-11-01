//! `Format`, StrSubstNo, Message and Error `%n` in code of a Decimal table field with AutoFormatType 11
//! (types/lt-y43-auto-format-11/Y43AfType11.al, corpus PR 966, en-US, BC 27.5, 28.5, 29.0 and SaaS): the
//! AutoFormatExpression, evaluated at each call, is the format string of `Format(Value, 0, Expression)`. Pinned:
//! - format number 0 without a length: a literal (`Lit11`), a procedure of a global record of a table extension (`Ext11`,
//!   once per call: `Ext11_CallCount`), a procedure of the row (`Own11`) and a string without a Precision code (`Ext11Int`);
//! - a blank expression formats as a plain Decimal (`Ext11Blank`);
//! - format numbers 1, 2 and 9 of `<Precision,a:b><Standard Format,0>`: the same text without the thousands separator
//!   (`Ext11_Format`).
//!
//! A length, any other format number and the format numbers 1, 2 and 9 of any other expression are refused.

use crate::native::{Ctx, Signal, lookup};
use crate::value::Value;

fn refuse<T>(why: String) -> Result<T, Signal> {
    Err(Signal::Unsupported(why))
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
        let f = lookup(id).ok_or_else(|| Signal::Unsupported(format!("{id} not registered")))?;
        f(ctx, None, &mut a)
    };
    if expr.trim().is_empty() {
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
    match number {
        0 => Ok(text),
        1 | 2 | 9 if precision_standard_0(expr) => match text {
            Value::Text(t) => Ok(Value::Text(t.as_str().replace(',', "").into())),
            other => Ok(other),
        },
        _ => refuse(format!(
            "Format of an AutoFormatType 11 field with the format number {number} and the format string `{expr}`"
        )),
    }
}

/// `<Precision,a:b><Standard Format,0>` with digits `a` and `b`.
fn precision_standard_0(expr: &str) -> bool {
    expr.strip_prefix("<Precision,")
        .and_then(|r| r.strip_suffix("><Standard Format,0>"))
        .and_then(|p| p.split_once(':'))
        .is_some_and(|(a, b)| {
            [a, b]
                .iter()
                .all(|x| !x.is_empty() && x.bytes().all(|c| c.is_ascii_digit()))
        })
}
