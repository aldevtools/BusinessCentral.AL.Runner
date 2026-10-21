//! `Format(Value, Length, FormatString)`: format strings with `<...>` codes.
//!
//! The format-string language is large. Only the codes a corpus test uses are understood, with the
//! widths they are used with: `<Standard Format,N>` (types/TestFormatBooleanContracts.al:36,44),
//! `<Integer>`, `<Integer,N>`, `<Integer Thousand>`, `<Decimals,N>`, `<Sign>`, `<Sign,N>`,
//! `<Filler Character,C>` and `<Precision,A:B>` on numbers, and `<Year>`, `<Year4>`, `<Month>`,
//! `<Month,2>`, `<Month Text>`, `<Day>`, `<Day,2>`, `<Weekday Text>`, `<Closing>`, `<Hours24,2>`,
//! `<Hours12,2>`, `<Minutes,2>`, `<Seconds,2>` and `<AM/PM>` on dates and times
//! (types/TestDateRoundFormatStringContracts.al:Format_PictureCodes; handlers/TestPageTemporalSetValue_Page.al:47-65).
//! Any other code, or one of these on another type or in another width, is refused.

use crate::describe::Describe;
use crate::format::{FormatCtx, civil_from_day, format_value};
use crate::value::Value;
use rust_decimal::{Decimal, RoundingStrategy};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTHS_DE: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Marks an error string of [`format_with_string`] as the platform's "invalid field or attribute" error (the caller
/// raises it; text/TestProbeTextDateFormatContracts.al:Format_FormatStrings_Number, Format_OptionFieldAndOptionVariable).
pub const INVALID_FORMAT_MARK: &str = "\u{1}invalid-format:";

/// What the codes that come after a number's digits need to know from the whole string.
struct Numeric {
    value: Decimal,
    /// The character `<Integer,N>` pads with (`<Filler Character,C>` anywhere in the string).
    filler: char,
    /// `<Precision,A:B>`: at least A and at most B decimals.
    precision: Option<(u32, u32)>,
    /// The precision was written as `<Precision,N>`: at most N decimals, how many at least is not pinned.
    precision_single: bool,
}

const CODE_NAMES: [&str; 21] = [
    "Standard Format",
    "Integer Thousand",
    "Integer",
    "Decimals",
    "Sign",
    "Filler Character",
    "Precision",
    "Year4",
    "Year",
    "Month Text",
    "Month",
    "Day",
    "Weekday Text",
    "Closing",
    "Hours24",
    "Hours12",
    "Minutes",
    "Seconds",
    "Thousands",
    "Second dec",
    "AM/PM",
];

/// The codes of `template` spelled as this module knows them: a code name in any case, and blanks after the comma and before the
/// closing bracket of its argument (`<precision, 2:2><standard format,0>`, `<Precision,2:2 >`). A blank around the name itself makes
/// the code invalid (`< Sign >`, `<Sign ><Integer >`, `<Sign , 1>`), which is left for the code to reject
/// (types/lt-format-codes-textbuilder-pagerun-labels/P144FormatCodes.al:P144_Int_SignInteger, P144_Int_Precision, P144_Dec_*).
/// `Filler Char` for `Filler Character` is not pinned (assumption `format-code-spelling`); `None` when the template needs no change.
fn canonical_template(template: &str) -> Option<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    let mut changed = false;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            out.push_str(&rest[open..]);
            return changed.then_some(out);
        };
        let code = &after[..close];
        let (name, arg) = match code.split_once(',') {
            Some((n, a)) => (n, Some(a)),
            None => (code, None),
        };
        // `<Decimal,3>` is read as `<Decimals,3>` (types/lt-format-currency-prefix/FormatCurrencyPrefix.al:P157_NotAtStart `beforeSign`,
        // `integerDecimal`: `1234.5` shows `1,234.50` and `1234.50`). The bare `<Decimal>` is not pinned and stays invalid.
        let name = if name.eq_ignore_ascii_case("Decimal") && arg.is_some() {
            "Decimals"
        } else {
            name
        };
        // `Filler Char` is the short spelling Microsoft's Data Exchange tests use (assumption `format-code-spelling`).
        let known = CODE_NAMES.iter().find(|c| {
            c.eq_ignore_ascii_case(name) || (**c == "Filler Character" && name.eq_ignore_ascii_case("Filler Char"))
        });
        if known.is_some_and(|c| *c == "Filler Character") && name.eq_ignore_ascii_case("Filler Char") {
            crate::assumptions::assume(crate::assumptions::Assumption::FormatCodeSpelling);
        }
        let canon = match known {
            Some(c) => {
                let arg = arg.map(|a| {
                    if *c == "Filler Character" {
                        a.trim_start()
                    } else {
                        a.trim()
                    }
                });
                match arg {
                    Some(a) => format!("{c},{a}"),
                    None => (*c).to_string(),
                }
            }
            None => code.to_string(),
        };
        changed |= canon != code;
        out.push('<');
        out.push_str(&canon);
        out.push('>');
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    changed.then_some(out)
}

