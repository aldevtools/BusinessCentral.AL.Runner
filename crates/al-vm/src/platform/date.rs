//! The `Date` table (2000000007): one computed row per period, for the period types Date, Week,
//! Month, Quarter and Year. The key is ("Period Type", "Period Start").
//!
//! Pinned by corpus record/TestDateVirtualTable.al:
//! - the columns of each period type (lines 33-121): Date is numbered Monday = 1 to Sunday = 7 and
//!   named for the weekday; Week starts on Monday, is numbered by its ISO week and named for the
//!   number; Month is numbered 1 to 12 and named for the month; Quarter and Year are named for
//!   their number; "Period End" is a closing date (lines 124-146);
//! - the filters (open ends, unions, inverted ranges: lines 148-348);
//! - the ends of the table (lines 351-451): a period exists only if it fits whole between
//!   3 January of year 1 and 31 December 9999, so the first row is 3 January 1 (Date), 8 January 1
//!   (Week), 1 February 1 (Month), 1 April 1 (Quarter), 1 January 2 (Year), and the last week
//!   starts on 20 December 9999;
//! - a keyed `Get` answers the row at both ends and not outside them (lines 453-483).
//!
//! "Period Invariant Name" is not pinned by any corpus test. The declaration describes it as the
//! name that does not change with the language, and the runtime's culture is en-US, so it is the
//! "Period Name" (docs/probes-needed.md, "Date table: Period Invariant Name").

use super::{blank, option, pos, ranges, refused};
use al_builtins::filter::FieldFilter;
use al_builtins::format::{civil_from_day, day_from_civil};
use al_builtins::value::Value;
use al_data::{Row, Scan, StoreError, TableDef};

const PERIOD_TYPE: u32 = 1;
const PERIOD_START: u32 = 2;
const PERIOD_END: u32 = 3;
const PERIOD_NO: u32 = 4;
const PERIOD_NAME: u32 = 5;
const PERIOD_INVARIANT_NAME: u32 = 6;

/// 3 January of year 1, the first Date period (TestDateVirtualTable.al:293, 405).
const FIRST_DAY: i64 = 3;
/// 31 December 9999, the last day the platform can represent (TestDateVirtualTable.al:341).
fn last_day() -> i64 {
    i64::from(day_from_civil(9999, 12, 31))
}
/// The most periods a read examines for a filter on a column that is not part of the key.
const SEARCH_LIMIT: u32 = 5_000_000;

const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
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

fn weekday(day: i64) -> i64 {
    (day - 1).rem_euclid(7) + 1
}

fn civil(day: i64) -> (i32, u32, u32) {
    civil_from_day(day as i32)
}

/// The first day of the period of type `ty` that holds `day`.
fn floor(ty: i64, day: i64) -> i64 {
    let (y, m, _) = civil(day);
    i64::from(match ty {
        0 => return day,
        1 => return day - (weekday(day) - 1),
        2 => day_from_civil(y, m, 1),
        3 => day_from_civil(y, (m - 1) / 3 * 3 + 1, 1),
        _ => day_from_civil(y, 1, 1),
    })
}

/// The last day of the period of type `ty` that starts on `start`.
fn end_of(ty: i64, start: i64) -> i64 {
    let (y, m, _) = civil(start);
    let next_month = |months: u32| {
        let (ny, nm) = (y + ((m - 1 + months) / 12) as i32, (m - 1 + months) % 12 + 1);
        i64::from(day_from_civil(ny, nm, 1)) - 1
    };
    match ty {
        0 => start,
        1 => start + 6,
        2 => next_month(1),
        3 => next_month(3),
        _ => next_month(12),
    }
}

/// Whether a period of type `ty` that starts on `start` is a row: it fits whole in the range.
fn fits(ty: i64, start: i64) -> bool {
    start >= FIRST_DAY && end_of(ty, start) <= last_day()
}

/// The smallest period start of type `ty` that is at or after `day`, if there is one.
fn start_at_or_after(ty: i64, day: i64) -> Option<i64> {
    let day = day.max(FIRST_DAY);
    let f = floor(ty, day);
    let s = if f == day { f } else { end_of(ty, f) + 1 };
    fits(ty, s).then_some(s)
}

/// The largest period start of type `ty` that is at or before `day`, if there is one.
fn start_at_or_before(ty: i64, day: i64) -> Option<i64> {
    let mut s = floor(ty, day.min(last_day()));
    while s >= FIRST_DAY {
        if fits(ty, s) {
            return Some(s);
        }
        s = floor(ty, s - 1);
    }
    None
}

