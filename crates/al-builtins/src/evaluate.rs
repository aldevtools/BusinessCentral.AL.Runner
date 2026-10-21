//! `Evaluate(var Variable, String [, Number])`.
//!
//! Only the (type, text form) pairs a corpus test pins are accepted; text that
//! plainly is not a value of the type (no digits for a number, empty) fails the
//! way the corpus shows, and every other form raises `Signal::Unsupported`.

use crate::describe::Describe;
use crate::format::day_from_civil;
use crate::native::{Ctx, Signal};
use crate::value::Value;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Whether `t` is a short list of numbers: two or three pieces of digits (one may hold a point) or nothing, with blanks around them,
/// joined by one of `;`, `|`, `:`, `/` or `&` used throughout. Format number 0 rejects such a text for Decimal, Integer and BigInteger
/// (types/lt-bl-evaluate-lists/EvaluateLists.al:13-29 Inputs, :48-142: `1;5;10`, `1;`, `;1`, `;`, `1|5|10`, `1:5`, `1/5`, `1&5`,
/// `1;5.5`, ` 1 ; 5 `). Longer lists and mixed separators are not pinned.
fn number_list(t: &str) -> bool {
    let Some(sep) = t.chars().find(|c| matches!(c, ';' | '|' | ':' | '/' | '&')) else {
        return false;
    };
    let pieces: Vec<&str> = t.split(sep).collect();
    (2..=3).contains(&pieces.len())
        && pieces.iter().all(|p| {
            let p = p.trim_matches(' ');
            p.is_empty()
                || (p.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                    && p.bytes().filter(|b| *b == b'.').count() <= 1
                    && p.bytes().any(|b| b.is_ascii_digit()))
        })
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// What parsing one text produced.
enum Parsed {
    Value(Value),
    /// The text is not a value of the type: `Evaluate` returns false.
    Rejected,
}

/// A whole number as `Evaluate` reads it into an Integer. Spaces around it are dropped, one sign is allowed and the
/// digits are ASCII; number 0 also reads properly grouped thousands (`1,234`), number 9 does not. Everything else is
/// rejected, an overflow too (types/lt-evaluate-format-lasterror-setview/P86Eval.al, codeunit "LT P86 Eval Integer":
/// `+1`, `007`, `-0`, `1 234`, `1.5`, `1e3`, `0x1F`, Arabic digits, `1-`, `--1`, `2147483648`).
fn integer(text: &str, number: i64) -> Result<Parsed, Signal> {
    // Number 0 reads the text as a Decimal and takes it when it is whole: `1,5` is 15, `1,000` is 1000, `1.0` is 1 and `1E3` is
    // 1000, while `1.5` is rejected (types/lt-scalars/ScT70055.al:EvaluateInteger_Invalid `comma`, `thousand`, `exp`,
    // `decimal0`, `decimal`, `hex`, `two_signs`, `bigval`; P86Eval.al "LT P86 Eval Integer").
    if number == 0 && text.is_ascii() && !text.contains('$') && text.chars().all(|c| !c.is_whitespace() || c == ' ') {
        return Ok(match decimal(text, 0)? {
            Parsed::Value(Value::Decimal(d)) if d.fract().is_zero() => {
                match d.to_i64().and_then(|n| i32::try_from(n).ok()).ok_or(()) {
                    Ok(n) => Parsed::Value(Value::Integer(n)),
                    Err(_) => Parsed::Rejected,
                }
            }
            _ => Parsed::Rejected,
        });
    }
    if text.chars().any(|c| c.is_whitespace() && c != ' ') {
        return unsupported(format!(
            "Evaluate of `{text}` into Integer with white space other than spaces (not pinned)"
        ));
    }
    let t = text.trim_matches(' ');
    let (negative, digits) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let ungrouped;
    let digits = if digits.contains(',') {
        let groups: Vec<&str> = digits.split(',').collect();
        let grouped = (1..=3).contains(&groups[0].len())
            && groups.iter().all(|g| g.bytes().all(|b| b.is_ascii_digit()))
            && groups[1..].iter().all(|g| g.len() == 3);
        if number != 0 {
            return Ok(Parsed::Rejected);
        }
        if !grouped {
            return unsupported(format!("Evaluate of `{text}` into Integer (grouping not pinned)"));
        }
        ungrouped = groups.concat();
        ungrouped.as_str()
    } else {
        digits
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(Parsed::Rejected);
    }
    let magnitude = digits.parse::<i64>().ok();
    Ok(
        match magnitude
            .map(|m| if negative { -m } else { m })
            .and_then(|n| i32::try_from(n).ok())
        {
            Some(n) => Parsed::Value(Value::Integer(n)),
            None => Parsed::Rejected,
        },
    )
}

/// A BigInteger as `Evaluate` reads it: spaces around it, one sign (a `+` may be followed by blanks: `+ 7` is 7, while an Integer
/// rejects it), digits with commas dropped (`1,000` is 1000, `1,00` is 100), and a fraction of zeros (`12.0`, `110.0`). A letter,
/// `%`, `_`, an embedded blank, a fraction that is not zero, no digit and a value outside the 64 bit range are rejected
/// (types/lt-scalars/ScT70055.al:EvaluateBigInteger; errors/lt-ms-refusals-129/P129Evaluate.al:
/// P129_Evaluate_IntegerAndBigInteger_BlanksAndOddForms; the BigInteger cases of types/TestProbeBigInteger: `%PDF-1.4`).
/// An exponent, commas in format number 9, a `-` followed by blanks and a point with no fraction are not pinned.
fn big_integer(text: &str, number: i64) -> Result<Parsed, Signal> {
    if text.chars().any(|c| c.is_whitespace() && c != ' ') {
        return unsupported(format!(
            "Evaluate of `{text}` into BigInteger with white space other than spaces (not pinned)"
        ));
    }
    let t = text.trim_matches(' ');
    let exponent_shape = t.bytes().any(|b| b == b'e' || b == b'E')
        && t.bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'e' | b'E' | b'+' | b'-' | b'.'));
    if exponent_shape {
        return unsupported(format!(
            "Evaluate of `{text}` into BigInteger with an exponent (not pinned)"
        ));
    }
    if t.bytes().any(|b| b.is_ascii_alphabetic() || matches!(b, b'%' | b'_')) || (number == 0 && number_list(t)) {
        return Ok(Parsed::Rejected);
    }
    if !t.is_ascii()
        || t.bytes()
            .any(|b| !(b.is_ascii_digit() || matches!(b, b',' | b'.' | b'+' | b'-' | b' ')))
    {
        return unsupported(format!(
            "Evaluate of `{text}` into BigInteger (form not pinned by the corpus)"
        ));
    }
    let (negative, rest) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').map_or(t, |r| r.trim_start_matches(' '))),
    };
    if negative && rest.starts_with(' ') {
        return unsupported(format!(
            "Evaluate of `{text}` into BigInteger with blanks after the minus sign (not pinned)"
        ));
    }
    if rest.contains(' ') || rest.contains(['+', '-']) {
        return Ok(Parsed::Rejected);
    }
    let (int, frac) = match rest.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (rest, None),
    };
    if int.contains(',') && number == 9 {
        return unsupported(format!(
            "Evaluate of `{text}` into BigInteger with commas and format number 9 (not pinned)"
        ));
    }
    let int: String = int.chars().filter(|c| *c != ',').collect();
    match frac {
        Some("") => {
            return unsupported(format!(
                "Evaluate of `{text}` into BigInteger with a point and no fraction (not pinned)"
            ));
        }
        Some(f) if f.contains([',', '.']) => return Ok(Parsed::Rejected),
        Some(f) if f.bytes().any(|b| b != b'0') => return Ok(Parsed::Rejected),
        _ => {}
    }
    if int.is_empty() {
        return Ok(Parsed::Rejected);
    }
    Ok(
        match format!("{}{int}", if negative { "-" } else { "" }).parse::<i64>() {
            Ok(n) => Parsed::Value(Value::BigInteger(n)),
            Err(_) => Parsed::Rejected,
        },
    )
}

