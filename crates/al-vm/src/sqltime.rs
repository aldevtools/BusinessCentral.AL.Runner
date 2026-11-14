//! What the SQL `datetime` column behind a Date, Time or DateTime field does to the values of a database (non-temporary)
//! table: it counts 1/300 s, and it cannot hold every date (corpus record/filters/P130DateFilters.al).
//! A temporary table keeps the values as they are: whether it rounds too is not pinned, and the old behavior stays.

use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::format::day_from_civil;
use al_builtins::native::{AlError, Signal};
use al_builtins::value::{RtType, Value};
use al_data::TableDef;

const MS_PER_DAY: i64 = 86_400_000;

/// Milliseconds of a day rounded to the 1/300 s the column holds and read back as whole milliseconds, a half rounding up:
/// 1 -> 0, 2 -> 3, 5 -> 7, 8 -> 7, 9 -> 10, 998 -> 997, 999 -> 1000 (P130DateFilters.al:162-183). The result can be a whole day.
fn round_ms(ms: i64) -> i64 {
    let (whole, part) = (ms.div_euclid(1000), ms.rem_euclid(1000));
    let ticks = (part * 6 + 10) / 20;
    whole * 1000 + (ticks * 20 + 3) / 6
}

/// Whether the column holds the millisecond part `ms` (0..1000) of a time as it is: 0, 3, 7, 10, 13, 17, ...
pub(crate) fn holds_ms(ms: i64) -> bool {
    round_ms(ms) == ms
}

/// A Time as the column holds it. A time that rounds to midnight reads 00:00:00 (streams/P132ReportTimeMs.al:190,
/// :197, 23:59:59.999 shows 12:00:00 AM; :192, :199, 00:00:00.001 too).
fn round_time(t: i64) -> i64 {
    if t == 0 {
        return 0;
    }
    round_ms(t - 1).rem_euclid(MS_PER_DAY) + 1
}

/// A DateTime as the column holds it: the milliseconds of its day rounded, 23:59:59.999 going on to the next day
/// (P130DateFilters.al:226, `eq prev .999` finds the row at midnight).
fn round_datetime(v: i64) -> i64 {
    if v == 0 {
        return 0;
    }
    let total = v - 1;
    let (day, in_day) = (total.div_euclid(MS_PER_DAY), total.rem_euclid(MS_PER_DAY));
    day * MS_PER_DAY + round_ms(in_day) + 1
}

/// A field value rounded as the column holds it; other values as they are.
pub(crate) fn rounded(v: &Value) -> Value {
    match v {
        Value::Time(t) => Value::Time(round_time(*t)),
        Value::DateTime(d) => Value::DateTime(round_datetime(*d)),
        other => other.clone(),
    }
}

fn round_expr(e: &FilterExpr) -> FilterExpr {
    use FilterExpr as F;
    let r = |o: &Option<Value>| o.as_ref().map(rounded);
    match e {
        F::Range(a, b) => F::Range(r(a), r(b)),
        F::Eq(v) => F::Eq(rounded(v)),
        F::RangeSame(v) => F::RangeSame(rounded(v)),
        F::Ne(v) => F::Ne(rounded(v)),
        F::Lt(v) => F::Lt(rounded(v)),
        F::Le(v) => F::Le(rounded(v)),
        F::Gt(v) => F::Gt(rounded(v)),
        F::Ge(v) => F::Ge(rounded(v)),
        F::Or(es) => F::Or(es.iter().map(round_expr).collect()),
        F::And(es) => F::And(es.iter().map(round_expr).collect()),
        F::UnpinnedText(inner) => F::UnpinnedText(Box::new(round_expr(inner))),
        F::Like(..) | F::NotLike(..) | F::AtSignAlone(_) | F::EmptyViewValue | F::NumCode(_) | F::UnpinnedMatch(..) => {
            e.clone()
        }
    }
}

/// A primary key as the SQL columns of a database table hold it: its Time and DateTime values rounded. Get, Modify, Rename and
/// Delete of a database record find the row by the rounded key: `Get('A', 10:00:00.001)` finds the row stored as 10:00:00,
/// `Get('A', 10:00:00.002)` looks for 10:00:00.003, and Modify, Rename and Delete of a record that holds 10:00:00.001 or
/// 12:00:00.001 reach their row (record/filters/P130DateFilters.al:603-608, :651-658, :720-733).
pub(crate) fn rounded_key(pk: &[Value]) -> Vec<Value> {
    pk.iter().map(rounded).collect()
}

