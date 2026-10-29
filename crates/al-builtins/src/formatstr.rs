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
    /// The decimals `<Decimals>` shows at least: the minimum places of a Decimal field (`format_with_field_places`), else 0.
    min_decimals: u32,
    /// `<Precision,A:B>`: at least A and at most B decimals.
    precision: Option<(u32, u32)>,
    /// The precision was written as `<Precision,N>`: at most N decimals, how many at least is not pinned.
    precision_single: bool,
    /// The precision code comes after a `<Standard Format,N>` code.
    precision_after_std: bool,
}

const CODE_NAMES: [&str; 22] = [
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
    "Quarter",
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
                    // A blank is a filler character of its own: `<Filler Character, >` pads with blanks (types/lt-scalars/
                    // ScT70052.al:84,94).
                    if *c == "Filler Character" && a == " " {
                        a
                    } else if *c == "Filler Character" {
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
    format_with_field_places(v, length, template, 0, ctx)
}

/// `format_with_string` of a Decimal field's value already rounded to the field's most places, where `<Decimals>` shows at
/// least the field's fewest places (DecimalPlaces 2:2 writes `.00` for 5, 0:5 nothing; types/lt-at-partner-gaps/
/// AtPrecisionStandardFormat2.al:969-1047, RO_FieldFormatStrings_DecimalPlacesFields_*). A `<Decimals,N>` keeps its own width.
pub fn format_with_field_places(
    v: &Value,
    length: i32,
    template: &str,
    min_decimals: u32,
    ctx: &dyn FormatCtx,
) -> Result<String, String> {
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
    // A Duration takes no date, time or number code: `<Integer Thousand>` and `<Hours24,2>:<Minutes,2>` are the invalid-format
    // error (types/lt-duration-sum-typed/DurationSumTyped.al:86,95 `af74`; types/lt-datetime/Format.al:473,499 `af279`).
    // Other codes on a Duration are not pinned.
    if matches!(v, Value::Duration(_))
        && pieces.iter().any(|(c, _)| *c)
        && pieces
            .iter()
            .all(|(c, t)| !c || matches!(*t, "Integer Thousand" | "Hours24,2" | "Minutes,2"))
    {
        return Err(format!("{INVALID_FORMAT_MARK}{template}"));
    }
    // `<Integer,6><Filler Character,0>` on a Byte is the invalid-format error (types/lt-scalars/ScT70052.al:182,197).
    if matches!(v, Value::Byte(_))
        && pieces.iter().any(|(c, _)| *c)
        && pieces
            .iter()
            .all(|(c, t)| !c || t.starts_with("Integer,") || *t == "Filler Character,0")
    {
        return Err(format!("{INVALID_FORMAT_MARK}{template}"));
    }
    if let Value::Text(t) = v {
        if let Some(out) = text_template(t.as_str(), &pieces)? {
            return Ok(crate::format::pad(out, length));
        }
    }
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
            // FormatQuarterDuration.al:83,94 (`undefined_date`).
            "Quarter",
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
    let numeric = numeric_of(v, &pieces, min_decimals).map_err(|e| {
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
        // At the start of the string the filler characters `#` and `*` change nothing on a date or time: `<Filler Character,#><Day,4>`
        // is `0005` on a Date, `<Filler Character,*><Hours12,4>` `   9` on a Time (types/lt-aj-probes/AjScalars.al:269-270,
        // 292, 323). Other characters are not pinned.
        if is_code
            && temporal
            && matches!(text, "Filler Character,#" | "Filler Character,*")
            && i == 1
            && pieces[0].1.is_empty()
        {
            continue;
        }
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
            let part = code(v, text, numeric.as_ref(), ctx).map_err(|e| {
                if e == INVALID_FORMAT_MARK {
                    format!("{INVALID_FORMAT_MARK}{template}")
                } else {
                    format!("format string `{template}`: {e}")
                }
            })?;
            match (v, text) {
                (Value::Time(_), "Hours24,2" | "Hours12,2") => out.push_str(&part.replace(' ', "0")),
                // `<Day,4>` and `<Month,4>` of a Date or DateTime pad with zeros already (AjScalars.al:266,272,282,295).
                (Value::Date(..) | Value::DateTime(_), t) if t.starts_with("Day,") || t.starts_with("Month,") => {
                    out.push_str(&part)
                }
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
    // A result longer than the length is cut to it (P86FormatDecimal.al:Len_WithFormatString `l4`; a Date:
    // types/lt-datetime/Format.al:527,537 `date len 5 custom`).
    if length > 0
        && out.chars().count() > length as usize
        && matches!(
            v,
            Value::Decimal(_) | Value::Integer(_) | Value::BigInteger(_) | Value::Date(..)
        )
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
                // TP_Format_PrecisionStrings on corpus branch al-runtime-rs-staging-pub-tp (`p2comma.`, `p2comma,`, `p2thousand`):
                // `1,234,50` for `<Precision,2:2><Integer Thousand><Decimals><Comma,,>` of 1234.5.
                "<Precision,2:2><Sign><Integer><Decimals><Comma,.>" => {
                    ("<Precision,2:2><Sign><Integer><Decimals>", '.')
                }
                "<Precision,2:2><Sign><Integer><Decimals><Comma,,>" => {
                    ("<Precision,2:2><Sign><Integer><Decimals>", ',')
                }
                "<Precision,2:2><Integer Thousand><Decimals><Comma,,>" => {
                    ("<Precision,2:2><Integer Thousand><Decimals>", ',')
                }
                // TQ_Format_PrecisionComma on corpus branch al-runtime-rs-staging-pub-tq: `1235`, `1,234.568`, `1234,568` of 1234.56789.
                "<Precision,0:0><Integer><Decimals><Comma,,>" => ("<Precision,0:0><Integer><Decimals>", ','),
                "<Precision,1:3><Integer Thousand><Decimals><Comma,.>" => {
                    ("<Precision,1:3><Integer Thousand><Decimals>", '.')
                }
                "<Precision,3:3><Sign><Integer><Decimals><Comma,,>" => {
                    ("<Precision,3:3><Sign><Integer><Decimals>", ',')
                }
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
            // The digits before the point carry no grouping or group with commas, so the first point is the decimal one.
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

fn numeric_of(v: &Value, pieces: &[(bool, &str)], min_decimals: u32) -> Result<Option<Numeric>, String> {
    let value = match v {
        Value::Integer(i) => Decimal::from(*i),
        Value::BigInteger(i) => Decimal::from(*i),
        Value::Decimal(d) => *d,
        _ => return Ok(None),
    };
    let mut n = Numeric {
        value,
        filler: ' ',
        min_decimals,
        precision: None,
        precision_single: false,
        precision_after_std: false,
    };
    let mut seen_integer = false;
    let mut seen_std = false;
    for (is_code, text) in pieces {
        if !is_code {
            continue;
        }
        seen_integer |= *text == "Integer" || text.starts_with("Integer,");
        seen_std |= text.starts_with("Standard Format,");
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
            n.precision_after_std = seen_std;
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
                        .filter(|(a, b)| a <= b && (*b <= 18 || (*a, *b) == (2, 26)));
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
    // `<Precision,A:B>` as the first code, without a standard format: the value is rounded half away from zero to B decimals and
    // `<Decimals>` shows at least A (corpus branches al-runtime-rs-staging-pub-tp and -tq, types/lt-format-comma-format-string/
    // FormatCommaFormatString.al TP_Format_PrecisionStrings, TQ_Format_PrecisionComma: `<Precision,2:2><Sign><Integer><Decimals>` of -1234.567 is `-1234.57`,
    // of 12 `12.00`; assumption format-precision-first-code). After another code it changes nothing (`signfirst`:
    // `<Sign><Precision,2:2><Integer><Decimals>` of -1234.567 is `-1234.567`).
    let first_code = pieces.iter().find(|(is_code, _)| *is_code).map(|(_, t)| *t);
    if let (Some((min, max)), false, false, Some(first)) = (n.precision, seen_std, n.precision_single, first_code)
        && first.starts_with("Precision,")
        && matches!(v, Value::Decimal(_))
        && pieces
            .iter()
            .any(|(is_code, t)| *is_code && (*t == "Integer" || *t == "Integer Thousand" || *t == "Decimals"))
    {
        let rounded = n
            .value
            .round_dp_with_strategy(max, RoundingStrategy::MidpointAwayFromZero);
        let decimals = pieces.iter().any(|(is_code, t)| *is_code && *t == "Decimals");
        // Marked only where the result shows it: the whole part or the sign changes, or `<Decimals>` shows other digits.
        let seen = rounded.trunc() != n.value.trunc()
            || rounded.is_sign_negative() != n.value.is_sign_negative()
            || (decimals && (rounded != n.value || min > n.value.fract().normalize().scale()));
        if seen {
            // Pinned for 0:0, 1:3 and 3:3 by TQ_Format_PrecisionComma (corpus branch al-runtime-rs-staging-pub-tq), 2:2 by TP.
            if !matches!((min, max), (0, 0) | (1, 3) | (2, 2) | (3, 3)) {
                return Err(format!(
                    "code `<Precision,{min}:{max}>` first without a standard format (not pinned)"
                ));
            }
            crate::assumptions::assume(crate::assumptions::Assumption::FormatPrecisionFirstCode);
        }
        n.value = rounded;
        n.min_decimals = n.min_decimals.max(min);
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
    // `<Decimals>` on an Integer is the invalid-format error (types/lt-scalars/ScT70052.al:85,96 `af52`).
    if code == "Decimals" && matches!(v, Value::Integer(_)) {
        return Err(INVALID_FORMAT_MARK.to_string());
    }
    if code == "Decimals" || code == "Decimals,0" {
        // Without a width: the decimals the value has (`5.5` shows `.5`, a whole number nothing).
        let frac = abs.fract().normalize();
        let digits = frac.scale().max(n.min_decimals);
        if digits == 0 {
            return Ok(String::new());
        }
        let frac = format!("{:.*}", digits as usize, frac);
        return Ok(frac.strip_prefix('0').map(str::to_string).unwrap_or_default());
    }
    if let Some(w) = width(code, "Decimals")? {
        // `<Decimals,1>` of a value with decimals is one star (types/lt-scalars/ScT70051.al:105,115: 1234567.891 is
        // `1234567*`); of a whole number it is not pinned.
        if w == 1 && !abs.fract().is_zero() {
            return Ok("*".into());
        }
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
            return precision_standard(n, (min, max), std, code, v, ctx);
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

/// `<Precision,A:B>` then `<Standard Format,N>` on a Decimal: the value rounded half away from zero to B decimals, written in
/// standard format N, with zeros added up to A decimals (types/lt-format-precision-standard/FormatPrecStdEn.al:43-150, codeunit
/// 71576, the same under GlobalLanguage 1031 in FormatPrecStdDe.al, codeunit 71577): format 0 groups thousands, 1, 2 and 9 do
/// not, 3 and 4 are 0 and 1 with the sign behind the number (`1,234.50 `, `0.50-`), 5 to 8 raise the error of `Format(x, 0, N)`
/// (:52-55); and types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al:105-660 (QS_P22S*, QS_P2S* for formats 0, 1, 2, 9 on 18
/// values). Other precisions under format 9: `0:0` gives `1235` and `-1`, `2:5` `1234.50`, `0:3` `1234.5`, `7:7` `0.0000001`
/// (FormatPrecStdEn.al:154-191). `<Precision,N>` acts as `N:N`: N = 2 under formats 0, 1, 2, 9 (AtPrecisionStandardFormat2.al
/// QS_P2S0/1/2/9, `0` gives `0.00`) and N = 3 under format 9 (FormatPrecStdEn.al:161 `1234.500`); another N or format with a
/// value of fewer decimals is refused. A precision after the standard format code changes nothing under format 9 (:163
/// `1234.5`); under another format that is refused. A negative value that rounds to zero is not pinned and is refused.
fn precision_standard(
    n: &Numeric,
    (min, max): (u32, u32),
    std: i32,
    code: &str,
    v: &Value,
    ctx: &dyn FormatCtx,
) -> Result<String, String> {
    if n.precision_after_std {
        if std == 9 {
            return format_value(v, 0, std, ctx).map_err(fail);
        }
        return Err(format!("code `<{code}>` before a `<Precision,..>` code (not pinned)"));
    }
    if !(0..=9).contains(&std) {
        return Err(format!("code `<{code}>` on {}", v.describe()));
    }
    if (5..=8).contains(&std) {
        return format_value(v, 0, std, ctx).map_err(fail);
    }
    // `<Precision,2:26>` (more than 18 decimals) is pinned only by TP_Format_PrecisionStrings on corpus branch
    // al-runtime-rs-staging-pub-tp, with standard formats 2 and 4 (`1234.50 `, `-1234.567`), and TQ_Format_PrecisionStd_* on
    // branch al-runtime-rs-staging-pub-tq with standard formats 0, 2 and 9.
    if max > 18 {
        crate::assumptions::assume(crate::assumptions::Assumption::FormatPrecisionFirstCode);
    }
    let rounded = n
        .value
        .round_dp_with_strategy(max, RoundingStrategy::MidpointAwayFromZero);
    if rounded.is_zero() && n.value.is_sign_negative() && !n.value.is_zero() {
        return Err(format!(
            "code `<{code}>` with a precision on a negative {} that rounds to zero (not pinned)",
            v.describe()
        ));
    }
    let mut text = format_value(&Value::Decimal(rounded), 0, std, ctx).map_err(fail)?;
    // Formats 3 and 4 end with the sign (a blank or a minus): the decimals are padded before it.
    let tail = if matches!(std, 3 | 4) { text.pop() } else { None };
    let have = text.split_once('.').map_or(0, |(_, f)| f.len() as u32);
    let min = if n.precision_single {
        let pinned = (max == 2 && matches!(std, 0 | 1 | 2 | 9)) || (max == 3 && std == 9);
        if !pinned && have < max {
            return Err(format!(
                "code `<Precision,{max}>` on a value with fewer decimals (not pinned)"
            ));
        }
        max
    } else {
        min
    };
    if have < min {
        if have == 0 {
            text.push('.');
        }
        text.push_str(&"0".repeat((min - have) as usize));
    }
    text.extend(tail);
    Ok(text)
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
    // `<Text,5>` on an Enum shows its caption cut to 5 characters: `Two Words Caption` is `Two W` (types/lt-enum-to-text-fieldref/
    // EnumToTextFieldRef.al:170,180). A caption shorter than the width is not pinned.
    if let (Value::Enum(..), Some(w)) = (v, code.strip_prefix("Text,")) {
        if let Ok(w) = w.parse::<usize>() {
            let caption = format_value(v, 0, 0, ctx).map_err(fail)?;
            if caption.chars().count() > w {
                return Ok(caption.chars().take(w).collect());
            }
        }
    }
    // `<Text>` on a RecordId is an invalid format (types/lt-implicit-conversions/P79RecordId.al:405,417).
    if code == "Text" && matches!(v, Value::Library(crate::library_kind::LibraryKind::RecordId, _)) {
        return Err(INVALID_FORMAT_MARK.to_string());
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
    // Widths on dates and times (types/lt-aj-probes/AjScalars.al:225-327): `<Day,4>` and `<Month,4>` of a Date or DateTime pad
    // with zeros (`0005`, `0003`; `<Day,3>` of a Date `005`: types/lt-datetime/Format.al:142,173); the clock codes of a DateTime pad
    // with zeros (`<Hours24,5>` `00009`, `<Minutes,5>` `00007`, `<Hours12,4>` `0009`), those of a Time pad the hour with blanks and
    // the minutes with zeros (`    9`, `00007`, `   9`). `<Hours24,5>` and `<Hours12,4>` on a Date and `<Day,4>` on a Time are the
    // invalid-format error (`<Day,2>` on a Time too: the template `<Filler Character,0><Day,2><Month,4><Year4>` raises,
    // AjScalars.al:302,317). Only these code, width and type triples are pinned; the others fall through to the arms below
    // (width 2) or are refused there.
    if let Some((base, w)) = code.split_once(',') {
        let w = w.trim().parse::<usize>().ok();
        match (v, base, w, date, clock) {
            (Value::Date(..), "Hours24", Some(5), ..)
            | (Value::Date(..), "Hours12", Some(4), ..)
            | (Value::Time(_), "Day", Some(2 | 4), ..) => {
                return Err(INVALID_FORMAT_MARK.to_string());
            }
            (Value::Date(..), "Day", Some(w @ (3 | 4)), Some((_, _, d)), _)
            | (Value::DateTime(_), "Day", Some(w @ 4), Some((_, _, d)), _) => {
                return Ok(format!("{d:0w$}"));
            }
            (Value::Date(..) | Value::DateTime(_), "Month", Some(w @ 4), Some((_, m, _)), _) => {
                return Ok(format!("{m:0w$}"));
            }
            (Value::DateTime(_), "Hours24", Some(w @ 5), _, Some(ms)) => return Ok(format!("{:0w$}", ms / 3_600_000)),
            (Value::DateTime(_), "Hours12", Some(w @ 4), _, Some(ms)) => return Ok(format!("{:0w$}", hour12(ms))),
            (Value::DateTime(_) | Value::Time(_), "Minutes", Some(w @ 5), _, Some(ms)) => {
                return Ok(format!("{:0w$}", ms / 60_000 % 60));
            }
            (Value::Time(_), "Hours24", Some(w @ 5), _, Some(ms)) => return Ok(format!("{:>w$}", ms / 3_600_000)),
            (Value::Time(_), "Hours12", Some(w @ 4), _, Some(ms)) => return Ok(format!("{:>w$}", hour12(ms))),
            _ => {}
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
        // Midnight is ` 0` too (types/lt-variant-time-enum/VariantTextTimeEnumFilter.al:105,120).
        ("Hours24,2", _, Some(ms)) if matches!(v, Value::Time(_)) => format!("{:>2}", ms / 3_600_000),
        ("Hours24,2", _, Some(ms)) => format!("{:02}", ms / 3_600_000),
        ("Hours24", _, Some(ms)) => format!("{}", ms / 3_600_000),
        // A minute below 10 without a width is not measured (Dates_CreateDateTimeRoundTrip shows 30).
        ("Minutes", _, Some(ms)) if ms / 60_000 % 60 >= 10 => format!("{}", ms / 60_000 % 60),
        // On a Time a minute below 10 has no padding: `0`, `5`, `9` (VariantTextTimeEnumFilter.al:113-115,128-130).
        ("Minutes", _, Some(ms)) if matches!(v, Value::Time(_)) => format!("{}", ms / 60_000 % 60),
        // The twelve-hour clock pads the hour with a space (:Format_PictureCodes).
        // On a DateTime it pads with a zero: 13:45 is `01` (types/lt-datetime/Format.al:420,430).
        ("Hours12,2", _, Some(ms)) if matches!(v, Value::DateTime(_)) => format!("{:02}", hour12(ms)),
        ("Hours12,2", _, Some(ms)) => format!("{:>2}", hour12(ms)),
        // `<Hours12>` is the hour without padding; midnight and noon are 12 (types/lt-datetime/Format.al:Time_Format_Custom).
        ("Hours12", _, Some(ms)) => format!("{}", hour12(ms)),
        ("Minutes,2", _, Some(ms)) => format!("{:02}", ms / 60_000 % 60),
        ("Seconds,2", _, Some(ms)) => format!("{:02}", ms / 1000 % 60),
        // Corpus types/lt-datetime/Format.al:Time_Format_Custom (134506.078T): `<Seconds>` 6, `<Thousands>` 78,
        // `<Thousands,3>` ` 78`, `<Second dec>` .078, `<Second dec,3>` ***.
        ("Seconds", _, Some(ms)) => format!("{}", ms / 1000 % 60),
        ("Thousands", _, Some(ms)) => format!("{}", ms % 1000),
        // On a DateTime it pads with zeros: `078` (types/lt-datetime/Format.al:419,429).
        ("Thousands,3", _, Some(ms)) if matches!(v, Value::DateTime(_)) => format!("{:03}", ms % 1000),
        ("Thousands,3", _, Some(ms)) => format!("{:>3}", ms % 1000),
        // On a DateTime with no milliseconds it is empty (types/lt-datetime/Format.al:QR_DateTime_Format_StampStrings_Afternoon
        // `a15`, `a16`); on a Time that is not pinned.
        ("Second dec", _, Some(ms)) if matches!(v, Value::DateTime(_)) && ms % 1000 == 0 => String::new(),
        ("Second dec", _, Some(ms)) => format!(".{:03}", ms % 1000),
        // The ISO 8601 week of a Date, without padding: 2024-03-05 is `10` (QR_Date_Format_StampStrings `d9`), 2021-01-04 `1`,
        // 2024-01-17 `3`, 2021-01-03 `53` and 2024-12-30 `1` (the ISO year is not shown: `<Week>.<Year4>` is `53.2021`, `1.2024`).
        // `<Week,2>` pads with a zero: `01` (types/lt-format-week-code/FormatWeekCodeIso.al:21-123; types/lt-datetime/
        // DateFuncs.al:427). The code on a DateTime and other widths are not pinned.
        ("Week" | "Week,2", Some(_), _) if matches!(v, Value::Date(..)) => {
            let week = iso_week(day_no.unwrap_or(1));
            // The pinned weeks are 1, 3, 10, 19, 28, 40, 52 and 53; the ISO rule for the others is an assumption.
            if !matches!(week, 1 | 3 | 10 | 19 | 28 | 40 | 52 | 53) {
                crate::assumptions::assume(crate::assumptions::Assumption::FormatWeekCodeIso);
            }
            if code == "Week,2" {
                format!("{week:02}")
            } else {
                week.to_string()
            }
        }
        // `<Weekday>` is the ISO day number, Monday 1 to Sunday 7: 2024-01-17 is `3` (types/lt-datetime/DateFuncs.al:415,425),
        // 2021-01-03 `7` (FormatWeekCodeIso.al:121,128). On a DateTime it is not pinned.
        ("Weekday", Some(_), _) if matches!(v, Value::Date(..)) => {
            let weekday = (day_no.unwrap_or(1) - 1).rem_euclid(7) + 1;
            // Wednesday 3 and Sunday 7 are pinned; the other days follow the same ISO numbering on an assumption.
            if !matches!(weekday, 3 | 7) {
                crate::assumptions::assume(crate::assumptions::Assumption::FormatWeekCodeIso);
            }
            format!("{weekday}")
        }
        // `<Quarter>` is 1 to 4, a closing date shows the quarter of its day, `<quarter>` is read too
        // (types/lt-quarter-duration/FormatQuarterDuration.al:30-110). On a DateTime it is not pinned.
        ("Quarter", Some((_, m, _)), _) if matches!(v, Value::Date(..)) => format!("{}", (m - 1) / 3 + 1),
        ("Second dec,3", _, Some(_)) => "***".to_string(),
        ("AM/PM", _, Some(ms)) => (if ms / 3_600_000 < 12 { "AM" } else { "PM" }).to_string(),
        // A name that is no code at all is an invalid format, as for a number (types/lt-datetime/Format.al:
        // Time_Format_Custom `<Foo>`, error InvalidFormatString).
        _ if matches!(v, Value::Time(_)) && !CODE_NAMES.contains(&code.split_once(',').map_or(code, |(n, _)| n)) => {
            return Err(INVALID_FORMAT_MARK.to_string());
        }
        // On a Date and a DateTime too: `<Day Text>` and `<Foo>` (types/lt-datetime/Format.al:157-158,186,191 on a Date,
        // :424,436 on a DateTime). A name the documentation lists as a code but this module does not build stays refused.
        _ if matches!(v, Value::Date(..) | Value::DateTime(_)) && !is_documented_code(code) => {
            return Err(INVALID_FORMAT_MARK.to_string());
        }
        _ => return Err(format!("code `<{code}>` on {}", v.describe())),
    };
    Ok(part)
}

/// A Text formatted with the pinned forms of `<Text>`: `<Text>` alone is the text (text/lt-text-functions/
/// TextFormatEvaluate.Codeunit.al:48,67, text/lt-label-textbuilder/TxCLabel.al:632,639); `<Text,N>` pads a shorter text with
/// blanks to N, and a `<Filler Character,C>` right after it pads with C instead (types/lt-ax-text-format-dates/
/// AxFillerBuilderAutoFormat.al:101-136: `ab` with `<Text,5>` is `ab   `, with `<Filler Char,0>` `ab000`, with `*` `ab***`).
/// A text longer than N, and `<Text>` with literal text or other codes, are not pinned. `Ok(None)`: no `<Text>` code.
fn text_template(t: &str, pieces: &[(bool, &str)]) -> Result<Option<String>, String> {
    let codes: Vec<&str> = pieces.iter().filter(|(c, _)| *c).map(|(_, t)| *t).collect();
    if !codes.iter().any(|c| *c == "Text" || c.starts_with("Text,")) {
        return Ok(None);
    }
    let refuse = || {
        Err(format!(
            "format string: a `<Text>` code in this form on Text '{t}' (not pinned)"
        ))
    };
    if pieces.iter().any(|(c, text)| !c && !text.is_empty()) {
        return refuse();
    }
    let filler = match codes.as_slice() {
        ["Text"] => return Ok(Some(t.to_string())),
        [_] => ' ',
        [_, f] => match f.strip_prefix("Filler Character,") {
            Some(c) if c.chars().count() == 1 => c.chars().next().unwrap_or(' '),
            _ => return refuse(),
        },
        _ => return refuse(),
    };
    let Some(width) = codes[0].strip_prefix("Text,").and_then(|w| w.parse::<usize>().ok()) else {
        return refuse();
    };
    let len = t.chars().count();
    if len > width {
        return refuse();
    }
    let mut out = t.to_string();
    out.extend(std::iter::repeat_n(filler, width - len));
    Ok(Some(out))
}

/// Whether the name of `code` is one of the codes of the Format documentation (or this module), whatever the value type.
fn is_documented_code(code: &str) -> bool {
    let name = code.split_once(',').map_or(code, |(n, _)| n);
    CODE_NAMES.contains(&name)
        || matches!(
            name,
            "Week" | "Week Year" | "Week Year4" | "Weekday" | "Text" | "Comma" | "Day Of Year" | "Era" | "Second"
        )
}

/// The ISO 8601 week number of the day `day` (day 1 is 0001-01-01, a Monday).
fn iso_week(day: i32) -> i32 {
    let thursday = day + (4 - ((day - 1).rem_euclid(7) + 1));
    let (y, _, _) = civil_from_day(thursday);
    (thursday - crate::format::day_from_civil(y, 1, 1)) / 7 + 1
}

/// The name `<Month Text>` or `<Weekday Text>` shows.
fn code_name(base: &str, date: Option<(i32, u32, u32)>, day_no: Option<i32>, language: i32) -> Result<String, String> {
    // The month names follow `GlobalLanguage`: German is pinned (platform/lt-platform-members/PlTSession.al:
    // GlobalLanguage_SetAndRestore, `März`); another language than English and German is not.
    if language != 1033 && language != 1031 {
        return Err(format!("{base} in the language {language} (not pinned)"));
    }
    let weekday = (day_no.unwrap_or(1) - 1).rem_euclid(7);
    Ok(match (base, date) {
        ("Month Text", Some((_, m, _))) if language == 1031 => MONTHS_DE[m as usize - 1].to_string(),
        // German weekday names: Monday `Montag` and Thursday `Donnerstag` are pinned (types/lt-format-codes-textbuilder-pagerun-labels/
        // P144DateTextWidth.al:165-180); the other days are not.
        (_, Some(_)) if language == 1031 => match weekday {
            0 => "Montag".to_string(),
            3 => "Donnerstag".to_string(),
            _ => {
                return Err(format!(
                    "{base} in the language {language} for weekday {} (not pinned)",
                    weekday + 1
                ));
            }
        },
        ("Month Text", Some((_, m, _))) => MONTHS[m as usize - 1].to_string(),
        (_, Some(_)) => WEEKDAYS[weekday as usize].to_string(),
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
            (day.clone(), "<Seconds,2>"),
            (day.clone(), "<Day,5>"),
            (day.clone(), "<Year4"),
            // Widths no corpus test pins (AjScalars.al:225-327 pins 3 and 4 for the day, 4 for the month, 5 and 4 for the hours).
            (day.clone(), "<Hours24,2>"),
            (day.clone(), "<Hours12,5>"),
            (day.clone(), "<Month,3>"),
            (day.clone(), "<Filler Character,x><Day,4>"),
            (Value::DateTime(1), "<Day,3>"),
            (Value::DateTime(1), "<Hours24,4>"),
            (Value::DateTime(1), "<Minutes,4>"),
            (Value::Time(1), "<Hours24,3>"),
            (Value::Time(1), "<Hours12,5>"),
            (Value::Time(1), "<Day,3>"),
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
            // types/lt-variant-time-enum/VariantTextTimeEnumFilter.al:105,120: midnight of a Time is ` 0`.
            " 0:00:00"
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
    // types/lt-format-week-code/FormatWeekCodeIso.al:21-123 and types/lt-datetime/DateFuncs.al:415-433 (`<Week>`, `<Weekday>`),
    // types/lt-quarter-duration/FormatQuarterDuration.al:30-110 (`<Quarter>`).
    #[test]
    fn week_weekday_and_quarter_codes() {
        let d = |y, m, dd| Value::Date(day_from_civil(y, m, dd), false);
        assert_eq!(text(fmt(d(2021, 1, 4), "<Week>")), "1");
        assert_eq!(text(fmt(d(2021, 1, 4), "<Week,2>")), "01");
        assert_eq!(text(fmt(d(2020, 12, 31), "<Week,2>")), "53");
        assert_eq!(text(fmt(d(2021, 1, 3), "<Week>.<Year4>")), "53.2021");
        assert_eq!(text(fmt(d(2024, 12, 30), "<Week>.<Year4>")), "1.2024");
        assert_eq!(text(fmt(d(2026, 1, 1), "<Week> <Day> <Month>")), "1 1 1");
        assert_eq!(text(fmt(d(2021, 1, 3), "<Week>-<Weekday>")), "53-7");
        assert_eq!(text(fmt(d(2024, 1, 17), "<Weekday>")), "3");
        assert_eq!(text(fmt(d(2024, 1, 17), "<Week>")), "3");
        // A week and a weekday no corpus test pins are flagged passes.
        use crate::assumptions::{Assumption, take_hits};
        take_hits();
        assert_eq!(text(fmt(d(2022, 2, 14), "<Week>")), "7");
        assert_eq!(take_hits(), vec![Assumption::FormatWeekCodeIso]);
        assert_eq!(text(fmt(d(2022, 2, 14), "<Weekday>")), "1");
        assert_eq!(take_hits(), vec![Assumption::FormatWeekCodeIso]);
        assert_eq!(text(fmt(d(2022, 3, 7), "<Week,2>")), "10");
        assert!(take_hits().is_empty());
        assert_eq!(text(fmt(d(2026, 5, 15), "<Quarter>/<Year>")), "2/26");
        assert_eq!(text(fmt(d(2026, 5, 15), "<quarter>")), "2");
        assert_eq!(text(fmt(d(2026, 5, 15), "<Quarter><Quarter>")), "22");
        assert_eq!(text(fmt(d(2026, 3, 31), "<Quarter>")), "1");
        assert_eq!(text(fmt(Value::Date(0, false), "<Quarter>")), "");
        assert_eq!(
            text(fmt(Value::Date(day_from_civil(2026, 12, 31), true), "<Quarter>")),
            "4"
        );
    }

    // text/lt-text-functions/TextFormatEvaluate.Codeunit.al:48,67; text/lt-label-textbuilder/TxCLabel.al:632,639;
    // types/lt-ax-text-format-dates/AxFillerBuilderAutoFormat.al:101-136.
    #[test]
    fn text_code_on_a_text() {
        assert_eq!(text(fmt(t("a b"), "<Text>")), "a b");
        assert_eq!(text(fmt(t("ab"), "<Text,5>")), "ab   ");
        assert_eq!(text(fmt(t("ab"), "<Text,5><Filler Character,0>")), "ab000");
        assert_eq!(text(fmt(t("ab"), "<Text,5><Filler Character,*>")), "ab***");
        // Cutting a longer text is not pinned.
        assert!(matches!(fmt(t("abcdefg"), "<Text,5>"), Err(Signal::Unsupported(_))));
    }
    // types/lt-datetime/Format.al:134-199 (`<Day,3>`, `<Day Text>`, `<Foo>`), :412-445 (`<Foo>` on a DateTime), :473,499 and
    // types/lt-duration-sum-typed/DurationSumTyped.al:86,95 (codes on a Duration).
    #[test]
    fn day_width_three_and_invalid_codes_on_dates_and_durations() {
        let d = Value::Date(day_from_civil(2024, 3, 5), false);
        assert_eq!(text(fmt(d.clone(), "<Day,3>")), "005");
        for bad in ["<Day Text>", "<Foo>"] {
            assert!(
                matches!(fmt(d.clone(), bad), Err(Signal::Error(ref e)) if e.code == "InvalidFormatString"),
                "{bad}"
            );
        }
        assert!(
            matches!(fmt(Value::DateTime(1), "<Foo>"), Err(Signal::Error(ref e)) if e.code == "InvalidFormatString")
        );
        for bad in ["<Integer Thousand>", "<Hours24,2>:<Minutes,2>"] {
            let r = fmt(Value::Duration(5_400_000), bad);
            assert!(
                matches!(r, Err(Signal::Error(ref e)) if e.code == "InvalidFormatString"),
                "{bad}"
            );
        }
    }
    // types/lt-aj-probes/AjScalars.al:225-327 (Format_FillerCharacter_On*): widths and fillers on a Date, a DateTime and a Time.
    #[test]
    fn widths_and_fillers_on_dates_datetimes_and_times() {
        let day = day_from_civil(2024, 3, 5);
        let clock = ((9 * 60 + 7) * 60 + 2) * 1000 + 123;
        let dt = Value::DateTime(i64::from(day - 1) * 86_400_000 + clock + 1);
        let d = Value::Date(day, false);
        let tm = Value::Time(clock + 1);
        let invalid =
            |r: Result<Value, Signal>| matches!(r, Err(Signal::Error(ref e)) if e.code == "InvalidFormatString");
        for v in [dt.clone(), d.clone()] {
            assert_eq!(text(fmt(v.clone(), "<Filler Character,0><Day,4>")), "0005");
            assert_eq!(text(fmt(v.clone(), "<Day,4><Filler Character,0>")), "0005");
            assert_eq!(
                text(fmt(v.clone(), "<Filler Character,0><Day,2><Month,4><Year4>")),
                "0500032024"
            );
            assert_eq!(text(fmt(v.clone(), "<Filler Character,#><Day,4>")), "0005");
            assert_eq!(text(fmt(v.clone(), "<Day,4><Filler Character,0><Month,4>")), "00050003");
        }
        assert_eq!(
            text(fmt(dt.clone(), "<Filler Character,0><Hours24,5><Minutes,5>")),
            "0000900007"
        );
        assert_eq!(text(fmt(dt.clone(), "<Filler Character,*><Hours12,4>")), "0009");
        assert!(invalid(fmt(d.clone(), "<Filler Character,0><Hours24,5><Minutes,5>")));
        assert!(invalid(fmt(d.clone(), "<Filler Character,*><Hours12,4>")));
        for f in [
            "<Filler Character,0><Day,4>",
            "<Day,4><Filler Character,0>",
            "<Filler Character,#><Day,4>",
        ] {
            assert!(invalid(fmt(tm.clone(), f)), "{f}");
        }
        assert!(invalid(fmt(tm.clone(), "<Day,4><Filler Character,0><Month,4>")));
        assert!(invalid(fmt(tm.clone(), "<Filler Character,0><Day,2><Month,4><Year4>")));
        assert_eq!(
            text(fmt(tm.clone(), "<Filler Character,0><Hours24,5><Minutes,5>")),
            "    900007"
        );
        assert_eq!(text(fmt(tm.clone(), "<Filler Character,*><Hours12,4>")), "   9");
    }
    // types/lt-variant-time-enum/VariantTextTimeEnumFilter.al:101-135; types/lt-scalars/ScT70052.al:182,197.
    #[test]
    fn hours_and_minutes_of_a_time_and_codes_on_a_byte() {
        let at = |h: i64, m: i64| Value::Time((h * 60 + m) * 60_000 + 1);
        assert_eq!(text(fmt(at(0, 0), "<Hours24,2>")), " 0");
        assert_eq!(text(fmt(at(12, 0), "<Minutes>")), "0");
        assert_eq!(text(fmt(at(12, 5), "<Minutes>")), "5");
        assert_eq!(text(fmt(at(12, 10), "<Minutes>")), "10");
        let r = fmt(Value::Byte(152), "<Integer,6><Filler Character,0>");
        assert!(
            matches!(r, Err(Signal::Error(ref e)) if e.code == "InvalidFormatString"),
            "{r:?}"
        );
    }
    // P144DateTextWidth.al:165-180: German weekday names.
    #[test]
    fn german_weekday_names_that_are_pinned() {
        use super::code_name;
        let mon = day_from_civil(2027, 3, 15);
        let tue = mon + 1;
        assert_eq!(
            code_name("Weekday Text", Some((2027, 3, 15)), Some(mon), 1031).unwrap(),
            "Montag"
        );
        assert_eq!(
            code_name("Weekday Text", Some((2027, 3, 18)), Some(mon + 3), 1031).unwrap(),
            "Donnerstag"
        );
        assert!(code_name("Weekday Text", Some((2027, 3, 16)), Some(tue), 1031).is_err());
    }
    #[test]
    fn decimals_of_width_one_on_a_value_with_decimals_is_a_star() {
        // types/lt-scalars/ScT70051.al:105,115.
        let d = Value::Decimal("1234567.891".parse().unwrap());
        assert_eq!(text(fmt(d, "<Integer><Decimals,1>")), "1234567*");
    }
    #[test]
    fn a_blank_filler_character_pads_with_blanks() {
        // types/lt-scalars/ScT70052.al:83-84,93-94.
        assert_eq!(
            text(fmt(Value::Integer(1234567), "<Integer,10><Filler Character, >")),
            "   1234567"
        );
        assert_eq!(
            text(fmt(Value::Integer(1234567), "<Integer,10><Filler Character,0>")),
            "0001234567"
        );
    }
}