/// `Err` names the part of the format string that is not understood.
pub fn format_with_string(v: &Value, length: i32, template: &str, ctx: &dyn FormatCtx) -> Result<String, String> {
    // A Guid takes no format string without codes: the platform asks for the standard format number -1
    // (types/lt-scalars/ScT70054.al:Guid_FormatStrings `af79`).
    if matches!(v, Value::Guid(_)) && !template.contains('<') {
        return Err(format!(
            "{RAISE_MARK}InvalidStandardFormat{RAISE_MARK}Standard format number -1 does not exist for the type 'GUID'."
        ));
    }
    if template.contains("<Comma") {
        return comma_template(v, length, template, ctx);
    }
    let canon = canonical_template(template);
    let template = canon.as_deref().unwrap_or(template);
    let mut pieces: Vec<(bool, &str)> = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('<') {
        pieces.push((false, &rest[..open]));
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            // `<Precision,2:2` on a Decimal is the invalid-format error (types/lt-evaluate-format-lasterror-setview/
            // P86FormatDecimal.al:Str_Mid_5 `s46`); on other types it is not pinned.
            if matches!(v, Value::Decimal(_) | Value::Integer(_) | Value::BigInteger(_)) {
                return Err(format!("{INVALID_FORMAT_MARK}{template}"));
            }
            return Err(format!("format string `{template}`: `<` without `>`"));
        };
        pieces.push((true, &after[..close]));
        rest = &after[close + 1..];
    }
    pieces.push((false, rest));
    // The undefined DateTime and Date format as the empty text, their separators too (types/lt-datetime/Format.al:
    // QR_DateTime_Format_StampStrings_Undefined `u1`-`u8`, QR_Date_Format_StampStrings `z1`-`z9`). Pinned for these codes only.
    let undefined_codes: &[&str] = match v {
        Value::DateTime(0) => &[
            "Year4",
            "Year,2",
            "Month,2",
            "Day,2",
            "Hours24,2",
            "Hours24",
            "Minutes,2",
            "Seconds,2",
            "Filler Character,0",
        ],
        Value::Date(0, false) => &[
            "Year4",
            "Year,2",
            "Month,2",
            "Day,2",
            "Day",
            "Month Text",
            "Weekday Text",
            "Week",
        ],
        _ => &[],
    };
    if !undefined_codes.is_empty()
        && pieces.iter().any(|(is_code, _)| *is_code)
        && pieces
            .iter()
            .all(|(is_code, text)| !is_code || undefined_codes.contains(text))
    {
        return Ok(crate::format::pad(String::new(), length));
    }
    let numeric = numeric_of(v, &pieces).map_err(|e| {
        if e == INVALID_FORMAT_MARK {
            format!("{INVALID_FORMAT_MARK}{template}")
        } else {
            e
        }
    })?;
    let mut out = String::new();
    let temporal = matches!(v, Value::Date(..) | Value::Time(_) | Value::DateTime(_));
    // `<Filler Character,0>` right after a code: on a Time it pads `<Hours24,2>` and `<Hours12,2>` with zeros instead of
    // blanks (types/lt-datetime/Format.al:QR_Time_Format_StampStrings `t1`, `t4`, `t5`, `z1`, `z4`, `z5`); after the other
    // codes of a date or time it changes nothing (QR_DateTime_Format_StampStrings_Morning `m1`, `m2`, `m11`). Before a code it
    // does not apply (types/lt-aj-probes/AjScalars.al:Format_FillerCharacter_OnTime_3 `    900007`).
    let filler_after = |i: usize| {
        temporal
            && pieces.get(i + 1).is_some_and(|(c, t)| !c && t.is_empty())
            && pieces.get(i + 2).is_some_and(|(c, t)| *c && *t == "Filler Character,0")
    };
    for (i, &(is_code, text)) in pieces.iter().enumerate() {
        if is_code && temporal && text == "Filler Character,0" {
            let at_start = i == 1 && pieces[0].1.is_empty();
            let after_code = i >= 2 && pieces[i - 1].1.is_empty() && pieces[i - 2].0;
            if at_start || after_code {
                continue;
            }
            return Err(format!(
                "format string `{template}`: `<Filler Character,0>` after text on {} (not pinned)",
                v.describe()
            ));
        }
        if is_code && filler_after(i) {
            let part = code(v, text, numeric.as_ref(), ctx).map_err(|e| format!("format string `{template}`: {e}"))?;
            match (v, text) {
                (Value::Time(_), "Hours24,2" | "Hours12,2") => out.push_str(&part.replace(' ', "0")),
                // The other codes of a date or time pinned before a filler pad with zeros already.
                (_, "Hours24,2" | "Minutes,2" | "Seconds,2" | "Day,2" | "Month,2" | "Year4" | "Year,2") => {
                    out.push_str(&part)
                }
                _ => {
                    return Err(format!(
                        "format string `{template}`: `<Filler Character,0>` after `<{text}>` on {} (not pinned)",
                        v.describe()
                    ));
                }
            }
            continue;
        }
        if is_code {
            out.push_str(&code(v, text, numeric.as_ref(), ctx).map_err(|e| {
                if e == INVALID_FORMAT_MARK {
                    format!("{INVALID_FORMAT_MARK}{template}")
                } else if e.starts_with(RAISE_MARK) {
                    e
                } else {
                    format!("format string `{template}`: {e}")
                }
            })?);
        } else {
            out.push_str(text);
        }
    }
    // A result longer than the length is cut to it (P86FormatDecimal.al:Len_WithFormatString `l4`).
    if length > 0
        && out.chars().count() > length as usize
        && matches!(v, Value::Decimal(_) | Value::Integer(_) | Value::BigInteger(_))
    {
        return Ok(out.chars().take(length as usize).collect());
    }
    Ok(crate::format::pad(out, length))
}