/// The next period start of type `ty` in `set` after (or, descending, before) `after`.
fn next_start(ty: i64, set: &[ranges::Iv], after: Option<i64>, ascending: bool) -> Option<i64> {
    let mut cur = after;
    loop {
        let n = ranges::step(set, cur, ascending)?;
        let s = if ascending {
            start_at_or_after(ty, n)
        } else {
            start_at_or_before(ty, n)
        }?;
        if set.iter().any(|&(a, b)| a <= s && s <= b) {
            return Some(s);
        }
        cur = Some(s);
    }
}

fn row(t: &TableDef, ty: i64, start: i64) -> Row {
    let end = end_of(ty, start);
    let (y, m, _) = civil(start);
    let (no, name): (i64, String) = match ty {
        0 => {
            let w = weekday(start);
            (w, WEEKDAYS[(w - 1) as usize].into())
        }
        1 => {
            // ISO 8601: the week that holds the year's first Thursday is week 1.
            let thursday = start + 3;
            let (ty_year, _, _) = civil(thursday);
            let week = (thursday - i64::from(day_from_civil(ty_year, 1, 1))) / 7 + 1;
            (week, week.to_string())
        }
        2 => (i64::from(m), MONTHS[(m - 1) as usize].into()),
        3 => {
            let q = i64::from((m - 1) / 3 + 1);
            (q, q.to_string())
        }
        _ => (i64::from(y), y.to_string()),
    };
    let mut r = blank(t);
    r[pos(t, PERIOD_TYPE)] = option(t, PERIOD_TYPE, ty as i32);
    r[pos(t, PERIOD_START)] = Value::Date(start as i32, false);
    // "Period End" is a closing date (TestDateVirtualTable.al:124-146).
    r[pos(t, PERIOD_END)] = Value::Date(end as i32, true);
    r[pos(t, PERIOD_NO)] = Value::Integer(no as i32);
    r[pos(t, PERIOD_NAME)] = Value::Text(name.clone().into());
    r[pos(t, PERIOD_INVARIANT_NAME)] = Value::Text(name.into());
    r
}

fn num(v: &Value) -> Option<i64> {
    match v {
        Value::Option(n, _) => Some(i64::from(*n)),
        Value::Integer(n) => Some(i64::from(*n)),
        Value::Date(n, _) => Some(i64::from(*n)),
        _ => None,
    }
}

struct Plan<'a> {
    types: ranges::Set,
    starts: ranges::Set,
    /// Filters on columns that are not part of the key.
    others: Vec<&'a FieldFilter>,
}

fn plan<'a>(t: &TableDef, filters: &'a [FieldFilter]) -> Result<Plan<'a>, StoreError> {
    let (p_ty, p_start) = (pos(t, PERIOD_TYPE), pos(t, PERIOD_START));
    let err = |e: String| refused(format!("Date: {e}"));
    Ok(Plan {
        types: ranges::of_field(filters, p_ty, &num, 0, 4).map_err(err)?,
        starts: ranges::of_field(filters, p_start, &num, FIRST_DAY, last_day()).map_err(err)?,
        others: filters
            .iter()
            .filter(|f| f.field != p_ty && f.field != p_start)
            .collect(),
    })
}

fn key_of(v: &Value) -> Option<i64> {
    num(v)
}

pub(super) fn get(t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
    let (Some(ty), Some(start)) = (pk.first().and_then(key_of), pk.get(1).and_then(key_of)) else {
        return Err(refused("Date: a key that is not (Period Type, Period Start)"));
    };
    if !(0..=4).contains(&ty) || !fits(ty, start) || floor(ty, start) != start {
        return Ok(None);
    }
    Ok(Some(row(t, ty, start)))
}

