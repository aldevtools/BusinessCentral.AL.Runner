//! `Evaluate(var Variable, String [, Number])`.
//!
//! Only the (type, text form) pairs a corpus test pins are accepted; text that
//! plainly is not a value of the type (no digits for a number, empty) fails the
//! way the corpus shows, and every other form raises `Signal::Unsupported`.

mod date_forms;
mod datetime_forms;
mod duration;
mod enum_text;
use duration::duration;
mod option_text;
mod platform_options;
use crate::describe::Describe;
use crate::format::day_from_civil;
use crate::native::{Ctx, Signal, unsupported};
use crate::value::Value;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Whether `t` is a short list of numbers: two or three pieces of digits (one may hold a point) or nothing, with blanks around them,
/// joined by one of `;`, `|`, `:`, `/` or `&` used throughout. Format number 0 rejects such a text for Decimal, Integer and BigInteger
/// (format-evaluate/EvaluateLists.al:13-29 Inputs, :48-142: `1;5;10`, `1;`, `;1`, `;`, `1|5|10`, `1:5`, `1/5`, `1&5`,
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

/// Blanks `Evaluate` drops around a number: spaces, tab, carriage return and line feed
/// (format-evaluate/EvaluateDecimal.al:QR_Evaluate_WhiteSpace_Number0, _Number9, QR_Evaluate_Integer_OddCharacters `int tab`).
const EDGE_BLANKS: [char; 4] = [' ', '\t', '\r', '\n'];

/// Characters that make a text no number wherever they stand, with format number 0 and 9: the punctuation
/// `/ : # & = ' " ; [ ] { } * @`, the currency signs `€` and `£`, letters outside ASCII (`ä`, `Ä1`), the no-break spaces
/// U+00A0 and U+202F, and the full-width and Arabic-Indic digits (format-evaluate/EvaluateDecimal.al:
/// QR_Evaluate_Punctuation_NoNumber, _Number0, _Number9, QR_Evaluate_WhiteSpace_Number0, _Number9,
/// QR_Evaluate_Statement_OddCharacters). Other characters outside ASCII are not pinned.
fn rejected_characters(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(
            c,
            '/' | ':' | '#' | '&' | '=' | '\'' | '"' | ';' | '[' | ']' | '{' | '}' | '*' | '@' | '\u{20AC}' | '\u{A3}'
                | '\u{A0}' | '\u{202F}' | '\u{FF10}'..='\u{FF19}' | '\u{0660}'..='\u{0669}'
        ) || (!c.is_ascii() && c.is_alphabetic())
    })
}

