//! `Evaluate` into a Date or DateTime local variable or parameter (`System.EvaluateDateVariable#1`), without a number or with
//! number 0, in an en-US session: the texts corpus PR 1503 measured and the pinned readers do not answer
//! (tests/al-language/format-evaluate/EvalShortUsDate.al, al-runner issues 8643 and 8644; open, not yet pinned). Two entries
//! mark them: `evaluate-date-variable-time-dropped` (a time after a date) and `evaluate-short-date-rejected-last-error` (the
//! last error a rejected text leaves when the result is used). Every other text, target, number and language keeps the readers
//! of `evaluate.rs`.

use super::{Parsed, date};
use crate::assumptions::{Assumption, assume};
use crate::native::{Ctx, Signal, unsupported};
use crate::value::Value;

/// What follows the date part of a measured text: exactly the tails the probe measured, each one text (a blank before it,
/// except `T12:00`).
#[derive(Clone, Copy, PartialEq)]
enum Tail {
    /// Nothing.
    None,
    /// ` 12:00 PM`.
    Pm1200,
    /// ` 12:00 AM`.
    Am1200,
    /// ` 1:20 AM`.
    Am0120,
    /// ` 13:00 PM`.
    Pm1300,
    /// `  12:00 PM` (two blanks).
    TwoBlanksPm1200,
    /// ` 23:59:59`.
    Seconds2359,
    /// ` 25:00`.
    H2500,
    /// ` noon`.
    Noon,
    /// `T12:00`.
    T1200,
}

/// A measured text: the date part, its digit lengths and its tail. `bad`: `mm/dd/yy` names no day in the measured way (below).
struct Shape<'a> {
    date: &'a str,
    /// `m/d/yy` or `mm/dd/yyyy`.
    other_parts: bool,
    bad: bool,
    tail: Tail,
}

fn digits(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|b| b.is_ascii_digit())
}

fn tail_of(rest: &str) -> Option<Tail> {
    Some(match rest {
        "" => Tail::None,
        " 12:00 PM" => Tail::Pm1200,
        " 12:00 AM" => Tail::Am1200,
        " 1:20 AM" => Tail::Am0120,
        " 13:00 PM" => Tail::Pm1300,
        "  12:00 PM" => Tail::TwoBlanksPm1200,
        " 23:59:59" => Tail::Seconds2359,
        " 25:00" => Tail::H2500,
        " noon" => Tail::Noon,
        "T12:00" => Tail::T1200,
        _ => return None,
    })
}

fn shape(text: &str) -> Option<Shape<'_>> {
    let end = text.find([' ', 'T']).unwrap_or(text.len());
    let (date, rest) = text.split_at(end);
    let p: Vec<&str> = date.split('/').collect();
    let [m, d, y] = p.as_slice() else {
        return None;
    };
    let lens = (m.len(), d.len(), y.len());
    if !(digits(m, lens.0) && digits(d, lens.1) && digits(y, lens.2)) {
        return None;
    }
    let two_digit = lens == (2, 2, 2);
    let other_parts = matches!(lens, (1, 1, 2) | (2, 2, 4));
    if !(two_digit || other_parts) {
        return None;
    }
    Some(Shape {
        date,
        other_parts,
        bad: two_digit && not_a_day(date),
        tail: tail_of(rest)?,
    })
}

/// Whether `mm/dd/yy` names no day in the measured way: the Date reader of `evaluate.rs` (which owns the two-digit year and the
/// month lengths) rejects it, and the day is from 1 to 31 (`13/01/25`, `00/01/25`, `02/30/25`). Day 0 and days past 31 are not
/// measured.
fn not_a_day(date: &str) -> bool {
    let day_measured = date
        .split('/')
        .nth(1)
        .and_then(|d| d.parse::<u32>().ok())
        .is_some_and(|d| (1..=31).contains(&d));
    day_measured && matches!(super::date(date, 0, None, None, true), Ok(Parsed::Rejected))
}

/// What a measured text does; `None`: the readers of `evaluate.rs` answer it.
enum Cell {
    /// The date part reads, the tail is not read: the Date, or the DateTime at midnight (time-dropped entry).
    DateOnly,
    /// Rejected; with the result used it leaves a last error of this code whose text is not measured (last-error entry). The
    /// statement form raises the type's own error (measured), unless `statement_measured` is false: then it refuses.
    LastError {
        code: &'static str,
        statement_measured: bool,
        time_dropped: bool,
    },
}

