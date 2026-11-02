use super::*;

/// `Message(String, Values...)`: the text is substituted like `StrSubstNo`
/// before it reaches the UI (corpus handlers/TestMessageHandler.al
/// Message_FormattedString_HandlerGetsFormatted).
pub(super) fn message(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let mut text = substitute(&text_arg(args, 0)?, &args[1..], ctx)?;
    // A CR LF in the text or in an argument reaches the handler as one backslash (misc/lt-system-functions/ErrorMessage.al:598-616,
    // LT_BU_Message_BackslashAndAppendLine).
    if text.contains("\r\n") {
        text = text.replace("\r\n", "\\");
    }
    ctx.ui_message(text)?;
    Ok(Value::Null)
}

/// `Confirm(String, Default, Values...)`. Values are not substituted into the
/// question a handler sees (corpus handlers/TestConfirmFormatArgs.al). An
/// omitted Default is false (docs/probes-needed.md).
pub(super) fn confirm(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let question = text_arg(args, 0)?;
    let default = match args.get(1) {
        None => false,
        Some(Value::Boolean(b)) => *b,
        Some(other) => return unsupported(&format!("Confirm default {}", other.describe())),
    };
    Ok(Value::Boolean(ctx.ui_confirm(question.to_string(), default)?))
}

/// `StrMenu(Options, Default, Instruction)`. An omitted Default is 1 and an
/// omitted Instruction is empty (docs/probes-needed.md).
pub(super) fn str_menu(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let options = text_arg(args, 0)?;
    let default = match args.get(1) {
        None => 1,
        Some(_) => int_arg(args, 1).map_or_else(|| unsupported("StrMenu default"), |n| Ok(n as i32))?,
    };
    let instruction = match args.get(2) {
        None => String::new(),
        Some(_) => text_arg(args, 2)?.to_string(),
    };
    Ok(Value::Integer(ctx.ui_str_menu(
        options.to_string(),
        default,
        instruction,
    )?))
}

/// `%n` replaced by `Format(arg n)`. A placeholder without an argument stays as written, `%0`, `%10` of one argument and a
/// `%` at the end of the text too; an argument is not scanned again (types/lt-variant/VaTTextConst.al:TextConst_StrSubstNo
/// `one_arg_only`, `no_args`, `pct10`, `pct0`, `pct5_not_given`, `pct_end`, `arg_has_percent`;
/// text/TestTextProcessingContracts.al:StrSubstNo_FewerArgsThanPlaceholders_NoError). `#n` fields are pinned for
/// `StrSubstNo` only ([`substitute_fields`]); here they are refused.
pub fn substitute(template: &str, args: &[Value], ctx: &dyn FormatCtx) -> Result<String, Signal> {
    substitute_with(template, args, ctx, false)
}

/// [`substitute`] that also fills `#n` fields, as `StrSubstNo` does (types/lt-evaluate-format-lasterror-setview/
/// P86StrSubstNo.al:Sub_FieldWidth, text/lt-text-functions/StrSubstNo.Codeunit.al:SubstNoHash): a run of `#`, one digit
/// n and a run of `#` is a field as wide as that run (`#1` 2, `##1##` 5, `#1#1` is `#1#` then `1`). `Format(arg n)` fills
/// it from the left, padded with spaces; a value longer than the field gives `*` over the whole width. A field whose
/// digit is 0 or names no argument, a `#` run without a digit and `# 1` stay as written. Two or more digits are not
/// pinned and are refused.
pub fn substitute_fields(template: &str, args: &[Value], ctx: &dyn FormatCtx) -> Result<String, Signal> {
    substitute_with(template, args, ctx, true)
}

pub(super) fn substitute_with(
    template: &str,
    args: &[Value],
    ctx: &dyn FormatCtx,
    fields: bool,
) -> Result<String, Signal> {
    let b = template.as_bytes();
    if !fields && b.windows(2).any(|w| w[0] == b'#' && w[1].is_ascii_digit()) {
        return unsupported("a #n field in an Error or Message text (pinned for StrSubstNo only)");
    }
    let mut out = String::new();
    // `lit` is the start of the text not yet copied; `i` scans. The markers are ASCII, so every cut is a char boundary.
    let (mut lit, mut i) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'%' => {
                out.push_str(&template[lit..i]);
                let rest = &template[i + 1..];
                // A number that starts with 0 is no placeholder (`%0`, `%01`, `%001` stay as written:
                // types/lt-evaluate-format-lasterror-setview/P86StrSubstNo.al:Sub_Percent t11, t19, t20).
                let digits = if rest.starts_with('0') {
                    0
                } else {
                    rest.bytes().take_while(u8::is_ascii_digit).count()
                };
                let n: usize = rest[..digits].parse().unwrap_or(0);
                match args.get(n.wrapping_sub(1)).filter(|_| digits > 0 && n >= 1) {
                    Some(v) => out.push_str(&try_format(v, 0, 0, ctx).map_err(Signal::Unsupported)?),
                    None => {
                        out.push('%');
                        out.push_str(&rest[..digits]);
                    }
                }
                i += 1 + digits;
                lit = i;
            }
            b'#' if fields => {
                let start = i;
                while i < b.len() && b[i] == b'#' {
                    i += 1;
                }
                let ds = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                if i == ds {
                    continue;
                }
                if i - ds > 1 {
                    return unsupported("StrSubstNo with a #n field of more than one digit (not pinned)");
                }
                let n = usize::from(b[ds] - b'0');
                while i < b.len() && b[i] == b'#' {
                    i += 1;
                }
                let Some(v) = args.get(n.wrapping_sub(1)).filter(|_| n >= 1) else {
                    continue;
                };
                out.push_str(&template[lit..start]);
                let width = i - start;
                let text = try_format(v, 0, 0, ctx).map_err(Signal::Unsupported)?;
                let len = text.encode_utf16().count();
                if len > width {
                    out.extend(std::iter::repeat_n('*', width));
                } else {
                    out.push_str(&text);
                    out.extend(std::iter::repeat_n(' ', width - len));
                }
                lit = i;
            }
            _ => i += 1,
        }
    }
    out.push_str(&template[lit..]);
    Ok(out)
}