/// Whether the SQL column rounds a value of a primary key.
pub(crate) fn key_rounds(pk: &[Value]) -> bool {
    pk.iter()
        .any(|v| !al_builtins::ops::compare(&rounded(v), v).is_ok_and(|o| o.is_eq()))
}

/// The filters of a database table read with their Time and DateTime values rounded as the column rounds them; the record keeps
/// its own (GetFilters shows `11:59:59.999 PM`): P130DateFilters.al:206-227.
pub(crate) fn round_filters(t: &TableDef, filters: &mut [FieldFilter]) {
    for f in filters {
        if t.fields[f.field].id < 2_000_000_000 && matches!(t.fields[f.field].ty, RtType::Time | RtType::DateTime) {
            f.expr = round_expr(&f.expr);
        }
    }
}

/// What writing `row` into the columns of database table `t` gives.
pub(crate) enum Stored {
    /// Every value fits; the row now holds them as the columns do.
    Fits,
    /// A value the column cannot hold, with the error the platform raises for it.
    Bad(BadValue),
}

/// A value the column cannot hold: the platform's error, the field, and which kind of value it is (the write gates of
/// `records/sql_range.rs` take the kinds their cells measured).
pub(crate) struct BadValue {
    pub(crate) error: AlError,
    /// The field's position in the table.
    pub(crate) field: usize,
    pub(crate) kind: BadKind,
    /// What measured the Insert of this value.
    pub(crate) insert: InsertEvidence,
}

/// What measured the Insert of a value the column cannot hold (`records/sql_range.rs`, `insert_sql_range`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InsertEvidence {
    /// The corpus pins it: 1753-01-01, 1752-12-31 (also as a closing date), 1753-01-01 00:00, 1752-12-31 12:00 and a DateTime
    /// that rounds past 9999-12-31 (P130DateFilters.al:255-293, :306-318).
    Pinned,
    /// The reporter of al-runner issue 8006 measured it (assumption `sql-date-first-year`, statement not reported): another
    /// Date in 1753 that is not a closing date, a DateTime in 1753 whose 12-hour clock hour has two digits and that stays in
    /// its minute.
    Reporter,
    /// Only corpus PR 1398 measured it: a DateTime in 1753 whose 12-hour clock hour has one digit and that stays in its
    /// minute, by the Insert statement into `LT SD Floor Row`, `Trig` with Insert(true), `Sub` and `Blob`
    /// (SqlDateFloor1753.al Insert_DateTime_MorningHours_1, Trigger_DateTime_1, Subscriber_DateTime_1, Blob_DateTime_1).
    ProbeHour,
    /// Only corpus PR 1398 measured it, by the Insert statement into a table without insert code (`LT SD Floor Row`:
    /// Insert_Date_1753_1, Insert_DateTime_RoundingEdge_1): a closing date in 1753, 1753-12-31 23:59:59.999.
    ProbePlain,
}

/// The kinds of value the column cannot hold that a write gate names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BadKind {
    /// A Date in 1753 that is not a closing date.
    Date1753,
    /// A closing date in 1753.
    ClosingDate1753,
    /// The Date 1752-12-31.
    Date1752,
    /// The closing date of 1752-12-31.
    ClosingDate1752,
    /// A DateTime on a day in 1753.
    DateTime1753,
    /// The DateTime 1752-12-31 12:00.
    DateTime1752,
    /// A DateTime that rounds past 9999-12-31.
    PastMax,
}

/// How a Date or DateTime the column cannot hold shows in the error, and its kind.
struct Shown {
    value: String,
    kind: BadKind,
    insert: InsertEvidence,
}

/// The days of 1752-12-31, 1753-01-01 and 1754-01-01.
fn edge_days() -> (i64, i64, i64) {
    (
        i64::from(day_from_civil(1752, 12, 31)),
        i64::from(day_from_civil(1753, 1, 1)),
        i64::from(day_from_civil(1754, 1, 1)),
    )
}