/// A Decimal as `Evaluate` reads it. Number 0 takes `[sign] digits [. digits] [e [sign] digits]` with spaces around it;
/// commas in the integer part are dropped wherever they stand (`12,5` is 125, `12,5.5` is 125.5), a comma after the point
/// rejects, and `$` in front is a currency sign (`$5` is 5). Number 9 does not take commas.
/// Pins: types/lt-evaluate-decimal-text/EvalDecimalEn.al:P175_En_ReturnValueAndLeftBehind and `...ret9[...]` (EvalDecimalDe.al
/// the same in German), types/lt-scalars/ScT70055.al:EvaluateDecimal_Valid and EvaluateDecimal_Invalid
/// (`+1.5`, `.5`, `5.`, `1E3`, `1.5e-2`, `1E`, `1E40`, `1.2.3`, `.`, `50%`, `$5`, `NaN`), types/lt-evaluate-text-decimal.
fn decimal(text: &str, number: i64) -> Result<Parsed, Signal> {
    if text.chars().any(|c| c.is_whitespace() && c != ' ') {
        return unsupported(format!(
            "Evaluate of `{text}` into Decimal with white space other than spaces (not pinned)"
        ));
    }
    let t = text.trim_matches(' ');
    if t.contains(' ') {
        // `1 3` (types/lt-evaluate-decimal-text via the oracle, P129Evaluate.al).
        return Ok(Parsed::Rejected);
    }
    if number == 0 && number_list(t) {
        return Ok(Parsed::Rejected);
    }
    // A trailing minus sign and parentheses mark a negative number: `100-` and `(100)` read as -100
    // (types/lt-evaluate-decimal/EvaluateDecimal.al:Evaluate_SignsBlanksAndGarbage `trailing_minus`, `parentheses`).
    // Pinned for a plain number (digits, commas, a point) with format number 0 only; other forms are refused.
    let plain = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit() || b == b',' || b == b'.');
    let trailing = t.strip_suffix('-').filter(|b| plain(b));
    let wrapped = t
        .strip_prefix('(')
        .and_then(|r| r.strip_suffix(')'))
        .filter(|b| plain(b));
    if let Some(body) = trailing.or(wrapped) {
        if number != 0 {
            return unsupported(format!(
                "Evaluate of `{text}` into Decimal with a trailing minus sign or parentheses and format number {number} (not pinned)"
            ));
        }
        return match decimal(body, number)? {
            Parsed::Value(Value::Decimal(d)) => Ok(Parsed::Value(Value::Decimal(-d))),
            other => Ok(other),
        };
    }
    let (currency, t) = match t.strip_prefix('$') {
        Some(rest) if number == 0 => (true, rest),
        _ => (false, t),
    };
    let (negative, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    // Split off the exponent.
    let (mant, exp) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], Some(&t[i + 1..])),
        None => (t, None),
    };
    if !mant.bytes().all(|b| b.is_ascii_digit() || b == b',' || b == b'.') {
        // Letters, `%` and the like are no part of a number (`12abc`, `12.5pt`, `50%`, `NaN`, `%PDF-1.4`); other
        // characters are not pinned.
        return if t
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'%' | b'_' | b'-' | b'.' | b',' | b'+' | b'$'))
        {
            Ok(Parsed::Rejected)
        } else {
            unsupported(format!(
                "Evaluate of `{text}` into Decimal (form not pinned by the corpus)"
            ))
        };
    }
    let (int, frac) = match mant.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mant, ""),
    };
    if frac.contains(['.', ',']) || (number == 9 && int.contains(',')) {
        return Ok(Parsed::Rejected);
    }
    let int: String = int.chars().filter(|c| *c != ',').collect();
    if int.is_empty() && frac.is_empty() {
        return Ok(Parsed::Rejected);
    }
    if currency && negative {
        return unsupported(format!(
            "Evaluate of `{text}` into Decimal (sign and currency sign together are not pinned)"
        ));
    }
    let exp: i64 = match exp {
        None => 0,
        Some(e) => {
            let (eneg, ed) = match e.strip_prefix('-') {
                Some(r) => (true, r),
                None => (false, e.strip_prefix('+').unwrap_or(e)),
            };
            if ed.is_empty() || !ed.bytes().all(|b| b.is_ascii_digit()) {
                return Ok(Parsed::Rejected);
            }
            match ed.parse::<i64>() {
                Ok(v) if v < 100_000 => {
                    if eneg {
                        -v
                    } else {
                        v
                    }
                }
                _ => {
                    return unsupported(format!(
                        "Evaluate of `{text}` into Decimal with a huge exponent (not pinned)"
                    ));
                }
            }
        }
    };
    let all = format!("{int}{frac}");
    let lead = all.bytes().take_while(|b| *b == b'0').count();
    if lead == all.len() {
        return Ok(Parsed::Value(Value::Decimal(Decimal::ZERO)));
    }
    // More than 17 digits before the point do not evaluate (`123456789012345678` fails, `123456789012.345678` reads;
    // `1E40` fails); a value below 1E-28 reads as zero (types/TestProbeStreamContracts.al:WriteDecimal_ManyDigits,
    // `19_digits_text`, `1e_minus_29`).
    let int_digits = int.len() as i64 + exp - lead as i64;
    if int_digits >= 18 {
        return Ok(Parsed::Rejected);
    }
    // 16 and 17 digits before the point do not evaluate either, with or without a fraction, a plus sign or format number 9;
    // 15 do (types/lt-evaluate-wide-decimal/EvaluateWideDecimal.al:Evaluate_Decimal_IntegerDigits d15 against d16, _Fraction, _Negative).
    // The same size written with an exponent is not pinned.
    if int_digits >= 16 {
        if exp != 0 {
            return unsupported(format!(
                "Evaluate of `{text}` into Decimal with an exponent and 16 or 17 digits before the point (not pinned)"
            ));
        }
        return Ok(Parsed::Rejected);
    }
    // With an integer part of zero, up to 29 fractional digits read (types/lt-decimal-digits/DecimalDigits.al:
    // P163_Decimal_Operands_RoundTrip `19_decimals`, `28_decimals`, `29_decimals`; Format shows 18 of them).
    // With a nonzero integer part, only 18 significant digits are pinned: more is refused.
    // With a nonzero integer part the digits past the 18th are dropped: `1.123456789012345678901` reads as 1.12345678901234567
    // (misc/lt-system-functions/EvalLastError.al:Evaluate_Decimal `many decimals`). Pinned without an exponent only.
    let mut frac = frac;
    if all.len() - lead > 18 && int_digits > 0 {
        if exp != 0 {
            return unsupported(format!(
                "Evaluate of `{text}` into Decimal with more than 18 significant digits and an exponent (not pinned)"
            ));
        }
        frac = &frac[..(18 - int_digits as usize).min(frac.len())];
    }
    // With an integer part of zero, the digits past the 18th fractional one are dropped, not rounded: `0.1234567890123456789`
    // and the 28 and 29 digit forms read as 0.123456789012345678 (types/lt-evaluate-decimal/EvaluateDecimal.al:37 and :50,
    // types/lt-decimal-digits/DecimalDigits.al:P163_Decimal_Operands_RoundTrip, AddSubtract_KeepsDigits). Pinned with a first
    // fractional digit other than zero and no exponent only; other forms keep every digit as before.
    if int_digits <= 0 && frac.len() > 18 && exp == 0 && !frac.starts_with('0') {
        frac = &frac[..18];
    }
    let sign = if negative { "-" } else { "" };
    let mantissa = match (int.is_empty(), frac.is_empty()) {
        (_, true) => format!("{sign}{int}"),
        (true, false) => format!("{sign}0.{frac}"),
        (false, false) => format!("{sign}{int}.{frac}"),
    };
    match Decimal::from_scientific(&format!("{mantissa}e{exp}")) {
        Ok(d) => Ok(Parsed::Value(Value::Decimal(d))),
        Err(_) if int_digits <= 0 => Ok(Parsed::Value(Value::Decimal(Decimal::ZERO))),
        Err(_) => unsupported(format!("Evaluate of `{text}` outside the Decimal range")),
    }
}

fn boolean(text: &str, number: i64) -> Result<Parsed, Signal> {
    // Format number 9 reads `true`, `false`, `1` and `0`, and rejects `Yes`, `No` and the empty text (misc/lt-system-functions/
    // EvalLastError.al:Evaluate_Boolean_Format9); other texts are read as with number 0 and `Maybe`-like garbage is rejected.
    if number == 9 {
        match text {
            "true" | "1" => return Ok(Parsed::Value(Value::Boolean(true))),
            "false" | "0" => return Ok(Parsed::Value(Value::Boolean(false))),
            "Yes" | "No" | "" => return Ok(Parsed::Rejected),
            _ => {}
        }
    }
    // text/TestEvaluate.al: 'true' / 'false'; types/TestFormatBooleanContracts.al:112
    // evaluates Format(true), which is 'Yes' (Format(false) is 'No'). In any case, `1` and `T` are
    // true and anything else, `Ja` for one, is refused
    // (types/TestFormatEvaluateTextContracts.al:Evaluate_Boolean_AcceptsTrueFalseYesNoOneAndT_IgnoringCase).
    // types/TestProbeScalarContracts.al:Evaluate_BooleanTexts: `0 F f 1 T t true TRUE false Yes no Y N y n` read, with
    // surrounding blanks; `2`, the empty text, `on` and `Ja` do not.
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "t" | "y" => Ok(Parsed::Value(Value::Boolean(true))),
        "false" | "no" | "0" | "f" | "n" => Ok(Parsed::Value(Value::Boolean(false))),
        // `1.0` reads as true (types/lt-boolean-format/Boolean_EvaluateFromText `1.0 = ok/1`); `0.0` and other numbers are not pinned.
        t if t
            .strip_prefix("1.")
            .is_some_and(|r| !r.is_empty() && r.bytes().all(|c| c == b'0')) =>
        {
            Ok(Parsed::Value(Value::Boolean(true)))
        }
        _ => Ok(Parsed::Rejected),
    }
}

