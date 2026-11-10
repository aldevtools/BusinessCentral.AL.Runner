//! More texts `Evaluate` reads into a Date and a DateTime with format 0, en-US (month first), and the same under
//! GlobalLanguage 1031 (the language does not change the order). Pinned by corpus master:
//! - format-evaluate/Y71EvaluateDashedDates.al (corpus PR 1247): Evaluate_Date_TrailingD_En and _De (al-runner issue 7363),
//!   Evaluate_Date_FilterOperatorTexts and _De (issue 7364), Evaluate_DateTime_DateOnlyText_En, _De and
//!   Evaluate_DateTime_DateAndTimeText_En (issue 7384);
//! - format-evaluate/format-matrix/FMEvalDate.al and FMEvalDateTime.al (corpus PR 1250).

use super::{Parsed, date};
use crate::native::Signal;
use crate::value::Value;

/// A date text with an operator before it, or `..` before it, is no date: `<=11/09/26`, `>=`, `<`, `>`, `<>`, `=`, `<= 11/09/26`,
/// `<=11-09-26`, `<=` alone, `..11/09/26` (Y71EvaluateDashedDates.al Evaluate_Date_FilterOperatorTexts, FMEvalDate.al:205-210). So
/// are `%today`, `CM`, NUL and `?` (FMEvalDate.al:214, :224, :226-230).
fn rejected_text(t: &str) -> bool {
    ["<", ">", "=", ".."].iter().any(|p| t.starts_with(p)) || matches!(t, "%today" | "CM" | "\0" | "?")
}

/// The part of `t` that is digits and `/`, `.` or `-`, and the rest.
fn split_core(t: &str) -> (&str, &str) {
    let end = t
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '/' | '.' | '-')))
        .unwrap_or(t.len());
    (&t[..end], &t[end..])
}

/// Whether `core` has the shape of a month-first date the reader decides on: three numbers split by one `/`, `.` or `-`, or six or
/// eight digits.
fn date_shaped(core: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.len() <= 4 && s.bytes().all(|b| b.is_ascii_digit());
    (core.bytes().all(|b| b.is_ascii_digit()) && matches!(core.len(), 6 | 8))
        || ['/', '.', '-'].iter().any(|sep| {
            let p: Vec<&str> = core.split(*sep).collect();
            p.len() == 3 && p.iter().all(|s| digits(s))
        })
}

/// What follows a date and is dropped: one or more `D` or `d`, then blanks (`12/09/26D`, `12/09/26d`, `12/09/26DD`, `12/09/26D `);
/// a blank and `D` (`12/09/26 D`); `T` (`12/09/26T`, FMEvalDate.al:85); a blank and a time of hours and minutes
/// (`12/09/26 12:00`, FMEvalDate.al:338).
fn dropped_suffix(rest: &str) -> bool {
    let after_d = rest.trim_start_matches(['D', 'd']);
    (after_d.len() < rest.len() && after_d.trim_matches(' ').is_empty())
        || rest == " D"
        || rest == "T"
        || rest.strip_prefix(' ').is_some_and(|clock| {
            let b = clock.as_bytes();
            b.len() == 5 && b[2] == b':' && clock.bytes().enumerate().all(|(i, c)| i == 2 || c.is_ascii_digit())
        })
}

/// A Date text with a form of this module: `None` when it has none of them. `month_first` is the Evaluate reader.
pub(super) fn date_text(t: &str, today: Option<i32>, work: Option<i32>) -> Option<Result<Parsed, Signal>> {
    if rejected_text(t) {
        return Some(Ok(Parsed::Rejected));
    }
    // A range reads as its first date when the second is a date or nothing: `11/09/26..12/09/26` and `11/09/26..` are
    // 2026-11-09 (Y71EvaluateDashedDates.al Evaluate_Date_FilterOperatorTexts `range`, `range_open_end`; FMEvalDate.al:211).
    if let Some((first, second)) = t.split_once("..") {
        if !date_shaped(first) || !(second.is_empty() || date_shaped(second)) {
            return None;
        }
        return Some(date(first, 0, today, work, true));
    }
    let (core, rest) = split_core(t);
    if rest.is_empty() || !date_shaped(core) || !dropped_suffix(rest) {
        return letters_between(t, today);
    }
    // `13/09/26D` is rejected as `13/09/26` is (Evaluate_Date_TrailingD_En `day_13_D`).
    Some(date(core, 0, today, work, true))
}

