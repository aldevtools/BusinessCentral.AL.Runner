use super::*;

pub(super) fn vm_only(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    unsupported("a builtin the VM runs itself")
}

pub(super) fn unsupported<T>(what: &str) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.to_string()))
}

pub(super) fn int_arg(args: &[Value], i: usize) -> Option<i64> {
    match args.get(i)? {
        Value::Integer(n) => Some(i64::from(*n)),
        Value::BigInteger(n) => Some(*n),
        Value::Byte(n) => Some(i64::from(*n)),
        Value::Char(n) => Some(i64::from(*n)),
        // An Option takes the place of an Integer (record/lt-record-conversion-assumptions/OptCopyStr_risky.al: CopyStr('abcdef', Opt)).
        Value::Option(n, _) => Some(i64::from(*n)),
        _ => None,
    }
}

/// Text of a Text/Code/Char argument (Text parameters accept all three). Text that holds the
/// untracked call stack marker is refused, so its content is never silently wrong.
pub fn text_of(v: &Value) -> Option<crate::text::Txt> {
    match v {
        Value::Text(s) | Value::Code(s) if s.has_marker() => None,
        Value::Text(s) | Value::Code(s) => Some(s.clone()),
        Value::Char(c) => char::from_u32(u32::from(*c)).map(|c| c.to_string().into()),
        _ => None,
    }
}

pub(super) fn text_arg(args: &[Value], i: usize) -> Result<crate::text::Txt, Signal> {
    // A Guid where a Text is expected is its text form, as in an assignment (`CopyStr(CreateGuid(), 1, 50)` in
    // Microsoft's Match General Jnl Lines UT, 61 tests; unprobed, docs/probes-needed.md "A Guid assigned to a Text
    // or Code"; hard rule 3 exception, owner 2026-10-15).
    if let Some(Value::Guid(g)) = args.get(i) {
        crate::assumptions::assume(crate::assumptions::Assumption::GuidAsTextArgument);
        return Ok(crate::format::format_guid(g).into());
    }
    args.get(i).and_then(text_of).map_or_else(
        || unsupported(&format!("text argument {i} of type {}", args.get(i).describe())),
        Ok,
    )
}