/// The format strings with a `<Comma,...>` element that the corpus pins (types/lt-format-comma-format-string/
/// FormatCommaFormatString.al): any other string with a `<Comma` element, or a value of another type, is refused.
/// On a Decimal, `<Comma,.>` and `<Comma,,>` set the character that `<Decimals>` writes and write nothing themselves
/// (:37-55 Decimal, :59-84 negative, :88-113 zero); `<Comma>` without an argument is an invalid format. `,` is pinned
/// after `<Decimals>` only. On an Integer, a Text and a Date the element alone writes nothing and every
/// longer string is an invalid format (:117-200).
fn comma_template(v: &Value, length: i32, template: &str, ctx: &dyn FormatCtx) -> Result<String, String> {
    const SIGN_INT_DEC: &str = "<Sign><Integer><Decimals>";
    let invalid = || Err(format!("{INVALID_FORMAT_MARK}{template}"));
    let pinned_other = [
        "<Sign><Integer><Decimals><Comma,.>",
        "<Integer><Comma,.><Decimals>",
        "<Sign><Integer><Decimals><Comma,,>",
        "<Sign><Integer><Decimals><Comma>",
        "<Decimals><Comma,.>",
        // types/lt-format-comma-more/FormatCommaMore.al (an Integer value; the same strings on a Decimal read below).
        "<Integer><Comma,,><Decimals>",
        "<Integer Thousand><Comma,.><Decimals,3>",
    ];
    // The strings that read on a Decimal as the same string without the `<Comma,..>` element, the decimal point staying a point
    // (FormatCommaMore.al:Format_CommaBeforeDecimals_Decimal, :Format_CommaWithThousandAndWidth_Decimal).
    const COMMA_DROPPED: [(&str, &str); 7] = [
        ("<Integer><Comma,,><Decimals>", "<Integer><Decimals>"),
        ("<Integer Thousand><Comma,,><Decimals>", "<Integer Thousand><Decimals>"),
        (
            "<Integer Thousand><Comma,.><Decimals,3>",
            "<Integer Thousand><Decimals,3>",
        ),
        ("<Integer,10><Comma,.><Decimals,3>", "<Integer,10><Decimals,3>"),
        (
            "<Integer Thousand,12><Comma,.><Decimals,3>",
            "<Integer Thousand,12><Decimals,3>",
        ),
        (
            "<Sign><Integer Thousand><Comma,.><Decimals,2>",
            "<Sign><Integer Thousand><Decimals,2>",
        ),
        (
            "<Integer,8><Filler Character,0><Comma,.><Decimals,2>",
            "<Integer,8><Filler Character,0><Decimals,2>",
        ),
    ];
    match v {
        Value::Decimal(_) => {
            let (plain, point) = match template {
                "<Sign><Integer><Decimals><Comma,.>" => (SIGN_INT_DEC, '.'),
                "<Integer><Comma,.><Decimals>" => ("<Integer><Decimals>", '.'),
                "<Sign><Integer><Decimals><Comma,,>" => (SIGN_INT_DEC, ','),
                "<Decimals><Comma,.>" => ("<Decimals>", '.'),
                "<Comma,.>" => return Ok(crate::format::pad(String::new(), length)),
                "<Sign><Integer><Decimals><Comma>" => return invalid(),
                t if COMMA_DROPPED.iter().any(|(k, _)| *k == t) => {
                    let plain = COMMA_DROPPED
                        .iter()
                        .find(|(k, _)| *k == t)
                        .map(|(_, p)| *p)
                        .unwrap_or(t);
                    return Ok(crate::format::pad(format_with_string(v, 0, plain, ctx)?, length));
                }
                _ => {
                    return Err(format!(
                        "format string `{template}`: the `<Comma` element is pinned for six strings only"
                    ));
                }
            };
            let out = format_with_string(v, 0, plain, ctx)?;
            // The digits before the point carry no grouping, so the first point is the decimal one.
            let out = if point == ',' { out.replacen('.', ",", 1) } else { out };
            Ok(crate::format::pad(out, length))
        }
        Value::Integer(_) | Value::Text(_) | Value::Date(..) => {
            if template == "<Comma,.>" {
                Ok(crate::format::pad(String::new(), length))
            } else if pinned_other.contains(&template) {
                invalid()
            } else {
                Err(format!(
                    "format string `{template}`: the `<Comma` element is pinned for six strings only"
                ))
            }
        }
        // A BigInteger, a Duration and a DateTime (FormatCommaMore.al:Format_CommaOnOtherTypes, :Format_CommaWithoutDecimals_OtherTypes):
        // the element alone writes nothing; `<Integer Thousand>` before it reads on a BigInteger and is invalid on the others;
        // a `<Decimals>` after it is invalid on all three.
        Value::BigInteger(_) | Value::Duration(_) | Value::DateTime(_) => match template {
            "<Comma,.>" => Ok(crate::format::pad(String::new(), length)),
            "<Integer Thousand><Comma,.>" if matches!(v, Value::BigInteger(_)) => Ok(crate::format::pad(
                format_with_string(v, 0, "<Integer Thousand>", ctx)?,
                length,
            )),
            "<Integer Thousand><Comma,.>"
            | "<Integer Thousand><Comma,.><Decimals>"
            | "<Integer><Comma,,><Decimals>" => invalid(),
            _ => Err(format!(
                "format string `{template}`: the `<Comma` element is pinned for six strings only"
            )),
        },
        _ => Err(format!(
            "format string `{template}`: the `<Comma` element is not pinned for {}",
            v.describe()
        )),
    }
}