fn guid(text: &str) -> Result<Parsed, Signal> {
    // Blanks around the text do not count (types/lt-scalars/ScT70054.al:Guid_EvaluateInvalid `spaces`).
    let text = text.trim_matches(' ');
    // The braced, the plain (36 characters) and the 32 digit forms; anything else, the empty text
    // included, is rejected (types/TestJsonTokenUnassignedEvaluateGuidContracts.al:EvaluateGuid_Forms_AndFailures).
    // Parentheses wrap it like braces (types/TestProbeScalarContracts.al:Guid_EvaluateAndFormat).
    let inner = text
        .strip_prefix('{')
        .and_then(|t| t.strip_suffix('}'))
        .or_else(|| text.strip_prefix('(').and_then(|t| t.strip_suffix(')')))
        .unwrap_or(text);
    let parts: Vec<&str> = inner.split('-').collect();
    let hex_only = |p: &str| p.bytes().all(|b| b.is_ascii_hexdigit());
    let hex: String = if parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, w)| p.len() == w && hex_only(p))
    {
        parts.concat()
    } else if parts.len() == 1 && inner.len() == 32 && hex_only(inner) && inner.len() == text.len() {
        inner.to_string()
    } else {
        return Ok(Parsed::Rejected);
    };
    let mut out = [0u8; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex digits checked");
    }
    Ok(Parsed::Value(Value::Guid(out)))
}

/// Number 0: the inverse of `Format(Date)` as the runtime formats it (`MM/DD/YY`,
/// docs/probes-needed.md); a two-digit year below 50 is 20yy and from 50 on 19yy. Number 9: ISO `yyyy-MM-dd`.
/// `today` is the host's current day, for the text `T`.
///
/// Pinned by types/lt-evaluate-format-lasterror-setview/P86Eval.al, codeunit "LT P86 Eval Date": number 0 reads
/// `yyyy-mm-dd`, `m/d/yyyy`, `m/d/yy` and `T`; it rejects `15.01.2024`, `150124`, `15012024`, `20240115`,
/// `2024-01-15T00:00:00Z`, `0D` and `15-01-2024`. Number 9 reads `yyyy-mm-dd` with spaces around it and the empty text
/// (which leaves the date empty) and rejects everything else, `T` and the slash forms included.
/// The month-first forms of a Date in the session's language (en-US): digits only, as `MMDD` followed by a year of 0 to 4 digits
/// (`1010` is that day of this year, `10101` is 2001-10-10, `101010` is 2010-10-10, `1010101` is 0101-10-10, `10162026`), and three numbers
/// split by one `/`, `.` or `-` (`1/2/3`, `10.16.26`, `10-16-2026`), month first. A year of one or two digits is 20xx below 50 and 19xx
/// from 50. A month or day that does not exist rejects (`161026`, `311299`, `16/10/26`, `16.10.2026`). `16 Oct 2026` reads, while
/// `Oct 16, 2026`, `16 10 26` and `16/10` do not (errors/lt-ms-refusals-130/P130EvaluateDate.al: Default_SixAndEightDigits,
/// Default_SlashAndDotForms; types/lt-datetime/Evaluate.al: Date_Other_Formats, Date_Default_Evaluate `formatted 4`). `None` for
/// every other shape, which the caller handles; a shape the corpus does not pin is refused.
fn date_month_first(text: &str, today: Option<i32>) -> Option<Result<Parsed, Signal>> {
    let ymd = |y: i32, m: u32, d: u32| -> Result<Parsed, Signal> {
        if !(1..=12).contains(&m) || d < 1 {
            return Ok(Parsed::Rejected);
        }
        let first = day_from_civil(y, m, 1);
        let next = if m == 12 {
            day_from_civil(y + 1, 1, 1)
        } else {
            day_from_civil(y, m + 1, 1)
        };
        if (d as i32) > next - first {
            return Ok(Parsed::Rejected);
        }
        Ok(Parsed::Value(Value::Date(day_from_civil(y, m, d), false)))
    };
    let year_of = |ys: &str| -> Result<Option<i32>, Signal> {
        let n: i32 = ys.parse().unwrap_or(0);
        match ys.len() {
            1 | 2 => Ok(Some(if n < 50 { 2000 + n } else { 1900 + n })),
            3 | 4 if n >= 1 => Ok(Some(n)),
            _ => unsupported(format!("Evaluate into a Date of a year written `{ys}` (not pinned)")),
        }
    };
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    // `0` and `16/10` are no dates (P130_EvaluateDate_Default_SixAndEightDigits `0`, Default_SlashAndDotForms `16/10`); other
    // texts of one to three digits, and two numbers with another separator, are not pinned.
    if text == "0" {
        return Some(Ok(Parsed::Rejected));
    }
    if let Some((a, b)) = text.split_once('/') {
        if all_digits(a) && all_digits(b) && a.len() <= 2 && b.len() <= 2 {
            return Some(Ok(Parsed::Rejected));
        }
    }
    if all_digits(text) && (4..=8).contains(&text.len()) {
        let (m, d): (u32, u32) = (text[0..2].parse().ok()?, text[2..4].parse().ok()?);
        let ys = &text[4..];
        let y = if ys.is_empty() {
            match today {
                Some(t) => crate::format::civil_from_day(t).0,
                None => return Some(unsupported("Evaluate of a month and day into a Date without a clock")),
            }
        } else {
            match year_of(ys) {
                Ok(Some(y)) => y,
                Ok(None) => return Some(Ok(Parsed::Rejected)),
                Err(e) => return Some(Err(e)),
            }
        };
        return Some(ymd(y, m, d));
    }
    for sep in ['/', '.', '-'] {
        let parts: Vec<&str> = text.split(sep).collect();
        if parts.len() == 3 && parts.iter().all(|p| all_digits(p) && p.len() <= 4) {
            if parts[0].len() > 2 || parts[1].len() > 2 {
                // Year first (`2024-1-5`) or a longer number: not a month-first form.
                return None;
            }
            let (m, d): (u32, u32) = (parts[0].parse().ok()?, parts[1].parse().ok()?);
            return Some(match year_of(parts[2]) {
                Ok(Some(y)) => ymd(y, m, d),
                Ok(None) => Ok(Parsed::Rejected),
                Err(e) => Err(e),
            });
        }
    }
    // A year of five digits after slashes is no date (types/lt-evaluate-date-3digit-year/EvaluateDate3DigitYear.al:
    // Evaluate_Date_ThreeDigitYear_English `en_slash_5digit` `01/02/12026`, also German and British English).
    {
        let parts: Vec<&str> = text.split('/').collect();
        if parts.len() == 3
            && parts.iter().all(|p| all_digits(p))
            && parts[0].len() <= 2
            && parts[1].len() <= 2
            && parts[2].len() == 5
        {
            return Some(Ok(Parsed::Rejected));
        }
    }
    let words: Vec<&str> = text.split(' ').collect();
    if words.len() == 3 && all_digits(words[0]) && all_digits(words[1]) && all_digits(words[2]) {
        return Some(Ok(Parsed::Rejected));
    }
    if words.len() == 3
        && all_digits(words[0])
        && words[0].len() <= 2
        && words[1].len() == 3
        && words[1].bytes().all(|b| b.is_ascii_alphabetic())
        && words[2].len() == 4
        && all_digits(words[2])
    {
        const MONTHS: [&str; 12] = [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
        ];
        let m = MONTHS.iter().position(|n| n.eq_ignore_ascii_case(words[1]))? as u32 + 1;
        return Some(ymd(words[2].parse().ok()?, m, words[0].parse().ok()?));
    }
    // `Oct 16, 2026` and `March 5, 2024`: a month name, a day with a comma and a year.
    if words.len() == 3
        && words[0].bytes().all(|b| b.is_ascii_alphabetic())
        && words[0].len() >= 3
        && words[1].ends_with(',')
        && all_digits(&words[1][..words[1].len() - 1])
        && all_digits(words[2])
    {
        return Some(Ok(Parsed::Rejected));
    }
    None
}