/// A Date before 1754-01-01 the column cannot hold, or `None` when it fits. Pinned: 1753-01-01 and 1752-12-31 (also as a
/// closing date) fail, 0D and 0001-01-01 store (P130DateFilters.al:255-293; 0001-01-01 reads back as it was written, :261-262,
/// which a blank date would not: types/date-time/Arithmetic.al:111-116). Assumed (`sql-date-first-year`, corpus PR 1398
/// SqlDateFloor1753.al Insert_Date_1753_1, Insert_Date_1753_2): every other date in 1753 fails, a closing date too, shown as
/// MM/DD/YYYY with a `C` before a closing date (`C06/15/1753`). Every other date before 1754-01-01 is refused.
fn bad_date(d: i32, closing: bool) -> Result<Option<Shown>, Signal> {
    let (d1752, d1753, d1754) = edge_days();
    let day = i64::from(d);
    if day >= d1754 || (d <= 1 && !closing) {
        return Ok(None);
    }
    let (y, m, dd) = al_builtins::format::civil_from_day(d);
    let shown = |kind, insert| Shown {
        value: format!("{}{m:02}/{dd:02}/{y:04}", if closing { "C" } else { "" }),
        kind,
        insert,
    };
    Ok(Some(match (day, closing) {
        (d, false) if d == d1753 => shown(BadKind::Date1753, InsertEvidence::Pinned),
        (d, false) if d == d1752 => shown(BadKind::Date1752, InsertEvidence::Pinned),
        (d, true) if d == d1752 => shown(BadKind::ClosingDate1752, InsertEvidence::Pinned),
        (d, false) if d > d1753 && d < d1754 => shown(BadKind::Date1753, InsertEvidence::Reporter),
        (d, true) if d >= d1753 && d < d1754 => shown(BadKind::ClosingDate1753, InsertEvidence::ProbePlain),
        _ => {
            return Err(Signal::unsupported(
                "a Date before 1754-01-01 in a database table, other than the measured ones (not pinned)",
            ));
        }
    }))
}

/// A DateTime the column cannot hold, or `None` when it fits. Pinned: 1753-01-01 00:00 and 1752-12-31 12:00 fail, and a value
/// that rounds past 9999-12-31, which shows to the minute (P130DateFilters.al:285-293, :315). Assumed (`sql-date-first-year`,
/// corpus PR 1398 SqlDateFloor1753.al Insert_DateTime_MorningHours_1 and _2, Insert_DateTime_AfternoonHours_1 and _2,
/// Insert_DateTime_RoundingEdge_1): every other value on a day in 1753 fails and shows as MM/DD/YYYY hh:mm with AM or PM,
/// the hour of the 12-hour clock in two digits and the time cut to the minute (00:00:01 `12:00 AM`, 01:00 `01:00 AM`, 13:00
/// `01:00 PM`, 21:59:59 `09:59 PM`). 1753-12-31 23:59:59.999, which the column would round into 1754, fails too and shows
/// `12/31/1753 11:59 PM`. Another value the rounding moves into another minute is refused, as is every other value before
/// 1754-01-01.
fn bad_datetime(v: i64) -> Result<Option<Shown>, Signal> {
    let (d1752, d1753, d1754) = edge_days();
    let (day, in_day) = ((v - 1).div_euclid(MS_PER_DAY) + 1, (v - 1).rem_euclid(MS_PER_DAY));
    let r = round_datetime(v);
    if (r - 1).div_euclid(MS_PER_DAY) + 1 > i64::from(day_from_civil(9999, 12, 31)) {
        return Ok(Some(Shown {
            value: "12/31/9999 11:59 PM".into(),
            kind: BadKind::PastMax,
            insert: InsertEvidence::Pinned,
        }));
    }
    if day >= d1754 {
        return Ok(None);
    }
    let refuse = || {
        Signal::unsupported(
            "a DateTime before 1754-01-01 in a database table, other than the measured ones (not pinned)",
        )
    };
    match (day, in_day) {
        (d, 43_200_000) if d == d1752 => {
            return Ok(Some(Shown {
                value: "12/31/1752 12:00 PM".into(),
                kind: BadKind::DateTime1752,
                insert: InsertEvidence::Pinned,
            }));
        }
        (d, _) if d < d1753 => return Err(refuse()),
        _ => {}
    }
    let same_minute = (r - 1).div_euclid(60_000) == (v - 1).div_euclid(60_000);
    let last_millisecond_of_1753 = day == d1754 - 1 && in_day == MS_PER_DAY - 1;
    if !(same_minute || last_millisecond_of_1753) {
        return Err(refuse());
    }
    let (hour, minute) = (in_day / 3_600_000, in_day / 60_000 % 60);
    let hour12 = if hour % 12 == 0 { 12 } else { hour % 12 };
    let (y, m, dd) = al_builtins::format::civil_from_day(day as i32);
    let half = if hour < 12 { "AM" } else { "PM" };
    let insert = match (day == d1753 && in_day == 0, same_minute, matches!(hour12, 10..=12)) {
        (true, _, _) => InsertEvidence::Pinned,
        (false, true, true) => InsertEvidence::Reporter,
        (false, true, false) => InsertEvidence::ProbeHour,
        (false, false, _) => InsertEvidence::ProbePlain,
    };
    Ok(Some(Shown {
        value: format!("{m:02}/{dd:02}/{y:04} {hour12:02}:{minute:02} {half}"),
        kind: BadKind::DateTime1753,
        insert,
    }))
}