pub(super) fn next(t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
    let plan = plan(t, scan.filters)?;
    let (after_ty, after_start) = match scan.after {
        Some(a) => (a.first().and_then(key_of), a.get(1).and_then(key_of)),
        None => (None, None),
    };
    let mut examined = 0u32;
    // Period types in key order, from the one the scan stands on.
    let mut types: Vec<i64> = plan.types.iter().flat_map(|&(a, b)| a..=b).collect();
    if !scan.ascending {
        types.reverse();
    }
    for ty in types {
        let from = match after_ty {
            Some(at) if scan.ascending && ty < at => continue,
            Some(at) if !scan.ascending && ty > at => continue,
            Some(at) if at == ty => after_start,
            _ => None,
        };
        let mut cur = from;
        while let Some(s) = next_start(ty, &plan.starts, cur, scan.ascending) {
            let r = row(t, ty, s);
            if plan.others.iter().all(|f| f.expr.matches(&r[f.field])) {
                return Ok(Some(r));
            }
            examined += 1;
            if examined > SEARCH_LIMIT {
                return Err(refused(
                    "Date: a filter on a column that is not the key over too many periods",
                ));
            }
            cur = Some(s);
        }
    }
    Ok(None)
}

pub(super) fn count(t: &TableDef, filters: &[FieldFilter]) -> Result<usize, StoreError> {
    let plan = plan(t, filters)?;
    let mut n = 0usize;
    let mut examined = 0u32;
    for ty in plan.types.iter().flat_map(|&(a, b)| a..=b) {
        if ty == 0 && plan.others.is_empty() {
            // Every day of the range is a period.
            n += ranges::size(&plan.starts) as usize;
            continue;
        }
        let mut cur = None;
        while let Some(s) = next_start(ty, &plan.starts, cur, true) {
            examined += 1;
            if examined > SEARCH_LIMIT {
                return Err(refused("Date: counting over too many periods"));
            }
            if plan.others.is_empty() || {
                let r = row(t, ty, s);
                plan.others.iter().all(|f| f.expr.matches(&r[f.field]))
            } {
                n += 1;
            }
            cur = Some(s);
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> i64 {
        i64::from(day_from_civil(y, m, day))
    }

    #[test]
    fn the_first_period_of_each_type_is_the_first_that_fits_whole_in_year_one() {
        // TestDateVirtualTable.al:405-436
        let first = |ty| start_at_or_after(ty, 0);
        assert_eq!(first(0), Some(d(1, 1, 3)));
        assert_eq!(first(1), Some(d(1, 1, 8)));
        assert_eq!(first(2), Some(d(1, 2, 1)));
        assert_eq!(first(3), Some(d(1, 4, 1)));
        assert_eq!(first(4), Some(d(2, 1, 1)));
    }

    #[test]
    fn the_last_period_of_each_type_is_the_last_that_fits_whole_in_year_9999() {
        // TestDateVirtualTable.al:351-399
        let last = |ty| start_at_or_before(ty, last_day());
        assert_eq!(last(0), Some(d(9999, 12, 31)));
        assert_eq!(last(1), Some(d(9999, 12, 20)));
        assert_eq!(last(2), Some(d(9999, 12, 1)));
        assert_eq!(last(3), Some(d(9999, 10, 1)));
        assert_eq!(last(4), Some(d(9999, 1, 1)));
    }

    #[test]
    fn iso_week_numbers_and_names() {
        let t = test_table();
        let r = row(&t, 1, d(2026, 1, 19));
        assert!(matches!(r[pos(&t, PERIOD_NO)], Value::Integer(4)));
        assert!(matches!(&r[pos(&t, PERIOD_NAME)], Value::Text(n) if n == "4"));
        let r = row(&t, 0, d(2026, 1, 25));
        assert!(matches!(r[pos(&t, PERIOD_NO)], Value::Integer(7)));
        assert!(matches!(&r[pos(&t, PERIOD_NAME)], Value::Text(n) if n == "Sunday"));
        let r = row(&t, 2, d(2024, 2, 1));
        assert!(matches!(r[pos(&t, PERIOD_END)], Value::Date(e, true) if i64::from(e) == d(2024, 2, 29)));
    }

    fn test_table() -> TableDef {
        use al_builtins::value::RtType;
        let f = |id, ty| al_data::FieldDef {
            id,
            name: format!("F{id}"),
            ty,
            stored: true,
            code_storage: Default::default(),
        };
        TableDef {
            object: 0,
            name: "Date".into(),
            fields: vec![
                f(
                    1,
                    RtType::Option(
                        vec![
                            "Date".into(),
                            "Week".into(),
                            "Month".into(),
                            "Quarter".into(),
                            "Year".into(),
                        ]
                        .into(),
                    ),
                ),
                f(2, RtType::Date),
                f(3, RtType::Date),
                f(4, RtType::Integer),
                f(5, RtType::Text(Some(31))),
                f(6, RtType::Text(Some(31))),
            ],
            keys: vec![vec![0, 1]],
            per_company: false,
        }
    }
}
