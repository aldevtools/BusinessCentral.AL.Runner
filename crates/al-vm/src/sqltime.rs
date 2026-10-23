//! What the SQL `datetime` column behind a Date, Time or DateTime field does to the values of a database (non-temporary)
//! table: it counts 1/300 s, and it cannot hold every date (corpus errors/lt-ms-refusals-130/P130DateFilters.al).
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

/// A Time as the column holds it. A time that rounds to midnight reads 00:00:00 (errors/lt-ms-refusals-132/P132ReportTimeMs.al:190,
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
        F::Like(..) | F::NotLike(..) => e.clone(),
    }
}

/// A primary key as the SQL columns of a database table hold it: its Time and DateTime values rounded. Get, Modify, Rename and
/// Delete of a database record find the row by the rounded key: `Get('A', 10:00:00.001)` finds the row stored as 10:00:00,
/// `Get('A', 10:00:00.002)` looks for 10:00:00.003, and Modify, Rename and Delete of a record that holds 10:00:00.001 or
/// 12:00:00.001 reach their row (errors/lt-ms-refusals-130/P130DateFilters.al:603-608, :651-658, :720-733).
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
    Bad(AlError),
}

/// Rounds the Time and DateTime values of `row` and checks its dates against what the column holds. Only the values the corpus
/// pins raise the platform's error (P130DateFilters.al:255-293, :306-318): the Dates 1753-01-01 and 1752-12-31 (also as a
/// closing date), the DateTimes 1753-01-01 00:00 and 1752-12-31 12:00, and a DateTime that rounds past 9999-12-31. Any other
/// value before 1753-01-02 is refused. 0001-01-01 reads back as it was written (:261-262: Format gives '' and it equals
/// the value written, which a blank date would not: types/lt-datetime/Arithmetic.al:111-116).
pub(crate) fn store(
    t: &TableDef,
    field_caption: &dyn Fn(usize) -> String,
    table_caption: &str,
    row: &mut [Value],
) -> Result<Stored, Signal> {
    let first_good = i64::from(day_from_civil(1753, 1, 2));
    let (d1752, d1753) = (day_from_civil(1752, 12, 31), day_from_civil(1753, 1, 1));
    for (i, f) in t.fields.iter().enumerate() {
        // The system fields (SystemCreatedAt, SystemModifiedAt) are the platform's stamps: whether the buffer then holds the
        // rounded value is not pinned, and they keep the value the runtime stamps.
        if !f.stored || f.id >= 2_000_000_000 {
            continue;
        }
        // A primary key field the rounding changes is stored rounded like any other field: the row stands under the
        // rounded key while the record keeps its own value (P130DateFilters.al:527-735, al-runner issue 5689).
        let bad = match (&f.ty, &row[i]) {
            (RtType::Date, Value::Date(0, false)) => None,
            (RtType::Date, Value::Date(1, false)) => None,
            (RtType::Date, Value::Date(d, closing)) if i64::from(*d) < first_good => {
                let (y, m, dd) = al_builtins::format::civil_from_day(*d);
                match (*d, *closing) {
                    (d, false) if d == d1753 || d == d1752 => Some(format!("{m:02}/{dd:02}/{y:04}")),
                    (d, true) if d == d1752 => Some(format!("C{m:02}/{dd:02}/{y:04}")),
                    _ => {
                        return Err(Signal::Unsupported(
                            "a Date before 1753-01-02 in a database table, other than the pinned ones (not pinned)"
                                .into(),
                        ));
                    }
                }
            }
            (RtType::Time, Value::Time(_)) => {
                row[i] = rounded(&row[i]);
                None
            }
            (RtType::DateTime, Value::DateTime(v)) if *v != 0 => {
                let total = v - 1;
                let (day, in_day) = (total.div_euclid(MS_PER_DAY) + 1, total.rem_euclid(MS_PER_DAY));
                let r = round_datetime(*v);
                if (r - 1).div_euclid(MS_PER_DAY) + 1 > i64::from(day_from_civil(9999, 12, 31)) {
                    // 9999-12-31 23:59:59.999 shows to the minute (P130DateFilters.al:315).
                    Some("12/31/9999 11:59 PM".to_string())
                } else if day < first_good {
                    match (day, in_day) {
                        (d, 0) if d == i64::from(d1753) => Some("01/01/1753 12:00 AM".to_string()),
                        (d, 43_200_000) if d == i64::from(d1752) => Some("12/31/1752 12:00 PM".to_string()),
                        _ => {
                            return Err(Signal::Unsupported(
                                "a DateTime before 1753-01-02 in a database table, other than the pinned ones (not pinned)".into(),
                            ));
                        }
                    }
                } else {
                    row[i] = Value::DateTime(r);
                    None
                }
            }
            _ => None,
        };
        if let Some(value) = bad {
            // The text names the field and the table; whether by name or caption is not pinned (the corpus table has neither).
            if field_caption(i) != f.name || table_caption != t.name {
                return Err(Signal::Unsupported(
                    "the SQL range error of a field or table with a caption (whether it names the caption is not pinned)".into(),
                ));
            }
            return Ok(Stored::Bad(AlError {
                message: format!(
                    "Cannot write the value {value} to the field {} in the table {} because the value is either too long or the content is invalid.",
                    f.name, t.name
                ),
                code: "NDBCS:BadCSideData".into(),
                call_stack: String::new(),
                ..Default::default()
            }));
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
