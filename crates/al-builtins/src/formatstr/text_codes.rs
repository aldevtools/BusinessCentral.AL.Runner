//! Format strings on a Text, a Code, a Boolean, an Option, an Enum, a Char and a Guid. Pinned by corpus master
//! format-evaluate/format-matrix/FMText.al, FMCode.al, FMBoolean.al, FMOption.al, FMEnum.al, FMChar.al and FMGuid.al (corpus PR
//! 1256; al-runner issue 7470), English and German alike:
//! - On a Text, a Boolean, an Option and an Enum, `<Text>` is the value's text (`Format(V)`: `Yes`, `Nein`, `Draft`, a caption,
//!   `99` for an ordinal no member has) and literal text around it stays; `<Text,N>` is that text cut to N characters or filled on
//!   the right to N with blanks, or with the character of a `<Filler Character,c>` before it. On a Code the fill goes on the left
//!   (`       ABC`); a Code longer than N is not pinned.
//! - `<Integer>`, `<Integer,5>`, `<Sign>`, `<Decimals>`, `<Day>` and `<Hours24>` are the invalid-format error on all of these types.
//! - On a Char and a Guid every one of those codes and the `<Text>` codes are the invalid-format error.
//! - `<Precision,2:2>` alone writes nothing, except on a Guid, which asks for the standard format number -1.
//!
//! Other codes are not pinned on these types: `None`, and the caller reads the string as before.

use super::{INVALID_FORMAT_MARK, RAISE_MARK, fail};
use crate::format::{FormatCtx, format_value};
use crate::value::Value;

const INVALID_CODES: [&str; 6] = ["Integer", "Integer,5", "Sign", "Decimals", "Day", "Hours24"];

pub(super) fn format(
    v: &Value,
    template: &str,
    pieces: &[(bool, &str)],
    ctx: &dyn FormatCtx,
) -> Option<Result<String, String>> {
    let text_like = matches!(
        v,
        Value::Text(_) | Value::TextConst(_) | Value::Code(_) | Value::Boolean(_) | Value::Option(..) | Value::Enum(..)
    );
    let no_text = matches!(v, Value::Char(_) | Value::Guid(_));
    if !text_like && !no_text {
        return None;
    }
    let invalid = || Some(Err(format!("{INVALID_FORMAT_MARK}{template}")));
    let codes: Vec<&str> = pieces.iter().filter(|(c, _)| *c).map(|(_, t)| *t).collect();
    if codes.iter().any(|c| INVALID_CODES.contains(c)) {
        return invalid();
    }
    if let [p] = codes.as_slice() {
        if p.starts_with("Precision,") && pieces.iter().all(|(c, t)| *c || t.is_empty()) {
            if matches!(v, Value::Guid(_)) {
                return Some(Err(format!(
                    "{RAISE_MARK}InvalidStandardFormat{RAISE_MARK}Standard format number -1 does not exist for the type 'GUID'."
                )));
            }
            return Some(Ok(String::new()));
        }
    }
    let is_text_code = |c: &&str| *c == "Text" || c.starts_with("Text,");
    if !codes.iter().any(is_text_code) {
        return None;
    }
    if no_text {
        return invalid();
    }
    let mut filler = ' ';
    let mut out = String::new();
    let mut texts = 0;
    for (is_code, t) in pieces {
        if !is_code {
            out.push_str(t);
            continue;
        }
        if let Some(c) = t.strip_prefix("Filler Character,") {
            let mut chars = c.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => filler = ch,
                _ => return None,
            }
            continue;
        }
        if !is_text_code(t) {
            return None;
        }
        texts += 1;
        let value = match format_value(v, 0, 0, ctx) {
            Ok(s) => s,
            Err(f) => return Some(Err(fail(f))),
        };
        let width = match t.strip_prefix("Text,") {
            Some(w) => match w.trim().parse::<usize>() {
                Ok(w) if w > 0 => Some(w),
                _ => return None,
            },
            None => None,
        };
        match width {
            None => out.push_str(&value),
            Some(w) => {
                let len = value.chars().count();
                if len >= w {
                    if matches!(v, Value::Code(_)) && len > w {
                        return Some(Err(format!(
                            "format string `{template}`: a Code longer than `<Text,{w}>` (not pinned)"
                        )));
                    }
                    out.extend(value.chars().take(w));
                } else {
                    let fill: String = std::iter::repeat_n(filler, w - len).collect();
                    if matches!(v, Value::Code(_)) {
                        out.push_str(&fill);
                        out.push_str(&value);
                    } else {
                        out.push_str(&value);
                        out.push_str(&fill);
                    }
                }
            }
        }
    }
    // A filler after the `<Text,N>` is pinned for a Text only (the existing reader handles it); one `<Text>` code only.
    if texts != 1 {
        return None;
    }
    Some(Ok(out))
}
