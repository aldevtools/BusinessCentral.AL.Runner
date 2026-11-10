//! `Evaluate` into a Duration.

use super::{Parsed, unsupported};
use crate::native::Signal;
use crate::value::Value;
use rust_decimal::Decimal;

/// A Duration as `Evaluate` reads it (types/date-time/Evaluate.al:380-447 Duration_Evaluate, :566-605 BT_Duration_UnitAbbreviations,
/// :640-670 BT_Duration_BeyondTimeSpan; types/duration/D_Duration.al:421-443). Only the pinned forms are
/// read; every other text is refused.
/// - Format 0: the empty text is 0; a plain whole number counts hours (`1500` is 62 days 12 hours, `-1500`, numbers beyond an
///   Integer), and so does an unsigned decimal (`1.5` is 90 minutes). Otherwise terms of a number and a unit, in any order, with or
///   without a blank between them (`1h`, `2days 4hours`): the units spelled `day`, `days`, `d`, `D`, `hour`, `hours`, `h`, `HOUR`,
///   `minute`, `minutes`, `min`, `m`, `second`, `seconds`, `sec`, `s`, `millisecond`, `milliseconds`, `milli`, `millis`; a fraction of
///   hours and of seconds (`1.5 hours`, `3.7 seconds`); a minus before a single hour or second term (`-1 hour`, `-1 second`). A plain
///   number may have blanks around it (`  7 `). Rejected: the same unit word twice (`1 hour 1 hour`), the words `mins`, `ms`, `week`
///   and `abc` (`12abc`), a fraction of milliseconds, a text of lower-case letters only (`abc`), a plus sign (`+5`), a comma or an
///   exponent in a number (`1,5`, `1e3`) (types/P131EvalScalars.al:144-226).
/// - Formats 1 to 3: a plain unsigned whole number, read as with format 0.
/// - Format 9: the form Format 9 gives (`P0DT1H30M0.0S`), unsigned. Rejected: the empty text, a text that starts with a digit or
///   with `-` and a digit, and a text of lower-case letters only.
///
/// The matrix of corpus master format-evaluate/format-matrix/FMEvalDuration.al (corpus PR 1250) adds: with format 9 a blank, NUL
/// and `?` are rejected (:68, :271, :274); with format 0 NUL, `?`, `1T` and the format 9 forms `P1DT2H` and `PT1.5S` are rejected
/// (:267-281); a minus before a text of terms negates the whole (`-1 millisecond`, `-1 day 2 hours` :88, :195); `Second` (:219);
/// a value beyond the range of a Duration is rejected (`9223372036854775807` and `9223372036854775808` hours, `999999999999 days`
/// :285-333). Under GlobalLanguage 1031 the English unit words are rejected (:373-433); other words are not pinned there.
pub(super) fn duration(text: &str, number: i64, language: i32) -> Result<Parsed, Signal> {
    let refuse = || {
        unsupported(format!(
            "Evaluate of `{text}` into Duration with format {number} (not pinned)"
        ))
    };
    let ms_of = |d: Decimal| -> Option<i64> {
        if !d.fract().is_zero() {
            return None;
        }
        i64::try_from(d.mantissa() / 10i128.pow(d.scale())).ok()
    };
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    let lower_letters = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_lowercase());
    let hours = |t: &str| -> Result<Parsed, Signal> {
        match t
            .parse::<Decimal>()
            .ok()
            .and_then(|d| d.checked_mul(Decimal::from(3_600_000)))
            .and_then(ms_of)
        {
            Some(ms) => Ok(Parsed::Value(Value::Duration(ms))),
            // A whole number of hours beyond the range (FMEvalDuration.al:285, :328).
            None if digits(t.trim_start_matches('-')) => Ok(Parsed::Rejected),
            None => refuse(),
        }
    };
    if number == 9 {
        if let Some(body) = text.strip_prefix('P') {
            // `P<d>DT<h>H<m>M<s>S` as Format 9 gives it, and with parts left out: `P1DT2H`, `PT1.5S` (format-evaluate/format-matrix/
            // FMEvalDuration.al:277, :280, corpus PR 1250). Each part once, in this order; the seconds may have a fraction.
            let parse = || -> Option<Decimal> {
                let (date_part, time_part) = match body.split_once('T') {
                    Some((d, t)) if !t.is_empty() => (d, t),
                    Some(_) => return None,
                    None => (body, ""),
                };
                let n = |t: &str| t.parse::<Decimal>().ok();
                let mut total = Decimal::ZERO;
                if !date_part.is_empty() {
                    let d = date_part.strip_suffix('D')?;
                    if !digits(d) {
                        return None;
                    }
                    total += n(d)? * Decimal::from(86_400_000);
                }
                let mut rest = time_part;
                for (unit, per) in [('H', 3_600_000i64), ('M', 60_000), ('S', 1000)] {
                    if let Some((v, r)) = rest.split_once(unit) {
                        let (whole, frac) = v.split_once('.').unwrap_or((v, "0"));
                        if !(digits(whole) && digits(frac)) || (unit != 'S' && v.contains('.')) {
                            return None;
                        }
                        total += n(&format!("{whole}.{frac}"))? * Decimal::from(per);
                        rest = r;
                    }
                }
                (rest.is_empty() && (!date_part.is_empty() || !time_part.is_empty())).then_some(total)
            };
            return match parse().and_then(ms_of) {
                Some(ms) => Ok(Parsed::Value(Value::Duration(ms))),
                None => refuse(),
            };
        }
        let first_digit = |t: &str| t.as_bytes().first().is_some_and(u8::is_ascii_digit);
        let rejected = text.trim_matches(' ').is_empty()
            || matches!(text, "\0" | "?")
            || first_digit(text)
            || text.strip_prefix('-').is_some_and(first_digit)
            || (lower_letters(text) && !text.starts_with('p'));
        return if rejected { Ok(Parsed::Rejected) } else { refuse() };
    }
    if number != 0 {
        // Formats 1 to 3: `1500` only (Evaluate.al:440-446).
        return if digits(text) { hours(text) } else { refuse() };
    }
    if text.is_empty() {
        return Ok(Parsed::Value(Value::Duration(0)));
    }
    if matches!(text, "\0" | "?" | "1T" | "P1DT2H" | "PT1.5S") {
        return Ok(Parsed::Rejected);
    }
    if language == 1031 && text.bytes().any(|b| b.is_ascii_alphabetic()) {
        const ENGLISH: [&str; 4] = ["millisecond", "milliseconds", "day", "days"];
        return if text.split(' ').any(|w| ENGLISH.contains(&w)) {
            Ok(Parsed::Rejected)
        } else {
            refuse()
        };
    }
    let plain = text.trim_matches(' ');
    if digits(plain.strip_prefix('-').unwrap_or(plain)) {
        return hours(plain);
    }
    if let Some((w, f)) = plain.split_once('.') {
        if digits(w) && digits(f) {
            return hours(plain);
        }
    }
    let split_by = |c: char| plain.split_once(c).is_some_and(|(a, b)| digits(a) && digits(b));
    if lower_letters(text) || plain.strip_prefix('+').is_some_and(digits) || split_by(',') || split_by('e') {
        return Ok(Parsed::Rejected);
    }
    let b = text.as_bytes();
    let mut i = 0;
    let mut total = Decimal::ZERO;
    let mut words: Vec<&str> = Vec::new();
    let mut units: Vec<i64> = Vec::new();
    let mut negative = false;
    let mut rejected = false;
    let mut terms = 0;
    while i < b.len() {
        while b.get(i) == Some(&b' ') {
            i += 1;
        }
        if i == b.len() {
            break;
        }
        if b[i] == b'-' {
            if terms > 0 || negative {
                return refuse();
            }
            negative = true;
            i += 1;
        }
        let start = i;
        while b.get(i).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
            i += 1;
        }
        if start == i {
            return refuse();
        }
        let num = &text[start..i];
        let (w, f) = num.split_once('.').unwrap_or((num, "0"));
        if !digits(w) || !digits(f) {
            return refuse();
        }
        let Ok(n) = num.parse::<Decimal>() else {
            return refuse();
        };
        while b.get(i) == Some(&b' ') {
            i += 1;
        }
        let ustart = i;
        while b.get(i).is_some_and(u8::is_ascii_alphabetic) {
            i += 1;
        }
        let word = &text[ustart..i];
        let per: i64 = match word {
            "day" | "days" | "d" | "D" => 86_400_000,
            "hour" | "hours" | "h" | "HOUR" => 3_600_000,
            "minute" | "minutes" | "min" | "m" => 60_000,
            "second" | "seconds" | "sec" | "s" | "Second" => 1000,
            "millisecond" | "milliseconds" | "milli" | "millis" => 1,
            "mins" | "ms" | "week" | "abc" => {
                rejected = true;
                terms += 1;
                continue;
            }
            _ => return refuse(),
        };
        let fraction = !n.fract().is_zero();
        if fraction && matches!(per, 86_400_000 | 60_000) {
            return refuse();
        }
        if fraction && per == 1 {
            rejected = true;
        }
        if words.contains(&word) {
            rejected = true;
        } else if units.contains(&per) {
            // The same unit in another spelling (`1 h 1 hour`) is not pinned.
            return refuse();
        }
        words.push(word);
        units.push(per);
        terms += 1;
        total += n * Decimal::from(per);
    }
    // A minus before the first term negates the whole text: `-1 millisecond`, `-1 day 2 hours` (FMEvalDuration.al:88, :195) and
    // the text Format gives for a negative Duration, `-1 day 1 hour 1 minute 1 second 1 millisecond` (format-matrix/FMFields.al
    // FieldRef_FormatThenEvaluate_Roundtrip_1, corpus PR 1256; al-runner issue 7471).
    if rejected {
        return Ok(Parsed::Rejected);
    }
    let Some(ms) = ms_of(total) else {
        // Beyond the range of a Duration (`999999999999 days`, FMEvalDuration.al:331).
        return if total.fract().is_zero() {
            Ok(Parsed::Rejected)
        } else {
            refuse()
        };
    };
    Ok(Parsed::Value(Value::Duration(if negative { -ms } else { ms })))
}