fn cell(datetime: bool, ret_used: bool, s: &Shape) -> Option<Cell> {
    // The date parts `m/d/yy` and `mm/dd/yyyy` are measured into a Date with `12:00 PM` and `1:20 AM` only.
    if s.other_parts && (datetime || !matches!(s.tail, Tail::Pm1200 | Tail::Am0120)) {
        return None;
    }
    match (datetime, s.tail) {
        // EvalShortUsDate.al Date_ShortDate_*_Return `13/01/25`, `02/30/25`, `00/01/25` (lasterr=Evaluate) and DateTime_ShortDate_*;
        // the statement form is pinned already (Date_Statement_*, DateTime_Statement_* answer as main does).
        (_, Tail::None) if s.bad && ret_used => Some(Cell::LastError {
            code: "Evaluate",
            statement_measured: true,
            time_dropped: false,
        }),
        (_, Tail::None) => None,
        // Date_WithTime_*_Return `13/04/25 12:00 PM` (lasterr=Evaluate), Date_Statement_* (the Date error with the whole text);
        // DateTime_WithTime_* and DateTime_Statement_* (the DateTime error).
        (_, Tail::Pm1200) if s.bad => Some(Cell::LastError {
            code: "Evaluate",
            statement_measured: true,
            time_dropped: !datetime,
        }),
        // A day that does not exist with another tail is not measured.
        _ if s.bad => None,
        // DateTime_WithTime_*_Return `01/04/25 13:00 PM`: lasterr=DateInvalid; the statement form is not measured.
        (true, Tail::Pm1300) => Some(Cell::LastError {
            code: "DateInvalid",
            statement_measured: false,
            time_dropped: false,
        }),
        // DateTime_WithTime_*_Return `01/04/25 25:00` (lasterr=Evaluate), DateTime_Statement_* (the DateTime error).
        (true, Tail::H2500) => Some(Cell::LastError {
            code: "Evaluate",
            statement_measured: true,
            time_dropped: false,
        }),
        // DateTime_WithTime_* `01/04/25T12:00`: midnight.
        (true, Tail::T1200) => Some(Cell::DateOnly),
        (true, _) => None,
        // Date_WithTime_* and Date_Statement_*: `12:00 PM`, `12:00 AM`, `1:20 AM`, `13:00 PM`, two blanks, `23:59:59`, `noon`,
        // `T12:00`. `hh:mm` after a blank (`25:00`) is pinned (date_forms `dropped_suffix`, FMEvalDate.al:338).
        (false, Tail::H2500) => None,
        (false, _) => Some(Cell::DateOnly),
    }
}

fn measured_cell(datetime: bool, ret_used: bool, text: &str) -> Option<(Shape<'_>, Cell)> {
    let s = shape(text)?;
    let c = cell(datetime, ret_used, &s)?;
    Some((s, c))
}

/// Whether `text` is a measured cell for a target of this kind (`datetime`: a DateTime, else a Date), in the en-US session of the
/// probe: GlobalLanguage 1033 and no `CurrReport.FormatRegion` set in a report (every date cell depends on the region).
pub(super) fn measured(c: &dyn Ctx, datetime: bool, text: &str) -> bool {
    c.language() == 1033
        && c.region().is_none()
        && c.region_unpinned().is_none()
        && measured_cell(datetime, c.ret_used(), text).is_some()
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
                Parsed::Value(Value::Date(d, false)) if datetime => Ok(Parsed::Value(super::date_start(d))),
                p @ Parsed::Value(Value::Date(_, false)) => Ok(p),
                _ => unsupported(format!("Evaluate of `{text}`: a date part not measured")),
            }
        }
        Cell::LastError {
            code,
            statement_measured,
            time_dropped,
        } => {
            if time_dropped {
                assume(Assumption::EvaluateDateVariableTimeDropped);
            }
            assume(Assumption::EvaluateShortDateRejectedLastError);
            if ret_used {
                return Ok(Parsed::RejectedUsedCode(code));
            }
            if !statement_measured {
                return unsupported(format!(
                    "the error of the statement Evaluate of `{text}` into a DateTime (not measured)"
                ));
            }
            Ok(Parsed::Rejected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::tests::TestCtx;

    fn is_measured(datetime: bool, text: &str) -> bool {
        measured(&TestCtx::default(), datetime, text)
    }

    /// Each cell is one measured text: the neighbors keep the readers of `evaluate.rs`.
    #[test]
    fn only_the_measured_texts_are_cells() {
        for (datetime, text) in [
            (true, "01/04/25 13:00 PM"),
            (true, "01/04/25 25:00"),
            (true, "01/04/25T12:00"),
            (false, "01/04/25 12:00 PM"),
            (false, "01/04/25 1:20 AM"),
            (false, "1/4/25 12:00 PM"),
            (false, "01/04/2025 1:20 AM"),
            (false, "13/04/25 12:00 PM"),
        ] {
            assert!(is_measured(datetime, text), "{text}");
        }
        for (datetime, text) in [
            (true, "01/04/25 24:00"),
            (true, "01/04/25 23:00 PM"),
            (true, "01/04/25 13:30 PM"),
            (true, "01/04/25 25:30"),
            (true, "01/04/25T25:00"),
            (true, "01/04/25T11:00"),
            (false, "1/04/2025 11:59 PM"),
            (false, "01/4/2025 12:00 PM"),
            (false, "1/4/25 11:59 PM"),
            (false, "01/04/2025 12:00 AM"),
            (false, "01/04/25 11:59 PM"),
            (false, "01/04/25 13:00 AM"),
            (false, "13/04/25 1:20 AM"),
        ] {
            assert!(!is_measured(datetime, text), "{text}");
        }
    }

    /// The statement form of `13:00 PM` into a DateTime is not measured; the form with the result used leaves `DateInvalid`.
    #[test]
    fn the_statement_form_of_13_pm_into_a_datetime_refuses() {
        let mut ctx = TestCtx::default();
        assert!(matches!(
            read(&mut ctx, true, "01/04/25 13:00 PM"),
            Ok(Parsed::RejectedUsedCode("DateInvalid"))
        ));
        ctx.result_ignored = true;
        assert!(matches!(
            read(&mut ctx, true, "01/04/25 13:00 PM"),
            Err(Signal::Unsupported(_))
        ));
    }

    /// A text outside the cells is refused by `read` as well (a caller that skipped `measured`).
    #[test]
    fn read_refuses_a_text_that_is_not_a_cell() {
        let mut ctx = TestCtx::default();
        assert!(matches!(
            read(&mut ctx, true, "01/04/25 24:00"),
            Err(Signal::Unsupported(_))
        ));
    }
}