/// A month of one or two digits, letters and a number, split by the same blank, `-` or `/`: the letters count as a separator and the
/// number gives the day: two digits are the day of the current year, four digits the day and a two-digit year
/// (`9 Mar 2026` is 2026-09-20, FMEvalDate.al:332; `01-xx-2012` is 2012-01-20, Y71EvaluateDashedDates.al
/// Evaluate_DashedDate_AfterLeadingBlanksRemoved `statement_garbage`; `01/xx/26` is 26 January of this year,
/// Evaluate_DateTime_DateAndTimeText_En `garbage`, FMEvalDateTime.al:319). A day above 12 before a month name is the
/// day-month-year form (types/P130EvaluateDate.al:98 `16 Oct 2026`), read by the caller.
fn letters_between(t: &str, today: Option<i32>) -> Option<Result<Parsed, Signal>> {
    let sep = [' ', '-', '/'].into_iter().find(|s| t.contains(*s))?;
    let p: Vec<&str> = t.split(sep).collect();
    let [m, letters, n] = p.as_slice() else {
        return None;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !(digits(m)
        && m.len() <= 2
        && !letters.is_empty()
        && letters.bytes().all(|b| b.is_ascii_alphabetic())
        && digits(n))
    {
        return None;
    }
    let month: u32 = m.parse().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    let text = match n.len() {
        2 => {
            let year = crate::format::civil_from_day(today?).0;
            format!("{m}/{n}/{year}")
        }
        4 => format!("{m}/{}/{}", &n[..2], &n[2..]),
        _ => return None,
    };
    Some(date(&text, 0, today, None, true))
}

/// A DateTime text with a form of this module, `t` without blanks around it: a month-first date alone, with a dropped suffix (`D`,
/// `T`) or with a time after one blank, is that date at midnight or at that time in UTC (FMEvalDateTime.al:79-136, :139-160,
/// :313-321; Y71EvaluateDashedDates.al Evaluate_DateTime_DateOnlyText_En). A time after a date that holds more than one time or
/// ends with `T` is rejected (`01/01/26 12:00:00T`, `01/01/26 12:00:00 12:00:00`, FMEvalDateTime.al:149, :325). A time alone of
/// hours, minutes and seconds, or with AM or PM, is rejected with `The date is not valid.` (FMEvalDateTime.al:205-210:
/// `12:00:00`, `12:00 AM`, code DateInvalid); the compact ISO form `20260101T120000Z` is rejected (:268).
pub(super) fn datetime_text(t: &str, today: Option<i32>) -> Option<Result<Parsed, Signal>> {
    let b = t.as_bytes();
    let clock = |s: &[u8]| {
        s.len() == 8
            && s[2] == b':'
            && s[5] == b':'
            && s.iter()
                .enumerate()
                .all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit())
    };
    if clock(b)
        || t.strip_suffix(" AM")
            .or_else(|| t.strip_suffix(" PM"))
            .is_some_and(|c| {
                let c = c.as_bytes();
                c.len() == 5 && c[2] == b':' && c.iter().enumerate().all(|(i, d)| i == 2 || d.is_ascii_digit())
            })
    {
        return Some(Ok(Parsed::RejectedWith("The date is not valid.", "DateInvalid")));
    }
    if b.len() == 16
        && b[8] == b'T'
        && b[15] == b'Z'
        && t[..8].bytes().chain(t[9..15].bytes()).all(|c| c.is_ascii_digit())
    {
        return Some(Ok(Parsed::Rejected));
    }
    let (day_text, clock_text) = match t.split_once(' ') {
        Some((d, c)) => (d, Some(c)),
        None => (t, None),
    };
    let (core, _) = split_core(day_text);
    if !date_shaped(core) && letters_between(day_text, today).is_none() {
        return None;
    }
    let day = match date_text(day_text, today, None) {
        Some(r) => r,
        None if day_text == core => date(core, 0, today, None, true),
        None => return None,
    };
    let day = match day {
        Ok(Parsed::Value(Value::Date(d, false))) => d,
        Ok(Parsed::Value(_)) => return None,
        Ok(_) => return Some(Ok(Parsed::Rejected)),
        Err(e) => return Some(Err(e)),
    };
    let Some(clock_text) = clock_text else {
        return Some(Ok(datetime_from(day, 1)));
    };
    if clock_text.ends_with('T') || clock_text.matches(':').count() > 2 {
        return Some(Ok(Parsed::Rejected));
    }
    Some(match super::time(clock_text, 0) {
        Ok(Parsed::Value(Value::Time(t))) => Ok(datetime_from(day, t)),
        Ok(Parsed::Value(_)) => return None,
        Ok(_) => Ok(Parsed::Rejected),
        Err(e) => Err(e),
    })
}

/// The DateTime of day `day` at the Time value `t` (milliseconds after midnight plus one, as a Time holds them), in UTC.
fn datetime_from(day: i32, t: i64) -> Parsed {
    Parsed::Value(Value::DateTime((i64::from(day) - 1) * 86_400_000 + t))
}

/// `<=`, `>=`, `<` or `>` and then a date of three numbers split by `/`.
pub(super) fn operator_before_slashed_date(t: &str) -> bool {
    let rest = ["<=", ">=", "<", ">"].iter().find_map(|op| t.strip_prefix(op));
    rest.is_some_and(|r| {
        let p: Vec<&str> = r.split('/').collect();
        p.len() == 3
            && p.iter()
                .all(|x| !x.is_empty() && x.len() <= 4 && x.bytes().all(|b| b.is_ascii_digit()))
    })
}
