//! `Evaluate` into a Date or DateTime local variable or parameter (`System.EvaluateDateVariable#1`), without a number or with
//! number 0, in an en-US session: the texts corpus PR 1503 measured and the pinned readers do not answer
//! (tests/al-language/format-evaluate/EvalShortUsDate.al, al-runner issues 8643 and 8644; open, not yet pinned). Two entries
//! mark them: `evaluate-date-variable-time-dropped` (a time after a date) and `evaluate-short-date-rejected-last-error` (the
//! last error a rejected text leaves when the result is used). Every other text, target, number and language keeps the readers
//! of `evaluate.rs`.

use super::{Parsed, date};
use crate::assumptions::{Assumption, assume};
use crate::native::{AlError, Ctx, Signal, unpinned_text_error, unsupported};
use crate::value::Value;

/// What follows the date part of a measured text.
#[derive(Clone, Copy, PartialEq)]
enum Tail {
    /// Nothing.
    None,
    /// One blank, `h:mm` or `hh:mm`, a blank and `AM` or `PM` (`01/04/25 12:00 PM`, `1/4/25 1:20 AM`); `hour` as written.
    Clock12 { hour: u32 },
    /// Two blanks before such a clock (`01/04/25  12:00 PM`).
    Clock12TwoBlanks,
    /// One blank and `hh:mm:ss` (`01/04/25 23:59:59`).
    ClockSeconds,
    /// One blank and `hh:mm` (`01/04/25 25:00`); `hour` as written.
    Clock24 { hour: u32 },
    /// One blank and `noon`.
    Noon,
    /// `T` and `hh:mm` (`01/04/25T12:00`).
    TClock,
}

/// A measured text: the date part (`m/d/yy`, `mm/dd/yy` or `mm/dd/yyyy`), whether it is `mm/dd/yy`, and its tail.
struct Shape<'a> {
    date: &'a str,
    two_digit: bool,
    tail: Tail,
}

fn digits(s: &str, n: std::ops::RangeInclusive<usize>) -> bool {
    n.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

/// `h:mm` (one or two digits for the hour) as the hour, `None` for another text.
fn hour_minute(s: &str, hour_digits: std::ops::RangeInclusive<usize>) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    (digits(h, hour_digits) && digits(m, 2..=2)).then(|| h.parse().ok())?
}

fn tail_of(rest: &str) -> Option<Tail> {
    if rest.is_empty() {
        return Some(Tail::None);
    }
    if rest == " noon" {
        return Some(Tail::Noon);
    }
    if let Some(clock) = rest.strip_prefix('T') {
        return hour_minute(clock, 2..=2).map(|_| Tail::TClock);
    }
    let twelve = |s: &str| {
        let c = s.strip_suffix(" AM").or_else(|| s.strip_suffix(" PM"))?;
        hour_minute(c, 1..=2)
    };
    if let Some(s) = rest.strip_prefix("  ") {
        return twelve(s).map(|_| Tail::Clock12TwoBlanks);
    }
    let s = rest.strip_prefix(' ')?;
    if let Some(hour) = twelve(s) {
        return Some(Tail::Clock12 { hour });
    }
    if let Some(hour) = hour_minute(s, 2..=2) {
        return Some(Tail::Clock24 { hour });
    }
    let b = s.as_bytes();
    (b.len() == 8 && b[2] == b':' && b[5] == b':' && digits(&s[..2], 2..=2) && digits(&s[3..5], 2..=2) && digits(&s[6..], 2..=2))
        .then_some(Tail::ClockSeconds)
}

fn shape(text: &str) -> Option<Shape<'_>> {
    let end = text.find([' ', 'T']).unwrap_or(text.len());
    let (date, rest) = text.split_at(end);
    let p: Vec<&str> = date.split('/').collect();
    let [m, d, y] = p.as_slice() else {
        return None;
    };
    if !(digits(m, 1..=2) && digits(d, 1..=2) && (digits(y, 2..=2) || digits(y, 4..=4))) {
        return None;
    }
    let two_digit = m.len() == 2 && d.len() == 2 && y.len() == 2;
    Some(Shape { date, two_digit, tail: tail_of(rest)? })
}

/// Whether `mm/dd/yy` names no day in the measured way: month 0 or past 12 (`00/01/25`, `13/01/25`), or a day from 1 to 31 past
/// the end of its month (`02/30/25`). Day 0 and days past 31 are not measured.
fn not_a_day(date: &str) -> bool {
    let p: Vec<u32> = date.split('/').filter_map(|s| s.parse().ok()).collect();
    let [m, d, y] = p.as_slice() else {
        return false;
    };
    if *m == 0 || *m > 12 {
        return (1..=31).contains(d);
    }
    let y = if *y < 50 { 2000 + *y as i32 } else { 1900 + *y as i32 };
    (1..=31).contains(d) && *d > crate::iso::days_in_month(y, *m)
}