/// `month_first` turns on the month-first forms (`date_month_first`) and the stricter closing date, as `Evaluate` reads a Date; a
/// reader of filter text keeps the forms it had (the filter forms are pinned apart from Evaluate, docs/probes-needed.md).
fn date(text: &str, number: i64, today: Option<i32>, work: Option<i32>, month_first: bool) -> Result<Parsed, Signal> {
    if number == 9 {
        let t = text.trim_matches(' ');
        if t.is_empty() {
            return Ok(Parsed::Value(Value::Date(0, false)));
        }
        // The first day of the calendar reads as the empty date, and year 0 as the day after it
        // (types/lt-datetime/Evaluate.al:Date_Format9 `iso min`, `iso 0D` and `iso year 0`).
        if t == "0001-01-01" {
            return Ok(Parsed::Value(Value::Date(0, false)));
        }
        if t == "0000-01-01" {
            return Ok(Parsed::Value(Value::Date(2, false)));
        }
        return Ok(
            match crate::iso::parse_date(t).filter(|_| t.len() == 10 && t.as_bytes()[4] == b'-') {
                Some(d) => Parsed::Value(Value::Date(d, false)),
                None => Parsed::Rejected,
            },
        );
    }
    // The empty text leaves the date empty in format 0 as in format 9 (types/lt-datetime/Evaluate.al:Date_Default_Evaluate,
    // oracle: Yes and an empty date).
    if text.is_empty() {
        return Ok(Parsed::Value(Value::Date(0, false)));
    }
    if text.eq_ignore_ascii_case("t") {
        return match today {
            Some(d) => Ok(Parsed::Value(Value::Date(d, false))),
            None => unsupported("Evaluate of `T` into a Date without a clock"),
        };
    }
    // `W` is the work date and `CD` the closing date of today; `C` and a date is that day's closing date
    // (types/lt-in-precedence-date-guid-filters/P143Date_FieldRef.al:P143_EvaluateRelations `eval_w_equals_workdate`,
    // `eval_cd_equals_today` No, `eval_closing_count_on`).
    if text.eq_ignore_ascii_case("w") {
        return match work {
            Some(d) => Ok(Parsed::Value(Value::Date(d, false))),
            None => unsupported("Evaluate of `W` into a Date without a session"),
        };
    }
    if text.eq_ignore_ascii_case("cd") {
        return match today {
            Some(d) => Ok(Parsed::Value(Value::Date(d, true))),
            None => unsupported("Evaluate of `CD` into a Date without a clock"),
        };
    }
    if let Some(rest) = text
        .strip_prefix(['C', 'c'])
        .filter(|r| r.starts_with(|c: char| c.is_ascii_digit()))
    {
        // The ISO form after the C is no closing date (types/lt-datetime/Evaluate.al:Date_Default_Evaluate `closing`: No).
        if month_first && rest.len() == 10 && rest.as_bytes()[4] == b'-' {
            return Ok(Parsed::Rejected);
        }
        return Ok(match date(rest, 0, today, work, month_first)? {
            Parsed::Value(Value::Date(d, false)) => Parsed::Value(Value::Date(d, true)),
            _ => Parsed::Rejected,
        });
    }
    // The ISO form is read in format 0 too (types/TestFormatEvaluateTextContracts.al:Evaluate_Date_TwoDigitYear_PivotsAt50).
    if text.len() == 10 && text.as_bytes()[4] == b'-' {
        return match crate::iso::parse_date(text) {
            Some(d) => Ok(Parsed::Value(Value::Date(d, false))),
            None => Ok(Parsed::Rejected),
        };
    }
    if month_first {
        if let Some(r) = date_month_first(text, today) {
            return r;
        }
    }
    let parts: Vec<&str> = text.split('/').collect();
    let digits = |s: &str, lens: std::ops::RangeInclusive<usize>| {
        lens.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
    };
    if parts.len() == 3
        && digits(parts[0], 1..=2)
        && digits(parts[1], 1..=2)
        && (parts[2].len() == 2 || parts[2].len() == 4)
        && digits(parts[2], 2..=4)
    {
        let (m, d, yy): (u32, u32, i32) = (
            parts[0].parse().unwrap(),
            parts[1].parse().unwrap(),
            parts[2].parse().unwrap(),
        );
        // types/TestFormatEvaluateTextContracts.al:Evaluate_Date_TwoDigitYear_PivotsAt50.
        let y = if parts[2].len() == 4 {
            yy
        } else if yy < 50 {
            2000 + yy
        } else {
            1900 + yy
        };
        let valid = (1..=12).contains(&m) && d >= 1 && {
            let first = day_from_civil(y, m, 1);
            let next = if m == 12 {
                day_from_civil(y + 1, 1, 1)
            } else {
                day_from_civil(y, m + 1, 1)
            };
            (d as i32) <= next - first
        };
        if valid {
            return Ok(Parsed::Value(Value::Date(day_from_civil(y, m, d), false)));
        }
        return Ok(Parsed::Rejected);
    }
    // Text with no digit at all is no date (types/TestFormatEvaluateTextContracts.al:
    // Evaluate_Failed_LeavesTheVariableUnchanged: `garbage`).
    if !text.bytes().any(|b| b.is_ascii_digit()) {
        return Ok(Parsed::Rejected);
    }
    let all_digits = text.bytes().all(|b| b.is_ascii_digit());
    let dotted = text.split('.').count() == 3 && text.split('.').all(|p| digits(p, 1..=4));
    let dashed = text.split('-').count() == 3 && text.split('-').all(|p| digits(p, 1..=4));
    let iso_with_time = text.len() > 10 && text.as_bytes()[4] == b'-' && crate::iso::parse_date(&text[..10]).is_some();
    if (all_digits && matches!(text.len(), 6 | 8))
        || dotted
        || dashed
        || iso_with_time
        || text.eq_ignore_ascii_case("0d")
    {
        return Ok(Parsed::Rejected);
    }
    unsupported(format!(
        "Evaluate of `{text}` into Date (form not pinned by the corpus)"
    ))
}

/// A time of day, as a `Value::Time` holds it (milliseconds since midnight, plus one).
///
/// Pinned by types/lt-evaluate-format-lasterror-setview/P86Eval.al, codeunit "LT P86 Eval Time". Number 0 reads
/// `h:m[:s[.f]]` with one or two digits per part, `h.m.s`, `hhmm`, and `h:m[:s] AM|PM`; it rejects hours from 24, minutes
/// or seconds from 60, a minus sign, a trailing `Z` and `0T`. Number 9 reads exactly `hh:mm:ss[.f]` with an optional `Z`
/// and rejects everything else. Spaces around the text are dropped and the empty text leaves the time empty in both.
fn time(text: &str, number: i64) -> Result<Parsed, Signal> {
    let t = text.trim_matches(' ');
    if t.is_empty() {
        return Ok(Parsed::Value(Value::Time(0)));
    }
    let value = |h: i64, m: i64, s: i64, f: i64| Parsed::Value(Value::Time(((h * 60 + m) * 60 + s) * 1000 + f + 1));
    let num = |s: &str, max_len: usize| -> Option<i64> {
        (!s.is_empty() && s.len() <= max_len && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse().ok())
            .flatten()
    };
    // `.f`: one to three digits, as tenths, hundredths or milliseconds.
    let fraction = |s: &str| -> Option<i64> {
        let n = num(s, 3)?;
        Some(n * 10_i64.pow(3 - s.len() as u32))
    };
    let in_range = |h: i64, m: i64, s: i64| h < 24 && m < 60 && s < 60;
    if number == 9 {
        let body = t.strip_suffix('Z').unwrap_or(t);
        let (clock, frac) = match body.split_once('.') {
            Some((c, f)) => (c, Some(f)),
            None => (body, None),
        };
        let parts: Vec<&str> = clock.split(':').collect();
        let strict = parts.len() == 3 && parts.iter().all(|p| p.len() == 2);
        let f = match frac {
            Some(f) => fraction(f),
            None => Some(0),
        };
        return Ok(
            match (
                strict,
                f,
                parts.iter().map(|p| num(p, 2)).collect::<Vec<_>>()[..].to_vec(),
            ) {
                (true, Some(f), nums) if nums.iter().all(Option::is_some) => {
                    let (h, m, s) = (nums[0].unwrap_or(0), nums[1].unwrap_or(0), nums[2].unwrap_or(0));
                    if in_range(h, m, s) {
                        value(h, m, s, f)
                    } else {
                        Parsed::Rejected
                    }
                }
                _ => Parsed::Rejected,
            },
        );
    }
    if t.starts_with('-') || t.ends_with(['Z', 'z']) || t.eq_ignore_ascii_case("0t") {
        return Ok(Parsed::Rejected);
    }
    if !t.bytes().any(|b| b.is_ascii_digit()) {
        return Ok(Parsed::Rejected);
    }
    let (body, meridiem) = match (t.strip_suffix(" AM"), t.strip_suffix(" PM")) {
        (Some(b), _) => (b, Some(false)),
        (_, Some(b)) => (b, Some(true)),
        _ => (t, None),
    };
    let nums: Option<(i64, i64, i64, i64)> = if body.contains(':') {
        let (clock, frac) = match body.split_once('.') {
            Some((c, f)) => (c, Some(f)),
            None => (body, None),
        };
        let parts: Vec<&str> = clock.split(':').collect();
        match (parts.len(), frac) {
            (2, None) => num(parts[0], 2).zip(num(parts[1], 2)).map(|(h, m)| (h, m, 0, 0)),
            (3, f) => {
                let f = match f {
                    Some(f) => fraction(f),
                    None => Some(0),
                };
                num(parts[0], 2)
                    .zip(num(parts[1], 2))
                    .zip(num(parts[2], 2))
                    .zip(f)
                    .map(|(((h, m), s), f)| (h, m, s, f))
            }
            _ => None,
        }
    } else if body.contains('.') {
        let parts: Vec<&str> = body.split('.').collect();
        match parts[..] {
            [h, m, s] => num(h, 2).zip(num(m, 2)).zip(num(s, 2)).map(|((h, m), s)| (h, m, s, 0)),
            _ => None,
        }
    } else if body.len() == 4 {
        num(&body[..2], 2).zip(num(&body[2..], 2)).map(|(h, m)| (h, m, 0, 0))
    } else if body.len() == 6 && number == 0 {
        // `hhmmss` and a bare hour (`23`) are not pinned: Microsoft's tests Evaluate `085606` (Calendar Management
        // CalcTimeDelta) and `Format(RandInt(23))` into a Time (assumption evaluate-time-digit-forms).
        let r = num(&body[..2], 2)
            .zip(num(&body[2..4], 2))
            .zip(num(&body[4..], 2))
            .map(|((h, m), s)| (h, m, s, 0));
        if r.is_some() {
            crate::assumptions::assume(crate::assumptions::Assumption::EvaluateTimeDigitForms);
        }
        r
    } else if (1..=2).contains(&body.len()) && number == 0 && body.bytes().all(|b| b.is_ascii_digit()) {
        let r = num(body, 2).map(|h| (h, 0, 0, 0));
        if r.is_some() {
            crate::assumptions::assume(crate::assumptions::Assumption::EvaluateTimeDigitForms);
        }
        r
    } else {
        None
    };
    let Some((mut h, m, s, f)) = nums else {
        return unsupported(format!(
            "Evaluate of `{text}` into Time (form not pinned by the corpus)"
        ));
    };
    if let Some(pm) = meridiem {
        if !(1..=12).contains(&h) {
            return unsupported(format!(
                "Evaluate of `{text}` into Time (hour with AM or PM not pinned)"
            ));
        }
        h = h % 12 + if pm { 12 } else { 0 };
    }
    Ok(if in_range(h, m, s) {
        value(h, m, s, f)
    } else {
        Parsed::Rejected
    })
}