fn numeric_of(v: &Value, pieces: &[(bool, &str)]) -> Result<Option<Numeric>, String> {
    let value = match v {
        Value::Integer(i) => Decimal::from(*i),
        Value::BigInteger(i) => Decimal::from(*i),
        Value::Decimal(d) => *d,
        _ => return Ok(None),
    };
    let mut n = Numeric {
        value,
        filler: ' ',
        precision: None,
        precision_single: false,
    };
    let mut seen_integer = false;
    for (is_code, text) in pieces {
        if !is_code {
            continue;
        }
        seen_integer |= *text == "Integer" || text.starts_with("Integer,");
        // `Filler Character 0` (a blank instead of the comma) is no code (Format_FormatStrings_Number).
        if text.starts_with("Filler Character") && !text.starts_with("Filler Character,") {
            return Err(INVALID_FORMAT_MARK.to_string());
        }
        if let Some(c) = text.strip_prefix("Filler Character,") {
            let mut it = c.chars();
            let (Some(f), None) = (it.next(), it.next()) else {
                return Err(format!("code `<{text}>`"));
            };
            // A filler before the integer code pads with blanks (P86FormatDecimal.al:Str_Mid_3 `s24`, `<Filler Character,#><Integer,8>`);
            // after it, the character is used (`s23`, and the other corpus tests).
            if seen_integer {
                n.filler = f;
            }
        } else if let Some(p) = text.strip_prefix("Precision,") {
            // Letters are an invalid format; digits and colons are accepted and a form that is not `A:B` shows nothing
            // (P86FormatDecimal.al:Str_Mid_1 `<Precision,2>`, `<Precision,:2>`, Str_Mid_6 `<Precision,a:b>`).
            if !p.bytes().all(|b| b.is_ascii_digit() || b == b':') {
                return Err(INVALID_FORMAT_MARK.to_string());
            }
            n.precision = match p.split_once(':') {
                Some((a, b)) if !a.is_empty() && !b.is_empty() => {
                    let parsed = a
                        .parse::<u32>()
                        .ok()
                        .zip(b.parse::<u32>().ok())
                        .filter(|(a, b)| a <= b && *b <= 18);
                    Some(parsed.ok_or_else(|| format!("code `<{text}>`"))?)
                }
                None => match p.parse::<u32>() {
                    Ok(digits) if digits <= 18 => {
                        n.precision_single = true;
                        Some((0, digits))
                    }
                    _ => None,
                },
                _ => None,
            };
        }
    }
    Ok(Some(n))
}

fn width(code: &str, name: &str) -> Result<Option<usize>, String> {
    match code.strip_prefix(name).and_then(|r| r.strip_prefix(',')) {
        None => Ok(None),
        // `<Integer,5,>` is an invalid format on a number (P86FormatDecimal.al:Str_Mid_6 `s51`).
        Some(w) => w.parse().map(Some).map_err(|_| INVALID_FORMAT_MARK.to_string()),
    }
}