/// What a measured text does; `None`: the readers of `evaluate.rs` answer it.
enum Cell {
    /// The date part reads, the tail is not read: the Date, or the DateTime at midnight (time-dropped entry).
    DateOnly,
    /// Rejected; with the result used it leaves a last error of this code whose text is not measured (last-error entry). The
    /// statement form raises the type's own error (measured), unless `statement_measured` is false: then it refuses.
    LastError { code: &'static str, statement_measured: bool, time_dropped: bool },
}

fn cell(datetime: bool, ret_used: bool, s: &Shape) -> Option<Cell> {
    let bad = s.two_digit && not_a_day(s.date);
    match (datetime, s.tail) {
        // EvalShortUsDate.al Date_ShortDate_*_Return `13/01/25`, `02/30/25`, `00/01/25` (lasterr=Evaluate) and DateTime_ShortDate_*;
        // the statement form is pinned already (Date_Statement_*, DateTime_Statement_* answer as main does).
        (_, Tail::None) if bad && ret_used => {
            Some(Cell::LastError { code: "Evaluate", statement_measured: true, time_dropped: false })
        }
        (_, Tail::None) => None,
        // Date_WithTime_*_Return `13/04/25 12:00 PM` (lasterr=Evaluate), Date_Statement_* (the Date error with the whole text);
        // DateTime_WithTime_* and DateTime_Statement_* (the DateTime error).
        (_, Tail::Clock12 { hour: 1..=12 }) if bad => {
            Some(Cell::LastError { code: "Evaluate", statement_measured: true, time_dropped: !datetime })
        }
        // A day that does not exist with another tail is not measured.
        _ if !s.two_digit || not_a_day(s.date) || bad => None,
        // DateTime_WithTime_*_Return `01/04/25 13:00 PM`: lasterr=DateInvalid; the statement form is not measured.
        (true, Tail::Clock12 { hour: 13..=23 }) => {
            Some(Cell::LastError { code: "DateInvalid", statement_measured: false, time_dropped: false })
        }
        // DateTime_WithTime_*_Return `01/04/25 25:00` (lasterr=Evaluate), DateTime_Statement_* (the DateTime error).
        (true, Tail::Clock24 { hour: 24..=99 }) => {
            Some(Cell::LastError { code: "Evaluate", statement_measured: true, time_dropped: false })
        }
        // DateTime_WithTime_* `01/04/25T12:00`: midnight.
        (true, Tail::TClock) => Some(Cell::DateOnly),
        (true, _) => None,
        // Date_WithTime_* and Date_Statement_*: `12:00 PM`, `12:00 AM`, `13:00 PM`, two blanks, `23:59:59`, `noon`, `T12:00`.
        // `hh:mm` after a blank is pinned (date_forms `dropped_suffix`, FMEvalDate.al:338).
        (false, Tail::Clock24 { .. } | Tail::None) => None,
        (false, Tail::Clock12 { .. } | Tail::Clock12TwoBlanks | Tail::ClockSeconds | Tail::Noon | Tail::TClock) => {
            Some(Cell::DateOnly)
        }
    }
}

/// The date parts of the measured `1/4/25` and `01/04/2025` forms are measured with `12:00 PM` and `1:20 AM` only; with another
/// tail the text keeps the readers of `evaluate.rs`.
fn measured_cell(datetime: bool, ret_used: bool, text: &str) -> Option<(Shape<'_>, Cell)> {
    let s = shape(text)?;
    if !s.two_digit && !matches!(s.tail, Tail::Clock12 { .. }) {
        return None;
    }
    let c = cell(datetime, ret_used, &s)?;
    Some((s, c))
}

/// Whether `text` is a measured cell for a target of this kind (`datetime`: a DateTime, else a Date).
pub(super) fn measured(c: &dyn Ctx, datetime: bool, text: &str) -> bool {
    c.language() == 1033 && measured_cell(datetime, c.ret_used(), text).is_some()
}

/// The reading of a text [`measured`] took.
pub(super) fn read(c: &mut dyn Ctx, datetime: bool, text: &str) -> Result<Parsed, Signal> {
    let ret_used = c.ret_used();
    let Some((s, cell)) = measured_cell(datetime, ret_used, text) else {
        return unsupported(format!("Evaluate of `{text}` outside the shapes of corpus PR 1503"));
    };
    match cell {
        Cell::DateOnly => {
            assume(Assumption::EvaluateDateVariableTimeDropped);
            match date(s.date, 0, c.today_day(), c.work_date_day(), true)? {
                Parsed::Value(Value::Date(d, false)) if datetime => {
                    Ok(Parsed::Value(Value::DateTime((i64::from(d) - 1) * 86_400_000 + 1)))
                }
                p @ Parsed::Value(Value::Date(_, false)) => Ok(p),
                _ => unsupported(format!("Evaluate of `{text}`: a date part not measured")),
            }
        }
        Cell::LastError { code, statement_measured, time_dropped } => {
            if time_dropped {
                assume(Assumption::EvaluateDateVariableTimeDropped);
            }
            assume(Assumption::EvaluateShortDateRejectedLastError);
            if ret_used {
                let e: AlError = unpinned_text_error(code, "the last error text of a rejected Evaluate (corpus PR 1503)");
                c.set_last_error(e);
            } else if !statement_measured {
                return unsupported(format!("the error of the statement Evaluate of `{text}` into a DateTime (not measured)"));
            }
            Ok(Parsed::Rejected)
        }
    }
}