/// A date and time in UTC (`Value::DateTime`).
///
/// Pinned by types/lt-evaluate-format-lasterror-setview/P86Eval.al, codeunit "LT P86 Eval DateTime". Number 9 reads
/// `yyyy-mm-dd[Thh:mm:ss[.f][Z|+hh:mm]]` (no zone means UTC) and the empty text, and rejects everything else. Number 0
/// reads the same with `Z` or an offset, the empty text, and `m/d/y h:mm[:ss] [AM|PM]`; it rejects a time past 23:59:59,
/// a day that does not exist and `0D`, `0T`, `0DT`. Not read: the ISO form without a zone in number 0 (BC raises
/// `The date is not valid.`) and `10:20`, which BC completes with the current year.
fn datetime(text: &str, number: i64) -> Result<Parsed, Signal> {
    let t = text.trim_matches(' ');
    if t.is_empty() {
        return Ok(Parsed::Value(Value::DateTime(0)));
    }
    let iso = t.len() >= 10 && t.as_bytes()[4] == b'-';
    if number == 9 {
        if t.len() == 10 && t.as_bytes()[4] == b'-' {
            return Ok(match crate::iso::parse_date(t) {
                Some(d) => Parsed::Value(Value::DateTime((i64::from(d) - 1) * 86_400_000 + 1)),
                None => Parsed::Rejected,
            });
        }
        return Ok(
            match crate::iso::parse_datetime(t).filter(|_| iso && t.as_bytes().get(10) == Some(&b'T')) {
                Some(v) => Parsed::Value(Value::DateTime(v)),
                None => Parsed::Rejected,
            },
        );
    }
    if ["0d", "0t", "0dt"].iter().any(|k| t.eq_ignore_ascii_case(k)) || !t.bytes().any(|b| b.is_ascii_digit()) {
        return Ok(Parsed::Rejected);
    }
    if iso && t.as_bytes().get(10) == Some(&b'T') {
        if !(t.ends_with(['Z', 'z']) || t[11..].contains(['+', '-'])) {
            return unsupported(format!(
                "Evaluate of `{text}` into DateTime without a zone (BC raises another error)"
            ));
        }
        return Ok(match crate::iso::parse_datetime(t) {
            Some(v) => Parsed::Value(Value::DateTime(v)),
            None => Parsed::Rejected,
        });
    }
    if let Some((d, clock)) = t.split_once(' ') {
        if d.contains('/') {
            return Ok(match (date(d, 0, None, None, false)?, time(clock, 0)?) {
                (Parsed::Value(Value::Date(day, _)), Parsed::Value(Value::Time(ms))) => {
                    Parsed::Value(Value::DateTime((i64::from(day) - 1) * 86_400_000 + ms))
                }
                _ => Parsed::Rejected,
            });
        }
    }
    unsupported(format!(
        "Evaluate of `{text}` into DateTime with format {number} (form not pinned)"
    ))
}

/// The value of a Date or a Guid in the form `Evaluate` reads it (and `Format` writes it), for a reader that has no
/// variable to evaluate into: `None` for any other type, a text that is no such value, or a form not pinned.
pub fn read_text(ty: &crate::value::RtType, text: &str) -> Option<Value> {
    use crate::value::RtType as T;
    // `Format(ClosingDate(D))` is `C` and the date (corpus types/lt-datetime/DateFuncs.al:47 `C12/31/23`,
    // misc/lt-system-functions/RandomFormat.al:432): the closing date of that day.
    if let (T::Date, Some(rest)) = (ty, text.strip_prefix('C')) {
        return match date(rest, 0, None, None, false).ok()? {
            Parsed::Value(Value::Date(d, false)) => Some(Value::Date(d, true)),
            _ => None,
        };
    }
    let parsed = match ty {
        T::Date => date(text, 0, None, None, false).ok()?,
        T::Guid => guid(text).ok()?,
        _ => return None,
    };
    match parsed {
        Parsed::Value(v) => Some(v),
        Parsed::Rejected => None,
    }
}

pub fn evaluate(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    evaluate_with(c, a, false)
}

/// `Evaluate` into an Option variable, which has no OptionCaption to name a member by.
pub fn evaluate_variable_option(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    evaluate_with(c, a, true)
}

