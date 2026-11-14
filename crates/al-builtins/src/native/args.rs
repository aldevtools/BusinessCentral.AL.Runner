use super::*;

pub(super) fn vm_only(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    unsupported("a builtin the VM runs itself")
}

pub(super) fn int_arg(args: &[Value], i: usize) -> Option<i64> {
    match args.get(i)? {
        Value::Integer(n) => Some(i64::from(*n)),
        Value::BigInteger(n) => Some(*n),
        Value::Byte(n) => Some(i64::from(*n)),
        Value::Char(n) => Some(i64::from(*n)),
        // An Option takes the place of an Integer (text/OptCopyStr_risky.al: CopyStr('abcdef', Opt)).
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
        Value::TextConst(_) => text_of(&crate::conv::text_const_as_text(v.clone())),
        _ => None,
    }
}

/// Like [`text_of`], and a call stack text (it holds `CALL_STACK_MARK`) is carried: for a builtin that only stores or writes its
/// text argument and never looks at its characters, so the mark travels on with the text and every later read of it is refused
/// (al-runner issue 6853; the corpus pins that the call stack text of a collected error is not empty and round-trips through a
/// stream, error-handling/lt-wd/WdCallStackToBlob.al:96-102). Text that carries unread bytes (`CARRIED_TEXT_MARK`) is still refused.
pub fn text_of_carrying_call_stack(v: &Value) -> Option<crate::text::Txt> {
    match v {
        Value::Text(s) | Value::Code(s)
            if s.audited_str().contains(CALL_STACK_MARK) && !s.audited_str().contains(CARRIED_TEXT_MARK) =>
        {
            Some(s.clone())
        }
        // Rendered content reaches a native only where `rendered_text::guard` allows it (ADR-040 amendment 2026-11-12, A5).
        Value::Text(s) | Value::Code(s) if s.has_marker() && crate::rendered_text::carried(s.audited_str()) => {
            Some(s.clone())
        }
        _ => text_of(v),
    }
}

pub(super) fn text_arg(args: &[Value], i: usize) -> Result<crate::text::Txt, Signal> {
    // A Guid where a Text is expected is its text form, as in an assignment (types/guid/
    // GuidAsTextArgument.al: CopyStr, StrLen, UpperCase, StrPos, Notification.SetData and more; record/
    // Y70Probes.al:264-343: EndsWith, LastIndexOf, Split, TextBuilder, Trim*, IndexOfAny, Replace, StrSubstNo).
    if let Some(Value::Guid(g)) = args.get(i) {
        return Ok(crate::format::format_guid(g).into());
    }
    args.get(i).and_then(text_of).map_or_else(
        || unsupported(format!("text argument {i} of type {}", args.get(i).describe())),
        Ok,
    )
}