fn group(int: &str) -> String {
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn numeric_code(n: &Numeric, code: &str, v: &Value, ctx: &dyn FormatCtx) -> Result<String, String> {
    let abs = n.value.abs();
    let int = abs.trunc().to_string();
    let refuse = || Err(format!("code `<{code}>` on {}", v.describe()));
    if code == "Sign" || code.starts_with("Sign,") {
        let w = width(code, "Sign")?;
        let neg = n.value.is_sign_negative() && !n.value.is_zero();
        return match (neg, w) {
            (true, w) => Ok(format!("{:>1$}", "-", w.unwrap_or(1))),
            (false, Some(w)) => Ok(" ".repeat(w)),
            // A sign code without a width shows nothing for a number that is not negative.
            (false, None) => Ok(String::new()),
        };
    }
    if code == "Integer" || code.starts_with("Integer,") {
        let w = width(code, "Integer")?.unwrap_or(0);
        // A number that does not fit shows stars (P86FormatDecimal.al:Str_NegBig_3 `s22`).
        if w > 0 && int.len() > w {
            return Ok("*".repeat(w));
        }
        let pad: String = std::iter::repeat_n(n.filler, w.saturating_sub(int.len())).collect();
        return Ok(format!("{pad}{int}"));
    }
    if code == "Integer Thousand" || code.starts_with("Integer Thousand,") {
        let grouped = group(&int);
        return Ok(match width(code, "Integer Thousand")? {
            // A number that does not fit shows stars (`<Integer Thousand,3>` of 1234567 is `***`).
            Some(w) if grouped.len() > w => "*".repeat(w),
            Some(w) => format!("{grouped:>w$}"),
            None => grouped,
        });
    }
    if code == "Decimals" || code == "Decimals,0" {
        // Without a width: the decimals the value has (`5.5` shows `.5`, a whole number nothing).
        let frac = abs.fract().normalize().to_string();
        return Ok(frac.strip_prefix('0').map(str::to_string).unwrap_or_default());
    }
    if let Some(w) = width(code, "Decimals")? {
        if w < 2 {
            return refuse();
        }
        let digits = (w - 1) as u32;
        // More decimals than the width holds show stars (`<Decimals,3>` of 1234.567 is `***`); fewer are padded with zeros.
        if abs.fract().normalize().scale() > digits {
            return Ok("*".repeat(w));
        }
        let frac = format!("{:.*}", digits as usize, abs.fract());
        return Ok(frac.trim_start_matches('0').to_string());
    }
    if code.starts_with("Filler Character,") || code.starts_with("Precision,") {
        return Ok(String::new());
    }
    if let Some(std) = code.strip_prefix("Standard Format,") {
        let std: i32 = std.trim().parse().map_err(|_| format!("code `<{code}>`"))?;
        // A Precision does not change an Integer (misc/lt-system-functions/RandomFormat.al:Format_Integer).
        if let (Some((min, max)), Value::Decimal(_)) = (n.precision, v) {
            if std != 0 {
                return refuse();
            }
            let rounded = n
                .value
                .round_dp_with_strategy(max, RoundingStrategy::MidpointAwayFromZero);
            let mut text = format_value(&Value::Decimal(rounded), 0, 0, ctx).map_err(fail)?;
            let have = text.split_once('.').map_or(0, |(_, f)| f.len() as u32);
            // `<Precision,N>` on a value with fewer than N decimals: whether it pads is not pinned.
            if n.precision_single && have < max {
                return Err(format!(
                    "code `<Precision,{max}>` on a value with fewer decimals (not pinned)"
                ));
            }
            if have < min {
                if have == 0 {
                    text.push('.');
                }
                text.push_str(&"0".repeat((min - have) as usize));
            }
            return Ok(text);
        }
        return format_value(v, 0, std, ctx).map_err(fail);
    }
    // A name that is no code at all (`<Decimal Separator>`, `<1000 Separator>`, `<Unknown>`, `<Standard Format>`) is an invalid
    // format (P86FormatDecimal.al:Str_Mid_2 to Str_Mid_5).
    let name = code.split_once(',').map_or(code, |(n, _)| n);
    if !CODE_NAMES.contains(&name) || code == "Standard Format" {
        return Err(INVALID_FORMAT_MARK.to_string());
    }
    refuse()
}

/// Marks an error string as one to raise with its own code: `<mark><code>\u{2}<message>`.
pub const RAISE_MARK: char = '\u{2}';

fn fail(f: crate::format::FormatFail) -> String {
    match f {
        crate::format::FormatFail::Unsupported(s) => s,
        // `<Standard Format,6>` raises the error of `Format(x, 0, 6)` (P86FormatDecimal.al:Str_Mid_3 `s30`).
        crate::format::FormatFail::Error { message, code } => format!("{RAISE_MARK}{code}{RAISE_MARK}{message}"),
    }
}

fn code(v: &Value, code: &str, numeric: Option<&Numeric>, ctx: &dyn FormatCtx) -> Result<String, String> {
    // `<C,GBP>`: the currency code Base Application's Amount Auto Format puts in front of a format string. Format accepts it
    // and writes nothing for it (corpus types/lt-format-currency-prefix/FormatCurrencyPrefix.al: P157_Gbp_AmountAutoFormatShape,
    // P157_Gbp_PrefixPlusOtherTags).
    // An empty code is accepted too (`<C,>`; FormatCurrencyPrefix.al:P157_UnknownAndEmptyCurrency `empty`, `emptyOnly`).
    // `<C>` alone reads as no code at all too (`noComma`), while `<CGBP>` is an invalid format.
    if code.starts_with("C,") || code == "C" {
        return Ok(String::new());
    }
    if let Some(n) = numeric {
        return numeric_code(n, code, v, ctx);
    }
    // `<Integer>` is no code of an Option (Format_OptionFieldAndOptionVariable, `integer_cast_default`).
    if matches!(v, Value::Option(..)) && code.starts_with("Integer") {
        return Err(INVALID_FORMAT_MARK.to_string());
    }
    if let Some(n) = code.strip_prefix("Standard Format,") {
        return match n.trim().parse::<i32>() {
            Ok(n) => format_value(v, 0, n, ctx).map_err(fail),
            Err(_) => Err(format!("code `<{code}>`")),
        };
    }
    let (day_no, closing) = match v {
        Value::Date(n, c) if *n > 0 => (Some(*n), *c),
        Value::DateTime(t) if *t > 0 => (Some((t - 1).div_euclid(86_400_000) as i32 + 1), false),
        _ => (None, false),
    };
    // The undefined Time formats as midnight for the codes the corpus pins (types/lt-datetime/Format.al:
    // QR_Time_Format_StampStrings `z1`-`z5`); `<Hours24,2>` pads it with a blank (`z2` ` 0-00-00`).
    if matches!(v, Value::Time(0)) {
        return match code {
            "Hours24,2" => Ok(" 0".into()),
            "Hours24" => Ok("0".into()),
            "Minutes,2" | "Seconds,2" => Ok("00".into()),
            "Hours12,2" => Ok("12".into()),
            "AM/PM" => Ok("AM".into()),
            _ if !CODE_NAMES.contains(&code.split_once(',').map_or(code, |(n, _)| n)) => {
                Err(INVALID_FORMAT_MARK.to_string())
            }
            _ => Err(format!("code `<{code}>` on {}", v.describe())),
        };
    }
    let clock = match v {
        Value::Time(t) if *t > 0 => Some(t - 1),
        Value::DateTime(t) if *t > 0 => Some((t - 1).rem_euclid(86_400_000)),
        _ => None,
    };
    if code == "Closing" {
        return match day_no {
            Some(_) => Ok(if closing { "C".into() } else { String::new() }),
            None => Err(format!("code `<Closing>` on {}", v.describe())),
        };
    }
    // A closing date shows its day in the date codes ('5.3.2005'); only `<Closing>` shows the mark.
    let date = day_no.map(civil_from_day);
    let hour12 = |ms: i64| {
        if ms / 3_600_000 % 12 == 0 {
            12
        } else {
            ms / 3_600_000 % 12
        }
    };
    // `<Month Text,N>` and `<Weekday Text,N>`: the name cut to N characters and padded with spaces to N when it is shorter;
    // N = 0 is the whole name (types/lt-format-codes-textbuilder-pagerun-labels/P144DateTextWidth.al: P144_MonthText_March,
    // P144_MonthText_December, P144_WeekdayText_Monday).
    if let Some((base, width)) = code.split_once(',') {
        if matches!(base, "Month Text" | "Weekday Text") {
            if let (Ok(w), Some(_)) = (width.trim().parse::<usize>(), date) {
                let name = code_name(base, date, day_no, ctx.language())?;
                if w == 0 {
                    return Ok(name);
                }
                let cut: String = name.chars().take(w).collect();
                return Ok(format!("{cut:<w$}"));
            }
        }
    }
    let part = match (code, date, clock) {
        ("Year4", Some((y, _, _)), _) => format!("{y:04}"),
        // A two-digit year below 10 would print as one digit or two: not measured.
        // Two digits for 1999 and for 2000 to 2030, four for 1905, 1929, 1930 and 2105
        // (text/TestProbeTextDateFormatContracts.al:Format_FormatStrings_Date); other years are not measured.
        ("Year", Some((y, _, _)), _) if y == 1999 || (2000..=2030).contains(&y) => format!("{:02}", y.rem_euclid(100)),
        ("Year", Some((y, _, _)), _) if matches!(y, 1905 | 1929 | 1930 | 2105) => format!("{y}"),
        ("Year,2", Some((y, _, _)), _) => format!("{:02}", y.rem_euclid(100)),
        ("Month", Some((_, m, _)), _) => format!("{m}"),
        ("Month,2", Some((_, m, _)), _) => format!("{m:02}"),
        ("Month Text", Some(_), _) => code_name("Month Text", date, day_no, ctx.language())?,
        ("Day", Some((_, _, d)), _) => format!("{d}"),
        ("Day,2", Some((_, _, d)), _) => format!("{d:02}"),
        ("Weekday Text", Some(_), _) => code_name("Weekday Text", date, day_no, ctx.language())?,
        // A Time with an hour of 1 to 9 is padded with a space: 09:00 is ` 9` (misc/lt-probes-needed-2/PnBDateFormat.al:
        // PictureCodes_Numbers `hours24`). A DateTime keeps the zero (types/lt-datetime RoundTrip_AndFormatCodes,
        // codeunit 70120: `02:30:00`); midnight of a Time is not pinned and stays `00`.
        ("Hours24,2", _, Some(ms)) if matches!(v, Value::Time(_)) && ms / 3_600_000 != 0 => {
            format!("{:>2}", ms / 3_600_000)
        }
        ("Hours24,2", _, Some(ms)) => format!("{:02}", ms / 3_600_000),
        ("Hours24", _, Some(ms)) => format!("{}", ms / 3_600_000),
        // A minute below 10 without a width is not measured (Dates_CreateDateTimeRoundTrip shows 30).
        ("Minutes", _, Some(ms)) if ms / 60_000 % 60 >= 10 => format!("{}", ms / 60_000 % 60),
        // The twelve-hour clock pads the hour with a space (:Format_PictureCodes).
        ("Hours12,2", _, Some(ms)) => format!("{:>2}", hour12(ms)),
        // `<Hours12>` is the hour without padding; midnight and noon are 12 (types/lt-datetime/Format.al:Time_Format_Custom).
        ("Hours12", _, Some(ms)) => format!("{}", hour12(ms)),
        ("Minutes,2", _, Some(ms)) => format!("{:02}", ms / 60_000 % 60),
        ("Seconds,2", _, Some(ms)) => format!("{:02}", ms / 1000 % 60),
        // Corpus types/lt-datetime/Format.al:Time_Format_Custom (134506.078T): `<Seconds>` 6, `<Thousands>` 78,
        // `<Thousands,3>` ` 78`, `<Second dec>` .078, `<Second dec,3>` ***.
        ("Seconds", _, Some(ms)) => format!("{}", ms / 1000 % 60),
        ("Thousands", _, Some(ms)) => format!("{}", ms % 1000),
        ("Thousands,3", _, Some(ms)) => format!("{:>3}", ms % 1000),
        // On a DateTime with no milliseconds it is empty (types/lt-datetime/Format.al:QR_DateTime_Format_StampStrings_Afternoon
        // `a15`, `a16`); on a Time that is not pinned.
        ("Second dec", _, Some(ms)) if matches!(v, Value::DateTime(_)) && ms % 1000 == 0 => String::new(),
        ("Second dec", _, Some(ms)) => format!(".{:03}", ms % 1000),
        // The ISO 8601 week of a Date: 2024-03-05 is `10` (QR_Date_Format_StampStrings `d9`). A week below 10 (padded or not)
        // and the code on a DateTime are not pinned.
        ("Week", Some(_), _) if matches!(v, Value::Date(..)) => {
            let day = day_no.unwrap_or(1);
            let thursday = day + (4 - ((day - 1).rem_euclid(7) + 1));
            let (y, _, _) = civil_from_day(thursday);
            let week = (thursday - crate::format::day_from_civil(y, 1, 1)) / 7 + 1;
            if week < 10 {
                return Err(format!("code `<Week>` for week {week} (not pinned)"));
            }
            week.to_string()
        }
        ("Second dec,3", _, Some(_)) => "***".to_string(),
        ("AM/PM", _, Some(ms)) => (if ms / 3_600_000 < 12 { "AM" } else { "PM" }).to_string(),
        // A name that is no code at all is an invalid format, as for a number (types/lt-datetime/Format.al:
        // Time_Format_Custom `<Foo>`, error InvalidFormatString).
        _ if matches!(v, Value::Time(_)) && !CODE_NAMES.contains(&code.split_once(',').map_or(code, |(n, _)| n)) => {
            return Err(INVALID_FORMAT_MARK.to_string());
        }
        _ => return Err(format!("code `<{code}>` on {}", v.describe())),
    };
    Ok(part)
}

/// The name `<Month Text>` or `<Weekday Text>` shows.
fn code_name(base: &str, date: Option<(i32, u32, u32)>, day_no: Option<i32>, language: i32) -> Result<String, String> {
    // The month names follow `GlobalLanguage`: German is pinned (platform/lt-platform-members/PlTSession.al:
    // GlobalLanguage_SetAndRestore, `März`); another language than English and German is not.
    if language != 1033 && !(language == 1031 && base == "Month Text") {
        return Err(format!("{base} in the language {language} (not pinned)"));
    }
    Ok(match (base, date) {
        ("Month Text", Some((_, m, _))) if language == 1031 => MONTHS_DE[m as usize - 1].to_string(),
        ("Month Text", Some((_, m, _))) => MONTHS[m as usize - 1].to_string(),
        (_, Some(_)) => WEEKDAYS[((day_no.unwrap_or(1) - 1).rem_euclid(7)) as usize].to_string(),
        _ => String::new(),
    })
}

#[cfg(test)]
mod tests {
    use crate::format::day_from_civil;
    use crate::native::Signal;
    use crate::native::tests::{call, t};
    use crate::value::Value;

    fn fmt(v: Value, template: &str) -> Result<Value, Signal> {
        call("System.Format#2", vec![v, Value::Integer(0), t(template)])
    }

    // Microsoft's Remittance Advice and Finance Charge Memo reports: Format(Amount, 0, AutoFormat.ResolveAutoFormat(.., 'GBP')) =
    // `<C,GBP><Precision,2:2><Standard Format,0>` (28 Tests-ERM tests raised the invalid-format error).
    #[test]
    fn the_currency_prefix_of_an_auto_format_string_writes_nothing() {
        let out = fmt(
            Value::Decimal(rust_decimal::Decimal::new(12345, 2)),
            "<C,GBP><Precision,2:2><Standard Format,0>",
        );
        assert!(
            matches!(out, Ok(Value::Text(ref s)) if s.as_str() == "123.45"),
            "{out:?}"
        );
    }
    fn text(r: Result<Value, Signal>) -> String {
        match r {
            Ok(Value::Text(s)) => s.to_string(),
            other => panic!("{other:?}"),
        }
    }

    /// Microsoft's code writes `<sign><integer>` and `<precision, 2:2><standard format,0>%`: codes in any case, blanks
    /// after the comma. Not pinned by a corpus test (assumption `format-code-spelling`, docs/probes-needed.md).
    #[test]
    fn codes_are_read_in_any_case_and_with_blanks_after_the_comma() {
        assert_eq!(text(fmt(Value::Integer(-5), "<sign><integer>")), "-5");
        assert_eq!(text(fmt(Value::Integer(5), "<SIGN><INTEGER>")), "5");
        assert_eq!(text(fmt(Value::Integer(10), "<integer, 4>")), "  10");
        assert_eq!(
            text(fmt(
                Value::Decimal("0.5".parse().unwrap()),
                "<precision, 2:2><standard format,0>%"
            )),
            text(fmt(
                Value::Decimal("0.5".parse().unwrap()),
                "<Precision,2:2><Standard Format,0>%"
            ))
        );
    }

    #[test]
    fn standard_format_code() {
        // types/TestFormatBooleanContracts.al:36,44.
        assert_eq!(text(fmt(Value::Boolean(true), "<Standard Format,2>")), "1");
        assert_eq!(text(fmt(Value::Boolean(false), "<Standard Format,2>")), "0");
    }

    #[test]
    fn text_codes_with_a_width() {
        let day = day_from_civil(2024, 3, 5);
        assert_eq!(text(fmt(Value::Date(day, false), "<Month Text,3>")), "Mar");
        assert_eq!(text(fmt(Value::Date(day, false), "<Month Text,10>")), "March     ");
        assert_eq!(text(fmt(Value::Date(day, false), "<Weekday Text,3>")), "Tue");
    }

    #[test]
    fn integer_code() {
        // json/TestJsonXmlDeepContracts.al:71.
        assert_eq!(text(fmt(Value::Integer(10), "<Integer>")), "10");
    }

    #[test]
    fn date_and_time_codes() {
        // handlers/TestPageTemporalSetValue_Page.al:47-65.
        let day = day_from_civil(2024, 3, 7);
        assert_eq!(
            text(fmt(Value::Date(day, false), "<Year4>-<Month,2>-<Day,2>")),
            "2024-03-07"
        );
        let dt = Value::DateTime((i64::from(day) - 1) * 86_400_000 + (14 * 3600 + 30 * 60 + 5) * 1000 + 1);
        assert_eq!(
            text(fmt(dt, "<Year4>-<Month,2>-<Day,2> <Hours24,2>:<Minutes,2>:<Seconds,2>")),
            "2024-03-07 14:30:05"
        );
        let time = Value::Time((14 * 3600 + 30 * 60) * 1000 + 1);
        assert_eq!(text(fmt(time, "<Hours24,2>:<Minutes,2>:<Seconds,2>")), "14:30:00");
    }

    #[test]
    fn integer_thousand_with_a_width_shows_stars_when_the_number_does_not_fit() {
        // text/TestProbeTextDateFormatContracts.al:Format_StringCodes, `integer_thousand_width_3` and `_10`.
        assert_eq!(text(fmt(Value::Integer(1234567), "<Integer Thousand,3>")), "***");
        assert_eq!(
            text(fmt(Value::Integer(1234567), "<Integer Thousand,10>")),
            " 1,234,567"
        );
    }

    #[test]
    fn other_codes_and_values_are_refused() {
        let day = Value::Date(day_from_civil(2024, 3, 7), false);
        for (v, f) in [
            (day.clone(), "<Hours24,2>"),
            (day.clone(), "<Day,3>"),
            (day.clone(), "<Year4"),
            (Value::Integer(1), "<Standard Format,x>"),
        ] {
            assert!(matches!(fmt(v, f), Err(Signal::Unsupported(_))), "{f}");
        }
    }

    #[test]
    fn the_picture_codes_the_probe_pins() {
        // types/TestDateRoundFormatStringContracts.al:Format_PictureCodes.
        let day = Value::Date(day_from_civil(2024, 3, 5), false);
        assert_eq!(text(fmt(day.clone(), "<Day>/<Month>/<Year>")), "5/3/24");
        assert_eq!(text(fmt(day.clone(), "<Month Text> <Day> <Year4>")), "March 5 2024");
        assert_eq!(text(fmt(day.clone(), "<Weekday Text>")), "Tuesday");
        assert_eq!(text(fmt(day, "<Closing><Day,2>")), "05");
        let time = Value::Time(52_205_001);
        assert_eq!(text(fmt(time, "<Hours12,2>:<Minutes,2> <AM/PM>")), " 2:30 PM");
        let dec = |s: &str| Value::Decimal(s.parse().unwrap());
        assert_eq!(text(fmt(dec("1234.5"), "<Integer Thousand><Decimals,3>")), "1,234.50");
        assert_eq!(text(fmt(Value::Integer(5), "<Integer,4><Filler Character,0>")), "0005");
        // Microsoft's Data Exchange tests (GetNodeID) write the code as `Filler Char`.
        assert_eq!(text(fmt(Value::Integer(5), "<Integer,4><Filler Char,0>")), "0005");
        assert_eq!(
            text(fmt(dec("1234.567"), "<Precision,2:2><Standard Format,0>")),
            "1,234.57"
        );
        assert_eq!(text(fmt(Value::Integer(-5), "<Sign><Integer>")), "-5");
        assert_eq!(text(fmt(Value::Integer(5), "<Sign,1><Integer>")), " 5");
        // text/TestProbeTextDateFormatContracts.al:Format_StringCodes, `sign_pos_dec`.
        assert_eq!(
            text(fmt(Value::Decimal("5.5".parse().unwrap()), "<Sign><Integer><Decimals>")),
            "5.5"
        );
    }

    #[test]
    fn length_pads_the_result() {
        let r = call(
            "System.Format#2",
            vec![Value::Integer(10), Value::Integer(5), t("<Integer>")],
        );
        assert_eq!(text(r), "   10");
    }
    // Mutation-testing additions (docs/status.md debt).

    #[test]
    fn the_last_millisecond_of_a_day_stays_in_that_day() {
        let day = day_from_civil(2024, 3, 7);
        let last = (i64::from(day) - 1) * 86_400_000 + 86_399_999 + 1;
        assert_eq!(
            text(fmt(
                Value::DateTime(last),
                "<Year4>-<Month,2>-<Day,2> <Hours24,2>:<Minutes,2>:<Seconds,2>"
            )),
            "2024-03-07 23:59:59"
        );
        // The first millisecond of the day is 00:00:00.
        let first = (i64::from(day) - 1) * 86_400_000 + 1;
        assert_eq!(
            text(fmt(
                Value::DateTime(first),
                "<Day,2> <Hours24,2>:<Minutes,2>:<Seconds,2>"
            )),
            "07 00:00:00"
        );
        assert_eq!(
            text(fmt(Value::Time(86_399_999 + 1), "<Hours24,2>:<Minutes,2>:<Seconds,2>")),
            "23:59:59"
        );
        assert_eq!(
            text(fmt(Value::Time(1), "<Hours24,2>:<Minutes,2>:<Seconds,2>")),
            "00:00:00"
        );
    }

    #[test]
    fn an_undefined_datetime_or_date_formats_empty_and_an_undefined_time_as_midnight() {
        // types/lt-datetime/Format.al:QR_DateTime_Format_StampStrings_Undefined, QR_Date_Format_StampStrings `z*`,
        // QR_Time_Format_StampStrings `z*`.
        assert_eq!(
            text(fmt(Value::DateTime(0), "<Year4>-<Month,2>-<Day,2>T<Hours24,2>")),
            ""
        );
        assert_eq!(
            text(fmt(Value::Date(0, false), "<DAY>. <MONTH text> <YEAR4> <Week>")),
            ""
        );
        assert_eq!(
            text(fmt(Value::Time(0), "<Hours24,2>-<Minutes,2>-<Seconds,2>")),
            " 0-00-00"
        );
        assert_eq!(
            text(fmt(
                Value::Time(0),
                "<Hours12,2><Filler Character,0>:<Minutes,2> <AM/PM>"
            )),
            "12:00 AM"
        );
        // Codes the corpus does not pin on an undefined value stay refused.
        for (v, f) in [
            (Value::DateTime(0), "<Thousands>"),
            (Value::Time(0), "<Thousands>"),
            (Value::Date(0, false), "<Closing>"),
        ] {
            assert!(matches!(fmt(v, f), Err(Signal::Unsupported(_))), "{f}");
        }
    }

    // types/lt-datetime/Format.al:QR_DateTime_Format_StampStrings_Morning, _Afternoon, QR_Time_Format_StampStrings,
    // QR_Date_Format_StampStrings.
    #[test]
    fn stamp_strings() {
        let day = day_from_civil(2024, 3, 5);
        let dt = Value::DateTime((i64::from(day) - 1) * 86_400_000 + ((9 * 60 + 7) * 60 + 2) * 1000 + 123 + 1);
        assert_eq!(
            text(fmt(
                dt.clone(),
                "<Year4><Month,2><Day,2><Hours24,2><Filler Character,0><Minutes,2><Seconds,2>"
            )),
            "20240305090702"
        );
        assert_eq!(text(fmt(dt.clone(), "<Seconds,2><Second dec>")), "02.123");
        let even = Value::DateTime((i64::from(day) - 1) * 86_400_000 + ((13 * 60 + 45) * 60 + 6) * 1000 + 1);
        assert_eq!(text(fmt(even.clone(), "<Second dec>")), "");
        assert_eq!(text(fmt(even, "<Thousands>")), "0");
        let time = Value::Time(((9 * 60 + 7) * 60 + 2) * 1000 + 123 + 1);
        assert_eq!(
            text(fmt(time.clone(), "<Hours24,2>-<Minutes,2>-<Seconds,2>")),
            " 9-07-02"
        );
        assert_eq!(
            text(fmt(time.clone(), "<Hours24,2><Filler Character,0><Minutes,2>")),
            "0907"
        );
        assert_eq!(
            text(fmt(time, "<Hours12,2><Filler Character,0>:<Minutes,2> <AM/PM>")),
            "09:07 AM"
        );
        assert_eq!(text(fmt(Value::Date(day, false), "<Week>")), "10");
        // A filler after a code nothing pins it with stays refused.
        assert!(matches!(
            fmt(dt, "<Hours12,2><Filler Character,0>"),
            Err(Signal::Unsupported(_))
        ));
    }
}