/// `Evaluate` into an Option field that declares an OptionCaption (the lowering appends the captions as a comma list): the
/// text names a member by its caption first, else by its name, else it is an ordinal of any size (corpus errors/
/// lt-ms-refusals-128/P128EvalBasic.al:P128_Evaluate_IntoOptionField_NumericCaptions: with captions `1,2,3` the text `1`
/// is the first member, `3` the third and `4` the ordinal 4; with `Ten,Twenty,Thirty` `twenty` is the second member, `Two`
/// the third by its name and `Three` is rejected). A rejected text gives `false`; a format number and a failure with the
/// result unused are not pinned.
pub fn evaluate_option_field(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let (Some(text), Some(Value::Text(captions)), Value::Option(_, names)) =
        (crate::native::text_of(&a[1]), a.get(3).cloned(), a[0].clone())
    else {
        return unsupported("Evaluate into an Option field without its captions");
    };
    if !matches!(a.get(2), Some(Value::Integer(0))) {
        return unsupported("Evaluate into an Option field with a format number (not pinned)");
    }
    let t = text.trim();
    // An empty or blank text names a blank member (errors/lt-ms-refusals-131/P131EvalOption.al:107-108: `''` and `' '` give the
    // member `" "` of caption `' '`, :146: a field without a blank member rejects `''`).
    let same = |n: &str| {
        if t.is_empty() {
            n.trim().is_empty()
        } else {
            n.eq_ignore_ascii_case(t)
        }
    };
    let by_caption = captions.as_str().split(',').position(same);
    let by_name = names.iter().position(|n| same(n));
    let digits = t.strip_prefix('+').unwrap_or(t);
    let ordinal = (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| digits.parse::<i32>().ok())
        .flatten();
    match by_caption.or(by_name).map(|p| p as i32).or(ordinal) {
        Some(o) => {
            a[0] = Value::Option(o, names);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => unsupported("Evaluate into an Option field failing with the result unused (error text not pinned yet)"),
    }
}

/// `Evaluate` into a Text or Code variable of a declared length: the lowering appends the length as a fourth argument.
pub fn evaluate_bounded(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let Some(Value::Integer(max)) = a.get(3).cloned() else {
        return unsupported("Evaluate into a bounded text without its length");
    };
    let max = u32::try_from(max).ok();
    evaluate_with_max(c, &mut a[..3], false, max, false)
}

fn evaluate_with(c: &mut dyn Ctx, a: &mut [Value], variable_option: bool) -> Result<Value, Signal> {
    evaluate_with_max(c, a, variable_option, None, false)
}

/// `Evaluate` into a Text or a Code (corpus errors/lt-ms-refusals-128/P128EvalBasic.al:P128_Evaluate_IntoText,
/// P128_Evaluate_IntoCode, P128_Evaluate_IntoCode_ResultUnused, P128_Evaluate_IntoShortText_ResultUnused): a Text takes the
/// text as written (blanks kept), a Code takes it trimmed and in capitals; text that is too long for the variable gives
/// `false` when the result is used and the length error (with the text as given) when it is not, and the variable keeps
/// its value. A format number is not pinned.
fn text_or_code(c: &mut dyn Ctx, a: &Value, text: &str, number: i64, max: Option<u32>) -> Result<Parsed, Signal> {
    // The numbers 0, 1, 2 and 9 read a Text and a Code alike (types/lt-evaluate-text-decimal/EvaluateTextCodeDecimal.al:
    // Text_FormatNumbers, Code_FormatNumbers); the others are not pinned.
    if !matches!(number, 0 | 1 | 2 | 9) {
        return unsupported(format!(
            "Evaluate into a Text or Code with format number {number} (not pinned)"
        ));
    }
    let code = matches!(a, Value::Code(_));
    let kept = if code { text.trim() } else { text };
    if let Some(m) = max {
        if Value::text_len(text) > m as usize {
            // A Code takes the text without its blanks when that fits (`abcde   ` and ` abcde` into Code[5], Code_FormatNumbers
            // `trailFits`, `leadFull`). A text that does not fit even without its blanks, and has blanks, is not pinned: the length
            // BC reports for it could be either one.
            if code && Value::text_len(kept) <= m as usize {
                return Ok(Parsed::Value(Value::Code(crate::ops::upper_code_checked(kept)?.into())));
            }
            if code && kept.len() != text.len() {
                return unsupported(
                    "Evaluate into a Code of a text with blanks that is too long even without them (not pinned)",
                );
            }
            let message = format!(
                "The length of the string is {}, but it must be less than or equal to {m} characters. Value: {}",
                Value::text_len(text),
                crate::ops::shown_in_length_error(text)
            );
            // With the result used the call answers false and leaves this error as the last one (types/lt-evaluate-text-decimal/
            // EvaluateTextCodeDecimal.al:Code_FormatNumbers `tooLong0Ok` No with `tooLong0Err`).
            if c.ret_used() {
                c.set_last_error(crate::native::AlError {
                    message,
                    code: "StringLengthExceeded".into(),
                    ..Default::default()
                });
                return Ok(Parsed::Rejected);
            }
            return crate::native::raise(message, "StringLengthExceeded");
        }
    }
    Ok(Parsed::Value(if code {
        Value::Code(crate::ops::upper_code_checked(kept)?.into())
    } else {
        Value::Text(text.to_string().into())
    }))
}

/// The value `text` reads as for type `ty` (`Evaluate` into a fresh variable of that type): `None` where it is rejected.
pub fn evaluate_typed(
    c: &mut dyn Ctx,
    ty: &crate::value::RtType,
    text: &str,
    number: i64,
) -> Result<Option<Value>, Signal> {
    let max = match ty {
        crate::value::RtType::Text(m) | crate::value::RtType::Code(m) => *m,
        _ => None,
    };
    let mut tmp = vec![
        Value::default_for(ty),
        Value::Text(text.to_string().into()),
        Value::Integer(number as i32),
    ];
    // The caller says the result is used: a rejected text is `false`, not an error.
    Ok(match evaluate_with_max(c, &mut tmp, false, max, false)? {
        Value::Boolean(true) => Some(tmp.swap_remove(0)),
        _ => None,
    })
}

/// `Evaluate(RecordId, Text)`: a table name and the primary key after a colon, or only the table name; the empty text gives the
/// blank RecordId, also for one that is not blank (types/lt-evaluate-recordid/EvaluateRecordIdComposite.al:Evaluate_IntoSetRecordId
/// `emptyResult`, `emptyUnusedResult`). A text that names no table, has more key values than the key has fields or holds a value that
/// is none of its field's type is rejected: with the result used the answer is false, the RecordId stays as it was and the error is the
/// last error; with the result unused the error is raised (`garbageResult`, `garbageUnusedErr`, Evaluate_WrongNumberOfValues). The
/// errors are `The value "<text>" can't be evaluated into type RecordID.` (code Evaluate) and the error of the value that failed.
fn into_record_id(c: &mut dyn Ctx, a: &mut [Value], text: &str, number: i64) -> Result<Value, Signal> {
    if !matches!(&a[0], Value::Library(..)) {
        return unsupported("Evaluate into a RecordId that is no RecordId");
    }
    let rejected = |c: &mut dyn Ctx, error: Signal| -> Result<Value, Signal> {
        match error {
            Signal::Error(e) if c.ret_used() => {
                c.set_last_error(*e);
                Ok(Value::Boolean(false))
            }
            other => Err(other),
        }
    };
    let new = if text.is_empty() {
        crate::value::LibraryValue::Empty
    } else {
        match c.evaluate_record_id(text, number) {
            Some(Ok(Some(v))) => v,
            Some(Ok(None)) => {
                let Err(e) = crate::native::raise::<()>(
                    format!("The value \"{text}\" can't be evaluated into type RecordID."),
                    "Evaluate",
                ) else {
                    unreachable!()
                };
                return rejected(c, e);
            }
            Some(Err(e)) => return rejected(c, e),
            None => return unsupported("Evaluate into a RecordId without a program"),
        }
    };
    // The variable gets a RecordId of its own: another variable assigned from it keeps its value
    // (types/lt-evaluate-recordid/EvaluateRecordIdComposite.al:Evaluate_IntoSetRecordId, `Rid := Start` then Evaluate(Rid, ...)).
    a[0] = Value::Library(
        crate::library_kind::LibraryKind::RecordId,
        std::rc::Rc::new(std::cell::RefCell::new(new)),
    );
    Ok(Value::Boolean(true))
}

/// `Evaluate(FieldRef, Text)`: the field takes the value the text reads as for the field's type, a Text or Code that is too
/// long for it, or a text that is no value, gives `false` and leaves the field (corpus errors/lt-ms-refusals-128/
/// P128EvalFieldRef.al: a Text, Code, Integer, Decimal, Boolean, Date and Option field, with the result used). A failure
/// with the result unused is pinned for an Integer field only (it raises the error of the type), and an Option field with an
/// OptionCaption is not pinned and stays refused.
fn into_field_ref(c: &mut dyn Ctx, a: &mut [Value], number: i64) -> Result<Value, Signal> {
    let target = match c.evaluate_field_target(&a[0]) {
        Some(t) => t?,
        None => return unsupported("Evaluate into a FieldRef that is not assigned (not pinned)"),
    };
    let current = target.rec.borrow().fields[target.pos].clone();
    // With the result unused a text that is no value raises the error of the type and leaves the field, as for a variable
    // (types/lt-evaluate-recordid/EvaluateRecordIdComposite.al:Evaluate_IntoFieldRef `invalidUnusedErr`, an Integer field;
    // errors/lt-ms-refusals-128/N582EvalFieldRefUnused.al:71-169: Decimal, Date, Boolean, Code, Text, an Option without an
    // OptionCaption and Integer). The other types are not pinned.
    if !c.ret_used() {
        use crate::value::RtType as T;
        let pinned = match &target.ty {
            T::Integer | T::Decimal | T::Date | T::Boolean | T::Code(_) | T::Text(_) => true,
            T::Option(_) => !target.option_caption,
            _ => false,
        };
        if !pinned {
            return unsupported(format!(
                "Evaluate into a FieldRef of type {} with the result unused (not pinned)",
                target.ty.describe()
            ));
        }
    }
    let max = match &target.ty {
        crate::value::RtType::Text(m) | crate::value::RtType::Code(m) => *m,
        _ => None,
    };
    let mut tmp = vec![current, a[1].clone(), Value::Integer(number as i32)];
    // A Date field names any text in the date error, letters too (N582EvalFieldRefUnused.al:92, `abc`).
    let ok = evaluate_with_max(c, &mut tmp, !target.option_caption, max, true)?;
    if matches!(ok, Value::Boolean(true)) {
        let v = crate::ops::convert(tmp.swap_remove(0), &target.ty)?;
        target.rec.borrow_mut().fields[target.pos] = v;
    }
    Ok(ok)
}

/// One value of the key text of `Evaluate(RecordId, Text)`: read as `Evaluate` reads a variable of type `ty`, and a text that is
/// no value raises the error of that type (a Date says so for any text), whether the caller uses the result or not; the caller
/// turns the error into the last error when it does (corpus types/lt-evaluate-recordid/EvaluateRecordIdComposite.al:
/// Evaluate_WrongNumberOfValues `badInteger`, `badEnum`, `badDate`, `codeTooLong`). The caller sets the context to "result unused".
pub fn evaluate_key_part(c: &mut dyn Ctx, ty: &crate::value::RtType, text: &str, number: i64) -> Result<Value, Signal> {
    let max = match ty {
        crate::value::RtType::Text(m) | crate::value::RtType::Code(m) => *m,
        _ => None,
    };
    // A Text or Code reads the same text with either format number (the key text of Format(RecordId, 0, 9) holds it as it is).
    let number = if matches!(ty, crate::value::RtType::Text(_) | crate::value::RtType::Code(_)) {
        0
    } else {
        number
    };
    let mut tmp = vec![
        Value::default_for(ty),
        Value::Text(text.to_string().into()),
        Value::Integer(number as i32),
    ];
    match evaluate_with_max(c, &mut tmp, false, max, true)? {
        Value::Boolean(true) => Ok(tmp.swap_remove(0)),
        _ => unsupported(format!(
            "Evaluate of the key part `{text}` into {} (not pinned)",
            ty.describe()
        )),
    }
}

fn evaluate_with_max(
    c: &mut dyn Ctx,
    a: &mut [Value],
    variable_option: bool,
    max: Option<u32>,
    any_date_text: bool,
) -> Result<Value, Signal> {
    let Some(text) = crate::native::text_of(&a[1]) else {
        return unsupported("Evaluate of a source that is not text");
    };
    let number = match a.get(2) {
        None => 0,
        Some(Value::Integer(n)) => i64::from(*n),
        Some(other) => return unsupported(format!("Evaluate format number {}", other.describe())),
    };
    // Number 2 is read for a DateFormula only, as number 0 (docs/probes-needed.md).
    if number != 0
        && number != 9
        && !(number == 2 && matches!(a[0], Value::DateFormula(_)))
        && !(matches!(number, 1 | 2) && matches!(a[0], Value::Text(_) | Value::Code(_)))
        && !matches!(a[0], Value::Date(..))
    {
        return unsupported(format!(
            "Evaluate with format number {number} (not pinned by the corpus)"
        ));
    }
    if matches!(&a[0], Value::Library(crate::library_kind::LibraryKind::FieldRef, _)) {
        return into_field_ref(c, a, number);
    }
    if matches!(&a[0], Value::Library(crate::library_kind::LibraryKind::RecordId, _)) {
        return into_record_id(c, a, &text, number);
    }
    let parsed = match &a[0] {
        Value::Text(_) | Value::Code(_) => text_or_code(c, &a[0], &text, number, max)?,
        Value::Integer(_) => integer(&text, number)?,
        Value::BigInteger(_) => big_integer(&text, number)?,
        Value::Decimal(_) => decimal(&text, number)?,
        Value::Boolean(_) => boolean(&text, number)?,
        Value::Guid(_) => guid(&text)?,
        // Every number but 9 reads a Date as the default does (types/lt-datetime/Evaluate.al:Date_Other_Formats `fmt 1` to `fmt 8`,
        // `fmt 10`, `fmt -1`; errors/lt-ms-refusals-130/P130EvaluateDate.al:P130_EvaluateDate_EveryFormat_*).
        Value::Date(..) => date(
            &text,
            if number == 9 { 9 } else { 0 },
            c.today_day(),
            c.work_date_day(),
            true,
        )?,
        Value::Time(_) => time(&text, number)?,
        Value::DateTime(_) => datetime(&text, number)?,
        // The text is kept as written once it is a form the runtime can calculate with
        // (types/TestDateFormulaContracts.al:21-139); what Format prints is docs/probes-needed.md.
        // `Evaluate(V, '1.2.3.4')`: the four-part form (session/TestMiscTypes.al:44-80).
        Value::Library(crate::library_kind::LibraryKind::Version, cell) => {
            let parts: Vec<&str> = text.split('.').collect();
            let nums: Option<Vec<i32>> = parts
                .iter()
                .map(|p| {
                    (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
                        .then(|| p.parse().ok())
                        .flatten()
                })
                .collect();
            match nums.filter(|n| n.len() == 4) {
                Some(n) => {
                    *cell.borrow_mut() = crate::value::LibraryValue::Version([n[0], n[1], n[2], n[3]]);
                    return Ok(Value::Boolean(true));
                }
                None => return unsupported(format!("Evaluate of `{text}` into Version (only a.b.c.d is pinned)")),
            }
        }
        // An enum reads an ordinal (also one no member has), a member name in any case or a caption, with
        // format 0 and with format 9 (types/ProbeTypeFormsAndFilters.al:Enum_Evaluate_Forms, BC 27.5; the
        // InitValue of a field reads a name the same way, types/TestBlankEnumInitTests.Codeunit.al:92). A
        // text that is none of these is rejected.
        Value::Enum(id, _) => {
            let Some(members) = c.enum_members(*id) else {
                return unsupported("Evaluate into an Enum without its members");
            };
            // White space around the text is ignored (types/W7EvaluateTests.al:EvaluateIntoAnEnum_DefaultFormat,
            // ` Second ` gives Second).
            let original = text.clone();
            let text = text.trim();
            let digits = text.strip_prefix('+').unwrap_or(text);
            let ordinal = (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
                .then(|| digits.parse::<i32>().ok())
                .flatten();
            // A sign is allowed before the digits and an ordinal no member has is kept, with format 9 too
            // (P86Eval.al, codeunit "LT P86 Eval Enum": `+5`, `00`, `11`).
            let captions = c.enum_captions(*id);
            let found = ordinal
                .or_else(|| {
                    members
                        .iter()
                        // A blank member matches the blank text (types/TestBlankEnumInitTests.Codeunit.al:
                        // EvaluateMatchesBlankMemberAndRejectsNonMember).
                        .find(|(_, n)| n.trim().eq_ignore_ascii_case(text))
                        .map(|(o, _)| *o)
                })
                .or_else(|| {
                    captions
                        .iter()
                        .find(|(_, n)| !text.is_empty() && n.eq_ignore_ascii_case(text))
                        .map(|(o, _)| *o)
                });
            match found {
                Some(o) => Parsed::Value(Value::Enum(*id, o)),
                None if c.ret_used() => Parsed::Rejected,
                // The error names every member by its caption, in ordinal order, and quotes the text as given.
                None => {
                    let options: Vec<String> = members
                        .iter()
                        .map(|(o, n)| {
                            captions
                                .iter()
                                .find(|(co, _)| co == o)
                                .map_or(n.clone(), |(_, cap)| cap.clone())
                        })
                        .collect();
                    return crate::native::raise(
                        format!(
                            "'{original}' is not an option. The existing options are: {}",
                            options.join(",")
                        ),
                        "InvalidOptionString",
                    );
                }
            }
        }
        // An ordinal into SecurityFilter: `Evaluate(Filter, '3')` gives Disallowed
        // (recordref/TestPlatformOptionsLockingContracts.al:42-43). Other options are not pinned.
        Value::Option(_, members) if crate::sysenum::is_security_filter(members) => {
            match text.trim().parse::<usize>().ok().filter(|n| *n < members.len()) {
                Some(n) => Parsed::Value(Value::Option(n as i32, members.clone())),
                None => {
                    return unsupported(format!(
                        "Evaluate of `{text}` into SecurityFilter (only an ordinal is pinned)"
                    ));
                }
            }
        }
        // ClientType has no OptionCaption: a number below 16 or a member name evaluates, any other text answers false
        // (types/PWMisc.al:ClientType_OrdinalsAndNames: 16 to 30 are rejected).
        Value::Option(_, members) if crate::sysenum::is_client_type(members) => {
            let t = text.trim();
            let by_name = members.iter().position(|m| m.eq_ignore_ascii_case(t));
            match (t.parse::<usize>().ok().filter(|n| *n < members.len()), by_name) {
                (Some(n), _) | (None, Some(n)) => Parsed::Value(Value::Option(n as i32, members.clone())),
                _ => Parsed::Rejected,
            }
        }
        // An Option reads an ordinal or a member name (types/PROptionInteger.al:Option_InCaseAndLoopBounds: `1` is B, `C` is C).
        // A variable: an ordinal of any size is kept (`3` of three members formats as `3`), a text that is neither an ordinal
        // nor a member name is rejected, and the error names the members (P86Eval.al, codeunit "LT P86 Eval Option").
        Value::Option(_, members) if variable_option => {
            let t = text.trim();
            // An empty text names no member, not even one declared as "" (types/lt-scalars/ScT70062.al:Option_EvaluateByName `empty`).
            let by_name = members.iter().position(|m| !t.is_empty() && m.eq_ignore_ascii_case(t));
            let ordinal = if t.starts_with(|ch: char| ch.is_ascii_digit() || ch == '+') {
                t.parse::<u64>().ok()
            } else {
                None
            };
            match (ordinal, by_name) {
                (Some(n), _) => match i32::try_from(n) {
                    Ok(n) => Parsed::Value(Value::Option(n, members.clone())),
                    Err(_) => {
                        return unsupported(format!("Evaluate of the ordinal `{text}` into an Option (not pinned)"));
                    }
                },
                (None, Some(n)) => Parsed::Value(Value::Option(n as i32, members.clone())),
                _ if c.ret_used() => return Ok(Value::Boolean(false)),
                _ => {
                    return crate::native::raise(
                        format!(
                            "'{text}' is not an option. The existing options are: {}",
                            members.join(",")
                        ),
                        "InvalidOptionString",
                    );
                }
            }
        }
        Value::Option(_, members) => {
            let t = text.trim();
            let by_name = members.iter().position(|m| m.eq_ignore_ascii_case(t));
            match (t.parse::<usize>().ok().filter(|n| *n < members.len()), by_name) {
                (Some(n), _) | (None, Some(n)) => Parsed::Value(Value::Option(n as i32, members.clone())),
                // An OptionCaption may name it (fieldref/TestPrecompiledFieldOptionCaption.al:Evaluate_Caption_SelectsThatMember);
                // the value does not carry the captions.
                _ => {
                    return unsupported(format!(
                        "Evaluate of `{text}` into an Option: neither an ordinal nor a member name (an OptionCaption could name it, not pinned)"
                    ));
                }
            }
        }
        // The blank-like texts store the empty formula; garbage fails and leaves the value
        // (types/TestDateFormulaDurationFormatContracts.al:28-74, TestDateFormulaEvaluateLeftoversContracts.al).
        Value::DateFormula(_) => match crate::datefns::evaluate_formula(&text)? {
            crate::datefns::Evaluated::Stored(f) => Parsed::Value(Value::DateFormula(f)),
            // A statement form raises the formula's error (types/lt-date-formula-edges/DateFormulaEdges.al:SignsAndDigits_ErrorTexts).
            crate::datefns::Evaluated::Rejected(Some(m)) if !c.ret_used() => {
                return crate::native::raise(m, "Evaluate");
            }
            crate::datefns::Evaluated::Rejected(_) => Parsed::Rejected,
            crate::datefns::Evaluated::Failed(m) if !c.ret_used() => return crate::native::raise(m, "Evaluate"),
            // With the result used, both errors of this kind answer false and leave the value
            // (errors/lt-ms-refusals-128/P128CalcDateForms.al:154-169: date texts, "should include a
            // time unit"; errors/lt-ms-refusals-131/P131CalcDate.al:140,152: `<1D>+`, "should include a number").
            crate::datefns::Evaluated::Failed(_) => Parsed::Rejected,
        },
        other => return unsupported(format!("Evaluate into {} (not pinned by the corpus)", other.describe())),
    };
    match parsed {
        Parsed::Value(v) => {
            a[0] = v;
            Ok(Value::Boolean(true))
        }
        Parsed::Rejected if c.ret_used() => Ok(Value::Boolean(false)),
        // The error of a failed statement form (session/TestProbeSessionMiscContracts.al:Arithmetic_ErrorTexts,
        // `evaluate_int_overflow_statement`; types/TestProbeScalarContracts.al:Evaluate_BooleanFailureError).
        Parsed::Rejected
            if matches!(
                a[0],
                Value::Integer(_)
                    | Value::BigInteger(_)
                    | Value::Decimal(_)
                    | Value::Boolean(_)
                    | Value::Time(_)
                    | Value::DateTime(_)
            ) =>
        {
            let ty = match a[0] {
                Value::Integer(_) => "Integer",
                Value::BigInteger(_) => "BigInteger",
                Value::Decimal(_) => "Decimal",
                Value::Time(_) => "Time",
                Value::DateTime(_) => "DateTime",
                _ => "Boolean",
            };
            // In German the text is pinned for Decimal only (types/lt-evaluate-decimal-text/EvalDecimalDe.al:
            // P175_De_StatementErrorText, P175_De_Format9StatementErrorText).
            match (c.session_locale().map(|l| l.0), ty) {
                (Some(1031), "Decimal") => crate::native::raise(
                    format!("Der Wert „{text}“ kann nicht in Typ Decimal ausgewertet werden."),
                    "Evaluate",
                ),
                (Some(1031), _) => unsupported(format!("the Evaluate error of {ty} in German (not pinned)")),
                _ => crate::native::raise(
                    format!("The value \"{text}\" can't be evaluated into type {ty}."),
                    "Evaluate",
                ),
            }
        }
        // The error of a rejected date that looks like one (P86Eval.al:Dflt3_Error_FNone, Fmt03_Error_F0, Fmt93_Error_F9: the same
        // text as a date filter gives, SetFilter on a Date).
        Parsed::Rejected
            if matches!(a[0], Value::Date(..))
                && (any_date_text
                    || text
                        .bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'-' | b'.' | b'/'))) =>
        {
            crate::native::raise(
                format!(
                    "The format of the Date '{text}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
                ),
                "Evaluate",
            )
        }
        Parsed::Rejected if matches!(a[0], Value::Guid(_)) => crate::native::raise(
            "Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.",
            "InvalidGuidFormat",
        ),
        Parsed::Rejected => unsupported(format!(
            "Evaluate of `{text}` failed with the result unused (error text not pinned yet)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(p: Result<Parsed, Signal>) -> Option<Value> {
        match p {
            Ok(Parsed::Value(v)) => Some(v),
            _ => None,
        }
    }

    // types/lt-evaluate-format-lasterror-setview/P86Eval.al, codeunit "LT P86 Eval Integer"
    #[test]
    fn integers_read_spaces_signs_and_grouping_by_format_number() {
        let int = |s: &str, n: i64| value(integer(s, n)).map(|v| matches!(v, Value::Integer(_)));
        assert!(matches!(value(integer("  +12 ", 0)), Some(Value::Integer(12))));
        assert!(matches!(value(integer("-0", 9)), Some(Value::Integer(0))));
        assert!(matches!(value(integer("1,234", 0)), Some(Value::Integer(1234))));
        assert_eq!(int("1,234", 9), None);
        assert_eq!(int("2147483648", 9), None);
        assert_eq!(int("1.5", 0), None);
        assert_eq!(int("1 234", 0), None);
    }

    // codeunit "LT P86 Eval Time"
    #[test]
    fn times_read_the_forms_of_each_format_number() {
        let ms = |s: &str, n: i64| match value(time(s, n)) {
            Some(Value::Time(t)) => Some(t - 1),
            _ => None,
        };
        assert_eq!(ms("12:00 PM", 0), Some(43_200_000));
        assert_eq!(ms("12:00:00 AM", 0), Some(0));
        assert_eq!(ms("1200", 0), Some(43_200_000));
        assert_eq!(ms("12.00.00", 0), Some(43_200_000));
        assert_eq!(ms("12:00:00.5", 9), Some(43_200_500));
        assert_eq!(ms("12:00", 9), None);
        assert_eq!(ms("24:00:00", 0), None);
        assert_eq!(ms("12:00:60", 0), None);
    }

    // Not pinned (assumption evaluate-time-digit-forms): six digits are hhmmss and one or two digits an hour, in format 0 only.
    #[test]
    fn times_read_six_digits_and_a_bare_hour_in_format_0() {
        let ms = |s: &str, n: i64| match value(time(s, n)) {
            Some(Value::Time(t)) => Some(t - 1),
            _ => None,
        };
        assert_eq!(ms("085606", 0), Some(((8 * 60 + 56) * 60 + 6) * 1000));
        assert_eq!(ms("23", 0), Some(23 * 3_600_000));
        assert_eq!(ms("5", 0), Some(5 * 3_600_000));
        assert_eq!(ms("246000", 0), None);
        assert_eq!(ms("24", 0), None);
        assert_eq!(ms("085606", 9), None);
    }

    // types/lt-datetime/Evaluate.al:Date_Default_Evaluate: the empty text reads as the empty date in format 0 too.
    #[test]
    fn the_empty_text_reads_as_the_empty_date_in_format_0() {
        assert!(matches!(
            value(date("", 0, Some(5), None, true)),
            Some(Value::Date(0, _))
        ));
    }

    // codeunit "LT P86 Eval Date"
    #[test]
    fn dates_read_slash_forms_in_format_0_and_only_iso_in_format_9() {
        let day = |s: &str, n: i64| match value(date(s, n, Some(5), Some(7), true)) {
            Some(Value::Date(d, _)) => Some(d),
            _ => None,
        };
        assert_eq!(day("1/15/24", 0), day("2024-01-15", 0));
        assert_eq!(day("01/15/2024", 0), day("2024-01-15", 9));
        assert_eq!(day("T", 0), Some(5));
        assert_eq!(day("T", 9), None);
        assert_eq!(day("01/15/2024", 9), None);
        assert_eq!(day("20240115", 0), None);
    }

    // types/lt-evaluate-decimal/EvaluateDecimal.al:Evaluate_SignsBlanksAndGarbage `trailing_minus`, `parentheses`
    #[test]
    fn decimals_read_a_trailing_minus_and_parentheses_as_negative() {
        let dec = |s: &str, n: i64| match decimal(s, n) {
            Ok(Parsed::Value(Value::Decimal(d))) => Some(d.to_string()),
            _ => None,
        };
        assert_eq!(dec("100-", 0).as_deref(), Some("-100"));
        assert_eq!(dec("(100)", 0).as_deref(), Some("-100"));
        assert_eq!(dec("  1,000.5- ", 0).as_deref(), Some("-1000.5"));
        // Other combinations are not pinned: refused, not guessed.
        assert!(decimal("(100)", 9).is_err());
        assert!(decimal("(100", 0).is_err());
        assert!(matches!(decimal("abc-", 0), Ok(Parsed::Rejected)));
    }
}
