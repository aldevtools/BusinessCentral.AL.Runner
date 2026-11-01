//! Date and DateTime filter texts of an English session that the readers in `filter.rs` leave out: a month and a day without
//! a year, slash forms with impossible numbers, and DateTime texts with a four-digit year
//! (filters/lt-y62-date-filter-short/Y62DateFilterShort.al, codeunit 93260, writer Y62).

use crate::value::{RtType, Value};

fn digits(p: &str) -> bool {
    !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())
}

fn read_date(m: &str, d: &str, y: &str) -> Option<i32> {
    match crate::evaluate::read_text(&RtType::Date, &format!("{m}/{d}/{y}")) {
        Some(Value::Date(day, false)) if day > 0 => Some(day),
        _ => None,
    }
}

/// A Date filter value of an English session: `Some(Ok)` the date, `Some(Err)` the invalid-date error, `None` a text of another
/// shape. Called after the other readers of `date_filter_value` found nothing.
/// - A month and a day, joined by `/`, `-` or `.`, one or two digits each, take the year of the work date: `1/1`, `1/28`, `01/28`,
///   `1-28`, `1.28` (SetFilter_Date_MonthDayOnly_WorkDate2030 and _2031). `28/1` and `99/99` are invalid.
/// - Month, day and year joined by `/`: a one-digit year reads as two digits (`1/1/0` is 01/01/00, SetFilter_Date_ImpossibleShortForms);
///   impossible numbers with a year of two or four digits (`99/99/9999`, `13/13/2026`, `02/30/2026`, `00/00/0000`,
///   SetFilter_Date_ImpossibleSlashForms) and a year of five digits or more (`1/1/99999`) are invalid. A three-digit year is not
///   pinned.
pub(crate) fn english_date(t: &str, work_date: i32) -> Option<Result<Value, ()>> {
    let sep = ['/', '-', '.'].into_iter().find(|c| t.contains(*c))?;
    let parts: Vec<&str> = t.split(sep).collect();
    if !parts.iter().all(|p| digits(p)) {
        return None;
    }
    let day = match parts[..] {
        [m, d] if m.len() <= 2 && d.len() <= 2 => {
            let year = crate::format::civil_from_day(work_date).0;
            read_date(m, d, &format!("{year:04}"))
        }
        [m, d, y] if sep == '/' && m.len() <= 2 && d.len() <= 2 => match y.len() {
            1 => read_date(m, d, &format!("0{y}")),
            2 | 4 => read_date(m, d, y),
            3 => return None,
            _ => None,
        },
        _ => return None,
    };
    Some(day.map(|d| Value::Date(d, false)).ok_or(()))
}

/// A DateTime filter value of an English session that `datetime_filter_text` does not read: `Some(Ok)` the value, `Some(Err)` the
/// evaluate error, `None` a text of another shape (SetFilter_DateTime_TextForms).
/// - `YYYY-MM-DD HH:MM:SS` and `MM/DD/YYYY HH:MM:SS AM` (`2024-01-01 10:30:00`, `01/01/2024 10:30:00 AM` filter to 01/01/24 10:30 AM).
/// - Digit groups that are no date or no time (`99/99/9999 99:99:99`) and words of letters (`not a datetime`) raise.
pub fn english_datetime(t: &str) -> Option<Result<Value, ()>> {
    if t.contains(' ') && t.chars().all(|c| c.is_ascii_alphabetic() || c == ' ') {
        return Some(Err(()));
    }
    let parts: Vec<&str> = t.split(' ').collect();
    let (date, clock, suffix) = match parts[..] {
        [d, c] => (d, c, None),
        [d, c, s @ ("AM" | "PM")] => (d, c, Some(s)),
        _ => return None,
    };
    let dp: Vec<&str> = date.split(['/', '-']).collect();
    let cp: Vec<&str> = clock.split(':').collect();
    if dp.len() != 3 || !dp.iter().all(|p| digits(p)) || cp.len() != 3 || !cp.iter().all(|p| digits(p) && p.len() == 2)
    {
        return None;
    }
    let day = if date.contains('-') && !date.contains('/') {
        if suffix.is_some() || dp[0].len() != 4 || dp[1].len() != 2 || dp[2].len() != 2 {
            return None;
        }
        read_date(dp[1], dp[2], dp[0])
    } else if date.contains('/') && !date.contains('-') {
        if dp[0].len() != 2 || dp[1].len() != 2 || dp[2].len() != 4 {
            return None;
        }
        read_date(dp[0], dp[1], dp[2])
    } else {
        return None;
    };
    let (h, m, s): (i64, i64, i64) = (cp[0].parse().ok()?, cp[1].parse().ok()?, cp[2].parse().ok()?);
    let h = match suffix {
        Some("AM") if h == 12 => Some(0),
        Some("PM") if h == 12 => Some(12),
        Some("AM") if (1..12).contains(&h) => Some(h),
        Some("PM") if (1..12).contains(&h) => Some(h + 12),
        // An hour past 12 with AM or PM is not pinned.
        Some(_) if h <= 23 => return None,
        Some(_) => None,
        None => (h < 24).then_some(h),
    };
    match (day, h) {
        (Some(day), Some(h)) if m < 60 && s < 60 => Some(Ok(Value::DateTime(
            (i64::from(day) - 1) * 86_400_000 + ((h * 60 + m) * 60 + s) * 1000 + 1,
        ))),
        _ => Some(Err(())),
    }
}

/// Whether a DateTime filter text is one [`english_datetime`] rejects: the error is pinned without a `MakeDateTimeFilter`
/// subscriber changing it (Y62DateFilterShort.al:SetFilter_DateTime_TextForms ran with the Base Application's subscribers).
pub fn datetime_rejected(t: &str) -> bool {
    matches!(english_datetime(t.trim()), Some(Err(())))
}