/// What parsing one text produced.
enum Parsed {
    Value(Value),
    /// The text is not a value of the type: `Evaluate` returns false.
    Rejected,
    /// Rejected, and the statement form raises this error (text, code) instead of the type's own.
    RejectedWith(&'static str, &'static str),
    /// Rejected with this error (text, code), which the form with a used result leaves as the last error.
    RejectedLastError(String, &'static str),
    /// Rejected; with the result used a last error is left whose text is not pinned, so that form is refused with this reason.
    RejectedUsedRefused(String),
}

/// A whole number as `Evaluate` reads it into an Integer. Spaces around it are dropped, one sign is allowed and the
/// digits are ASCII; number 0 also reads properly grouped thousands (`1,234`), number 9 does not. Everything else is
/// rejected, an overflow too (format-evaluate/P86Eval.al, codeunit "LT P86 Eval Integer":
/// `+1`, `007`, `-0`, `1 234`, `1.5`, `1e3`, `0x1F`, Arabic digits, `1-`, `--1`, `2147483648`).
fn integer(text: &str, number: i64) -> Result<Parsed, Signal> {
    // Number 0 reads the text as a Decimal and takes it when it is whole: `1,5` is 15, `1,000` is 1000, `1.0` is 1 and `1E3` is
    // 1000, while `1.5` is rejected (types/decimal-integer/ScT70055.al:EvaluateInteger_Invalid `comma`, `thousand`, `exp`,
    // `decimal0`, `decimal`, `hex`, `two_signs`, `bigval`; P86Eval.al "LT P86 Eval Integer").
    // A tab around the number is dropped and a no-break space rejects, as for a Decimal (format-evaluate/
    // EvaluateDecimal.al:QR_Evaluate_Integer_OddCharacters `int tab`, `int nbsp`).
    if number == 0 && text.contains(['\u{A0}', '\u{202F}']) {
        return Ok(Parsed::Rejected);
    }
    let edged = text.trim_matches(EDGE_BLANKS);
    // A line feed or tab inside the number goes to the Decimal reading, which rejects it
    // (types/decimal-integer/ScEvalDecimalWhitespace.al:158,166). So do a carriage return, vertical tab and
    // form feed: dropped before and after the number, rejected inside it and alone (Y6EvalWhitespaceMore.al:111-143, corpus
    // master).
    if number == 0
        && text.is_ascii()
        && !text.contains('$')
        && edged
            .chars()
            .all(|c| !c.is_whitespace() || matches!(c, ' ' | '\n' | '\t' | '\r' | '\u{B}' | '\u{C}'))
    {
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
    // Number 9 drops a TAB before the number and reads a point with no fraction (`\t5`, `5.`: format-evaluate/format-matrix/
    // FMEvalInteger.al:152, :232, corpus PR 1249).
    let text = if number == 9 {
        text.trim_start_matches(['\t', ' '])
    } else {
        text
    };
    if text.chars().any(|c| c.is_whitespace() && c != ' ') {
        return unsupported(format!(
            "Evaluate of `{text}` into Integer with white space other than spaces (not pinned)"
        ));
    }
    let t = text.trim_matches(' ');
    let t = match t.strip_suffix('.') {
        Some(w) if number == 9 && !w.is_empty() && w.bytes().all(|b| b.is_ascii_digit()) => w,
        _ => t,
    };
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
/// (types/decimal-integer/ScT70055.al:EvaluateBigInteger; types/P129Evaluate.al:
/// P129_Evaluate_IntegerAndBigInteger_BlanksAndOddForms; the BigInteger cases of types/TestProbeBigInteger: `%PDF-1.4`).
/// An exponent, commas in format number 9, a `-` followed by blanks and a point with no fraction are not pinned.
fn big_integer(text: &str, number: i64) -> Result<Parsed, Signal> {
    // The matrix (format-evaluate/format-matrix/FMEvalBigInteger.al, corpus PR 1249): a TAB before the number is dropped (`\t5`
    // :231-233); NUL, `?` and `1''000` are rejected (:225-230, :301-303); `(5)` is -5 with format 0 and rejected with 9 (:304,
    // :340-341); an exponent reads with format 0 when the value is whole and in range (`1e3`, `1E3` :163-168), else it is
    // rejected (`09e738195`, `1e-5` :169-215), and format 9 rejects it; commas with format 9 are rejected (:99, :146); a point
    // with no fraction is dropped (`5.` :151-153).
    let text = text.strip_prefix('\t').unwrap_or(text);
    if matches!(text, "\0" | "?" | "1''000") {
        return Ok(Parsed::Rejected);
    }
    if let Some(inner) = text.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        if !inner.is_empty() && inner.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(match (number, format!("-{inner}").parse::<i64>()) {
                (0, Ok(n)) => Parsed::Value(Value::BigInteger(n)),
                (0, Err(_)) => {
                    return unsupported(format!(
                        "Evaluate of `{text}` into BigInteger beyond its range (not pinned)"
                    ));
                }
                _ => Parsed::Rejected,
            });
        }
    }
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
        if number == 9 {
            return Ok(Parsed::Rejected);
        }
        return match decimal(t, 0)? {
            Parsed::Value(Value::Decimal(d)) if d.fract().is_zero() => Ok(d
                .to_i64()
                .map_or(Parsed::Rejected, |n| Parsed::Value(Value::BigInteger(n)))),
            Parsed::Value(_) | Parsed::Rejected => Ok(Parsed::Rejected),
            _ => unsupported(format!(
                "Evaluate of `{text}` into BigInteger with an exponent (not pinned)"
            )),
        };
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
    // `100-` reads as -100 with format number 0 (format-evaluate/EvaluateDecimal.al:QR_Evaluate_Integer_OddCharacters
    // `big trailing minus`); other trailing-sign forms are not pinned.
    if number == 0 {
        if let Some(digits) = t
            .strip_suffix('-')
            .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        {
            return Ok(match format!("-{digits}").parse::<i64>() {
                Ok(n) => Parsed::Value(Value::BigInteger(n)),
                Err(_) => Parsed::Rejected,
            });
        }
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
        return Ok(Parsed::Rejected);
    }
    let int: String = int.chars().filter(|c| *c != ',').collect();
    match frac {
        Some("") if !int.is_empty() => {}
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
/// Pins: format-evaluate/EvalDecimalEn.al:P175_En_ReturnValueAndLeftBehind and `...ret9[...]` (EvalDecimalDe.al
/// the same in German), types/decimal-integer/ScT70055.al:EvaluateDecimal_Valid and EvaluateDecimal_Invalid
/// (`+1.5`, `.5`, `5.`, `1E3`, `1.5e-2`, `1E`, `1E40`, `1.2.3`, `.`, `50%`, `$5`, `NaN`), format-evaluate.
fn decimal(text: &str, number: i64) -> Result<Parsed, Signal> {
    // NUL and `?` are no number (format-evaluate/format-matrix/FMEvalDecimal.al:225-230, FMEvalInteger.al:225-230, corpus PR 1249).
    if rejected_characters(text) || matches!(text, "\0" | "?") {
        return Ok(Parsed::Rejected);
    }
    // Tab, carriage return and line feed around the number are dropped like spaces, with format number 0 and 9
    // (format-evaluate/EvaluateDecimal.al:QR_Evaluate_WhiteSpace_Number0, _Number9). Inside it they are not pinned.
    let mut t = text.trim_matches(EDGE_BLANKS);
    if number == 0 {
        // A trailing vertical tab or form feed is dropped too (types/decimal-integer/
        // ScEvalDecimalWhitespace.al:58-59,69-70), and leading ones too: `<VT>1.5`, `<FF>1.5`, `<VT><FF>1.5`, `<FF>-1.5`
        // (types/decimal-integer/Y6EvalWhitespaceMore.al:49-73, corpus master).
        t = t.trim_end_matches([' ', '\t', '\r', '\n', '\u{B}', '\u{C}']);
        t = t.trim_start_matches(['\u{B}', '\u{C}']);
        // A line feed or tab inside the number rejects it, like a space: `1<LF>5`, `1<TAB>5`, `-<LF>1.5`, `1<LF>.5`
        // (ScEvalDecimalWhitespace.al:116-120,126-140; for an Integer :158,166). Before a trailing minus it is not pinned
        // (`100 -` reads as -100), nor before a group of three digits (`1<TAB>000` might group thousands). A carriage return,
        // vertical tab or form feed inside rejects the same way: `1<CR>5`, `1<CR><LF>5`, `1<VT>5`, `1<FF>5`, `1<CR>.5`,
        // `1.<CR>5`, `-<CR>1.5` (Y6EvalWhitespaceMore.al:75-109, corpus master).
        let inner = |c: char| matches!(c, '\n' | '\t' | '\r' | '\u{B}' | '\u{C}');
        let before_group = t.char_indices().filter(|(_, c)| inner(*c)).any(|(i, _)| {
            let rest = &t.as_bytes()[i + 1..];
            rest.len() >= 3
                && rest[..3].iter().all(u8::is_ascii_digit)
                && rest.get(3).is_none_or(|b| !b.is_ascii_digit())
        });
        if t.contains(inner)
            && !t.ends_with('-')
            && !before_group
            && t.chars().all(|c| !c.is_whitespace() || c == ' ' || inner(c))
        {
            return Ok(Parsed::Rejected);
        }
    }
    if t.chars().any(|c| c.is_whitespace() && c != ' ') {
        return unsupported(format!(
            "Evaluate of `{text}` into Decimal with white space other than spaces (not pinned)"
        ));
    }
    // `100 -` reads as -100 with format number 0, and is rejected with 9 (EvaluateDecimal.al:QR_Evaluate_Signs_Number0,
    // _Number9 `100 -`).
    let plain = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit() || b == b',' || b == b'.');
    if number == 0 {
        if let Some(body) = t.strip_suffix('-').map(|b| b.trim_end_matches(' ')) {
            // Pinned with one space only; more spaces are refused.
            if body.len() + 2 < t.len() && plain(body) {
                return unsupported(format!(
                    "Evaluate of `{text}` into Decimal (form not pinned by the corpus)"
                ));
            }
            if body.len() + 2 == t.len() && plain(body) {
                return match decimal(body, number)? {
                    Parsed::Value(Value::Decimal(d)) => Ok(Parsed::Value(Value::Decimal(-d))),
                    other => Ok(other),
                };
            }
        }
    }
    if t.contains(' ') {
        // `1 3` (format-evaluate via the oracle, P129Evaluate.al).
        return Ok(Parsed::Rejected);
    }
    if number == 0 && number_list(t) {
        return Ok(Parsed::Rejected);
    }
    // A trailing minus sign and parentheses mark a negative number: `100-` and `(100)` read as -100
    // (format-evaluate/EvaluateDecimal.al:Evaluate_SignsBlanksAndGarbage `trailing_minus`, `parentheses`).
    // Pinned for a plain number (digits, commas, a point); format number 9 rejects both (EvaluateDecimal.al:
    // QR_Evaluate_Signs_Number9 `100-`, `(100)`; QR_Evaluate_Statement_OddCharacters `parentheses 9`). `(-100)` is rejected
    // (QR_Evaluate_Signs_Number0); other forms are refused.
    if t.strip_prefix("(-")
        .and_then(|r| r.strip_suffix(')'))
        .is_some_and(plain)
    {
        return Ok(Parsed::Rejected);
    }
    let trailing = t.strip_suffix('-').filter(|b| plain(b));
    let wrapped = t
        .strip_prefix('(')
        .and_then(|r| r.strip_suffix(')'))
        .filter(|b| plain(b));
    if let Some(body) = trailing.or(wrapped) {
        if number == 9 {
            return Ok(Parsed::Rejected);
        }
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
    // With format number 0 a `$` after a plain number is a currency sign too: `5$` is 5 (EvaluateDecimal.al:
    // QR_Evaluate_Punctuation_Number0, _NoNumber). Format number 9 rejects every `$` form (_Number9).
    if number == 0 {
        if let Some(body) = t.strip_suffix('$').filter(|b| b.bytes().all(|c| c.is_ascii_digit())) {
            if !body.is_empty() {
                return decimal(body, number);
            }
        }
    }
    let (currency, t) = match t.strip_prefix('$') {
        Some(rest) if number == 0 => (true, rest),
        _ => (false, t),
    };
    let (sign, t) = match t.strip_prefix('-') {
        Some(rest) => (Some('-'), rest),
        None => match t.strip_prefix('+') {
            Some(rest) => (Some('+'), rest),
            None => (None, t),
        },
    };
    let negative = sign == Some('-');
    // `$-5` and `-$5` read as -5 with format number 0 (QR_Evaluate_Punctuation_Number0 and _NoNumber).
    let (currency, t) = match t.strip_prefix('$') {
        Some(rest) if number == 0 && !currency && negative => (true, rest),
        _ => (currency, t),
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
    if currency && sign == Some('+') {
        return unsupported(format!(
            "Evaluate of `{text}` into Decimal (a plus sign and a currency sign together are not pinned)"
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
                // A huge positive exponent of a number that is not zero is beyond the range: rejected (format-evaluate/format-matrix/
                // FMEvalDecimal.al:169-171, FMEvalInteger.al:169-171 `09e738195`, corpus PR 1249).
                _ if !eneg && format!("{int}{frac}").bytes().any(|b| b != b'0') => return Ok(Parsed::Rejected),
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
    // 15 do (format-evaluate/EvaluateWideDecimal.al:Evaluate_Decimal_IntegerDigits d15 against d16, _Fraction, _Negative).
    // The same size written with an exponent is not pinned.
    if int_digits >= 16 {
        if exp != 0 {
            return unsupported(format!(
                "Evaluate of `{text}` into Decimal with an exponent and 16 or 17 digits before the point (not pinned)"
            ));
        }
        return Ok(Parsed::Rejected);
    }
    // With an integer part of zero, up to 29 fractional digits read (types/decimal-integer/DecimalDigits.al:
    // P163_Decimal_Operands_RoundTrip `19_decimals`, `28_decimals`, `29_decimals`; Format shows 18 of them).
    // With a nonzero integer part, only 18 significant digits are pinned: more is refused.
    // With a nonzero integer part the digits past the 18th are dropped: `1.123456789012345678901` reads as 1.12345678901234567
    // (format-evaluate/EvalLastError.al:Evaluate_Decimal `many decimals`). Pinned without an exponent only.
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
    // and the 28 and 29 digit forms read as 0.123456789012345678 (format-evaluate/EvaluateDecimal.al:37 and :50,
    // types/decimal-integer/DecimalDigits.al:P163_Decimal_Operands_RoundTrip, AddSubtract_KeepsDigits). Pinned with a first
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
    // The matrix (format-evaluate/format-matrix/FMEvalBoolean.al, corpus PR 1256) pins number 9 in English and German: `true`,
    // `false` in any case, `1`, `0`, `T` and `F` read; `yes`, `Y`, `n`, `Yes `, ` No`, `-1`, `2`, `Ja`, `on`, NUL, `?`, blanks
    // and the empty text are rejected. Lower-case `t` and `f` and blanks around a word that reads are not pinned.
    if number == 9 {
        return match text {
            t if t.eq_ignore_ascii_case("true") || t == "1" || t == "T" => Ok(Parsed::Value(Value::Boolean(true))),
            t if t.eq_ignore_ascii_case("false") || t == "0" || t == "F" => Ok(Parsed::Value(Value::Boolean(false))),
            "t" | "f" => unsupported(format!(
                "Evaluate of `{text}` into a Boolean with format 9 (not pinned)"
            )),
            t if ["true", "false", "1", "0"]
                .iter()
                .any(|w| t.trim().eq_ignore_ascii_case(w)) =>
            {
                unsupported(format!(
                    "Evaluate of `{text}` into a Boolean with format 9 (not pinned)"
                ))
            }
            _ => Ok(Parsed::Rejected),
        };
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

/// A Boolean text under a language other than English (see [`crate::filter_boolean_words::evaluate_boolean_word`]).
/// German texts that are no word of the table are pinned too (xmlports/shapes-properties/Y83XpMiscTests.al:Boolean_VT_German,
/// Boolean_XML_German: `TRUE` reads as in English). Other texts are not pinned and are refused.
fn language_boolean(language: i32, text: &str) -> Option<Result<Parsed, Signal>> {
    use crate::filter_boolean_words::{EvaluateWord, evaluate_boolean_word};
    Some(match evaluate_boolean_word(language, text)? {
        EvaluateWord::Value(b) => Ok(Parsed::Value(Value::Boolean(b))),
        EvaluateWord::Rejected => Ok(Parsed::Rejected),
        EvaluateWord::NotPinned => unsupported(format!(
            "Evaluate of the Boolean text `{text}` under GlobalLanguage {language} (not pinned)"
        )),
    })
}

/// [`boolean`] with the words of the session language when it has pinned ones (number 0 only).
fn boolean_in_language(language: Option<i32>, text: &str, number: i64) -> Result<Parsed, Signal> {
    match language.filter(|_| number == 0).and_then(|l| language_boolean(l, text)) {
        Some(r) => r,
        None => boolean(text, number),
    }
}

fn raise_boolean_error(language: i32, text: &str) -> Result<Value, Signal> {
    crate::native::raise(boolean_error(language, text), "Evaluate")
}

/// The error text of a rejected Boolean under Danish, French or Dutch (the quotes of the French text are \u{ab} and \u{bb}
/// with a no-break space inside; format-evaluate/EvalLanguageBoolean.al EL_Errors_NLB_2067, _NLD_1043, _FRA_1036, _DAN_1030).
fn boolean_error(language: i32, text: &str) -> String {
    match language {
        1036 => {
            format!("La valeur \u{ab}\u{a0}{text}\u{a0}\u{bb} ne peut pas \u{ea}tre \u{e9}valu\u{e9}e en type Boolean.")
        }
        1030 => format!("V\u{e6}rdien \"{text}\" kan ikke evalueres til typen Boolean."),
        _ => format!("De waarde \"{text}\" kan niet worden ge\u{eb}valueerd tot type Boolean."),
    }
}

fn guid(text: &str) -> Result<Parsed, Signal> {
    // Blanks around the text do not count (types/decimal-integer/ScT70054.al:Guid_EvaluateInvalid `spaces`).
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

/// `Evaluate` into a Byte or a Char (types/decimal-integer/ScT70055.al:205-249, EvaluateByte_Char; misc/lt-system-functions/
/// EvalLastError.al:342-343): one character is its code (`0` is 48, `x`, `A`); a number of two or more characters (`255`,
/// `256`, `-1`, `65`, `70000`) and, for a Char, the empty text are rejected. Every other text is not pinned and is refused.
fn byte_or_char(target: &Value, text: &str) -> Result<Parsed, Signal> {
    let mut chars = text.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        return match target {
            Value::Byte(_) => match u8::try_from(u32::from(ch)) {
                Ok(b) if ch.is_ascii() => Ok(Parsed::Value(Value::Byte(b))),
                _ => unsupported(format!(
                    "Evaluate of `{text}` into a Byte (only an ASCII character is pinned)"
                )),
            },
            _ => match u16::try_from(u32::from(ch)) {
                Ok(c) if ch.is_ascii() => Ok(Parsed::Value(Value::Char(c))),
                _ => unsupported(format!(
                    "Evaluate of `{text}` into a Char (only an ASCII character is pinned)"
                )),
            },
        };
    }
    let digits = text.strip_prefix('-').unwrap_or(text);
    if text.len() >= 2 && !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(Parsed::Rejected);
    }
    if text.is_empty() && matches!(target, Value::Char(_)) {
        return Ok(Parsed::Rejected);
    }
    // Letters into a Char: the statement raises the Evaluate error of a Char (format-evaluate/
    // Y49EvaluateDateBlanks.al:299-330 `ab`); with the result used it is not pinned.
    if matches!(target, Value::Char(_)) && text.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Ok(Parsed::RejectedUsedRefused(format!(
            "Evaluate of `{text}` into a Char with the result used (not pinned)"
        )));
    }
    unsupported(format!(
        "Evaluate of `{text}` into a {} (not pinned)",
        target.describe()
    ))
}

/// Number 0: the inverse of `Format(Date)` as the runtime formats it (`MM/DD/YY`,
/// docs/probes-needed.md); a two-digit year below 50 is 20yy and from 50 on 19yy. Number 9: ISO `yyyy-MM-dd`.
/// `today` is the host's current day, for the text `T`.
///
/// Pinned by format-evaluate/P86Eval.al, codeunit "LT P86 Eval Date": number 0 reads
/// `yyyy-mm-dd`, `m/d/yyyy`, `m/d/yy` and `T`; it rejects `15.01.2024`, `150124`, `15012024`, `20240115`,
/// `2024-01-15T00:00:00Z`, `0D` and `15-01-2024`. Number 9 reads `yyyy-mm-dd` with spaces around it and the empty text
/// (which leaves the date empty) and rejects everything else, `T` and the slash forms included.
/// The month-first forms of a Date in the session's language (en-US): digits only, as `MMDD` followed by a year of 0 to 4 digits
/// (`1010` is that day of this year, `10101` is 2001-10-10, `101010` is 2010-10-10, `1010101` is 0101-10-10, `10162026`), and three numbers
/// split by one `/`, `.` or `-` (`1/2/3`, `10.16.26`, `10-16-2026`), month first. A year of one or two digits is 20xx below 50 and 19xx
/// from 50. A month or day that does not exist rejects (`161026`, `311299`, `16/10/26`, `16.10.2026`). `16 Oct 2026` reads, while
/// `Oct 16, 2026`, `16 10 26` and `16/10` do not (types/P130EvaluateDate.al: Default_SixAndEightDigits,
/// Default_SlashAndDotForms; types/date-time/Evaluate.al: Date_Other_Formats, Date_Default_Evaluate `formatted 4`). `None` for
/// every other shape, which the caller handles; a shape the corpus does not pin is refused.
fn date_month_first(text: &str, today: Option<i32>) -> Option<Result<Parsed, Signal>> {
    let ymd = |y: i32, m: u32, d: u32| -> Result<Parsed, Signal> {
        if !(1..=12).contains(&m) || d < 1 {
            return Ok(Parsed::Rejected);
        }
        // The first day of the calendar is the empty date, as in the ISO form (format-evaluate/format-matrix/FMEvalDate.al:196-198
        // `1/1/0001`, corpus PR 1250).
        if (y, m, d) == (1, 1, 1) {
            return Ok(Parsed::Value(Value::Date(0, false)));
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
    // A year of five digits after slashes is no date (format-evaluate/EvaluateDate3DigitYear.al:
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

/// The text without its blanks when it has blanks around three groups of digits split by one `-`, `.` or `/`.
fn blank_padded_numeric_date(text: &str) -> Option<&str> {
    let t = text.trim_matches(' ');
    if t.len() == text.len() || t.is_empty() {
        return None;
    }
    ['-', '.', '/']
        .into_iter()
        .any(|sep| {
            let parts: Vec<&str> = t.split(sep).collect();
            parts.len() == 3
                && parts
                    .iter()
                    .all(|p| (1..=4).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
        })
        .then_some(t)
}

/// `month_first` turns on the month-first forms (`date_month_first`) and the stricter closing date, as `Evaluate` reads a Date; a
/// reader of filter text keeps the forms it had (the filter forms are pinned apart from Evaluate, docs/probes-needed.md).
fn date(text: &str, number: i64, today: Option<i32>, work: Option<i32>, month_first: bool) -> Result<Parsed, Signal> {
    if number == 9 {
        // White space alone reads as the empty date, and around a date it is dropped (corpus master, corpus PR 886:
        // format-evaluate/EvaluateDateFormat9Space.al:49-81 TAB, blanks, CR, LF, CR LF, VT, FF,
        // no-break space alone; :90-113 TAB, blank, CR LF, LF and no-break space around 2026-10-16). A TAB inside the date rejects
        // it (:114-120). Other white space, VT or FF next to a date and other white space inside one are not pinned.
        const PINNED_WS: [char; 7] = [' ', '\t', '\r', '\n', '\u{B}', '\u{C}', '\u{A0}'];
        if text.chars().any(|c| c.is_whitespace() && !PINNED_WS.contains(&c)) {
            return unsupported(format!(
                "Evaluate of `{text}` into a Date with format 9 and this white space (not pinned)"
            ));
        }
        let t = text.trim_matches(PINNED_WS);
        if t.is_empty() {
            return Ok(Parsed::Value(Value::Date(0, false)));
        }
        if t.len() != text.trim_matches(' ').len() && text.contains(['\u{B}', '\u{C}']) {
            return unsupported(format!(
                "Evaluate of `{text}` into a Date with format 9: VT or FF next to a date (not pinned)"
            ));
        }
        // Blanks inside keep their old reading (a rejected date, P86Eval.al:Fmt92_Return_F9 `Jan 15 2024`).
        if t.contains(|c: char| c != ' ' && PINNED_WS.contains(&c)) {
            if t.contains('\t') && t.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '\t') {
                return Ok(Parsed::Rejected);
            }
            return unsupported(format!(
                "Evaluate of `{text}` into a Date with format 9: white space inside (not pinned)"
            ));
        }
        // The first day of the calendar reads as the empty date, and year 0 as the day after it
        // (types/date-time/Evaluate.al:Date_Format9 `iso min`, `iso 0D` and `iso year 0`).
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
    // The empty text leaves the date empty in format 0 as in format 9 (types/date-time/Evaluate.al:Date_Default_Evaluate,
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
    // (types/date-time/P143Date_FieldRef.al:P143_EvaluateRelations `eval_w_equals_workdate`,
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
        // The ISO form after the C is no closing date (types/date-time/Evaluate.al:Date_Default_Evaluate `closing`: No).
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
        if let Some(r) = date_forms::date_text(text, today, work) {
            return r;
        }
        // Blanks before and after a date of three numbers split by `-`, `.` or `/`, or an ISO date, are dropped
        // (format-evaluate/Y71EvaluateDashedDates.al:73-145: `01-01-2012 `, ` 01-01-2012 `, `11-11-12 `,
        // `2012-01-31 `, `05.03.2024 `, ` 3/5/24 `, and the statement form with one or two trailing blanks).
        if let Some(trimmed) = blank_padded_numeric_date(text) {
            return date(trimmed, number, today, work, month_first);
        }
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
    // Digits in groups split by one blank are no date: `2024 03 05`, and `12 34`, which with the result used leaves a last
    // error (format-evaluate/Y49EvaluateDateBlanks.al:79-107, :207-233 `err=Yes`).
    let groups: Vec<usize> = text.split(' ').map(str::len).collect();
    if text.bytes().all(|b| b.is_ascii_digit() || b == b' ') {
        match groups.as_slice() {
            [4, 2, 2] => return Ok(Parsed::Rejected),
            [2, 2] => {
                return Ok(Parsed::RejectedUsedRefused(format!(
                    "Evaluate of `{text}` into Date with the result used: the last error it leaves is not pinned"
                )));
            }
            _ => {}
        }
    }
    // A day, a month written as the German word `Dezember` and a year is no date in either language
    // (xmlports/shapes-properties/Y83XpMiscTests.al:DateDecimal_VT_German, DateDecimal_VT_English `31. Dezember 2024`, needs the
    // corpus bump). Another month word is not pinned.
    if let Some((day, rest)) = text.split_once(". Dezember ") {
        if digits(day, 1..=2) && digits(rest, 4..=4) {
            return Ok(Parsed::Rejected);
        }
    }
    // A day, a month word and a year, in every session language (format-evaluate/EvalLanguageBoolean.al EL_Date_*, the same on the
    // three supported majors and under 1030, 1031, 1033, 1036, 1043 and 2067): `31 december 2026` reads as 2026-12-31, while
    // `31 Dezember 2026` and `Dec 31 2026` are rejected. Other month words and days are not pinned.
    {
        let words: Vec<&str> = text.split(' ').collect();
        if let [d, m, y] = words.as_slice() {
            if digits(d, 1..=2) && digits(y, 4..=4) {
                if *m == "december" {
                    if let (Ok(day), Ok(year)) = (d.parse::<u32>(), y.parse::<i32>()) {
                        if (1..=31).contains(&day) {
                            return Ok(Parsed::Value(Value::Date(day_from_civil(year, 12, day), false)));
                        }
                    }
                }
                if *m == "Dezember" {
                    return Ok(Parsed::Rejected);
                }
            }
            if *d == "Dec" && digits(m, 1..=2) && digits(y, 4..=4) {
                return Ok(Parsed::Rejected);
            }
        }
    }
    unsupported(format!(
        "Evaluate of `{text}` into Date (form not pinned by the corpus)"
    ))
}

/// `31. Dezember 2024`: a day, the German month word and a year is no date in either language, and gets the date error
/// (xmlports/shapes-properties/Y83XpMiscTests.al:DateDecimal_VT_German, needs the corpus bump).
fn dezember_date_text(text: &str) -> bool {
    let all =
        |t: &str, n: std::ops::RangeInclusive<usize>| n.contains(&t.len()) && t.bytes().all(|b| b.is_ascii_digit());
    text.split_once(". Dezember ")
        .is_some_and(|(d, y)| all(d, 1..=2) && all(y, 4..=4))
}

/// A time of day, as a `Value::Time` holds it (milliseconds since midnight, plus one).
///
/// Pinned by format-evaluate/P86Eval.al, codeunit "LT P86 Eval Time". Number 0 reads
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
    // Format 0 (format-evaluate/format-matrix/FMEvalTime.al, corpus PR 1250): a clock followed by `T` or `D` (`12:00:00T`
    // :196-201), a colon at either end (`12:`, `:30` :271-276, :385-390) and four digits of a second (`12:00:00.1234` :145-147)
    // are rejected; a comma before the milliseconds reads as a point (`12:00:00,123` :148-150).
    if number != 9 {
        let clock = |c: &str| {
            let p: Vec<&str> = c.split(':').collect();
            p.len() == 3 && p.iter().all(|x| x.len() == 2 && x.bytes().all(|b| b.is_ascii_digit()))
        };
        if t.strip_suffix(['T', 'D']).is_some_and(clock) || t.starts_with(':') || t.ends_with(':') {
            return Ok(Parsed::Rejected);
        }
        if let Some((c, f)) = t.split_once(['.', ',']) {
            if clock(c) && f.bytes().all(|b| b.is_ascii_digit()) {
                if f.len() == 4 {
                    return Ok(Parsed::Rejected);
                }
                if t.contains(',') && f.len() == 3 {
                    return time(&format!("{c}.{f}"), number);
                }
            }
        }
    }
    if number == 9 {
        let body = t.strip_suffix('Z').unwrap_or(t);
        let (clock, frac) = match body.split_once('.') {
            Some((c, f)) => (c, Some(f)),
            None => (body, None),
        };
        let parts: Vec<&str> = clock.split(':').collect();
        let strict = parts.len() == 3 && parts.iter().all(|p| p.len() == 2);
        let f = match frac {
            // Four digits keep the milliseconds: `13:45:06.1234` is 13:45:06.123 (types/date-time/Evaluate.al:209,233
            // Time_Format9 `hms ms 4 digits`). Whether a fourth digit of 5 or more rounds up, and more digits, are not pinned.
            Some(f) if f.len() >= 4 && f.bytes().all(|b| b.is_ascii_digit()) => {
                if f.len() == 4 && f.as_bytes()[3] < b'5' {
                    fraction(&f[..3])
                } else {
                    return unsupported(format!(
                        "Evaluate of the Time `{t}` with format 9: more than three digits of a second (rounding not pinned)"
                    ));
                }
            }
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
    // AM or PM after a blank, after a narrow no-break space (U+202F; Format itself writes a blank) or right after the
    // number, in any case, and a narrow no-break space after it (format-evaluate/AjScalars.al:QR_Evaluate_Time_NarrowNoBreakSpace:
    // `12:00:00~PM`, `12:00:00~pm`, `12:00:00PM`, `12:00:00 PM`, `12:00:00~PM~`; P86Eval.al `h:m[:s] AM|PM`).
    let designated = t.strip_suffix('\u{202F}').unwrap_or(t);
    let (body, meridiem) = match designated
        .len()
        .checked_sub(2)
        .and_then(|i| designated.get(i..).map(|m| (i, m)))
    {
        // Only the spellings AM, PM, am and pm are pinned; mixed case is refused.
        Some((_, m))
            if (m.eq_ignore_ascii_case("AM") || m.eq_ignore_ascii_case("PM"))
                && !matches!(m, "AM" | "PM" | "am" | "pm") =>
        {
            return unsupported(format!(
                "Evaluate of `{text}` into Time (form not pinned by the corpus)"
            ));
        }
        Some((i, m)) if matches!(m, "AM" | "PM" | "am" | "pm") => {
            let b = &designated[..i];
            let old_form = b.ends_with(' ') && (m == "AM" || m == "PM") && designated.len() == t.len();
            let b = b.strip_suffix([' ', '\u{202F}']).unwrap_or(b);
            // The new forms are pinned after `h:m[:s]` only.
            if old_form || b.contains(':') {
                (b, Some(m.eq_ignore_ascii_case("PM")))
            } else {
                (t, None)
            }
        }
        _ => (t, None),
    };
    // Two date-shaped texts read as a time with format number 0: `2021-12-19` is 20:21:12.19 (hhmm, seconds, hundredths) and
    // `12/19/21` is 12:19:21 (format-evaluate/AjScalars.al:Evaluate_Time_FromDateText). Pinned for these exact shapes and
    // values inside the clock only; anything else of that kind stays refused.
    let digits_at = |s: &str, at: &[usize]| {
        s.bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_digit() == !at.contains(&i))
    };
    if meridiem.is_none()
        && body.len() == 10
        && digits_at(body, &[4, 7])
        && body.as_bytes()[4] == b'-'
        && body.as_bytes()[7] == b'-'
    {
        let (h, m, s, f) = (
            num(&body[..2], 2),
            num(&body[2..4], 2),
            num(&body[5..7], 2),
            num(&body[8..], 2),
        );
        if let (Some(h), Some(m), Some(s), Some(f)) = (h, m, s, f) {
            if in_range(h, m, s) {
                return Ok(value(h, m, s, f * 10));
            }
        }
    }
    if meridiem.is_none()
        && body.len() == 8
        && digits_at(body, &[2, 5])
        && body.as_bytes()[2] == b'/'
        && body.as_bytes()[5] == b'/'
    {
        let (h, m, s) = (num(&body[..2], 2), num(&body[3..5], 2), num(&body[6..], 2));
        if let (Some(h), Some(m), Some(s)) = (h, m, s) {
            if in_range(h, m, s) {
                return Ok(value(h, m, s, 0));
            }
        }
    }
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
        // `hhmmss` reads as hours, minutes and seconds and an hour from 24 is rejected (format-evaluate/
        // AxEvaluateText.al:28-102: `085606`, `246000`), and so are minutes or seconds from 60 (record/
        // Y70Probes.al:123-170 `086000`, `085960`).
        num(&body[..2], 2)
            .zip(num(&body[2..4], 2))
            .zip(num(&body[4..], 2))
            .map(|((h, m), s)| (h, m, s, 0))
    } else if (1..=2).contains(&body.len()) && number == 0 && body.bytes().all(|b| b.is_ascii_digit()) {
        // One or two digits are an hour; 24 is rejected (AxEvaluateText.al:28-102: `23`, `5`, `24`).
        num(body, 2).map(|h| (h, 0, 0, 0))
    } else {
        None
    };
    let Some((mut h, m, s, f)) = nums else {
        return unsupported(format!(
            "Evaluate of `{text}` into Time (form not pinned by the corpus)"
        ));
    };
    if let Some(pm) = meridiem {
        // `0:30:00 AM` is 00:30:00 and `13:00:00 PM` is rejected (AjScalars.al:QR_Evaluate_Time_NarrowNoBreakSpace); an hour
        // of 0 with PM, 13 with AM and 14 to 23 are not pinned.
        if h == 0 && !pm {
            return Ok(if in_range(h, m, s) {
                value(h, m, s, f)
            } else {
                Parsed::Rejected
            });
        }
        if h == 13 && pm {
            return Ok(Parsed::Rejected);
        }
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
/// Pinned by format-evaluate/P86Eval.al, codeunit "LT P86 Eval DateTime". Number 9 reads
/// `yyyy-mm-dd[Thh:mm:ss[.f][Z|+hh:mm]]` (no zone means UTC) and the empty text, and rejects everything else. Number 0
/// reads the same with `Z` or an offset, the empty text, `m/d/y h:mm[:ss] [AM|PM]` and the forms of `datetime_forms`; it
/// rejects a time past 23:59:59, a day that does not exist and `0D`, `0T`, `0DT`. Not read: the ISO form without a zone in
/// number 0 (BC raises `The date is not valid.`).
fn datetime(text: &str, number: i64, today: Option<i32>) -> Result<Parsed, Signal> {
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
    if let Some(p) = rfc1123(t) {
        return Ok(p);
    }
    match datetime_forms::read(t, today) {
        Some(Some(p)) => return Ok(p),
        Some(None) => {
            return unsupported(format!(
                "Evaluate of `{text}` into DateTime with format {number} (value not pinned)"
            ));
        }
        None => {}
    }
    if iso && t.as_bytes().get(10) == Some(&b'T') {
        // Without a zone it is rejected, and the statement form raises `The date is not valid.` (P86Eval.al:933-1110, `t2`
        // `2024-01-15T10:20:30` without a number and with 0).
        if !(t.ends_with(['Z', 'z']) || t[11..].contains(['+', '-'])) {
            return Ok(Parsed::RejectedWith("The date is not valid.", "DateInvalid"));
        }
        return Ok(match crate::iso::parse_datetime(t) {
            Some(v) => Parsed::Value(Value::DateTime(v)),
            None => Parsed::Rejected,
        });
    }
    if let Some(r) = date_forms::datetime_text(t, today) {
        return r;
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

/// The RFC 1123 form of an HTTP date header, `Fri, 05 Sep 2025 14:23:45 GMT`, in number 0 (and without a number): read as UTC
/// with `GMT` or `+0000`; a weekday that is not the date's is rejected without an error; without the weekday the text is rejected
/// and the error is `The date is not valid.`, also when the result is used (format-evaluate/UfEvaluateJsonRfc.al:
/// 19-60, :62-85; al-runner issue 6172). Number 9 rejects it (the ISO reader above). Another zone or offset, a one-digit day and
/// a time or date out of range are not pinned: `None`, and the caller refuses.
fn rfc1123(t: &str) -> Option<Parsed> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let parts: Vec<&str> = t.split(' ').collect();
    let (weekday, rest) = match parts.as_slice() {
        [w, d, m, y, c, z] if w.len() == 4 && w.ends_with(',') => (Some(&w[..3]), [*d, *m, *y, *c, *z].to_vec()),
        [d, m, y, c] => (None, vec![*d, *m, *y, *c]),
        _ => return None,
    };
    let [d, m, y, clock] = [rest[0], rest[1], rest[2], rest[3]];
    let month = MONTHS.iter().position(|x| *x == m)? + 1;
    if d.len() != 2 || y.len() != 4 || clock.len() != 8 || !d.bytes().chain(y.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    if weekday.is_none() {
        // Pinned by format-evaluate/UfEvaluateJsonRfc.al DateTime_Rfc1123 (was assumption `evaluate-rfc-date-time`).
        return Some(Parsed::RejectedLastError(
            "The date is not valid.".into(),
            "DateInvalid",
        ));
    }
    if !matches!(rest.get(4), Some(&"GMT" | &"+0000")) {
        return None;
    }
    let weekday = DAYS.iter().position(|x| Some(*x) == weekday)? as i32 + 1;
    let v = crate::iso::parse_datetime(&format!("{y}-{month:02}-{d}T{clock}Z"))?;
    let day = crate::iso::parse_date(&format!("{y}-{month:02}-{d}"))?;
    // Pinned by format-evaluate/UfEvaluateJsonRfc.al DateTime_Rfc1123 (was assumption `evaluate-rfc-date-time`).
    if crate::datefns::weekday(day) != weekday {
        return Some(Parsed::Rejected);
    }
    Some(Parsed::Value(Value::DateTime(v)))
}

/// A TAB, two TABs or one space: the texts Evaluate reads as an empty Date and shows as `""` in a Boolean's error.
fn is_blank_tabs(text: &str) -> bool {
    matches!(text, "\t" | "\t\t" | " ")
}

/// The value of a Date or a Guid in the form `Evaluate` reads it (and `Format` writes it), for a reader that has no
/// variable to evaluate into: `None` for any other type, a text that is no such value, or a form not pinned.
pub fn read_text(ty: &crate::value::RtType, text: &str) -> Option<Value> {
    use crate::value::RtType as T;
    // `Format(ClosingDate(D))` is `C` and the date (corpus types/date-time/DateFuncs.al:47 `C12/31/23`,
    // format-evaluate/RandomFormat.al:432): the closing date of that day.
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
        Parsed::Rejected
        | Parsed::RejectedWith(..)
        | Parsed::RejectedLastError(..)
        | Parsed::RejectedUsedRefused(_) => None,
    }
}

/// A text of ASCII letters, digits and `-./` with letters, read with number 0, 9 or none: its failed Evaluate statement into a
/// Date raises the Date-format error (types/date-time/Evaluate.al:188-197 Date_Evaluate_Raises `abc` with no number;
/// types/P130EvaluateDate.al UN_Evaluate_Statement_NotADate_ByTarget on writer UN's corpus branch,
/// commit d3bcc580: `not-a-date` with 9, 0 and the default, former assumption `evaluate-date-letters-statement` (pinned on corpus master since bump 1030)). Other texts and
/// numbers are not pinned.
fn letters_date_text(text: &str, number: i64) -> bool {
    dezember_date_text(text)
        || (matches!(number, 0 | 9)
            && text.bytes().any(|b| b.is_ascii_alphabetic())
            && text
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'/')))
}

pub fn evaluate(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    evaluate_with(c, a, false)
}

/// `Evaluate` into an Option variable, which has no OptionCaption to name a member by.
pub fn evaluate_variable_option(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    evaluate_with(c, a, true)
}

/// `Evaluate` into an Option field that declares an OptionCaption (the lowering appends the captions as a comma list, or the
/// OptionCaptionML property text and a fifth argument 1): the text names a member by its caption first, else by its name, else
/// it is an ordinal of any size (corpus recordref-fieldref/P128EvalBasic.al:P128_Evaluate_IntoOptionField_NumericCaptions:
/// with captions `1,2,3` the text `1` is the first member, `3` the third and `4` the ordinal 4; with `Ten,Twenty,Thirty` `twenty`
/// is the second member, `Two` the third by its name and `Three` is rejected). Pinned by format-evaluate/
/// OptionEvaluateCaptions.al:
/// - a caption that is the name of another member names its own member (:162 CaptionIsNameOfAnotherMember); of two members with
///   the same caption the first is taken (:177 SameCaptionTwice);
/// - blanks, a TAB and CR LF around the text and around a caption do not count, a text of only a TAB names the blank member;
///   a double space inside or a prefix does not match (:192 BlanksAndPrefixes);
/// - OptionCaptionML: the captions of the session language, ENU where it has none, and the member names; under 1031 the ENU
///   caption is not one (:236 CaptionML_ByLanguage);
/// - the format numbers 2 and 9 read as 0 (:270 FormatNumber);
/// - with the result unused a rejected text raises `'<text>' is not an option. The existing options are: <captions>` with code
///   InvalidOptionString (:330 ResultUnused_Errors).
///
/// A field that declares no caption reads the same way with its member names as captions: a name in any case, else an ordinal of
/// any size; with the result unused the error lists the names (record/filters/P68I_TransferEnumOption.al:303-332 `16`, `6`, `M9`;
/// errors/P137OptionEvaluate.al:155-231 field Three).
///
/// A rejected text gives `false` with the result used.
pub fn evaluate_option_field(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let (Some(text), Some(Value::Text(captions)), Value::Option(_, names)) =
        (crate::native::text_of(&a[1]), a.get(3).cloned(), a[0].clone())
    else {
        return unsupported("Evaluate into an Option field without its captions");
    };
    if !matches!(a.get(2), Some(Value::Integer(0 | 2 | 9))) {
        return unsupported("Evaluate into an Option field with a format number other than 0, 2 and 9 (not pinned)");
    }
    let captions: String = if matches!(a.get(4), Some(Value::Integer(1))) {
        let list = crate::format::ml_text_in_language(captions.as_str(), c.language())
            .map_err(|e| Signal::unsupported(format!("Evaluate into an Option field with OptionCaptionML {e}")))?;
        if list.split(',').count() != names.len() {
            return unsupported("OptionCaptionML with another number of entries than the members (not pinned)");
        }
        list
    } else {
        captions.as_str().to_string()
    };
    let t = text.trim();
    // A field without a caption (fifth argument 2, the fourth its member names): an empty or blank text names a member
    // declared as `" "`, not one declared as `""` (format-evaluate/AxFormatOption.al:208-211).
    let no_caption = matches!(a.get(4), Some(Value::Integer(2)));
    // An empty or blank text names a blank member (format-evaluate/P131EvalOption.al:107-108: `''` and `' '` give the
    // member `" "` of caption `' '`, :146: a field without a blank member rejects `''`).
    let same = |n: &str| {
        if t.is_empty() {
            n.trim().is_empty() && !(no_caption && n.is_empty())
        } else {
            n.trim().eq_ignore_ascii_case(t)
        }
    };
    let by_caption = captions.split(',').position(same);
    let by_name = names.iter().position(|n| same(n));
    // A blank text that names a member only by its blank caption, while the member has a name (`OptionMembers = None,First`,
    // `OptionCaption = ' ,First cap'`): the pins are members named " " or "" (P131EvalOption.al:107-108, FMOptionBlankField.al).
    // Not pinned, listed by the probe of corpus pull request 1283 (EvalNamedBlankCaption.al:41-58): refused.
    if t.is_empty()
        && by_name.is_none()
        && by_caption.is_some_and(|p| names.get(p).is_some_and(|n| !n.trim().is_empty()))
    {
        return unsupported(
            "Evaluate of a blank text into an Option field whose member has a name and a blank OptionCaption (not pinned)",
        );
    }
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
        // The code is pinned for a FieldRef (FMFields.al:510-511 `Draft `, ` Draft`), the text is not.
        None if text.as_str() != t || text.chars().any(char::is_control) => {
            Err(Signal::Error(Box::new(crate::native::unpinned_text_error(
                "InvalidOptionString",
                "the text of the Option error of a text with blanks or control characters",
            ))))
        }
        None => crate::native::raise(
            format!("'{t}' is not an option. The existing options are: {captions}"),
            "InvalidOptionString",
        ),
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

/// `Evaluate` into a Text or a Code (corpus recordref-fieldref/P128EvalBasic.al:P128_Evaluate_IntoText,
/// P128_Evaluate_IntoCode, P128_Evaluate_IntoCode_ResultUnused, P128_Evaluate_IntoShortText_ResultUnused): a Text takes the
/// text as written (blanks kept), a Code takes it trimmed and in capitals; text that is too long for the variable gives
/// `false` when the result is used and the length error (with the text as given) when it is not, and the variable keeps
/// its value. A format number is not pinned.
fn text_or_code(c: &mut dyn Ctx, a: &Value, text: &str, number: i64, max: Option<u32>) -> Result<Parsed, Signal> {
    // The numbers 0, 1, 2 and 9 read a Text and a Code alike (format-evaluate/EvaluateTextCodeDecimal.al:
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
            // With the result used the call answers false and leaves this error as the last one (format-evaluate/
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

/// The tail of the error for an unknown table name: a space, then each part after a real backslash and six spaces, the last one after
/// four spaces (xmlports/shapes-properties/Y88XmlPortProbes.al:209 and :241; W4PlatformAndMeta.al:706 pins the head up to the backslash).
const METADATA_NOT_FOUND_TAIL: &str = " \\      The metadata object Table -1 was not found.\\      The application might be incompatible with the current compiler. Please unpublish the application that contains the application object Table -1 and republish it. \\      Or, run a technical upgrade of the application database with the 'Force' flag set.\\    ";

/// `Evaluate(TableFilterField, Text)` (al-runner issues 5998 and 6006), pinned by record/W4PlatformAndMeta.al (writer SI:
/// TableFilterEvaluate_AcceptedForms, _RejectedForms, _ErrorsAsStatement, _StoredValue; writer SR: TableFilterEvaluate_SeveralCodeFields,
/// _SrMultiFieldForms, _SrSecondFieldTypes, _SrExpressionForms, _SrRejectedForms, _SrStoredSpaceValue). The text is
/// `<table>: <field>=<filter>,<field>=<filter>`: the table by its name in any casing, quoted or by number (not by caption); `:`
/// with or without a space; the conditions split at each comma, trimmed, a trailing comma dropped; each field by name or caption
/// in any casing, quoted or not. The stored value names the table by its name and each field by its caption, the conditions in
/// the order of their field numbers (a field given twice keeps both), joined by a comma alone; Format with 0 and 9 gives it
/// back. The empty text stores the empty value. Each filter is read by [`table_filter_value`]; a form it does not know, another
/// field type and every format number but 0 and 9 are refused.
fn into_table_filter(c: &mut dyn Ctx, a: &mut [Value], text: &str, number: i64) -> Result<Value, Signal> {
    let refuse = |why: &str| {
        unsupported(format!(
            "Evaluate of `{text}` into TableFilter: {why} (not pinned by the corpus)"
        ))
    };
    if number != 0 && number != 9 {
        return refuse("format number");
    }
    let ret_used = c.ret_used();
    // A rejected text: with the result used false and the error as the last one, else the error.
    let reject = |c: &mut dyn Ctx, more: &str, code: &str| -> Result<Value, Signal> {
        let e = crate::native::AlError {
            message: format!("The value \"{text}\" can't be evaluated into type TableFilter.{more}"),
            code: code.into(),
            ..Default::default()
        };
        if ret_used {
            c.set_last_error(e);
            return Ok(Value::Boolean(false));
        }
        Err(Signal::Error(Box::new(e)))
    };
    if text.is_empty() {
        return Ok(store_table_filter(a, ""));
    }
    let Some((table, rest)) = text.split_once(':') else {
        return reject(c, "", "Evaluate");
    };
    let table = table.trim();
    let table = table
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(table);
    let def = match c.table_filter_table(table) {
        Some(Ok(Some(def))) => def,
        // An unknown table name: the whole text is pinned by xmlports/shapes-properties/Y88XmlPortProbes.al:209 (Y88_EvaluateTableFilter_
        // UnknownTableErrorText, 27-29), see METADATA_NOT_FOUND_TAIL.
        Some(Ok(None)) if table.parse::<i64>().is_err() => {
            return reject(c, METADATA_NOT_FOUND_TAIL, "Evaluate:MetadataNotFound");
        }
        Some(Ok(None)) => return reject(c, "", "Evaluate:MetadataNotFound"),
        Some(Err(e)) => return Err(e),
        None => return unsupported("Evaluate into TableFilter without a program"),
    };
    // `<table>: ` with no condition (TableFilterEvaluate_RejectedForms `ret_table_colon_only`).
    if rest == " " {
        return Ok(store_table_filter(a, &format!("{}: ", def.name)));
    }
    let mut parts: Vec<&str> = crate::evaluate_table_filter_forms::split_conditions(rest);
    // A trailing comma is dropped (TableFilterEvaluate_SrRejectedForms `trailing_comma`).
    if parts.len() > 1 && parts.last().is_some_and(|p| p.trim().is_empty()) {
        parts.pop();
    }
    let mut conditions: Vec<(u32, String)> = Vec::new();
    let all_parts = parts.clone();
    for (i, part) in parts.into_iter().enumerate() {
        let part = part.trim();
        if part.is_empty() {
            return refuse("an empty condition");
        }
        let Some((field, value)) = part.split_once('=') else {
            // A second condition without '=': false and no error (`missing_equals_second`). The first condition without '=',
            // and the error as a statement, are not pinned.
            // The thousands comma of an unquoted Decimal `>=100,000` cuts the condition: the whole text is the error
            // (format-evaluate/tablefilter-stored/TableFilterStoredText.al TableFilter_Decimal_UnquotedGroupedGE, a statement).
            if !ret_used && i > 0 && crate::evaluate_table_filter_forms::grouped_split_error(all_parts[i - 1], part) {
                return reject(c, "", "Evaluate");
            }
            if ret_used && i > 0 {
                return Ok(Value::Boolean(false));
            }
            return refuse("a condition without '=' that is not a second condition of a used result");
        };
        let field = field.trim();
        let field = field
            .strip_prefix('"')
            .and_then(|f| f.strip_suffix('"'))
            .unwrap_or(field);
        let mut by_name = def.fields.iter().filter(|f| f.name.eq_ignore_ascii_case(field));
        let found = match (by_name.next(), by_name.next()) {
            (Some(f), None) => Some(f),
            (Some(_), Some(_)) => return refuse("a field name two fields have"),
            (None, _) => {
                let mut by_caption = def
                    .fields
                    .iter()
                    .filter(|f| f.caption.as_ref().is_ok_and(|c| c.eq_ignore_ascii_case(field)));
                match (by_caption.next(), by_caption.next()) {
                    (Some(f), None) => Some(f),
                    (Some(_), Some(_)) => return refuse("a field caption two fields have"),
                    // The declared Caption of a field with a CaptionClass is not pinned as a field of the text.
                    (None, _)
                        if def
                            .fields
                            .iter()
                            .any(|f| f.class_caption.as_ref().is_some_and(|c| c.eq_ignore_ascii_case(field))) =>
                    {
                        return refuse("the declared caption of a field with a CaptionClass");
                    }
                    (None, _) => None,
                }
            }
        };
        // A field by its number when no name or caption matches (types/TestTableFilterEvaluate.al:26-36
        // TableFilter_Evaluate_NumericIds_NormalizesValue: `67021: 2=1`). A number no field has is not pinned.
        let found = match found {
            None if !field.is_empty() && field.chars().all(|ch| ch.is_ascii_digit()) => {
                match field
                    .parse::<u32>()
                    .ok()
                    .and_then(|n| def.fields.iter().find(|f| f.number == n))
                {
                    Some(f) => Some(f),
                    None => return refuse("a field number no field has"),
                }
            }
            found => found,
        };
        let Some(f) = found else {
            // The system fields' captions are refused; they are named as their names (SystemId, SystemCreatedAt, ...).
            // A field with a literal CaptionClass is not found by the caption it resolves to: `Displayed Price`, which a bound
            // OnResolveCaptionClass subscriber gives for `Nominal Price`, is an unknown field (format-evaluate/tablefilter-neutral/
            // TableFilterEnumForms.al:551 TableFilter_Evaluate_ResolvedCaptionClass_RejectsUnknownField, pinned by corpus PR 1157).
            if def
                .fields
                .iter()
                .any(|f| f.caption.is_err() && f.class_caption.is_none() && f.number < 2_000_000_000)
            {
                return refuse("an unknown field of a table whose captions are refused");
            }
            if def.fields.iter().any(|f| f.class_caption.is_some()) {
                crate::assumptions::assume(crate::assumptions::Assumption::TablefilterCaptionClassDeclared);
            }
            return reject(
                c,
                &format!(
                    " The supplied field number '{field}' cannot be found in the '{}' table.",
                    def.caption
                ),
                "Evaluate:FieldNotFound",
            );
        };
        let caption = match (&f.caption, &f.class_caption) {
            (Ok(caption), _) => caption.clone(),
            // A field with a literal CaptionClass, named by its name, is found (format-evaluate/tablefilter-neutral/
            // TableFilterEnumForms.al:539 TableFilter_Evaluate_CaptionClassFieldName_SelectsPricedRow, pinned by corpus PR 1157:
            // `Nominal Price=>=40` selects the priced row through page Table Filter). It is stored by its declared caption,
            // which the virtual Field table gives as "Field Caption"; that stored text is an assumption.
            (Err(_), Some(declared)) => {
                crate::assumptions::assume(crate::assumptions::Assumption::TablefilterCaptionClassDeclared);
                declared.clone()
            }
            (Err(why), None) => return unsupported(why.clone()),
        };
        match table_filter_value(f, value)? {
            Ok(v) => conditions.push((f.number, format!("{caption}={v}"))),
            Err(more) => return reject(c, &format!(" {more}"), "Evaluate:InvalidFilterExpression"),
        }
    }
    conditions.sort_by_key(|(n, _)| *n);
    let joined = conditions.into_iter().map(|(_, s)| s).collect::<Vec<_>>().join(",");
    Ok(store_table_filter(a, &format!("{}: {joined}", def.name)))
}

/// The filter `value` of a TableFilter condition on field `f` as the stored text gives it back, `Ok(Err(text))` with the error
/// text after `can't be evaluated into type TableFilter.`, or a refusal for a form no test pins (record/W4PlatformAndMeta.al):
/// - a value starting `'`: a comma inside cut it at the split, `An single quote (') is missing.` (`single_quoted_comma`); a Code
///   value in quotes is unquoted, kept in quotes when it holds `|` (`single_quoted_space`, `single_quoted_pipe`); a Code value in
///   double quotes is unquoted (`double_quoted_value`);
/// - Code: trimmed and in capitals, of letters, digits and inner spaces, with `<>` before one value, `|` and `..` between values
///   (`range_space_values`, `or_space_values`, `not_equal_value`) or `@` and `*` (`code_wildcard`); one value too long gives the
///   length error (`value_too_long`); the empty value is kept (`empty_value`);
/// - Text: letters, digits and spaces kept as written (`text_space_value`);
/// - Integer: numbers with `<>` before one, `..` and `|` (`integer_expression`, `integer_not_zero`); letters give the Integer
///   error (`ret_bad_integer`);
/// - Option: one member by name in any casing, caption or ordinal, stored as its caption (`name_form`, `option_caption_value`,
///   `option_number_value`); letters that are none give `'X' is not an option` (`ret_bad_option`); `FILTER(` or `CONST(` gives
///   the `(` error (`filter_wrapper`, `const_wrapper`).
fn table_filter_value(f: &crate::native::TableFilterField, value: &str) -> Result<Result<String, String>, Signal> {
    use crate::value::RtType;
    let refuse = || {
        unsupported(format!(
            "Evaluate into TableFilter of the filter `{value}` on field {} (not pinned by the corpus)",
            f.name
        ))
    };
    let v = value.trim();
    // Chains of values with spaces on a Code or Text field, `''|A` too (crate::evaluate_table_filter_forms::spaced_chain_form).
    if v == value {
        if let Some(stored) = crate::evaluate_table_filter_forms::spaced_chain_form(&f.ty, v) {
            return Ok(Ok(stored));
        }
    }
    if v.starts_with('\'') && (v.len() < 2 || !v.ends_with('\'')) {
        return Ok(Err("An single quote (') is missing.".into()));
    }
    let words = |s: &str| {
        !s.is_empty()
            && s.split(' ')
                .all(|w| !w.is_empty() && w.chars().all(|ch| ch.is_ascii_alphanumeric()))
    };
    // Forms stored as pinned by format-evaluate/tablefilter-stored/TableFilterStoredText.al (al-runner issues 7188 and 7190).
    let ordinals: Option<Vec<i32>> = match &f.options {
        Ok(Some(m)) => Some(m.iter().map(|x| x.0).collect()),
        _ => None,
    };
    if let Some(r) = crate::evaluate_table_filter_forms::kept_form(&f.ty, v, ordinals.as_deref()) {
        return Ok(r.map(|(text, assumed)| {
            if assumed {
                crate::assumptions::assume(crate::assumptions::Assumption::TablefilterQuotedCommaTextKept);
            }
            text
        }));
    }
    match &f.ty {
        RtType::Code(len) => {
            if v.is_empty() {
                return Ok(Ok(String::new()));
            }
            // `<>''`: the nonempty codes (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:485
            // TableFilter_Evaluate_NotEqualQuotedBlankCode_SelectsNonemptyCode, pinned by corpus pull request 1152). The text is
            // kept as written; that stored form is an assumption.
            // `''` alone and in a compound condition: the blank codes (TableFilterEnumForms.al:579
            // TableFilter_Evaluate_QuotedBlankCode10_SelectsBlankService and :587 _CompoundQuotedBlankCode10_SelectsBlankService,
            // pinned by corpus pull request 1157). Kept as written too.
            if v == "<>''" || v == "''" {
                crate::assumptions::assume(crate::assumptions::Assumption::TablefilterQuotedBlankKept);
                return Ok(Ok(v.to_string()));
            }
            if let Some(inner) = v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                if inner.contains('\'') {
                    return refuse();
                }
                if inner.contains('|') && inner.split('|').all(words) {
                    return Ok(Ok(format!("'{}'", inner.to_uppercase())));
                }
                if words(inner) {
                    return Ok(Ok(inner.to_uppercase()));
                }
                return refuse();
            }
            if let Some(inner) = v.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                if words(inner) && !inner.contains(' ') {
                    return Ok(Ok(inner.to_uppercase()));
                }
                return refuse();
            }
            if words(v) {
                if let Some(n) = len.filter(|n| v.chars().count() > *n as usize) {
                    // The error shows the value as given: pinned for a value in capitals (`value_too_long`).
                    if v.chars().any(|ch| ch.is_ascii_lowercase()) {
                        return refuse();
                    }
                    return Ok(Err(format!(
                        "The length of the string is {}, but it must be less than or equal to {n} characters. Value: {v}",
                        v.chars().count()
                    )));
                }
                return Ok(Ok(v.to_uppercase()));
            }
            let fits = |w: &str| words(w) && len.is_none_or(|n| w.chars().count() <= n as usize);
            if let Some(w) = v.strip_prefix("<>").filter(|w| fits(w)) {
                return Ok(Ok(format!("<>{}", w.to_uppercase())));
            }
            if let Some((x, y)) = v.split_once("..").filter(|(x, y)| fits(x) && fits(y)) {
                return Ok(Ok(format!("{}..{}", x.to_uppercase(), y.to_uppercase())));
            }
            if v.contains('|') && v.split('|').all(fits) {
                return Ok(Ok(v.to_uppercase()));
            }
            if let Some(w) = v.strip_prefix('@').and_then(|w| w.strip_suffix('*')) {
                if !w.is_empty() && w.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                    return Ok(Ok(format!("@{}*", w.to_uppercase())));
                }
            }
            if crate::evaluate_table_filter_forms::code_chained_form(v, *len) {
                crate::assumptions::assume(crate::assumptions::Assumption::TablefilterCodeChainedKept);
                return Ok(Ok(v.to_string()));
            }
            refuse()
        }
        // Words joined by `|` are kept as written too (xmlports/shapes-properties/Y67XpTableFilter.al:52 `Note=a|b`).
        RtType::Text(len)
            if (words(v) || (v.contains('|') && v.split('|').all(words)))
                && v == value
                && len.is_none_or(|n| v.chars().count() <= n as usize) =>
        {
            Ok(Ok(v.to_string()))
        }
        RtType::Integer => {
            let int = |s: &str| {
                let d = s.strip_prefix('-').unwrap_or(s);
                !d.is_empty() && d.chars().all(|ch| ch.is_ascii_digit()) && s.parse::<i32>().is_ok()
            };
            let range = |s: &str| int(s) || s.split_once("..").is_some_and(|(x, y)| int(x) && int(y));
            if v.strip_prefix("<>").is_some_and(int) || v.split('|').all(range) {
                return Ok(Ok(v.to_string()));
            }
            // A number in quotes is stored without them (Y67XpTableFilter.al:207 `Id='1'` gives `Id=1`).
            if let Some(n) = v
                .strip_prefix('\'')
                .and_then(|w| w.strip_suffix('\''))
                .filter(|w| int(w))
            {
                return Ok(Ok(n.to_string()));
            }
            // An alternative after `|` that reads as another condition, `<name>=...`: the name is the bad Integer
            // (Y67XpTableFilter.al:206 `Id=1..5|Note=a`, `The value "Note" can't be evaluated into type Integer.`).
            let (first, rest) = v.split_once('|').unwrap_or((v, ""));
            if range(first) {
                if let Some((name, _)) = rest
                    .split_once('=')
                    .filter(|(n, _)| !n.is_empty() && n.chars().all(|ch| ch.is_ascii_alphabetic()))
                {
                    return Ok(Err(format!(
                        "The value \"{name}\" can't be evaluated into type Integer."
                    )));
                }
            }
            if !v.is_empty() && v.chars().all(|ch| ch.is_ascii_alphabetic()) {
                return Ok(Err(format!("The value \"{v}\" can't be evaluated into type Integer.")));
            }
            refuse()
        }
        // Decimal `>=<digits>`: the rows at or above it (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:539
        // TableFilter_Evaluate_CaptionClassFieldName_SelectsPricedRow, pinned by corpus pull request 1157: `Nominal Price=>=40`
        // selects 42.5, not 17.5, through page Table Filter). Kept as written; that stored form is an assumption. Every other
        // Decimal filter is not pinned.
        RtType::Decimal
            if v.strip_prefix(">=")
                .is_some_and(|d| !d.is_empty() && d.len() <= 9 && d.chars().all(|ch| ch.is_ascii_digit())) =>
        {
            crate::assumptions::assume(crate::assumptions::Assumption::TablefilterDecimalAtLeastKept);
            Ok(Ok(v.to_string()))
        }
        // Option and Enum members, one or in a list, `<>` or a range (crate::evaluate_table_filter_member).
        RtType::Option(_) | RtType::Enum(_) => {
            let members = match &f.options {
                Ok(Some(m)) => m,
                Ok(None) => return refuse(),
                Err(why) => return unsupported(why.clone()),
            };
            // `''` on an Enum with a blank member: that member (TableFilterEnumForms.al:468
            // TableFilter_Evaluate_QuotedBlankEnum_SelectsBlankMember, pinned by corpus pull request 1152). Kept as written; that
            // stored form is an assumption. An Option, or an Enum without a blank member, is not pinned.
            if v == "''" && matches!(f.ty, RtType::Enum(_)) && members.iter().any(|m| m.1.trim().is_empty()) {
                crate::assumptions::assume(crate::assumptions::Assumption::TablefilterQuotedBlankKept);
                return Ok(Ok(v.to_string()));
            }
            match crate::evaluate_table_filter_member::member_filter(members, v) {
                // One Option member: W4PlatformAndMeta.al. Enum members by name, caption or ordinal, lists, `<>` and ranges of
                // Option and Enum members: format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:253
                // TableFilterEvaluate_EnumMemberForms, :277 _EnumExpressions, :301 _OptionExpressions, :317 _EnumRoundTrip.
                Some(r) => Ok(r),
                None => refuse(),
            }
        }
        _ => refuse(),
    }
}

/// The TableFilter field `a[0]` holds `text`; Evaluate answers true.
fn store_table_filter(a: &mut [Value], text: &str) -> Value {
    a[0] = Value::Library(
        crate::library_kind::LibraryKind::from_name("TableFilter"),
        std::rc::Rc::new(std::cell::RefCell::new(crate::value::LibraryValue::Text(
            text.to_string(),
        ))),
    );
    Value::Boolean(true)
}

/// `Evaluate(RecordId, Text)`: a table name and the primary key after a colon, or only the table name; the empty text gives the
/// blank RecordId, also for one that is not blank (format-evaluate/EvaluateRecordIdComposite.al:Evaluate_IntoSetRecordId
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
    // A text of blanks is the empty text (recordref-fieldref/N582EvalFieldRefUnused.al:87 `blank`, :97-99: no error, the blank RecordId).
    let new = if text.trim().is_empty() {
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
    // (format-evaluate/EvaluateRecordIdComposite.al:Evaluate_IntoSetRecordId, `Rid := Start` then Evaluate(Rid, ...)).
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
    // (format-evaluate/EvaluateRecordIdComposite.al:Evaluate_IntoFieldRef `invalidUnusedErr`, an Integer field;
    // recordref-fieldref/N582EvalFieldRefUnused.al:71-169: Decimal, Date, Boolean, Code, Text, an Option without an
    // OptionCaption and Integer; :218-249 N582_Evaluate_FieldRef_Guid_Statement, a Guid: the spellings, `abc` and an empty
    // text give InvalidGuidFormat and keep the field; :251-332 BigInteger, Time, DateTime, DateFormula and Duration; :334-364
    // an Option with an OptionCaption and an Enum; :362-662 a RecordId, waldo run 2 of al-runner issue 6924: the error of
    // `into_record_id` and the field keeps its value). Another type is not pinned.
    if !c.ret_used() {
        use crate::value::RtType as T;
        let record_id = matches!(&target.ty, T::Library(k) if *k == crate::library_kind::LibraryKind::RecordId);
        let pinned = record_id
            || matches!(
                &target.ty,
                T::Integer
                    | T::BigInteger
                    | T::Decimal
                    | T::Date
                    | T::Time
                    | T::DateTime
                    | T::DateFormula
                    | T::Duration
                    | T::Boolean
                    | T::Code(_)
                    | T::Text(_)
                    | T::Guid
                    | T::Enum(_)
                    | T::Option(_)
            );
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
    // An Option field with an OptionCaption reads as `evaluate_option_field` does: the empty text, a blank and NUL name the blank
    // member, an ordinal of any size is kept, a member name or a caption names its member, and every other text raises
    // InvalidOptionString with the result unused (format-evaluate/format-matrix/FMFields.al:467-521 FieldRef_Evaluate_Option_1,
    // corpus PR 1256; al-runner issue 7472).
    if let (Some(captions), Value::Option(..)) = (&target.option_captions, &current) {
        let text = match crate::native::text_of(&a[1]) {
            Some(t) if t.as_str() == "\0" => Value::Text(String::new().into()),
            _ => a[1].clone(),
        };
        let mut args = vec![
            current,
            text,
            Value::Integer(number as i32),
            Value::Text(captions.clone().into()),
        ];
        let ok = evaluate_option_field(c, None, &mut args)?;
        if matches!(ok, Value::Boolean(true)) {
            target.rec.borrow_mut().fields[target.pos] = args.swap_remove(0);
        }
        return Ok(ok);
    }
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
/// turns the error into the last error when it does (corpus format-evaluate/EvaluateRecordIdComposite.al:
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

/// The format number `Evaluate` reads `target` with, for the number given, or `None` where that number is not pinned for it.
fn pinned_number(number: i64, target: &Value, variable_option: bool) -> Option<i64> {
    let pinned = number == 0
        || number == 9
        // Number 2 is read for a DateFormula only, as number 0 (docs/probes-needed.md).
        || (number == 2 && matches!(target, Value::DateFormula(_)))
        || (matches!(number, 1 | 2) && matches!(target, Value::Text(_) | Value::Code(_)))
        || (matches!(number, 1..=3) && matches!(target, Value::Duration(_)))
        || matches!(target, Value::Date(..))
        // An Option variable reads number 2 as 0 (format-evaluate/OptionEvaluateCaptions.al:270 FormatNumber
        // `var 2 lower`).
        || (number == 2 && variable_option && matches!(target, Value::Option(..)))
        // An Enum reads number 2 as 0 (format-evaluate/format-matrix/FMOptionBlankEnum.al:127 `evf2_empty`, :133, :139).
        || (number == 2 && matches!(target, Value::Enum(..)))
        // So do the platform's option types (format-evaluate/SoEvaluateSystemOptions.al:264 Format9 `f2 1`).
        || (number == 2 && matches!(target, Value::Option(_, m) if crate::system_option_eval::reads(m)))
        // Number 10 is the bookmark of a RecordId (`into_record_id`).
        || (number == 10 && matches!(target, Value::Library(crate::library_kind::LibraryKind::RecordId, _)));
    // A Text, an Integer and a Guid read number 10 as the default (format-evaluate/Y49RecordIdFormat10.al:265-301
    // `text` `Yes [x]`, `integer` `Yes 12`, `guid`, also as a statement).
    if number == 10 && matches!(target, Value::Text(_) | Value::Integer(_) | Value::Guid(_)) {
        return Some(0);
    }
    pinned.then_some(number)
}

fn evaluate_with_max(
    c: &mut dyn Ctx,
    a: &mut [Value],
    variable_option: bool,
    max: Option<u32>,
    any_date_text: bool,
) -> Result<Value, Signal> {
    // A Guid source reads as its text `{XXXXXXXX-...}`: into a Text, a Code and a Guid (types/duration/
    // E_Guid.al:224-311). Other sources that are not text are not pinned.
    let text = match &a[1] {
        Value::Guid(g) => Some(crate::text::Txt::from(crate::format::format_guid(g))),
        other => crate::native::text_of(other),
    };
    let Some(text) = text else {
        return unsupported("Evaluate of a source that is not text");
    };
    let number = match a.get(2) {
        None => 0,
        Some(Value::Integer(n)) => i64::from(*n),
        Some(other) => return unsupported(format!("Evaluate format number {}", other.describe())),
    };
    let Some(number) = pinned_number(number, &a[0], variable_option) else {
        return unsupported(format!(
            "Evaluate with format number {number} (not pinned by the corpus)"
        ));
    };
    if matches!(&a[0], Value::Library(crate::library_kind::LibraryKind::FieldRef, _)) {
        return into_field_ref(c, a, number);
    }
    if matches!(&a[0], Value::Library(crate::library_kind::LibraryKind::RecordId, _)) {
        return into_record_id(c, a, &text, number);
    }
    if matches!(&a[0], Value::Library(name, _) if name == "TableFilter") {
        return into_table_filter(c, a, &text, number);
    }
    // Into a JsonArray, without a format number (format-evaluate/UfEvaluateJsonRfc.al:88-194); with one it is not pinned.
    if matches!(&a[0], Value::Library(name, _) if name == "JsonArray") {
        if a.len() > 2 {
            return unsupported("Evaluate into a JsonArray with a format number (not pinned)");
        }
        return crate::json::evaluate_array(c, &mut a[0], &text);
    }
    let parsed = match &a[0] {
        Value::Text(_) | Value::Code(_) => text_or_code(c, &a[0], &text, number, max)?,
        Value::Integer(_) => integer(&text, number)?,
        Value::BigInteger(_) => big_integer(&text, number)?,
        Value::Decimal(_) => decimal(&text, number)?,
        Value::Duration(_) => duration(&text, number, c.language())?,
        Value::Boolean(_) => boolean_in_language(c.session_locale().map(|l| l.0), &text, number)?,
        Value::Guid(_) => guid(&text)?,
        // A Char reads one character, any (a blank, `?`, NUL, `é`, `¶`, `€`), with format 0 and 9 alike, in English and German; the
        // empty text, two characters and more (`AB`, `65`, `256`, `-1`, `a b`, `1D`) are rejected (format-evaluate/format-matrix/
        // FMEvalChar.al, corpus PR 1256). A character outside the 16-bit range is not pinned.
        Value::Char(_) if matches!(number, 0 | 9) => {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => match u16::try_from(u32::from(ch)) {
                    Ok(u) => Parsed::Value(Value::Char(u)),
                    Err(_) => {
                        return unsupported(format!(
                            "Evaluate of `{text}` into a Char (a character beyond 16 bits, not pinned)"
                        ));
                    }
                },
                _ => Parsed::Rejected,
            }
        }
        Value::Byte(_) if number == 0 => byte_or_char(&a[0], &text)?,
        // Every number but 9 reads a Date as the default does (types/date-time/Evaluate.al:Date_Other_Formats `fmt 1` to `fmt 8`,
        // `fmt 10`, `fmt -1`; types/P130EvaluateDate.al:P130_EvaluateDate_EveryFormat_*).
        // Evaluate reads a TAB, two TABs and one space as the empty date (corpus master, corpus PR 871:
        // format-evaluate/Np4Misc.al NP4_Evaluate_Tab_Date). Pinned for
        // Evaluate only: `read_text` and the other readers of `date` do not take it.
        Value::Date(..) if number != 9 && is_blank_tabs(&text) => Parsed::Value(Value::Date(0, false)),
        // In German `T` is no date (format-evaluate/format-matrix/FMEvalDate.al:376-378, corpus PR 1250).
        Value::Date(..) if text.as_str() == "T" && c.language() == 1031 => Parsed::Rejected,
        Value::Date(..) => date(
            &text,
            if number == 9 { 9 } else { 0 },
            c.today_day(),
            c.work_date_day(),
            true,
        )?,
        Value::Time(_) => time(&text, number)?,
        Value::DateTime(_) => datetime(&text, number, c.today_day())?,
        // The text is kept as written once it is a form the runtime can calculate with
        // (types/TestDateFormulaContracts.al:21-139); what Format prints is docs/probes-needed.md.
        // `Evaluate(V, '1.2.3.4')`: the four-part form (session-system/TestMiscTypes.al:44-80).
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
        Value::Enum(id, _) => enum_text::enum_text(c, id, &text)?,
        Value::Option(_, members) if platform_options::reads(members) => platform_options::read(c, members, &text)?,
        Value::Option(_, members) if variable_option => option_text::variable(c, members, &text)?,
        Value::Option(_, members) => option_text::field(c, members, &text)?,
        // The blank-like texts store the empty formula; garbage fails and leaves the value
        // (types/TestDateFormulaDurationFormatContracts.al:28-74, TestDateFormulaEvaluateLeftoversContracts.al).
        // The letters of the global language (format-evaluate/Y62DateFormulaFormat.al, `date_formula_language`).
        // A comparison operator before a slashed date is no formula, with the result used: Filter Tokens.MakeDateFilter gives
        // `<=11/09/26`, `>=11/09/26` and `<=11/09/2026` back unchanged, which needs its Evaluate into a DateFormula to answer
        // false (format-evaluate/Y71EvaluateDashedDates.al FilterTokens_MakeDateFilter_OperatorTexts, corpus PR 1247; System
        // Application Filter Tokens Impl.ResolveDateFilterToken). The statement form is not pinned.
        Value::DateFormula(_) if c.ret_used() && date_forms::operator_before_slashed_date(&text) => Parsed::Rejected,
        Value::DateFormula(_) => match crate::date_formula_language::evaluate(&text, c.language())? {
            crate::datefns::Evaluated::Stored(f) => Parsed::Value(Value::DateFormula(f)),
            // A statement form raises the formula's error (types/date-time/DateFormulaEdges.al:SignsAndDigits_ErrorTexts).
            crate::datefns::Evaluated::Rejected(Some(m)) if !c.ret_used() => {
                return crate::native::raise(m, "Evaluate");
            }
            crate::datefns::Evaluated::Rejected(_) => Parsed::Rejected,
            crate::datefns::Evaluated::Failed(m) if !c.ret_used() => return crate::native::raise(m, "Evaluate"),
            // With the result used, both errors of this kind answer false and leave the value
            // (types/date-time/P128CalcDateForms.al:154-169: date texts, "should include a
            // time unit"; types/date-time/P131CalcDate.al:140,152: `<1D>+`, "should include a number").
            crate::datefns::Evaluated::Failed(_) => Parsed::Rejected,
        },
        other => return unsupported(format!("Evaluate into {} (not pinned by the corpus)", other.describe())),
    };
    match parsed {
        Parsed::RejectedUsedRefused(why) if c.ret_used() => unsupported(why),
        Parsed::Value(v) => {
            a[0] = v;
            Ok(Value::Boolean(true))
        }
        Parsed::Rejected | Parsed::RejectedWith(..) if c.ret_used() => Ok(Value::Boolean(false)),
        Parsed::RejectedWith(message, code) => crate::native::raise(message, code),
        Parsed::RejectedLastError(message, code) => match crate::native::raise::<Value>(&message, code) {
            Err(Signal::Error(e)) if c.ret_used() => {
                c.set_last_error(*e);
                Ok(Value::Boolean(false))
            }
            other => other,
        },
        // The error of a failed statement form (session-system/TestProbeSessionMiscContracts.al:Arithmetic_ErrorTexts,
        // `evaluate_int_overflow_statement`; types/TestProbeScalarContracts.al:Evaluate_BooleanFailureError).
        Parsed::Rejected | Parsed::RejectedUsedRefused(_)
            if matches!(
                a[0],
                Value::Integer(_)
                    | Value::BigInteger(_)
                    | Value::Decimal(_)
                    | Value::Boolean(_)
                    | Value::Time(_)
                    | Value::DateTime(_)
                    | Value::Char(_)
            ) =>
        {
            let ty = match a[0] {
                Value::Integer(_) => "Integer",
                Value::BigInteger(_) => "BigInteger",
                Value::Decimal(_) => "Decimal",
                Value::Time(_) => "Time",
                Value::DateTime(_) => "DateTime",
                Value::Char(_) => "Char",
                _ => "Boolean",
            };
            // A Boolean's error shows a TAB, two TABs or a space as the empty value `""` (corpus master, corpus PR 871:
            // format-evaluate/Np4Misc.al NP4_Evaluate_Tab_Boolean).
            // Integer and Decimal show them as written (NP4_Evaluate_Tab_Integer, _Decimal).
            let text = if ty == "Boolean" && is_blank_tabs(&text) {
                String::new()
            } else {
                text.to_string()
            };
            // In German the text is pinned for Decimal only (format-evaluate/EvalDecimalDe.al:
            // P175_De_StatementErrorText, P175_De_Format9StatementErrorText).
            match (c.session_locale().map(|l| l.0), ty) {
                // ... and Boolean (errors/WcUnpinnedSingles.al:79-113 `vielleicht`).
                (Some(1031), "Decimal" | "Boolean") => crate::native::raise(
                    format!("Der Wert „{text}“ kann nicht in Typ {ty} ausgewertet werden."),
                    "Evaluate",
                ),
                // The code of the other types is pinned (format-evaluate/format-matrix/FMEvalInteger.al:370-447, FMEvalDateTime.al:372-442,
                // corpus PRs 1249 and 1250), the German text is not.
                (Some(1031), _) => Err(Signal::Error(Box::new(crate::native::unpinned_text_error(
                    "Evaluate",
                    "the German text of the Evaluate error of a number, a Time or a DateTime",
                )))),
                // format-evaluate/EvalLanguageBoolean.al EL_Errors_NLB_2067, _NLD_1043, _FRA_1036, _DAN_1030 (the quotes of the
                // French text are « and » with a no-break space inside).
                (Some(n @ (1030 | 1036 | 1043 | 2067)), "Boolean") => raise_boolean_error(n, &text),
                // The flag hides the text (errors/last-error/P70_02LastErrFlag.al:Flag_EvaluateAndOverflow
                // `eval_int|true`, `eval_int_var|true`); for the other types it is not pinned.
                (_, "Integer") => crate::native::raise_redacted(
                    format!("The value \"{text}\" can't be evaluated into type {ty}."),
                    "Evaluate",
                    "The value \"<redacted>\" can't be evaluated into type Integer.",
                ),
                _ => crate::native::raise(
                    format!("The value \"{text}\" can't be evaluated into type {ty}."),
                    "Evaluate",
                ),
            }
        }
        // The error of a rejected date that looks like one (P86Eval.al:Dflt3_Error_FNone, Fmt03_Error_F0, Fmt93_Error_F9: the same
        // text as a date filter gives, SetFilter on a Date).
        // In English every text gives this error (format-evaluate/Y49EvaluateDateBlanks.al:79-205: blanks,
        // `,`, `_`, `!`, with and without number 9).
        Parsed::Rejected | Parsed::RejectedUsedRefused(_)
            if matches!(a[0], Value::Date(..))
                && (any_date_text
                    || c.language() == 1033
                    || text
                        .bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'-' | b'.' | b'/'))
                    // A TAB inside an ISO date with format 9 (corpus master, corpus PR 886: EvaluateDateFormat9Space.al:114-120).
                    || (number == 9 && text.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'-' | b'\t')))
                    || letters_date_text(&text, number)) =>
        {
            // Letters alone with no number are pinned on master (types/date-time/Evaluate.al:188-197 Date_Evaluate_Raises `abc`,
            // errors/last-error/P70_02LastErrFlag.al:286 `notadate`); number 9 and letters with `-./` wait for the
            // branch pin.
            // Number 9 and letters with `-./`: pinned by types/P130EvaluateDate.al
            // UN_Evaluate_Statement_NotADate_ByTarget (was assumption `evaluate-date-letters-statement`).
            // In German (format-evaluate/EvalLastError.al:463-481, BT_Evaluate_DottedDate_Language1031). Another
            // language is not pinned. The flag hides the text in English (P70_02LastErrFlag.al:Flag_EvaluateAndOverflow
            // `eval_date|true`); its German text is not pinned.
            match c.language() {
                1033 => crate::native::raise_redacted(
                    format!(
                        "The format of the Date '{text}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
                    ),
                    "Evaluate",
                    "The format of the Date '<redacted>' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'.",
                ),
                1031 => crate::native::raise(
                    format!(
                        "Das Format Date \u{201e}{text}\u{201c} stimmt nicht mit den Datumseinstellungen Ihres Ger\u{e4}ts \u{fc}berein.\\\\Stellen Sie sicher, dass Tag, Monat und Jahr die richtige Reihenfolge aufweisen und das richtige Trennzeichen verwendet wird (z. b. - oder /). Alternativ k\u{f6}nnen Sie das Format \u{201e}jjjj-mm-tt\u{201c} verwenden."
                    ),
                    "Evaluate",
                ),
                n => unsupported(format!("the Evaluate error of a Date in the language {n} (not pinned)")),
            }
        }
        // With format 9 (types/date-time/Evaluate.al:640-661 `eval 3000000000 fmt 9`) and 0 (format-evaluate/
        // Y49EvaluateDateBlanks.al:251-297 `abc`). With another format it is not pinned.
        // In German the code is pinned and the text is not (format-evaluate/format-matrix/FMEvalDuration.al:375-433,
        // FMEvalDate.al:442 `March 9, 2026`, corpus PR 1250).
        Parsed::Rejected if matches!(a[0], Value::Duration(_) | Value::Date(..)) && c.language() == 1031 => {
            Err(Signal::Error(Box::new(crate::native::unpinned_text_error(
                "Evaluate",
                "the German text of the Evaluate error of a Duration or a Date",
            ))))
        }
        Parsed::Rejected if matches!(a[0], Value::Duration(_)) && matches!(number, 0 | 9) => crate::native::raise(
            format!("The value \"{text}\" can't be evaluated into type Duration."),
            "Evaluate",
        ),
        Parsed::Rejected if matches!(a[0], Value::Guid(_)) => crate::native::raise(
            "Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.",
            "InvalidGuidFormat",
        ),
        Parsed::Rejected | Parsed::RejectedUsedRefused(_) => unsupported(format!(
            "Evaluate of `{text}` failed with the result unused (error text not pinned yet)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Corpus master (corpus PR 871) format-evaluate/Np4Misc.al NP4_Evaluate_Tab_Date: a TAB, two TABs and a space read as
    /// the empty date in format 0.
    #[test]
    fn tabs_and_a_space_read_as_the_empty_date() {
        for t in ["\t", "\t\t", " "] {
            assert!(is_blank_tabs(t), "{t:?}");
        }
    }

    /// The TAB rule is pinned for Evaluate only: `date` and `read_text` do not read a TAB or a space as the empty date.
    #[test]
    fn a_reader_other_than_evaluate_does_not_read_tabs_as_the_empty_date() {
        for t in ["\t", "\t\t", " "] {
            assert!(
                !matches!(date(t, 0, None, None, false), Ok(Parsed::Value(Value::Date(0, false)))),
                "{t:?}"
            );
            assert!(read_text(&crate::value::RtType::Date, t).is_none(), "{t:?}");
        }
    }

    /// types/date-time/Evaluate.al:209,233 Time_Format9: `13:45:06.1234` with format 9 is 13:45:06.123; a fourth digit of 5 or
    /// more and longer fractions are refused (rounding not pinned).
    #[test]
    fn a_time_with_four_digits_of_a_second_keeps_the_milliseconds() {
        assert!(
            matches!(value(time("13:45:06.1234", 9)), Some(Value::Time(t)) if t == ((13 * 60 + 45) * 60 + 6) * 1000 + 123 + 1)
        );
        assert!(matches!(time("13:45:06.1235", 9), Err(Signal::Unsupported(_))));
        assert!(matches!(time("13:45:06.12345", 9), Err(Signal::Unsupported(_))));
    }

    fn value(p: Result<Parsed, Signal>) -> Option<Value> {
        match p {
            Ok(Parsed::Value(v)) => Some(v),
            _ => None,
        }
    }

    // format-evaluate/P86Eval.al, codeunit "LT P86 Eval Integer"
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

    // types/decimal-integer/ScEvalDecimalWhitespace.al:51-170 (format number 0): a trailing vertical tab or form
    // feed is dropped, a line feed or tab inside the number rejects, for a Decimal and an Integer.
    #[test]
    fn decimals_and_integers_drop_edge_white_space_and_reject_inner_line_feeds() {
        let dec = |s: &str| match decimal(s, 0) {
            Ok(Parsed::Value(Value::Decimal(d))) => Some(Some(d.to_string())),
            Ok(Parsed::Rejected) => Some(None),
            _ => None,
        };
        assert_eq!(dec("1.5\u{B}"), Some(Some("1.5".into())));
        assert_eq!(dec("1.5\u{C}"), Some(Some("1.5".into())));
        assert_eq!(dec("1,234.5\n"), Some(Some("1234.5".into())));
        assert_eq!(dec("1\n5"), Some(None));
        assert_eq!(dec("1\t5"), Some(None));
        assert_eq!(dec("-\n1.5"), Some(None));
        assert_eq!(dec("1\n.5"), Some(None));
        assert_eq!(dec("\n"), Some(None));
        // Not pinned: a line feed before a trailing minus or before a group of three digits.
        assert_eq!(dec("100\n-"), None);
        assert_eq!(dec("1\n000"), None);
        assert!(matches!(integer("1\n5", 0), Ok(Parsed::Rejected)));
        assert!(matches!(value(integer("\t15", 0)), Some(Value::Integer(15))));
    }

    // types/decimal-integer/Y6EvalWhitespaceMore.al (corpus master, format number 0): a leading vertical tab or form
    // feed is dropped; one alone, or a carriage return, vertical tab or form feed inside the number, rejects. For an Integer the
    // three are dropped on either side and reject inside.
    #[test]
    fn vertical_tab_form_feed_and_carriage_return_around_and_inside_a_number() {
        let dec = |s: &str| match decimal(s, 0) {
            Ok(Parsed::Value(Value::Decimal(d))) => Some(Some(d.to_string())),
            Ok(Parsed::Rejected) => Some(None),
            _ => None,
        };
        for s in ["\u{B}1.5", "\u{C}1.5", "\u{B}\u{C}1.5", "\u{B}1.5\u{B}"] {
            assert_eq!(dec(s), Some(Some("1.5".into())), "{s:?}");
        }
        assert_eq!(dec("\u{C}-1.5"), Some(Some("-1.5".into())));
        for s in [
            "\u{B}", "\u{C}", "1\r5", "1\r\n5", "1\u{B}5", "1\u{C}5", "1\r.5", "1.\r5", "-\r1.5",
        ] {
            assert_eq!(dec(s), Some(None), "{s:?}");
        }
        let int = |s: &str| match integer(s, 0) {
            Ok(Parsed::Value(Value::Integer(n))) => Some(Some(n)),
            Ok(Parsed::Rejected) => Some(None),
            _ => None,
        };
        for s in ["\u{B}15", "\u{C}15", "\r15", "15\u{B}", "15\u{C}", "15\r"] {
            assert_eq!(int(s), Some(Some(15)), "{s:?}");
        }
        for s in ["1\r5", "1\u{B}5", "1\u{C}5", "\u{C}"] {
            assert_eq!(int(s), Some(None), "{s:?}");
        }
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

    // types/date-time/Evaluate.al:Date_Default_Evaluate: the empty text reads as the empty date in format 0 too.
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

    // format-evaluate/EvaluateDecimal.al:Evaluate_SignsBlanksAndGarbage `trailing_minus`, `parentheses`
    #[test]
    fn decimals_read_a_trailing_minus_and_parentheses_as_negative() {
        let dec = |s: &str, n: i64| match decimal(s, n) {
            Ok(Parsed::Value(Value::Decimal(d))) => Some(d.to_string()),
            _ => None,
        };
        assert_eq!(dec("100-", 0).as_deref(), Some("-100"));
        assert_eq!(dec("(100)", 0).as_deref(), Some("-100"));
        assert_eq!(dec("  1,000.5- ", 0).as_deref(), Some("-1000.5"));
        // Format number 9 rejects both (EvaluateDecimal.al:QR_Evaluate_Signs_Number9); other combinations are not pinned.
        assert!(matches!(decimal("(100)", 9), Ok(Parsed::Rejected)));
        assert!(matches!(decimal("100-", 9), Ok(Parsed::Rejected)));
        assert!(decimal("(100", 0).is_err());
        assert!(matches!(decimal("abc-", 0), Ok(Parsed::Rejected)));
    }

    // format-evaluate/EvaluateDecimal.al:QR_Evaluate_Punctuation_*, QR_Evaluate_Signs_*, QR_Evaluate_WhiteSpace_*
    #[test]
    fn decimals_reject_punctuation_and_no_break_spaces_and_read_the_currency_forms() {
        let dec = |s: &str, n: i64| match decimal(s, n) {
            Ok(Parsed::Value(Value::Decimal(d))) => Ok(Some(d.to_string())),
            Ok(_) => Ok(None),
            Err(_) => Err(()),
        };
        for s in [
            "New/Updated",
            "#1",
            "1#",
            "A&B",
            "1=1",
            "'1'",
            "\"1\"",
            "[1]",
            "{1}",
            "*1",
            "1*",
            "@1",
            "1'000",
            "\u{20AC}5",
            "\u{A3}5",
            "\u{E4}",
            "\u{C4}1",
            "(-100)",
            "\u{A0}100",
            "1\u{202F}000",
            "\u{FF11}\u{FF12}",
            "\u{661}",
        ] {
            assert_eq!(dec(s, 0), Ok(None), "{s}");
            assert_eq!(dec(s, 9), Ok(None), "{s} 9");
        }
        assert_eq!(dec("5$", 0), Ok(Some("5".into())));
        assert_eq!(dec("$-5", 0), Ok(Some("-5".into())));
        assert_eq!(dec("-$5", 0), Ok(Some("-5".into())));
        assert_eq!(dec("100 -", 0), Ok(Some("-100".into())));
        for s in ["5$", "$-5", "-$5", "$5", "100 -"] {
            assert_eq!(dec(s, 9), Ok(None), "{s} 9");
        }
        assert_eq!(dec("\t100", 0), Ok(Some("100".into())));
        assert_eq!(dec("100\r\n", 9), Ok(Some("100".into())));
        // Not pinned: more than one space before a trailing minus.
        assert!(dec("100  -", 0).is_err());
        assert!(dec("100   -", 0).is_err());
        // Not pinned: white space inside the number.
        assert!(dec("1\t000", 0).is_err());
    }

    // EvaluateDecimal.al:QR_Evaluate_Integer_OddCharacters
    #[test]
    fn integers_and_big_integers_read_the_pinned_odd_forms() {
        assert!(matches!(value(integer("\t100", 0)), Some(Value::Integer(100))));
        assert!(matches!(integer("1\u{A0}000", 0), Ok(Parsed::Rejected)));
        assert!(matches!(integer("New/Updated", 0), Ok(Parsed::Rejected)));
        assert!(matches!(value(big_integer("100-", 0)), Some(Value::BigInteger(-100))));
    }

    // format-evaluate/AjScalars.al:QR_Evaluate_Time_NarrowNoBreakSpace
    #[test]
    fn times_read_am_and_pm_after_a_narrow_no_break_space() {
        let ms = |s: &str, n: i64| match time(s, n) {
            Ok(Parsed::Value(Value::Time(t))) => Ok(Some(t - 1)),
            Ok(_) => Ok(None),
            Err(_) => Err(()),
        };
        assert_eq!(ms("12:00:00\u{202F}PM", 0), Ok(Some(43_200_000)));
        assert_eq!(ms("1:05:09\u{202F}PM", 0), Ok(Some(((13 * 60 + 5) * 60 + 9) * 1000)));
        assert_eq!(ms("12:00:00\u{202F}pm", 0), Ok(Some(43_200_000)));
        assert_eq!(ms("12:00:00PM", 0), Ok(Some(43_200_000)));
        assert_eq!(ms("12:00:00\u{202F}PM\u{202F}", 0), Ok(Some(43_200_000)));
        assert_eq!(ms("0:30:00\u{202F}AM", 0), Ok(Some(30 * 60_000)));
        assert_eq!(ms("13:00:00\u{202F}PM", 0), Ok(None));
        assert_eq!(ms("12:00:00\u{202F}PM", 9), Ok(None));
        // Not pinned: hour 0 with PM.
        assert!(ms("0:30:00 PM", 0).is_err());
        // Not pinned: mixed case, two trailing narrow no-break spaces, hours 14 to 23 with PM, 13 with AM.
        assert!(ms("12:00:00\u{202F}Pm", 0).is_err());
        assert!(ms("12:00:00 pM", 0).is_err());
        assert!(ms("12:00:00\u{202F}PM\u{202F}\u{202F}", 0).is_err());
        assert!(ms("14:00:00\u{202F}PM", 0).is_err());
        assert!(ms("23:00:00 PM", 0).is_err());
        assert!(ms("13:00:00 AM", 0).is_err());
        // AjScalars.al:Evaluate_Time_FromDateText.
        assert_eq!(ms("2021-12-19", 0), Ok(Some(((20 * 60 + 21) * 60 + 12) * 1000 + 190)));
        assert_eq!(ms("12/19/21", 0), Ok(Some(((12 * 60 + 19) * 60 + 21) * 1000)));
        assert_eq!(ms("2021-12-19", 9), Ok(None));
        assert!(ms("2099-12-19", 0).is_err());
    }
    // types/date-time/Evaluate.al:380-447, 566-605, 640-670.
    #[test]
    fn evaluate_into_a_duration() {
        let ms = |t: &str, n: i64| match duration(t, n, 1033) {
            Ok(Parsed::Value(Value::Duration(ms))) => Some(ms),
            Ok(Parsed::Rejected) => None,
            other => panic!("{t}: {}", other.is_err()),
        };
        assert_eq!(ms("1500", 0), Some(1500 * 3_600_000));
        assert_eq!(ms("1.5", 0), Some(5_400_000));
        assert_eq!(ms("", 0), Some(0));
        assert_eq!(ms("-1 hour", 0), Some(-3_600_000));
        assert_eq!(ms("1 minute 1 hour", 0), Some(3_660_000));
        assert_eq!(
            ms("2days 4hours 3.7 seconds 17 milliseconds", 0),
            Some(2 * 86_400_000 + 4 * 3_600_000 + 3717)
        );
        assert_eq!(ms("1 HOUR", 0), Some(3_600_000));
        assert_eq!(ms("5 millis", 0), Some(5));
        assert_eq!(ms("1 hour 1 hour", 0), None);
        assert_eq!(ms("1 mins", 0), None);
        assert_eq!(ms("1.5 milliseconds", 0), None);
        assert_eq!(ms("abc", 0), None);
        assert_eq!(ms("P0DT1H30M0.0S", 9), Some(5_400_000));
        assert_eq!(ms("1500", 9), None);
        assert_eq!(ms("", 9), None);
        assert_eq!(ms("-1500", 9), None);
        assert_eq!(ms("2 days 3 h 4 min 5 sec 6 ms", 0), None);
        assert_eq!(ms("1500", 2), Some(1500 * 3_600_000));
        // types/P131EvalScalars.al:144-226.
        assert_eq!(ms("  7 ", 0), Some(7 * 3_600_000));
        for t in ["+5", "12abc", "0 ms", "1,5", "1e3"] {
            assert_eq!(ms(t, 0), None, "{t}");
        }
        assert_eq!(ms("1 millisecond", 0), Some(1));
        assert_eq!(ms("-1 second", 0), Some(-1000));
        assert_eq!(ms("1 day 1 hour 1 minute 1 second 1 millisecond", 0), Some(90_061_001));
        // Forms no corpus test pins are refused.
        for (t, n) in [
            ("1 Hour", 0),
            ("1.5 days", 0),
            ("1.5 minutes", 0),
            ("+5 hours", 0),
            ("1;5", 0),
            ("-1.5", 0),
            ("1 h 1 hour", 0),
            ("5 ms 1 foo", 0),
            ("hour 1", 0),
            ("Abc", 0),
            ("1 hour", 1),
            ("-P0DT1H0M0.0S", 9),
            ("Abc", 9),
        ] {
            assert!(matches!(duration(t, n, 1033), Err(Signal::Unsupported(_))), "{t} {n}");
        }
    }
}
