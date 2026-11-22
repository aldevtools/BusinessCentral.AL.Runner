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

/// What the column of a [`keeps_past_midnight`] field holds for 23:59:59.999: midnight of the next day, after every time of the
/// day (record/filters/TimeRolledPastMidnight.al:143-158). No AL Time reaches it (Time arithmetic wraps at midnight), so a value
/// of it in a row is always this rollover.
const ROLLED: i64 = MS_PER_DAY + 1;

/// Whether field `i` of database table `t` has the shape record/filters/TimeRolledPastMidnight.al measures (table 75011, :11-25):
/// a Normal Time field the table itself declares (:17), outside the primary key (:22), with a secondary key of its own that the
/// table itself declares with no properties (:23).
/// Its column keeps 23:59:59.999 past the day (`ROLLED`). Every other field rounds as [`rounded`] does; a Time primary key
/// field among them (:305-346, `eq0` and `rows` still differ from BC).
pub(crate) fn keeps_past_midnight(t: &TableDef, schema: &al_ir::Table, i: usize) -> bool {
    let f = &t.fields[i];
    f.ty == RtType::Time
        && f.stored
        && f.id < 2_000_000_000
        && schema.fields.get(i).is_some_and(|s| s.declared_in == Some(t.object))
        && !t.keys[0].contains(&i)
        && schema.keys.iter().enumerate().skip(1).any(|(k, key)| {
            key.as_slice() == [i]
                && schema.key_declared_in.get(k) == Some(&Some(t.object))
                && schema.key_props.get(k).is_some_and(Vec::is_empty)
        })
}

/// Whether `t` (a Time, not 0) rounds to a whole day in the column.
fn rolls(t: i64) -> bool {
    t != 0 && round_ms(t - 1) == MS_PER_DAY
}

/// A filter value of a [`keeps_past_midnight`] field rounded as the column rounds it: 23:59:59.999 compares after every time
/// of the day, as the stored value does (`ROLLED`; TimeRolledPastMidnight.al:113-158, `lt999`, `eq999`, `gt997`; FieldRef
/// SetRange :369-376).
fn rounded_filter(v: &Value) -> Value {
    match v {
        Value::Time(t) if rolls(*t) => Value::Time(ROLLED),
        other => rounded(other),
    }
}

/// Whether `v` is a Time a database column holds past the day (`ROLLED`).
pub(crate) fn is_rolled(v: &Value) -> bool {
    matches!(v, Value::Time(ROLLED))
}

/// The values of a database row as AL reads them: a Time held past the day reads 00:00:00 (TimeRolledPastMidnight.al:143
/// `read`, :257 `set_read`, :371-375 FlowField max, min and lookup, FieldRef).
pub(crate) fn read_back(row: &mut [Value]) {
    for v in row {
        if is_rolled(v) {
            *v = Value::Time(1);
        }
    }
}

/// [`read_back`] of one value.
pub(crate) fn read_back_value(mut v: Value) -> Value {
    read_back(std::slice::from_mut(&mut v));
    v
}

/// A Modify writes only what changed: after the record read the row (`read`: the variable's own Get or ModifyAll's read,
/// `records/time_rollover.rs`), a field that still holds the 00:00:00 it read from a Time held past the day keeps it
/// (TimeRolledPastMidnight.al:252 `note_eq999`, :254 `midnight_eq999` after `Tm := 000000T`, :260 `modifyallnote_eq999`).
/// A record that did not read the row writing 00:00:00 there is not measured and refused. `old` is the stored row, `new` the row
/// about to be written.
pub(crate) fn keep_rolled(old: &[Value], new: &mut [Value], read: bool) -> Result<(), Signal> {
    for (o, n) in old.iter().zip(new) {
        if is_rolled(o) && matches!(n, Value::Time(1)) {
            if !read {
                return Err(Signal::unsupported(
                    "a Modify of a row that holds a Time stored as 23:59:59.999 by a record that did not read it (not measured)",
                ));
            }
            *n = o.clone();
        }
    }
    Ok(())
}