/// Field `i` of database table `t` holding `v`: `Some` when its column cannot hold the value (`bad_date`, `bad_datetime`).
/// A field that is not stored and the system fields (SystemCreatedAt, SystemModifiedAt: the platform's stamps) are not
/// checked.
pub(crate) fn bad_field(
    t: &TableDef,
    i: usize,
    v: &Value,
    field_caption: &dyn Fn(usize) -> String,
    table_caption: &str,
) -> Result<Option<BadValue>, Signal> {
    let f = &t.fields[i];
    if !f.stored || f.id >= 2_000_000_000 {
        return Ok(None);
    }
    let shown = match (&f.ty, v) {
        (RtType::Date, Value::Date(d, closing)) => bad_date(*d, *closing)?,
        (RtType::DateTime, Value::DateTime(v)) if *v != 0 => bad_datetime(*v)?,
        _ => None,
    };
    let Some(Shown { value, kind, insert }) = shown else {
        return Ok(None);
    };
    // The text names the field and the table; whether by name or caption is not pinned (the corpus tables have neither).
    if field_caption(i) != f.name || table_caption != t.name {
        return Err(Signal::Unsupported(
            "the SQL range error of a field or table with a caption (whether it names the caption is not pinned)"
                .into(),
        ));
    }
    let error = AlError {
        message: format!(
            "Cannot write the value {value} to the field {} in the table {} because the value is either too long or the content is invalid.",
            f.name, t.name
        ),
        code: "NDBCS:BadCSideData".into(),
        call_stack: String::new(),
        ..Default::default()
    };
    Ok(Some(BadValue {
        error,
        field: i,
        kind,
        insert,
    }))
}

/// Rounds the Time and DateTime values of `row` and checks its dates against what the column holds (`bad_field`). A row with
/// two such values reports the first in field order (not measured).
pub(crate) fn store(
    t: &TableDef,
    field_caption: &dyn Fn(usize) -> String,
    table_caption: &str,
    row: &mut [Value],
) -> Result<Stored, Signal> {
    for (i, (f, v)) in t.fields.iter().zip(row.iter_mut()).enumerate() {
        if let Some(bad) = bad_field(t, i, v, field_caption, table_caption)? {
            return Ok(Stored::Bad(bad));
        }
        // A primary key field the rounding changes is stored rounded like any other field: the row stands under the
        // rounded key while the record keeps its own value (P130DateFilters.al:527-735, al-runner issue 5689). The system
        // fields keep the value the runtime stamps: whether the buffer then holds the rounded value is not pinned.
        if f.stored && f.id < 2_000_000_000 && matches!(f.ty, RtType::Time | RtType::DateTime) {
            *v = rounded(v);
        }
    }
    Ok(Stored::Fits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milliseconds_round_to_three_hundredths() {
        let got: Vec<i64> = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 998, 999, 1001].map(round_ms).to_vec();
        assert_eq!(got, vec![0, 0, 3, 3, 3, 7, 7, 7, 7, 10, 997, 1000, 1000]);
    }
}