fn round_expr(e: &FilterExpr, rolls_over: bool) -> FilterExpr {
    use FilterExpr as F;
    let one = |v: &Value| if rolls_over { rounded_filter(v) } else { rounded(v) };
    let r = |o: &Option<Value>| o.as_ref().map(one);
    match e {
        F::Range(a, b) => F::Range(r(a), r(b)),
        F::Eq(v) => F::Eq(one(v)),
        F::RangeSame(v) => F::RangeSame(one(v)),
        F::Ne(v) => F::Ne(one(v)),
        F::Lt(v) => F::Lt(one(v)),
        F::Le(v) => F::Le(one(v)),
        F::Gt(v) => F::Gt(one(v)),
        F::Ge(v) => F::Ge(one(v)),
        F::Or(es) => F::Or(es.iter().map(|e| round_expr(e, rolls_over)).collect()),
        F::And(es) => F::And(es.iter().map(|e| round_expr(e, rolls_over)).collect()),
        F::UnpinnedText(inner) => F::UnpinnedText(Box::new(round_expr(inner, rolls_over))),
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
/// its own (GetFilters shows `11:59:59.999 PM`): P130DateFilters.al:206-227. A [`keeps_past_midnight`] field compares
/// 23:59:59.999 after the day ([`rounded_filter`]).
pub(crate) fn round_filters(t: &TableDef, schema: &al_ir::Table, filters: &mut [FieldFilter]) {
    for f in filters {
        if t.fields[f.field].id < 2_000_000_000 && matches!(t.fields[f.field].ty, RtType::Time | RtType::DateTime) {
            f.expr = round_expr(&f.expr, keeps_past_midnight(t, schema, f.field));
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
    /// The reporter of al-runner issue 8006 measured it (statement not reported): another
    /// Date in 1753 that is not a closing date, a DateTime in 1753 whose 12-hour clock hour has two digits and that stays in
    /// its minute.
    Reporter,
    /// Only record/filters/SqlDateFloor1753.al measured it: a DateTime in 1753 whose 12-hour clock hour has one digit and that stays in its
    /// minute, by the Insert statement into `LT SD Floor Row`, `Trig` with Insert(true), `Sub` and `Blob`
    /// (SqlDateFloor1753.al Insert_DateTime_MorningHours_1, Trigger_DateTime_1, Subscriber_DateTime_1, Blob_DateTime_1).
    ProbeHour,
    /// Only record/filters/SqlDateFloor1753.al measured it, by the Insert statement into a table without insert code (`LT SD Floor Row`:
    /// Insert_Date_1753_1, Insert_DateTime_RoundingEdge_1): a closing date in 1753, 1753-12-31 23:59:59.999.
    ProbePlain,
    /// Measured only by the Insert() statement into a table without insert code (`LT SD Floor Row`:
    /// record/filters/SqlDateFloor1753.al:1584-1633, Outside_Date_Insert, Outside_DateTime_Insert): the Date 1700-01-01,
    /// the DateTimes 1700-01-01 12:00 and 1752-12-31 23:59.
    ProbeOutside,
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
    /// The Date 1700-01-01.
    DatePre1753,
    /// The DateTime 1700-01-01 12:00 or 1752-12-31 23:59.
    DateTimePre1753,
    /// A DateTime that rounds past 9999-12-31.
    PastMax,
}

/// How a Date or DateTime the column cannot hold shows in the error, and its kind.
struct Shown {
    value: String,
    kind: BadKind,
    insert: InsertEvidence,
}

/// The days of 1700-01-01, 1752-12-31, 1753-01-01 and 1754-01-01.
fn edge_days() -> (i64, i64, i64, i64) {
    (
        i64::from(day_from_civil(1700, 1, 1)),
        i64::from(day_from_civil(1752, 12, 31)),
        i64::from(day_from_civil(1753, 1, 1)),
        i64::from(day_from_civil(1754, 1, 1)),
    )
}

/// A Date before 1754-01-01 the column cannot hold, or `None` when it fits. Pinned: 1753-01-01 and 1752-12-31 (also as a
/// closing date) fail, 0D and 0001-01-01 store (P130DateFilters.al:255-293; 0001-01-01 reads back as it was written, :261-262,
/// which a blank date would not: types/date-time/Arithmetic.al:111-116). Every other date in 1753 fails, a closing date
/// too, shown as MM/DD/YYYY with a `C` before a closing date (`C06/15/1753`): record/filters/SqlDateFloor1753.al:1274
/// (Insert_Date_1753_1, _2), :1342-1400 (Modify_Date_ClosingAndOutside_1 to _5), and the reporter of al-runner issue 8006.
/// 1700-01-01 fails as well, shown the same way (`01/01/1700`): SqlDateFloor1753.al:1584-1615 (Outside_Date_Insert,
/// Outside_Date_Modify). The days between 1700-01-01 and 1752-12-31 are not measured, and every other date before
/// 1754-01-01 is refused.
fn bad_date(d: i32, closing: bool) -> Result<Option<Shown>, Signal> {
    let (d1700, d1752, d1753, d1754) = edge_days();
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
        (d, false) if d == d1700 => shown(BadKind::DatePre1753, InsertEvidence::ProbeOutside),
        _ => {
            return Err(Signal::unsupported(
                "a Date before 1754-01-01 in a database table, other than the measured ones (not pinned)",
            ));
        }
    }))
}

/// A DateTime the column cannot hold, or `None` when it fits. Pinned: 1753-01-01 00:00 and 1752-12-31 12:00 fail, and a value
/// that rounds past 9999-12-31, which shows to the minute (P130DateFilters.al:285-293, :315). Pinned by
/// record/filters/SqlDateFloor1753.al (Insert_DateTime_MorningHours_1 and _2, Insert_DateTime_AfternoonHours_1 and _2,
/// Insert_DateTime_RoundingEdge_1) and the reporter of al-runner issue 8006: every other value on a day in 1753 fails and shows as MM/DD/YYYY hh:mm with AM or PM,
/// the hour of the 12-hour clock in two digits and the time cut to the minute (00:00:01 `12:00 AM`, 01:00 `01:00 AM`, 13:00
/// `01:00 PM`, 21:59:59 `09:59 PM`). 1753-12-31 23:59:59.999, which the column would round into 1754, fails too and shows
/// `12/31/1753 11:59 PM`. 1700-01-01 12:00 and 1752-12-31 23:59 fail as well and show the same way (`01/01/1700 12:00 PM`;
/// Outside_DateTime_Insert, Outside_DateTime_Modify). Another value the rounding moves into another minute is refused, as is
/// every other value before 1754-01-01.
fn bad_datetime(v: i64) -> Result<Option<Shown>, Signal> {
    let (d1700, d1752, d1753, d1754) = edge_days();
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
        (d, _) if d < d1700 => return Err(refuse()),
        _ => {}
    }
    let same_minute = (r - 1).div_euclid(60_000) == (v - 1).div_euclid(60_000);
    let last_millisecond_of_1753 = day == d1754 - 1 && in_day == MS_PER_DAY - 1;
    if !(same_minute || last_millisecond_of_1753) {
        return Err(refuse());
    }
    let in_1753 = day >= d1753;
    // Before 1753 only the measured values: 1700-01-01 12:00 and 1752-12-31 23:59 (SqlDateFloor1753.al:1620-1647,
    // Outside_DateTime_Insert, Outside_DateTime_Modify). The minutes between are not measured.
    let measured_before_1753 = (day == d1700 && in_day == 43_200_000) || (day == d1752 && in_day == 86_340_000);
    if !(in_1753 || measured_before_1753) {
        return Err(refuse());
    }
    let (hour, minute) = (in_day / 3_600_000, in_day / 60_000 % 60);
    let hour12 = if hour % 12 == 0 { 12 } else { hour % 12 };
    let (y, m, dd) = al_builtins::format::civil_from_day(day as i32);
    let half = if hour < 12 { "AM" } else { "PM" };
    let (kind, insert) = match (
        in_1753,
        day == d1753 && in_day == 0,
        same_minute,
        matches!(hour12, 10..=12),
    ) {
        (false, _, _, _) => (BadKind::DateTimePre1753, InsertEvidence::ProbeOutside),
        (true, true, _, _) => (BadKind::DateTime1753, InsertEvidence::Pinned),
        (true, false, true, true) => (BadKind::DateTime1753, InsertEvidence::Reporter),
        (true, false, true, false) => (BadKind::DateTime1753, InsertEvidence::ProbeHour),
        (true, false, false, _) => (BadKind::DateTime1753, InsertEvidence::ProbePlain),
    };
    Ok(Some(Shown {
        value: format!("{m:02}/{dd:02}/{y:04} {hour12:02}:{minute:02} {half}"),
        kind,
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

/// Rounds the Time and DateTime values of `row` and checks its dates against what the column holds (`bad_field`). The cells
/// of record/filters/SqlDateFloor1753.al write one such value per row; a row with two is refused (which one the error names
/// is not measured).
pub(crate) fn store(
    t: &TableDef,
    schema: &al_ir::Table,
    field_caption: &dyn Fn(usize) -> String,
    table_caption: &str,
    row: &mut [Value],
) -> Result<Stored, Signal> {
    for (i, f) in t.fields.iter().enumerate() {
        let Some(v) = row.get(i) else { break };
        if let Some(bad) = bad_field(t, i, v, field_caption, table_caption)? {
            for (j, w) in row
                .iter()
                .enumerate()
                .skip(i + 1)
                .take(t.fields.len().saturating_sub(i + 1))
            {
                if bad_field(t, j, w, field_caption, table_caption)?.is_some() {
                    return Err(Signal::unsupported(
                        "a row with two values their SQL columns cannot hold (which one the error names is not measured)",
                    ));
                }
            }
            return Ok(Stored::Bad(bad));
        }
        // A primary key field the rounding changes is stored rounded like any other field: the row stands under the
        // rounded key while the record keeps its own value (P130DateFilters.al:527-735, al-runner issue 5689). The system
        // fields keep the value the runtime stamps: whether the buffer then holds the rounded value is not pinned.
        if f.stored && f.id < 2_000_000_000 && matches!(f.ty, RtType::Time | RtType::DateTime) {
            // The pinned field shape keeps the rollover after the day (TimeRolledPastMidnight.al:143-158).
            if matches!(&row[i], Value::Time(v) if rolls(*v)) && keeps_past_midnight(t, schema, i) {
                row[i] = Value::Time(ROLLED);
            } else {
                row[i] = rounded(&row[i]);
            }
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
