//! Date, Time and DateTime builtins: `DMY2Date`, `CreateDateTime`, `Date2DMY`, `CalcDate`, ...
//!
//! Representations follow `Value`: a Date is a day number (1 = 0001-01-01) and a closing flag, a
//! Time is milliseconds since midnight plus one, a DateTime is milliseconds since 0001-01-01 UTC
//! plus one (0 is the undefined value of each). The session time zone is taken to be UTC, as in
//! `format.rs` (docs/probes-needed.md).
//!
//! Every function accepts the forms a corpus test pins and refuses the rest with
//! `Signal::Unsupported`, never a guess.

use crate::describe::Describe;
use crate::format::{civil_from_day, day_from_civil};
use crate::native::{Ctx, NativeFn, Signal, raise, unsupported};
use crate::value::Value;

mod brackets;
mod numbers;
mod range;

pub const REGISTRY: &[(&str, NativeFn)] = &[
    ("System.DMY2Date#1", dmy2date),
    ("System.CreateDateTime#1", create_datetime),
    ("System.DT2Date#1", dt2date),
    ("System.DT2Time#1", dt2time),
    ("DateTime.Date#1", datetime_date),
    ("DateTime.Time#1", datetime_time),
    ("System.DaTi2Variant#1", create_datetime),
    ("System.Date2DMY#1", date2dmy),
    ("System.Date2DWY#1", date2dwy),
    ("System.DWY2Date#1", dwy2date),
    ("System.NormalDate#1", normal_date),
    ("System.ClosingDate#1", closing_date),
    ("System.RoundDateTime#1", round_datetime),
    ("System.CalcDate#1", calcdate_formula),
    ("System.CalcDate#2", calcdate_text),
    ("Date.DayOfWeek#1", day_of_week),
    ("Date.WeekNo#1", week_no),
    ("Date.Day#1", |_, r, _| date_part(r, "Date.Day", |(_, _, d)| d as i32)),
    ("Date.Month#1", |_, r, _| {
        date_part(r, "Date.Month", |(_, m, _)| m as i32)
    }),
    ("Date.Year#1", |_, r, _| date_part(r, "Date.Year", |(y, _, _)| y)),
    ("Time.Hour#1", |_, r, _| time_part(r, 3_600_000, 24)),
    ("Time.Minute#1", |_, r, _| time_part(r, 60_000, 60)),
    ("Time.Second#1", |_, r, _| time_part(r, 1000, 60)),
    ("Time.Millisecond#1", |_, r, _| time_part(r, 1, 1000)),
];

type Out = Result<Value, Signal>;

const MS_PER_DAY: i64 = 86_400_000;
const LAST_DAY: i32 = 3_652_059; // 9999-12-31

pub(crate) fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

/// A whole number of any kind DMY2Date takes: an Integer, and a Byte, BigInteger, Decimal, Option or Char that holds the same
/// number (format-evaluate/Y71EvaluateDashedDates.al:150-228, all give 2026-03-01 for 3). A Decimal with a
/// fraction and a BigInteger beyond Integer are no number it takes ([`int_overflow`]).
fn int_of(v: Option<&Value>) -> Option<i32> {
    use rust_decimal::prelude::ToPrimitive;
    match v? {
        Value::Integer(n) | Value::Option(n, _) => Some(*n),
        Value::BigInteger(n) => i32::try_from(*n).ok(),
        Value::Decimal(d) if d.fract().is_zero() => d.to_i32(),
        Value::Char(c) => Some(*c as i32),
        Value::Byte(b) => Some(i32::from(*b)),
        _ => None,
    }
}

/// The ConversionCast a Decimal with a fraction or a BigInteger beyond Integer raises as a DMY2Date argument, before any range
/// check (types/Y81DMY2Date.al:183-215 Y81_DMY2Date_DecimalArguments, _BigIntegerArguments; needs corpus bump).
fn int_overflow(v: Option<&Value>) -> Option<String> {
    let (shown, ty) = match v? {
        Value::Decimal(d) if !d.fract().is_zero() => {
            (d.normalize().to_string(), "Microsoft.Dynamics.Nav.Runtime.Decimal18")
        }
        Value::BigInteger(n) if i32::try_from(*n).is_err() => (n.to_string(), "System.Int64"),
        _ => return None,
    };
    Some(format!(
        "Overflow under conversion of {ty} value {shown} to System.Int32."
    ))
}

/// A defined, non-closing date's day number.
fn plain_date(v: &Value, what: &str) -> Result<i32, Signal> {
    match v {
        Value::Date(n, false) if *n > 0 => Ok(*n),
        other => unsupported(format!(
            "{what} of {other}: only a defined, non-closing Date is pinned",
            other = other.describe()
        )),
    }
}

/// `DMY2Date(Day [, Month [, Year]])` (recordref-fieldref/fieldref/TestFieldRefValue.al:122, record/TestDateVirtualTableFlowField.al:65,83).
/// A missing month and year come from Today, never from the work date, and then bound the day like given ones
/// (types/P133DateEvaluateFormat.al:104-218); without a host clock the short forms are refused.
/// One part outside the calendar raises the range error that names it (types/date-time/DateFuncs.al:243-302,
/// DMY2Date_Invalid): the day against the length of its month (30 February 2024: 1 to 29), the month against 1 to 12, the year
/// against 1 to 9999. With several parts out of range the month is named first, then the year, then the day. Year 0 is
/// valid only as (1, 1, 0).
fn dmy2date(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    if let Some(message) = a.iter().take(3).find_map(|v| int_overflow(Some(v))) {
        return raise(message, "ConversionCast");
    }
    let today = || ctx.today_day().map(civil_from_day);
    let given = (int_of(a.first()), int_of(a.get(1)), int_of(a.get(2)));
    let (d, m, y) = match (a.len(), given, today()) {
        (3, (Some(d), Some(m), Some(y)), _) => (d, m, y),
        (2, (Some(d), Some(m), None), Some((ty, _, _))) => (d, m, ty),
        (1, (Some(d), None, None), Some((ty, tm, _))) => (d, tm as i32, ty),
        _ => {
            return unsupported(
                "DMY2Date with these arguments, or a short form without a clock (not pinned by the corpus)",
            );
        }
    };
    // GetLastErrorText(true) hides the value and keeps the range, for the three-argument form: parameter 2
    // (errors/last-error/LastErrorFlagRuntimeCodes.al:236, :245), parameters 1 and 3 (DateAndConversionSites_ExcludeFlag
    // `dmy2dateday_true`, `dmy2dateyear_true`; corpus pull request 1327). Short forms are not measured.
    let three = a.len() == 3;
    let range = |n: u8, v: i32, hi: u32| {
        let r = raise(
            format!(
                "The value of DMY2Date parameter {n} is outside of the permitted range. The current value is: {v}. The permitted range is: from 1 to {hi}."
            ),
            "OutsidePermittedRange",
        );
        if !three {
            return r;
        }
        crate::native::with_origin(
            r,
            crate::native::ErrorOrigin::Redacted(format!(
                "The value of DMY2Date parameter {n} is outside of the permitted range. The current value is: <redacted>. The permitted range is: from 1 to {hi}."
            )),
        )
    };
    // The parts are checked in a fixed order, and the first one out of range is named: the month, then the year, then the day
    // (types/P133DateEvaluateFormat.al:608-629 P133_DMY2Date_ThreeArgs_SeveralPartsOutOfRange: 32.13.2024, 0.0.2024 and
    // 57.52.0 name the month, 31.2.0 the year; :588-605 57.52.2076 the month). The day's range is the length of its month
    // (types/date-time/DateFuncs.al:243-302 DMY2Date_Invalid; P133DateEvaluateFormat.al:599 31.2.2024: 1 to 29).
    if !(1..=12).contains(&m) {
        return range(2, m, 12);
    }
    // Year 0 is valid only as (1, 1, 0): a date with the parts 1, 1, 0 that formats as 0001-01-02, which is (2, 1, 1)
    // (P133DateEvaluateFormat.al:667-718 P133_DMY2Date_YearZero_*; types/Y81DMY2Date.al:41-64). Any other day and month in year 0
    // is the year's error (P133DateEvaluateFormat.al:674 15.6.0, :623 31.2.0).
    if y == 0 && (d, m) == (1, 1) {
        return Ok(Value::Date(day_from_civil(1, 1, 2), false));
    }
    if !(1..=9999).contains(&y) {
        return range(3, y, 9999);
    }
    let last = days_in_month(y, m as u32);
    if d < 1 || d as u32 > last {
        return range(1, d, last);
    }
    // DMY2Date(1, 1, 1) is the undefined date, not the 00010101D literal (types/date-time/DateFuncs.al:213, :227: Format(.., 0, 9)
    // is ''; record/filters/P130DateFilters.al:261-262).
    if (d, m, y) == (1, 1, 1) {
        return Ok(Value::Date(0, false));
    }
    Ok(Value::Date(day_from_civil(y, m as u32, d as u32), false))
}

/// `CreateDateTime(Date, Time)`. A defined date with `0T` is midnight
/// (testpage/TestPageTemporalSetValue_Tests.al:85). Two blanks give the blank DateTime; a blank
/// date with a time is an error (types/TestDateRoundFormatStringContracts.al:CreateDateTime_MidnightBlankAndInvalid).
fn create_datetime(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Value::Time(t) = a[1] else {
        return unsupported(format!("CreateDateTime with {}", a[1].describe()));
    };
    if matches!(a[0], Value::Date(0, false)) {
        if t == 0 {
            return Ok(Value::DateTime(0));
        }
        return raise("The date is not valid.", "DateInvalid");
    }
    // A closing date is its day plus 23:59:59: with 12:00 PM the result reads 11:59 AM of the next day, with 0T 11:59 PM of the same
    // day (errors/PnBDateFormat.al:CreateDateTime_ClosingDate pins the minutes; the seconds are 59:
    // types/variant/Y67VariantNumeric.al:226-236 Variant_DateTimes_A `dt closing` 2026-01-16T11:59:59Z).
    let (day, closing) = match &a[0] {
        Value::Date(n, c) if *n > 0 => (*n, *c),
        other => return plain_date(other, "CreateDateTime").map(|_| unreachable!()),
    };
    // 0001-01-01 is no date a DateTime can be made of (types/date-time/DateTimeFuncs.al:CreateDateTime_Variants `af32`,
    // RoundDateTime_Edges `af118`).
    if day == 1 {
        return raise("The date is not valid.", "DateInvalid");
    }
    let time_of_day = (t - 1).max(0) + if closing { 86_399_000 } else { 0 };
    Ok(Value::DateTime((i64::from(day) - 1) * MS_PER_DAY + time_of_day + 1))
}

/// `RoundDateTime(DateTime [, Precision [, Direction]])` (types/TestRoundDateTimeContracts.al): the
/// precision is in milliseconds (default 1000; a BigInteger, Integer or Duration), the direction `=`
/// (nearest, a tie rounds up; the default), `<` (down) or `>` (up). A precision of 0 or less and an
/// unknown direction raise Argument errors; the undefined DateTime stays undefined.
///
/// Where the multiples of a precision start is pinned only by one case: 12:00:31 on 2024-03-05 with 700000 ms
/// rounds to 12:00:00 and 12:11:40 (types/TestRoundDateTimeContracts.al:61-72, BC 27.5). Every origin at a
/// midnight that fits it is a Saturday (a day shifts the multiples by 300000 ms, a week by none), so the
/// candidates (1899-12-30, 2000-01-01, ...) differ by whole weeks. They give the same multiples exactly when a
/// week is a whole number of precisions; any other precision is refused instead of choosing an origin.
fn round_datetime(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let invalid = |what: String, n: u32| {
        raise(
            format!("The contents '{what}' of ROUNDDATETIME parameter number {n} are invalid."),
            "Argument",
        )
    };
    let precision = match a.get(1) {
        None => 1000,
        Some(Value::Integer(n)) => i64::from(*n),
        Some(Value::BigInteger(n)) | Some(Value::Duration(n)) => *n,
        Some(other) => return unsupported(format!("RoundDateTime with a precision of {}", other.describe())),
    };
    if precision <= 0 {
        return invalid(precision.to_string(), 2);
    }
    let direction = match a.get(2) {
        None => "=".to_string(),
        Some(v) => match crate::native::text_of(v) {
            Some(t) => t,
            None => return unsupported("RoundDateTime with a direction that is not text"),
        }
        .to_string(),
    };
    if !matches!(direction.as_str(), "=" | "<" | ">") {
        return invalid(direction, 3);
    }
    let Value::DateTime(t) = a[0] else {
        return unsupported(format!("RoundDateTime of {}", a[0].describe()));
    };
    if t == 0 {
        return Ok(Value::DateTime(0));
    }
    if (7 * MS_PER_DAY) % precision != 0 {
        return unsupported(format!(
            "RoundDateTime with a precision of {precision} ms, which a week does not hold a whole number of: where its multiples start is not pinned (docs/probes-needed.md)"
        ));
    }
    // The days from 0001-01-01 to 1899-12-30, a Saturday: any other Saturday gives the same multiples here.
    let origin = 693_593 * MS_PER_DAY;
    let ms = t - 1;
    let below = ms - (ms - origin).rem_euclid(precision);
    let rounded = if below == ms {
        ms
    } else {
        match direction.as_str() {
            "<" => below,
            ">" => below + precision,
            _ if ms - below >= precision - (ms - below) => below + precision,
            _ => below,
        }
    };
    // A result past 9999-12-31 is not a DateTime (DateTimeFuncs.al:RoundDateTime_Edges `af126`, `af130`, RoundDateTime_Errors).
    if rounded >= 3_652_059 * MS_PER_DAY {
        return raise("The date is not valid.", "DateInvalid");
    }
    Ok(Value::DateTime(rounded + 1))
}

fn defined_datetime(v: &Value, what: &str) -> Result<i64, Signal> {
    match v {
        Value::DateTime(n) if *n > 0 => Ok(*n - 1),
        other => unsupported(format!(
            "{what} of {}: only a defined DateTime is pinned",
            other.describe()
        )),
    }
}

/// testpage/TestPageTemporalSetValue_Tests.al:118.
fn dt2date(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    // The undefined DateTime gives the undefined Date (text/TestProbeTextDateFormatContracts.al:Dates_CreateDateTimeRoundTrip).
    if matches!(a[0], Value::DateTime(0)) {
        return Ok(Value::Date(0, false));
    }
    let ms = defined_datetime(&a[0], "DT2Date")?;
    Ok(Value::Date(ms.div_euclid(MS_PER_DAY) as i32 + 1, false))
}

/// `DateTime.Date()` is `DT2Date` of the receiver, the undefined DateTime included
/// (types/date-time/DateTimeDateTimeMethods.al:27 DateTime_DateAndTimeMethods_SetValue, :51 _Undefined).
fn datetime_date(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let Some(v) = r else {
        return unsupported("DateTime.Date without a receiver");
    };
    dt2date(c, None, &mut [v.clone()])
}

/// `DateTime.Time()` is `DT2Time` of the receiver, the undefined DateTime included
/// (types/date-time/DateTimeDateTimeMethods.al:27 DateTime_DateAndTimeMethods_SetValue, :51 _Undefined).
fn datetime_time(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let Some(v) = r else {
        return unsupported("DateTime.Time without a receiver");
    };
    dt2time(c, None, &mut [v.clone()])
}

/// testpage/TestPageTemporalSetValue_Tests.al:117.
fn dt2time(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    if matches!(a[0], Value::DateTime(0)) {
        return Ok(Value::Time(0));
    }
    let ms = defined_datetime(&a[0], "DT2Time")?;
    Ok(Value::Time(ms.rem_euclid(MS_PER_DAY) + 1))
}

/// `Date2DMY(Date, 1|2|3)`: day, month, year (session-system/TestCalcDateNotificationContracts.al:21,22,32).
/// Any other part raises (types/TestDateRoundFormatStringContracts.al:Date2DMY_Part4_Throws, measured for 4).
fn date2dmy(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let day = match &a[0] {
        // The undefined date raises DateInvalid for the parts 1, 2 and 3 (errors/PnBDateFormat.al:135-163,
        // Date2DWY_PartsOutOfRange `undefinedDmy`; corpus master types/date-time/DateFuncs.al:507-530,
        // Date2DMY_UndefinedDate_DayMonthYear_RaiseDateInvalid) and the same for 0 and 4 (types/date-time/DateFuncs.al
        // Date2DMY_Invalid `0D what 0`, `0D what 4`).
        // Part 5 too (format-evaluate/format-matrix/FMDateTime.al:4000, :4013, corpus PR 1250).
        Value::Date(0, _) if matches!(int_of(a.get(1)), Some(0..=5)) => {
            return raise("The date is not valid.", "DateInvalid");
        }
        Value::Date(0, _) => {
            return unsupported("Date2DMY of an undefined date with a part other than 0 to 4 (not pinned)");
        }
        Value::Date(n, _) if *n > 0 => *n,
        other => return plain_date(other, "Date2DMY").map(|_| unreachable!()),
    };
    // 0001-01-02, the first defined date, reads as 1 January of year 0 (types/date-time/DateFuncs.al:477 UN_DMY2Date_YearZero_Value
    // `1.1.0`: day=1, month=1, year=0; types/Y81DMY2Date.al:62 `2.1.1` reads the same; needs corpus bump for the second).
    let (y, m, d) = if day == 2 { (0, 1, 1) } else { civil_from_day(day) };
    match int_of(a.get(1)) {
        Some(1) => Ok(Value::Integer(d as i32)),
        Some(2) => Ok(Value::Integer(m as i32)),
        Some(3) => Ok(Value::Integer(y)),
        Some(n) => raise(
            format!(
                "The value of DATE2DMY parameter 2 is outside of the permitted range. The current value is: {n}. The permitted range is: from 1 to 3."
            ),
            "OutsidePermittedRange",
        ),
        None => unsupported("Date2DMY with a part that is not an Integer"),
    }
}

/// `Date2DWY(Date, 1|2|3)`: the weekday (Monday is 1), the ISO 8601 week number
/// (types/TestDateRoundFormatStringContracts.al:Date2DWY_WeekNumbers_AreIso8601) and the year. The year is
/// the ISO week-year (text/TestProbeTextDateFormatContracts.al:Dates_Date2DWY_WeekAndYear).
fn date2dwy(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    // text/TestProbeTextDateFormatContracts.al:Dates_Date2DWY_BadPart: the undefined date raises, a closing date counts as
    // its day, a part outside 1 to 3 raises.
    let day = match &a[0] {
        Value::Date(0, _) => return raise("The date is not valid.", "DateInvalid"),
        Value::Date(n, _) if *n > 0 => *n,
        other => return plain_date(other, "Date2DWY").map(|_| unreachable!()),
    };
    let thursday = day + (4 - weekday(day));
    let (iso_year, _, _) = civil_from_day(thursday);
    match int_of(a.get(1)) {
        Some(1) => Ok(Value::Integer(weekday(day))),
        Some(2) => Ok(Value::Integer((thursday - day_from_civil(iso_year, 1, 1)) / 7 + 1)),
        // The ISO year of the week (text/TestProbeTextDateFormatContracts.al:Dates_Date2DWY_WeekAndYear: 2021-01-01 is in 2020).
        Some(3) => Ok(Value::Integer(iso_year)),
        Some(n) => raise(
            format!(
                "The value of DATE2DWY parameter 2 is outside of the permitted range. The current value is: {n}. The permitted range is: from 1 to 3."
            ),
            "OutsidePermittedRange",
        ),
        None => unsupported("Date2DWY with a part that is not an Integer"),
    }
}

/// `DWY2Date(WeekDay [, Week [, Year]])`: the date of an ISO 8601 weekday (Monday is 1), week and year
/// (types/date-time/DateFuncs.al:311-346 `DWY2Date_Valid`). A missing Week is the ISO week of today and a missing
/// Year the ISO year of today (`only day`, `day week`, lines 324-325 and 339-340). Week 1 holds January 4th
/// (`mon w1 2020` is 2019-12-30, line 334). A week 53 the year does not have, and a day after 9999-12-31, raise
/// "The date is not valid." (`DateInvalid`, `DWY2Date_Invalid` lines 349-399). A part outside its range raises with the
/// range text, and the date `0001-01-01` is the blank date (`w1 1`, lines 326 and 341).
/// Not pinned, so refused: two parts outside their range in one call (which one BC reports first), and a part that
/// is not an Integer.
fn dwy2date(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Some(wd) = int_of(a.first()) else {
        return unsupported("DWY2Date with a WeekDay that is not an Integer");
    };
    let today = if a.len() < 3 { ctx.today_day() } else { None };
    if a.len() < 3 && today.is_none() {
        return unsupported("DWY2Date without Week or Year needs today's date from the host");
    }
    let today_iso = today.map(|t| {
        let thursday = t + (4 - weekday(t));
        let (iso_year, _, _) = civil_from_day(thursday);
        ((thursday - day_from_civil(iso_year, 1, 1)) / 7 + 1, iso_year)
    });
    let week = match a.get(1) {
        Some(Value::Integer(n)) => *n,
        Some(_) => return unsupported("DWY2Date with a Week that is not an Integer"),
        None => today_iso.map_or(0, |t| t.0),
    };
    let year = match a.get(2) {
        Some(Value::Integer(n)) => *n,
        Some(_) => return unsupported("DWY2Date with a Year that is not an Integer"),
        None => today_iso.map_or(0, |t| t.1),
    };
    let checks = [(1, wd, 1, 7), (2, week, 1, 53), (3, year, 1, 9999)];
    let bad: Vec<_> = checks.iter().filter(|c| c.1 < c.2 || c.1 > c.3).collect();
    match bad.as_slice() {
        [] => {}
        [(n, v, lo, hi)] => {
            return raise(
                format!(
                    "The value of DWY2Date parameter {n} is outside of the permitted range. The current value is: {v}. The permitted range is: from {lo} to {hi}."
                ),
                "OutsidePermittedRange",
            );
        }
        _ => return unsupported("DWY2Date with several parts outside their range (the reported one is not pinned)"),
    }
    let jan4 = day_from_civil(year, 1, 4);
    let week1_monday = jan4 - (weekday(jan4) - 1);
    let day = week1_monday + (week - 1) * 7 + (wd - 1);
    let in_week_year = {
        let thursday = day + (4 - weekday(day));
        civil_from_day(thursday).0 == year
    };
    if !(1..=LAST_DAY).contains(&day) || !in_week_year {
        return raise("The date is not valid.", "DateInvalid");
    }
    // 0001-01-01 is BC's blank date (`w1 1` formats as '', DateFuncs.al:326 and :341), the one day this can reach below week 1 of year 2.
    Ok(Value::Date(if day == 1 { 0 } else { day }, false))
}

/// record/TestDateVirtualTable.al:188-190: a closing date gives its plain day, a plain date is unchanged.
fn normal_date(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    match &a[0] {
        Value::Date(n, _) if *n > 0 => Ok(Value::Date(*n, false)),
        other => unsupported(format!("NormalDate of {}", other.describe())),
    }
}

/// record/TestFilterContracts.al:56,58.
fn closing_date(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    match &a[0] {
        Value::Date(n, _) if *n > 0 => Ok(Value::Date(*n, true)),
        // streams/TestProbeStreamContracts.al:WriteDate_ClosingDates, `closing_0D`.
        Value::Date(0, _) => raise("The date is not valid.".to_string(), "DateInvalid"),
        other => unsupported(format!("ClosingDate of {}", other.describe())),
    }
}

/// 1 = Monday (session-system/TestSystemExtended.al:278). Day 1 (0001-01-01) is a Monday.
pub(crate) fn weekday(day: i32) -> i32 {
    (day - 1).rem_euclid(7) + 1
}

/// The day number of a defined date; a closing date counts as its day (format-evaluate/P93Date.al
/// BT_DayOfWeek_WeekNo_Year `closing 2024-12-30`: `1|1|2024`, as 2024-12-30).
fn receiver_date(r: Option<&mut Value>, what: &str) -> Result<i32, Signal> {
    match r {
        Some(Value::Date(n, true)) if *n > 0 => Ok(*n),
        Some(v) => plain_date(v, what),
        None => unsupported(format!("{what} without a receiver")),
    }
}

/// The undefined date raises DateInvalid (P93Date.al BT_DayOfWeek_WeekNo_Year `0D`).
fn day_of_week(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    if let Some(Value::Date(0, _)) = r.as_deref() {
        return raise("The date is not valid.", "DateInvalid");
    }
    Ok(Value::Integer(weekday(receiver_date(r, "Date.DayOfWeek")?)))
}

/// `Date.Day/Month/Year()`: the parts of a normal or closing date (format-evaluate/P93Date.al:51-78,
/// :98-121); the undefined date raises "The date is not valid." (`DateInvalid`, P93Date.al:81-95).
fn date_part(r: Option<&mut Value>, what: &str, pick: fn((i32, u32, u32)) -> i32) -> Out {
    match r {
        Some(Value::Date(0, _)) => raise("The date is not valid.", "DateInvalid"),
        // 0001-01-01: BC raises "The date is not valid." before the part is taken (P93Date.al:59-60, also :169-172 for
        // ToText), outside the TryFunction that takes it; where exactly is not pinned, so the part is refused.
        Some(Value::Date(1, _)) => unsupported(format!("{what} of 0001-01-01")),
        Some(Value::Date(n, _)) if *n > 1 => Ok(Value::Integer(pick(civil_from_day(*n)))),
        other => unsupported(format!("{what} of {}", other.describe())),
    }
}

/// The ISO 8601 week: the week that holds the year's first Thursday is week 1. The corpus pins
/// the range and the first week of a year (session-system/TestSystemExtended.al:285-300); the rule
/// itself is the documented one (docs/probes-needed.md).
fn week_no(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let day = receiver_date(r, "Date.WeekNo")?;
    let thursday = day + (4 - weekday(day));
    let (y, _, _) = civil_from_day(thursday);
    Ok(Value::Integer((thursday - day_from_civil(y, 1, 1)) / 7 + 1))
}

/// `Time.Hour/Minute/Second/Millisecond` (session-system/TestMiscTypes.al:22-42). The undefined time is not pinned.
fn time_part(r: Option<&mut Value>, unit_ms: i64, modulo: i64) -> Out {
    match r {
        Some(Value::Time(t)) if *t > 0 => Ok(Value::Integer((((*t - 1) / unit_ms) % modulo) as i32)),
        other => unsupported(format!(
            "a Time part of {}: only a defined Time is pinned",
            other.describe()
        )),
    }
}

// ---- CalcDate ------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

/// One term of a date formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Term {
    /// `+nD`, `-nW`, `nM`, `nQ`, `nY`: move by `n` units.
    Step(Unit, i32),
    /// `CD`, `CW`, `CM`, `CQ`, `CY`: the end of the current period; `-C..`: its start. A week runs
    /// from Monday to Sunday.
    Edge { unit: Unit, end: bool },
    /// `WDn`: the next day (`-WDn`: the previous one) that is weekday `n`, Monday being 1.
    WeekDay { n: i32, forward: bool },
    /// `Mn`: the first day of month `n` after the month of the date (`-Mn`: before it).
    MonthLetterFirst { n: i32, forward: bool },
    /// `Dn`, the day letter before its number: day n of the month, forward or backward (see `apply`).
    DayLetterFirst { n: i32, forward: bool },
    /// `Wn`: the Monday of ISO week `n` after the date (`W17`, `W21`, `W25` are pinned).
    WeekLetterFirst { n: i32 },
    /// `Qn`: the first day of quarter `n` after the date (only `Q3` is pinned).
    QuarterLetterFirst { n: u32 },
}

/// A formula as stored or typed, without the angle brackets: how `Format` prints it
/// (types/TestDateRoundFormatStringContracts.al:DateFormula_Evaluate_ThenFormat_PrintsTheNormalizedFormula).
pub fn formula_text(text: &str) -> String {
    let t = text.strip_prefix('<').unwrap_or(text);
    t.strip_suffix('>').unwrap_or(t).to_string()
}

/// The time-unit error of `<=` and a formatted date (`'<=' + Format(D)`, a date filter text used as a formula): BC shows the whole
/// text; Evaluate with its result used answers false (record/filters/DateFormulaOperatorFilterText.al:149-182, staging r6980;
/// al-runner issue 6998). Only `<=` before digits and slashes is pinned.
fn less_or_equal_date(text: &str) -> Option<String> {
    let rest = text.strip_prefix("<=")?;
    (rest.contains('/') && rest.bytes().all(|c| c.is_ascii_digit() || c == b'/')).then(|| {
        format!(
            "The date formula '{text}' should include a time unit.\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning current time unit based on date, can be specified as a prefix to the time unit."
        )
    })
}

/// Parses `<term...>` or the same without brackets (types/TestDateRoundFormatStringContracts.al:
/// CalcDate_WeeksQuartersWeekDaysAndCurrentPeriods), where a term is `[+|-][digits](D|W|M|Q|Y)`,
/// `[+|-]C(D|W|M|Q|Y)` or `[+|-]WD(1-7)`. After the first term every term carries its sign. A formula
/// with no term, lower case letters and the like are not pinned.
fn parse_formula(text: &str) -> Result<Vec<Term>, Signal> {
    let refuse = || unsupported(format!("date formula `{text}`: this form is not pinned by the corpus"));
    // The empty formula, `<>`, `<` and a text of white space are formulas without a term; CalcDate with
    // them returns the date (types/TestDateFormulaDurationFormatContracts.al:28-74,
    // types/date-time/DateFormulaEdges.al:ClosingBracket_MissingAfterPlus).
    if is_blank_formula(text) {
        return Ok(Vec::new());
    }
    if let Some(message) = less_or_equal_date(text) {
        return raise(message, "Evaluate");
    }
    if let Some(shown) = bracketed_steps_without_sign(text) {
        return raise(
            format!("The date formula '{shown}' must include plus (+) or minus (-)."),
            "Evaluate",
        );
    }
    let body = normalize_formula(text)?;
    if body.is_empty() {
        return Ok(Vec::new());
    }
    let body = body.as_str();
    numbers::too_large(body)?;
    let b = body.as_bytes();
    let unit_of = |c: Option<&u8>| match c {
        Some(b'D') => Some(Unit::Day),
        Some(b'W') => Some(Unit::Week),
        Some(b'M') => Some(Unit::Month),
        Some(b'Q') => Some(Unit::Quarter),
        Some(b'Y') => Some(Unit::Year),
        _ => None,
    };
    // A formula opened with one `<` and never closed shows the bracket in the message; with a closing bracket it shows the
    // body alone (text/text-methods/P136Text.al:P136_CalcDate_OneAngleBracket `noNumber`, `noNumberOpen`). More than
    // one opening bracket without a closing one is not pinned.
    let err_number = || {
        let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        let opens = compact.chars().take_while(|c| *c == '<').count();
        if !compact.contains('>') && opens > 1 {
            return unsupported(format!(
                "the number error of date formula `{text}`: several opening brackets and no closing one are not pinned"
            ));
        }
        // Brackets inside the outer pair stay in the message: `<CY>+<-10Y>` shows 'CY>+<-10Y' (al-runner issue 6648; corpus
        // branch al-runtime-rs-staging-y64 85d86e7b, types/date-time/DateFormulaEdges.al:705-706). Pinned for an
        // upper-case formula in one outer pair.
        let inner = compact
            .strip_prefix('<')
            .and_then(|t| t.strip_suffix('>'))
            .filter(|t| t.contains(['<', '>']));
        let shown = if let Some(inner) = inner {
            if inner.chars().any(|c| c.is_ascii_lowercase()) {
                return unsupported(format!(
                    "the number error of date formula `{text}` in lower case (not pinned)"
                ));
            }
            inner.to_string()
        } else if !compact.contains('>') && opens == 1 {
            format!("<{body}")
        } else {
            body.to_string()
        };
        raise(
            format!("The date formula '{shown}' should include a number."),
            "Evaluate",
        )
    };
    let err_unit = || {
        raise(
            format!(
                "The date formula '{body}' should include a time unit.\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning current time unit based on date, can be specified as a prefix to the time unit."
            ),
            "Evaluate",
        )
    };
    let mut terms = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if i > 0 && !matches!(b[i], b'+' | b'-') {
            // Two terms need a sign on the second (Dates_CalcDate_Formulas, `<1D1W>`).
            return raise(
                format!("The date formula '{body}' must include plus (+) or minus (-)."),
                "Evaluate",
            );
        }
        let mut plus = false;
        let negative = match b[i] {
            b'-' => {
                i += 1;
                true
            }
            b'+' => {
                i += 1;
                plus = true;
                false
            }
            _ => false,
        };
        // A sign followed by a sign has no number (DateFormulaEdges.al:SignsAndDigits_ErrorTexts, `++1D`, `+-1D`).
        if plus && matches!(b.get(i), Some(b'+' | b'-')) {
            return err_number();
        }
        // A sign at the end of a formula opened with `<` has no number. The message shows the text between the brackets
        // when it ends with `>`, else the whole text (types/date-time/P131CalcDate.al:80-81 `<+` and `<->`,
        // :116-117 `<1D>+` and `<1D+>`; types/P132DateFormula.al:142 `<+>`). Without the opening bracket, or with lower case letters, it is not
        // pinned.
        if (plus || negative) && i == b.len() {
            let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            if !compact.starts_with('<') || compact.chars().any(|c| c.is_ascii_lowercase()) {
                return refuse();
            }
            let shown = if compact.ends_with('>') { body } else { compact.as_str() };
            return raise(
                format!("The date formula '{shown}' should include a number."),
                "Evaluate",
            );
        }
        if b.get(i) == Some(&b'C') {
            let Some(unit) = unit_of(b.get(i + 1)) else {
                return refuse();
            };
            terms.push(Term::Edge { unit, end: !negative });
            i += 2;
            continue;
        }
        if b.get(i) == Some(&b'W') && b.get(i + 1) == Some(&b'D') {
            let Some(n) = b.get(i + 2).filter(|c| (b'1'..=b'7').contains(c)) else {
                // Only `WD8` is pinned (Dates_CalcDate_Formulas, `wd_with_number_8`); `WD0` is not.
                if b.get(i + 2).is_some_and(|c| *c == b'8' || *c == b'9') {
                    return raise(
                        format!(
                            "The date formula 'WD{}' contains a number that is too large.\\\\The number must be in the range 1 - 7.",
                            b[i + 2] as char
                        ),
                        "Evaluate",
                    );
                }
                if i + 2 == b.len() {
                    return err_number();
                }
                // `WD0` is an error whose text is not pinned (types/date-time/CalcDate.al:188,204: Evaluate answers false).
                if b.get(i + 2) == Some(&b'0') && !b.get(i + 3).is_some_and(u8::is_ascii_digit) {
                    return Err(Signal::Error(Box::new(crate::native::unpinned_error())));
                }
                return refuse();
            };
            terms.push(Term::WeekDay {
                n: i32::from(n - b'0'),
                forward: !negative,
            });
            i += 3;
            continue;
        }
        // `W17` and `Q3` forward (corpus master, corpus PR 871: types/date-time/Np4DateFormulas.al NP4_CalcDate_W17,
        // NP4_CalcDate_Q3): see `apply`. Other numbers, `-Wn` and
        // `-Qn` are not pinned.
        if !negative && matches!(b.get(i), Some(b'W' | b'Q')) && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
            let letter = b[i];
            let start = i + 1;
            i = start;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            match (letter, &body[start..i]) {
                // `W21` and `W25`: corpus master (corpus PR 886) types/date-time/DateFormulaInsidePeriod.al:64, :90.
                (b'W', "17") => terms.push(Term::WeekLetterFirst { n: 17 }),
                (b'W', "21") => terms.push(Term::WeekLetterFirst { n: 21 }),
                (b'W', "25") => terms.push(Term::WeekLetterFirst { n: 25 }),
                (b'Q', "3") => terms.push(Term::QuarterLetterFirst { n: 3 }),
                _ => return refuse(),
            }
            continue;
        }
        // `Mn`: the first day of month n after the month of the date (`-Mn`: before it). `M0` and `M13` raise the range error
        // (corpus master types/date-time/DateFormulaEdges.al:433-505, TV_MonthNumber_*); other numbers above 12
        // are not pinned.
        if b.get(i) == Some(&b'M') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
            let start = i + 1;
            i = start;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            let digits = &body[start..i];
            match digits.parse::<i32>() {
                Ok(n @ 1..=12) if !digits.starts_with('0') => {
                    terms.push(Term::MonthLetterFirst { n, forward: !negative })
                }
                _ if digits == "0" || digits == "13" => {
                    return raise(
                        format!(
                            "The date formula 'M{digits}' contains a number that is too large.\\\\The number must be in the range 1 - 12."
                        ),
                        "Evaluate",
                    );
                }
                _ => return refuse(),
            }
            continue;
        }
        // A `D` before its number is accepted (`D1`, types/TestDateFormulaEvaluateLeftoversContracts.al:
        // Evaluate_Accepted); what CalcDate does with it is pinned for one date only (`apply`). The other letters before a
        // number are not pinned.
        if b.get(i) == Some(&b'D') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
            let start = i + 1;
            i = start;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            let Ok(n) = body[start..i].parse::<i32>() else {
                return refuse();
            };
            terms.push(Term::DayLetterFirst { n, forward: !negative });
            continue;
        }
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if start == i {
            // A unit with no number, as the whole formula, with or without a sign (DateFormulaEdges.al:
            // SignsAndDigits_ErrorTexts `D`, `+D`, `-D`; WeekAndWeekdayUnits `W`, `WD`; TextsFromTheTestLibraries `Y`;
            // Placeholders_PercentN `+%D`). The other units (M, Q) are not pinned.
            let rest = &b[i..];
            // `%Q` and `%Y` too, with or without the brackets (types/date-time/DateFormulaSignedWd.al:89-136).
            if matches!(rest, b"D" | b"W" | b"WD" | b"Y" | b"%D" | b"%Q" | b"%Y") {
                return err_number();
            }
            // Letters that start with no unit letter are the unit error: `abc` shows `ABC` (types/date-time/CalcDate.al:463,
            // 478). Other letters are not pinned.
            if i == 0
                && rest.iter().all(u8::is_ascii_alphabetic)
                && !matches!(rest[0], b'D' | b'W' | b'M' | b'Q' | b'Y' | b'C')
            {
                return err_unit();
            }
            return refuse();
        }
        let Ok(n) = body[start..i].parse::<i32>() else {
            return refuse();
        };
        // `nWD` moves n days (DateFormulaEdges.al:WeekAndWeekdayUnits `2WD`, `1WD+1D`; types/W7EvaluateTests.al:
        // EvaluateDateFormula_Format2_NonEnglishFormulas, Friday + `1WD` is the Saturday). Pinned without a sign only.
        if b.get(i) == Some(&b'W') && b.get(i + 1) == Some(&b'D') && !b.get(i + 2).is_some_and(u8::is_ascii_digit) {
            // With a sign as the first term, `+1WD` and `-1WD` move one day too, with or without the brackets
            // (types/date-time/DateFormulaSignedWd.al:41-88). Another number with a sign, and a signed later
            // term, are not pinned.
            if (plus || negative) && !(n == 1 && start == 1) {
                return refuse();
            }
            terms.push(Term::Step(Unit::Day, if negative { -n } else { n }));
            i += 2;
            continue;
        }
        // A number before a current period (`2CW`) is an error whose text is not pinned (types/date-time/CalcDate.al:100,117:
        // Evaluate answers false).
        if b.get(i) == Some(&b'C') && unit_of(b.get(i + 1)).is_some() {
            return Err(Signal::Error(Box::new(crate::native::unpinned_error())));
        }
        let Some(unit) = unit_of(b.get(i)) else {
            // A number with no unit: at the end of the formula or before a slash (`+5`, `5`, `01/01/26`), or
            // a text of digits and hyphens, a date (types/date-time/P128CalcDateForms.al:
            // P128_CalcDate_DateTextIsNotAFormula `2026-10-16`, `10-16-26`).
            // A number and one letter that is no unit is the unit error (types/date-time/CalcDate.al:465,480 `1X`).
            if i + 1 == b.len() && start == 0 && b[i].is_ascii_alphabetic() && b[i] != b'C' {
                return err_unit();
            }
            let date_with_hyphens = b.iter().all(|c| c.is_ascii_digit() || *c == b'-');
            if i == b.len() || b[i] == b'/' || (b[i] == b'-' && date_with_hyphens) {
                return err_unit();
            }
            return refuse();
        };
        i += 1;
        terms.push(Term::Step(unit, if negative { -n } else { n }));
    }
    if terms.is_empty() {
        return refuse();
    }
    Ok(terms)
}

/// The formulas without a term: the empty text, `<>` and white space. `Evaluate` accepts them and
/// stores the empty formula, which `Format` prints as an empty text
/// (types/TestDateFormulaDurationFormatContracts.al:28-74: `''`, `'<>'`, `' '`).
pub fn is_blank_formula(text: &str) -> bool {
    matches!(text, "<>" | "<") || text.trim().is_empty()
}

/// `<A><B>`: two bracketed steps (`digits` and a unit) with no sign between them. The second step has no sign, so the
/// formula raises the missing-sign error, with the text between the outer brackets
/// (types/date-time/P131CalcDate.al:111 `<1D><1D>` gives '1D><1D'). Only this shape is pinned; any other
/// bracket inside a term stays refused (see [`normalize_formula`]).
fn bracketed_steps_without_sign(text: &str) -> Option<&str> {
    let body = text.strip_prefix('<')?.strip_suffix('>')?;
    let (a, b) = body.split_once("><")?;
    let step = |t: &str| {
        let digits = t.len() - 1;
        digits > 0
            && t.as_bytes()[..digits].iter().all(u8::is_ascii_digit)
            && matches!(t.as_bytes()[digits], b'D' | b'W' | b'M' | b'Q' | b'Y')
    };
    (step(a) && step(b)).then_some(body)
}

/// `<A>+` or `<A>-`: one bracketed step (`digits` and a unit) followed by a sign. `Evaluate` with its result used answers
/// false (types/date-time/P131CalcDate.al:152 `<1D>+`); the statement form and other trailing signs are not
/// pinned for `Evaluate`.
fn bracketed_step_then_sign(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('<') else {
        return false;
    };
    let Some(step) = rest.strip_suffix(">+").or_else(|| rest.strip_suffix(">-")) else {
        return false;
    };
    let digits = step.len().saturating_sub(1);
    digits > 0
        && step.as_bytes()[..digits].iter().all(u8::is_ascii_digit)
        && matches!(step.as_bytes()[digits], b'D' | b'W' | b'M' | b'Q' | b'Y')
}

/// The formula in upper case without spaces, angle brackets and `%` placeholders. Brackets are read where the corpus
/// pins them: any number of `<` at the start or right after a sign, and of `>` at the end or right before a sign
/// (DateFormulaEdges.al:ClosingBracket_MissingAfterPlus `<1D>>`, `<<1D>`; TextsFromTheTestLibraries `8D+<1D>`,
/// `<8D>+<1D>`, `<8D+<1D>>`). Elsewhere they are refused. A `%` before a number is dropped after a sign or at the
/// start, for the units D, W and M (DateFormulaEdges.al:Placeholders_PercentN).
fn normalize_formula(text: &str) -> Result<String, Signal> {
    let refuse = |why: &str| unsupported(format!("date formula `{text}`: {why} is not pinned by the corpus"));
    // Spaces are ignored: inside the brackets (types/date-time/Evaluate.al:Format_Evaluate `< 1D >`) and between the terms
    // (types/date-time/DateFormulaSpaces.al: CalcDate_Spaces, EvaluateDateFormula_Spaces).
    let chars: Vec<char> = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let mut out = String::with_capacity(chars.len());
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '<' => {
                // After `>`: the `><` between two bracketed terms, which the `>` arm below accepted.
                let edge = i == 0 || matches!(chars[i - 1], '<' | '+' | '-' | '>');
                if !edge && matches!(brackets::open(text, &chars, i)?, brackets::Inside::Refuse) {
                    return refuse("an angle bracket inside a term");
                }
            }
            '>' => {
                let tail_ok = chars[i + 1..].iter().all(|c| matches!(c, '<' | '>'))
                    || matches!(chars.get(i + 1), Some('+' | '-'))
                    || brackets::between_terms(text, &chars, i)?;
                if !tail_ok && matches!(brackets::close(&chars, i), brackets::Inside::Refuse) {
                    return refuse("an angle bracket inside a term");
                }
            }
            '%' if chars.get(i + 1).is_some_and(char::is_ascii_digit) => {
                let at_term_start = out.is_empty() || out.ends_with(['+', '-']);
                let unit = chars[i + 1..].iter().find(|c| !c.is_ascii_digit());
                let unit_ok = match unit {
                    Some('D' | 'M') => true,
                    Some('W') => !chars[i + 1..]
                        .iter()
                        .skip_while(|c| c.is_ascii_digit())
                        .nth(1)
                        .is_some_and(|c| *c == 'D'),
                    _ => false,
                };
                if !at_term_start || !unit_ok {
                    return refuse("a % placeholder here");
                }
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

/// Whether `Evaluate` rejects `text` as a date formula: it holds a letter that no formula uses
/// (the letters are C, D, W, M, Q and Y). Only `xyz` is pinned
/// (types/TestDateFormulaDurationFormatContracts.al:68); the rest of the texts the parser does not
/// know stay refused, not rejected (docs/probes-needed.md).
pub fn is_garbage_formula(text: &str) -> bool {
    text.bytes()
        .any(|b| b.is_ascii_alphabetic() && !matches!(b.to_ascii_uppercase(), b'C' | b'D' | b'W' | b'M' | b'Q' | b'Y'))
}

/// The "should include a time unit" error of a formula text with a letter no formula uses (`<1T>`: `1T`), which `evaluate_formula`
/// rejects without a message. xmlports/PWXmlPortFormatEvaluateMoreTypes.al:Import_Xml_DateFormula `df_localized_german` pins
/// this text for an XmlPort import; the statement form of Evaluate is not pinned for such a text.
pub fn time_unit_error(text: &str) -> Option<String> {
    match parse_formula(text) {
        Err(Signal::Error(e)) if e.message.contains("should include a time unit") => Some(e.message),
        _ => None,
    }
}

/// What `Evaluate` makes of a text for a DateFormula.
pub enum Evaluated {
    /// The formula to store, in the normal form `<TERMS>` (empty for the empty formula).
    Stored(String),
    /// `Evaluate` answers false when its result is used. With a message, the statement form raises it
    /// (code Evaluate); without one, that error text is not pinned.
    Rejected(Option<String>),
    /// The statement form raises this error (code Evaluate); the form with a used result answers false.
    Failed(String),
}

/// `Evaluate` into a DateFormula (types/TestDateFormulaEvaluateLeftoversContracts.al:35-69, BC 27.5):
/// lower case letters are read, angle brackets and white space are ignored (`< >` is the empty
/// formula), a sign in front is kept. A `D` or `WD` before its number is a formula (`D1`, `WD1`).
/// Rejected: a letter no formula uses (`1X`, `LW`), a `D` alone, a number alone, a decimal point,
/// and two parts without a sign (`1D1W`). Any other text stays refused (docs/probes-needed.md).
pub fn evaluate_formula(text: &str) -> Result<Evaluated, Signal> {
    if let Some(message) = less_or_equal_date(text) {
        return Ok(Evaluated::Failed(message));
    }
    // Evaluate with its result used answers false (types/date-time/P131CalcDate.al:147, 152); the statement
    // form is not pinned.
    // The statement form raises the missing-sign error (DateFormulaEdges.al:627-628, `<1D><1D>`).
    if let Some(shown) = bracketed_steps_without_sign(text) {
        return Ok(Evaluated::Rejected(Some(format!(
            "The date formula '{shown}' must include plus (+) or minus (-)."
        ))));
    }
    if bracketed_step_then_sign(text) {
        return Ok(Evaluated::Rejected(None));
    }
    // A bracket inside a term can raise a missing number or time unit error (see [`brackets`]).
    let n = match normalize_formula(text) {
        Err(Signal::Error(e)) if e.message.contains("should include a") => return Ok(Evaluated::Failed(e.message)),
        // Two bracketed terms, the second without a sign: as `<1D><1D>` above (`brackets::between_terms`).
        Err(Signal::Error(e)) if e.message.contains("must include plus (+) or minus (-)") => {
            return Ok(Evaluated::Rejected(Some(e.message)));
        }
        n => n?,
    };
    if n.is_empty() {
        return Ok(Evaluated::Stored(String::new()));
    }
    // A number of five digits does not fit (`99999D`).
    let long_number = n.as_bytes().windows(5).any(|w| w.iter().all(u8::is_ascii_digit));
    if n == "D" {
        return Ok(Evaluated::Rejected(Some(
            "The date formula 'D' should include a number.".to_string(),
        )));
    }
    if long_number {
        return Ok(Evaluated::Rejected(None));
    }
    if n.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(Evaluated::Rejected(match parse_formula(&n) {
            Err(Signal::Error(e)) => Some(e.message),
            _ => None,
        }));
    }
    // Letters only, none a time unit or `C`: the statement raises the time-unit error with the text upper-cased
    // (format-evaluate/Y49EvaluateDateBlanks.al:299-330 `zz`).
    if n.bytes().all(|b| b.is_ascii_uppercase() && !b"DWMQYC".contains(&b)) {
        return Ok(Evaluated::Rejected(Some(format!(
            "The date formula '{n}' should include a time unit.\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning current time unit based on date, can be specified as a prefix to the time unit."
        ))));
    }
    if n.contains('.') || is_garbage_formula(&n) || long_number {
        return Ok(Evaluated::Rejected(None));
    }
    // A step (`digits` then a unit) followed by a digit starts a second part with no sign.
    let b = n.as_bytes();
    for i in 0..b.len().saturating_sub(2) {
        let after_step = b[i].is_ascii_digit()
            && matches!(b[i + 1], b'D' | b'W' | b'M' | b'Q' | b'Y')
            && b[i + 2].is_ascii_digit()
            && !(b[i + 1] == b'D' && i > 0 && b[i - 1] == b'W');
        if after_step {
            return Ok(Evaluated::Rejected(None));
        }
    }
    match parse_formula(&n) {
        Ok(_) => {}
        // The errors of a formula without a number or a time unit. Read from the text as written, so that a bracket inside the
        // outer pair stays in the message, as in CalcDate (`<CY>+<-10Y>`, DateFormulaEdges.al:705-706 on corpus branch
        // al-runtime-rs-staging-y64).
        Err(Signal::Error(e)) if e.message.contains("should include a") => {
            return Ok(Evaluated::Failed(match parse_formula(text) {
                Err(Signal::Error(w)) if w.message.contains("should include a") => w.message,
                _ => e.message,
            }));
        }
        Err(e) => return Err(e),
    }
    Ok(Evaluated::Stored(format!("<{n}>")))
}

/// Whether `text` is a date formula `Evaluate` can store (see [`parse_formula`]).
pub fn check_formula(text: &str) -> Result<(), Signal> {
    parse_formula(text).map(|_| ())
}

/// The day `months` months from `(y, m, d)`; the day of the month is cut back to the month's length
/// (types/TestDateRoundFormatStringContracts.al:CalcDate_MonthYearClamping). `None` when the year leaves 0001-9999.
fn add_months(y: i32, m: u32, d: u32, months: i64) -> Option<i32> {
    let total = i64::from(y) * 12 + i64::from(m) - 1 + months;
    let (ny, nm) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
    (1..=9999)
        .contains(&ny)
        .then(|| day_from_civil(ny as i32, nm, d.min(days_in_month(ny as i32, nm))))
}

fn outside_range<T>() -> Result<T, Signal> {
    unsupported("CalcDate result outside 0001-9999 (error text not pinned)")
}

/// Where [`apply`] ended.
enum Applied {
    /// The result, a day in 0001-01-01..9999-12-31.
    Day(i32),
    /// The last term, a step, left the range ([`range::past_edge`] decides).
    PastEdge,
}

/// The day a step term moves to, `None` when it leaves 0001-01-01..9999-12-31.
fn step(day: i32, unit: Unit, n: i32) -> Option<i32> {
    let (y, m, d) = civil_from_day(day);
    let to = match unit {
        Unit::Day => i64::from(day) + i64::from(n),
        Unit::Week => i64::from(day) + i64::from(n) * 7,
        Unit::Month => i64::from(add_months(y, m, d, i64::from(n))?),
        Unit::Quarter => i64::from(add_months(y, m, d, i64::from(n) * 3)?),
        Unit::Year => i64::from(add_months(y, m, d, i64::from(n) * 12)?),
    };
    i32::try_from(to).ok().filter(|t| (1..=LAST_DAY).contains(t))
}

fn apply(terms: &[Term], from: i32) -> Result<Applied, Signal> {
    let mut day = from;
    for (i, term) in terms.iter().enumerate() {
        let (y, m, d) = civil_from_day(day);
        day = match *term {
            Term::Step(unit, n) => match step(day, unit, n) {
                Some(to) => to,
                None if i + 1 == terms.len() => return Ok(Applied::PastEdge),
                None => return outside_range(),
            },
            Term::Edge { unit: Unit::Day, .. } => day,
            Term::Edge { unit: Unit::Week, end } => {
                if end {
                    day + (7 - weekday(day))
                } else {
                    day - (weekday(day) - 1)
                }
            }
            Term::Edge { unit: Unit::Month, end } => day_from_civil(y, m, if end { days_in_month(y, m) } else { 1 }),
            Term::Edge { unit: Unit::Year, end } => {
                if end {
                    day_from_civil(y, 12, 31)
                } else {
                    day_from_civil(y, 1, 1)
                }
            }
            Term::Edge {
                unit: Unit::Quarter,
                end,
            } => {
                let first = (m - 1) / 3 * 3 + 1;
                if end {
                    day_from_civil(y, first + 2, days_in_month(y, first + 2))
                } else {
                    day_from_civil(y, first, 1)
                }
            }
            // `Dn` forward: day n of the date's month, cut back to the month's length, when the date's day is below n; else day n
            // of the next month, cut back the same way (`<D29>` on 2023-02-28 stays 2023-02-28, `<D31>` on 2023-01-31 is
            // 2023-02-28). Pinned for D1 (types/date-time/DateFormulaInsidePeriod.al:36-47, types/
            // TestDateFormulaEvaluateLeftoversContracts.al:45), D15, D28, D29, D31 (types/date-time/Np4DateFormulas.al
            // NP4_CalcDate_D15, _D28, _D29, _D31), D29, D30, D31 on seven dates each (DateFormulaEdges.al SG_DayOfMonth_D29_D30_D31)
            // and D5 after CM (SG_DayOfMonth_Combined); every number of 1 to 31 follows the same rule.
            Term::DayLetterFirst { n, forward: true } if (1..=31).contains(&n) => {
                let n = n as u32;
                if d < n {
                    day_from_civil(y, m, n.min(days_in_month(y, m)))
                } else {
                    add_months(y, m, n, 1).map_or_else(outside_range, Ok)?
                }
            }
            // `-Dn`: day n of the date's month when the date is after it, else of the month before, cut back to the month's length
            // (DateFormulaInsidePeriod.al:46-47; DateFormulaEdges.al SG_DayOfMonth_Backward `-D1`, `-D15`, `-D31` on seven dates each:
            // `<-D31>` on 2024-12-31 is 2024-11-30).
            Term::DayLetterFirst { n, forward: false } if (1..=31).contains(&n) => {
                let n = n as u32;
                if d > n {
                    day_from_civil(y, m, n)
                } else {
                    add_months(y, m, n, -1).map_or_else(outside_range, Ok)?
                }
            }
            Term::DayLetterFirst { .. } => {
                return unsupported("date formula with a day letter before a number out of 1 to 31 (not pinned)");
            }
            // The Monday of ISO week n of the date's year when it is after the date, else of the next year's week n
            // (NP4_CalcDate_W17: 2023-04-15 2023-04-24, 2023-06-30 2024-04-22). A date inside week n, its Monday included, goes to the
            // next year too (DateFormulaInsidePeriod.al:64-77 `<W21>` on 2024-05-20 to 2024-05-26 is 2025-05-19, :114-125 `<W17>`).
            Term::WeekLetterFirst { n } => {
                let monday = |year: i32| {
                    // ISO week 1 holds January 4.
                    let jan4 = day_from_civil(year, 1, 4);
                    jan4 - (weekday(jan4) - 1) + (n - 1) * 7
                };
                let this = monday(y);
                if this > day { this } else { monday(y + 1) }
            }
            // The first day of quarter n of the date's year when it is after the date, else of the next year
            // (NP4_CalcDate_Q3: 2023-06-30 2023-07-01, 2023-12-31 2024-07-01). A date inside quarter n, its first day included, goes
            // to the next year (DateFormulaInsidePeriod.al:134-147).
            Term::QuarterLetterFirst { n } => {
                let first = (n - 1) * 3 + 1;
                if m < first {
                    day_from_civil(y, first, 1)
                } else {
                    day_from_civil(y + 1, first, 1)
                }
            }
            // DateFormulaEdges.al:433-505 (corpus master): from a date in month n the term goes a year on (`<M2>` on 02-29 is the
            // next year's 02-01, `<-M2>` the year before's). The first day of month n itself is not pinned.
            Term::MonthLetterFirst { n, forward } => {
                let n = n as u32;
                // Forward on the first day of month n: the next year's (NP4_CalcDate_M1 2023-01-01 2024-01-01). Backward is not pinned.
                if m == n && d == 1 && !forward {
                    return unsupported("date formula `-Mn` on the first day of month n (not pinned)");
                }
                let year = match (forward, m < n, m > n) {
                    (true, true, _) | (false, _, true) => y,
                    (true, false, _) => y + 1,
                    (false, _, false) => y - 1,
                };
                day_from_civil(year, n, 1)
            }
            Term::WeekDay { n, forward } => {
                let wd = weekday(day);
                // On the weekday itself the term moves a whole week (Dates_CalcDate_Formulas: `<WD1>` on a Monday).
                if forward {
                    day + (n - wd - 1).rem_euclid(7) + 1
                } else {
                    day - ((wd - n - 1).rem_euclid(7) + 1)
                }
            }
        };
        if !(1..=LAST_DAY).contains(&day) {
            return outside_range();
        }
    }
    Ok(Applied::Day(day))
}

fn calcdate(ctx: &dyn Ctx, formula: &str, base: Option<&Value>, overload: range::Overload) -> Out {
    let terms = parse_formula(formula)?;
    // Without a date the calculation counts from TODAY, not from WorkDate (types/
    // TestDateTimeIntegerCalcDateContracts.al:CalcDateWithoutADate_UsesToday_NotWorkDate).
    let today = base
        .is_none()
        .then(|| ctx.today_day())
        .flatten()
        .map(|d| Value::Date(d, false));
    let Some(base) = base.or(today.as_ref()) else {
        return unsupported("CalcDate without a date needs the host's clock");
    };
    if matches!(base, Value::Date(0, _)) {
        // types/TestDateRoundFormatStringContracts.al:CalcDate_UndefinedDate_Throws (CR and backslash line breaks).
        return raise(
            format!(
                "You cannot base a date calculation on an undefined date.\\\\Date: 0D\\Formula: {}.",
                formula_text(formula)
            ),
            "Argument",
        );
    }
    // From a closing date the day is pinned (Dates_CalcDate_Formulas, `closing_date_base`, read with Format(R, 0, 9)); whether the
    // result is a closing date is pinned only from C9999-12-31 (`range::closing_result`).
    let base = match base {
        Value::Date(n, closing) if *n > 0 => range::Base {
            day: *n,
            closing: *closing,
        },
        other => return plain_date(other, "CalcDate").map(|_| unreachable!()),
    };
    range::low_base(&terms, base, overload)?;
    range::year_step_too_large(formula, base, overload)?;
    match apply(&terms, base.day)? {
        Applied::PastEdge => range::past_edge(&terms, base, overload),
        Applied::Day(day) => {
            range::first_day_result(day)?;
            Ok(Value::Date(day, range::closing_result(&terms, base, overload)))
        }
    }
}

fn calcdate_text(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    match crate::native::text_of(&a[0]) {
        Some(f) => calcdate(ctx, &f, a.get(1), range::Overload::Text),
        None => unsupported("CalcDate with a formula that is not text"),
    }
}

fn calcdate_formula(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    match &a[0] {
        Value::DateFormula(f) => calcdate(ctx, f, a.get(1), range::Overload::Formula),
        other => unsupported(format!("CalcDate with {}", other.describe())),
    }
}

#[cfg(test)]
mod tests {
    use crate::format::day_from_civil;
    use crate::native::tests::{TestCtx, call, t};
    use crate::native::{Signal, lookup};
    use crate::value::Value;

    fn date(y: i32, m: u32, d: u32) -> Value {
        Value::Date(day_from_civil(y, m, d), false)
    }
    fn is_date(r: Result<Value, Signal>, y: i32, m: u32, d: u32) {
        match r {
            Ok(Value::Date(n, false)) => assert_eq!(n, day_from_civil(y, m, d), "wrong date"),
            other => panic!("{other:?}"),
        }
    }
    #[track_caller]
    fn refused(r: Result<Value, Signal>) {
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?} should be a refusal");
    }
    fn int(r: Result<Value, Signal>) -> i32 {
        match r {
            Ok(Value::Integer(i)) => i,
            other => panic!("{other:?}"),
        }
    }
    fn on(id: &str, recv: Value) -> Result<Value, Signal> {
        let f = lookup(id).unwrap_or_else(|| panic!("{id} not registered"));
        let mut recv = recv;
        f(&mut TestCtx::default(), Some(&mut recv), &mut [])
    }
    fn calc(formula: &str, from: Value) -> Result<Value, Signal> {
        call("System.CalcDate#2", vec![t(formula), from])
    }

    /// Corpus master (corpus PR 871) types/date-time/Np4DateFormulas.al NP4_CalcDate_*.
    #[test]
    fn letter_first_formulas_follow_the_pins() {
        type Ymd = (i32, u32, u32);
        let cases: &[(&str, Ymd, Ymd)] = &[
            ("<D15>", (2023, 2, 14), (2023, 2, 15)),
            ("<D15>", (2024, 6, 15), (2024, 7, 15)),
            ("<D29>", (2023, 1, 31), (2023, 2, 28)),
            ("<D29>", (2023, 2, 28), (2023, 2, 28)),
            ("<D31>", (2023, 6, 30), (2023, 6, 30)),
            ("<D31>", (2024, 3, 31), (2024, 4, 30)),
            ("<D28>", (2023, 2, 28), (2023, 3, 28)),
            ("<W17>", (2023, 4, 15), (2023, 4, 24)),
            ("<W17>", (2023, 6, 30), (2024, 4, 22)),
            ("<W17>", (2024, 6, 15), (2025, 4, 21)),
            ("<Q3>", (2023, 6, 30), (2023, 7, 1)),
            ("<Q3>", (2023, 12, 31), (2024, 7, 1)),
            ("<M1>", (2023, 1, 1), (2024, 1, 1)),
            ("<M3>", (2024, 3, 31), (2025, 3, 1)),
        ];
        for &(f, (y, m, d), (ry, rm, rd)) in cases {
            match calc(f, date(y, m, d)) {
                Ok(Value::Date(n, false)) => assert_eq!(n, day_from_civil(ry, rm, rd), "{f} on {y}-{m}-{d}"),
                other => panic!("{f} on {y}-{m}-{d}: {other:?}"),
            }
        }
    }

    /// Corpus master (corpus PR 886) types/date-time/DateFormulaInsidePeriod.al: `<D1>` and `<-D1>` from any day,
    /// `<W21>`, `<W25>`, `<W17>` and `<Q3>` from a date inside the period.
    #[test]
    fn letter_first_formulas_inside_the_period_go_to_the_next_one() {
        type Ymd = (i32, u32, u32);
        let cases: &[(&str, Ymd, Ymd)] = &[
            ("<D1>", (2024, 1, 1), (2024, 2, 1)),
            ("<D1>", (2024, 1, 2), (2024, 2, 1)),
            ("<D1>", (2024, 2, 28), (2024, 3, 1)),
            ("<D1>", (2023, 2, 27), (2023, 3, 1)),
            ("<D1>", (2024, 12, 1), (2025, 1, 1)),
            ("<D1>", (2024, 12, 30), (2025, 1, 1)),
            ("<-D1>", (2024, 1, 15), (2024, 1, 1)),
            ("<-D1>", (2024, 1, 1), (2023, 12, 1)),
            ("<W21>", (2024, 5, 19), (2024, 5, 20)),
            ("<W21>", (2024, 5, 20), (2025, 5, 19)),
            ("<W21>", (2024, 5, 26), (2025, 5, 19)),
            ("<W21>", (2024, 5, 27), (2025, 5, 19)),
            ("<W21>", (2020, 5, 18), (2021, 5, 24)),
            ("<W21>", (2021, 1, 1), (2021, 5, 24)),
            ("<W21>", (2026, 10, 30), (2027, 5, 24)),
            ("<W25>", (2024, 6, 16), (2024, 6, 17)),
            ("<W25>", (2024, 6, 23), (2025, 6, 16)),
            ("<W25>", (2026, 6, 17), (2027, 6, 21)),
            ("<W17>", (2024, 4, 22), (2025, 4, 21)),
            ("<W17>", (2023, 4, 30), (2024, 4, 22)),
            ("<Q3>", (2024, 7, 1), (2025, 7, 1)),
            ("<Q3>", (2024, 9, 30), (2025, 7, 1)),
            ("<Q3>", (2023, 9, 15), (2024, 7, 1)),
        ];
        for &(f, (y, m, d), (ry, rm, rd)) in cases {
            match calc(f, date(y, m, d)) {
                Ok(Value::Date(n, false)) => assert_eq!(n, day_from_civil(ry, rm, rd), "{f} on {y}-{m}-{d}"),
                other => panic!("{f} on {y}-{m}-{d}: {other:?}"),
            }
        }
        // Other week and quarter numbers are not pinned.
        refused(calc("<W22>", date(2024, 1, 1)));
    }

    /// types/date-time/DateFuncs.al Date2DMY_Invalid `0D what 0`, `0D what 4`: the undefined date raises DateInvalid for the parts
    /// 0 and 4 as for 1. Corpus master types/date-time/DateFuncs.al:507-530 (Date2DMY_UndefinedDate_DayMonthYear_RaiseDateInvalid,
    /// al-runner issue 7468): the parts 2 and 3 raise the same.
    #[test]
    fn date2dmy_of_the_undefined_date_raises_for_parts_0_to_4() {
        for part in [0, 1, 2, 3, 4] {
            match call("System.Date2DMY#1", vec![Value::Date(0, false), Value::Integer(part)]) {
                Err(Signal::Error(e)) => {
                    assert_eq!(
                        (e.code.as_str(), e.message.as_str()),
                        ("DateInvalid", "The date is not valid."),
                        "{part}"
                    );
                }
                other => panic!("{part}: {other:?}"),
            }
        }
    }

    // `D1` on 2024-01-31 is 2024-02-01 (types/TestDateFormulaEvaluateLeftoversContracts.al:45).
    #[test]
    fn d1_at_the_end_of_a_month_and_d31_at_the_start() {
        is_date(calc("D1", date(2024, 1, 31)), 2024, 2, 1);
        // `<D31>` is pinned on corpus master now (Np4DateFormulas.al NP4_CalcDate_D31).
        is_date(calc("<D31>", date(2024, 1, 1)), 2024, 1, 31);
    }

    /// types/date-time/DateFormulaEdges.al SG_DayOfMonth_D29_D30_D31 and SG_DayOfMonth_Backward (the dates
    /// 2024-01-10, 01-15, 01-31, 02-29, 2023-02-28, 2024-04-30, 12-31), SG_DayOfMonth_Combined (`D5` after `CM`).
    #[test]
    fn a_day_letter_before_its_number_follows_the_corpus_tables() {
        type Ymd = (i32, u32, u32);
        let from: [Ymd; 7] = [
            (2024, 1, 10),
            (2024, 1, 15),
            (2024, 1, 31),
            (2024, 2, 29),
            (2023, 2, 28),
            (2024, 4, 30),
            (2024, 12, 31),
        ];
        let tables: &[(&str, [Ymd; 7])] = &[
            (
                "<D30>",
                [
                    (2024, 1, 30),
                    (2024, 1, 30),
                    (2024, 2, 29),
                    (2024, 2, 29),
                    (2023, 2, 28),
                    (2024, 5, 30),
                    (2025, 1, 30),
                ],
            ),
            (
                "<-D15>",
                [
                    (2023, 12, 15),
                    (2023, 12, 15),
                    (2024, 1, 15),
                    (2024, 2, 15),
                    (2023, 2, 15),
                    (2024, 4, 15),
                    (2024, 12, 15),
                ],
            ),
            (
                "<-D31>",
                [
                    (2023, 12, 31),
                    (2023, 12, 31),
                    (2023, 12, 31),
                    (2024, 1, 31),
                    (2023, 1, 31),
                    (2024, 3, 31),
                    (2024, 11, 30),
                ],
            ),
            (
                "<-D1>",
                [
                    (2024, 1, 1),
                    (2024, 1, 1),
                    (2024, 1, 1),
                    (2024, 2, 1),
                    (2023, 2, 1),
                    (2024, 4, 1),
                    (2024, 12, 1),
                ],
            ),
        ];
        for (f, want) in tables {
            for (&(y, m, d), &(ry, rm, rd)) in from.iter().zip(want) {
                match calc(f, date(y, m, d)) {
                    Ok(Value::Date(n, false)) => assert_eq!(n, day_from_civil(ry, rm, rd), "{f} on {y}-{m}-{d}"),
                    other => panic!("{f} on {y}-{m}-{d}: {other:?}"),
                }
            }
        }
        // `<D5>` after `<CM>` of 2024-01-31 (CM+D5): the fifth of the next month.
        is_date(calc("<CM+D5>", date(2024, 1, 31)), 2024, 2, 5);
    }

    // types/TestRoundDateTimeContracts.al:61-72: a precision of 700000 ms does not divide a day but a week.
    #[test]
    fn round_date_time_counts_a_precision_that_divides_a_week_but_not_a_day() {
        let at = |h: u32, m: u32, s: u32| {
            let Value::Date(d, _) = date(2024, 3, 5) else {
                unreachable!()
            };
            Value::DateTime((i64::from(d) - 1) * 86_400_000 + i64::from(h * 3600 + m * 60 + s) * 1000 + 1)
        };
        let round = |dir: &str| {
            call(
                "System.RoundDateTime#1",
                vec![at(12, 0, 31), Value::Integer(700_000), t(dir)],
            )
        };
        for (dir, want) in [("=", at(12, 0, 0)), ("<", at(12, 0, 0)), (">", at(12, 11, 40))] {
            let (Ok(Value::DateTime(got)), Value::DateTime(want)) = (round(dir), want) else {
                panic!("{dir}")
            };
            assert_eq!(got, want, "{dir}");
        }
    }

    #[test]
    fn round_date_time_refuses_a_precision_whose_multiples_depend_on_the_origin() {
        let r = call(
            "System.RoundDateTime#1",
            vec![Value::DateTime(86_400_000 * 700_000 + 1), Value::Integer(1_100_000)],
        );
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains("not pinned")),
            "{r:?}"
        );
    }

    #[test]
    fn dmy2date_knows_the_length_of_every_month_and_the_leap_year_rules() {
        let d = |a: i32, b: i32, c: i32| {
            call(
                "System.DMY2Date#1",
                vec![Value::Integer(a), Value::Integer(b), Value::Integer(c)],
            )
        };
        for (month, days) in [
            (1, 31),
            (2, 28),
            (3, 31),
            (4, 30),
            (5, 31),
            (6, 30),
            (7, 31),
            (8, 31),
            (9, 30),
            (10, 31),
            (11, 30),
            (12, 31),
        ] {
            is_date(d(days, month, 2023), 2023, month as u32, days as u32);
            raises(d(days + 1, month, 2023), "OutsidePermittedRange");
        }
        // Leap years: divisible by 4, not by 100 unless by 400.
        is_date(d(29, 2, 2024), 2024, 2, 29);
        is_date(d(29, 2, 2000), 2000, 2, 29);
        raises(d(29, 2, 1900), "OutsidePermittedRange");
        raises(d(29, 2, 2023), "OutsidePermittedRange");
        raises(d(30, 2, 2024), "OutsidePermittedRange");
    }

    #[test]
    fn dmy2date_builds_a_date() {
        // recordref-fieldref/fieldref/TestFieldRefValue.al:122 `DMY2Date(1, 1, 2024)`; record/TestDateVirtualTableFlowField.al:65
        // `DMY2Date(3, 1, 1)` and :83 `DMY2Date(31, 12, 9999)` are the ends of the range.
        let d = |a: i32, b: i32, c: i32| {
            call(
                "System.DMY2Date#1",
                vec![Value::Integer(a), Value::Integer(b), Value::Integer(c)],
            )
        };
        is_date(d(1, 1, 2024), 2024, 1, 1);
        is_date(d(31, 12, 9999), 9999, 12, 31);
        is_date(d(3, 1, 1), 1, 1, 3);
        is_date(d(29, 2, 2024), 2024, 2, 29);
        // DMY2Date(1, 1, 1) is the undefined date: Format(.., 0, 9) gives '' (types/date-time/DateFuncs.al:213, :227), which
        // the 00010101D literal does not (it raises), and a Date field stored with it reads back equal and blank
        // (record/filters/P130DateFilters.al:261-262).
        assert!(matches!(d(1, 1, 1), Ok(Value::Date(0, false))), "{:?}", d(1, 1, 1));
    }

    #[test]
    fn dmy2date_year_zero_is_pinned_only_as_the_first_defined_date() {
        let d = |args: Vec<i32>| call("System.DMY2Date#1", args.into_iter().map(Value::Integer).collect());
        // (1, 1, 0) is the first defined date, 0001-01-02 (types/date-time/DateFuncs.al:477 UN_DMY2Date_YearZero_Value).
        assert!(
            matches!(d(vec![1, 1, 0]), Ok(Value::Date(2, false))),
            "{:?}",
            d(vec![1, 1, 0])
        );
    }

    #[test]
    fn dmy2date_refuses_what_the_corpus_does_not_pin() {
        let d = |args: Vec<i32>| call("System.DMY2Date#1", args.into_iter().map(Value::Integer).collect());
        refused(d(vec![1]));
        refused(d(vec![1, 1]));
    }

    /// types/date-time/DateFuncs.al:243-302 DMY2Date_Invalid: the one part out of range is named with its number, its value and
    /// its range; the day's range is the length of its month; a month out of range in year 0 is the month's error.
    #[test]
    fn dmy2date_names_the_part_out_of_range() {
        let d = |a: i32, b: i32, c: i32| {
            call(
                "System.DMY2Date#1",
                vec![Value::Integer(a), Value::Integer(b), Value::Integer(c)],
            )
        };
        let text = |r: Result<Value, Signal>| match r {
            Err(Signal::Error(e)) if e.code == "OutsidePermittedRange" => e.message,
            other => panic!("{other:?}"),
        };
        let want = |n: i32, v: i32, hi: i32| {
            format!(
                "The value of DMY2Date parameter {n} is outside of the permitted range. The current value is: {v}. The permitted range is: from 1 to {hi}."
            )
        };
        for ((a, b, c), (n, v, hi)) in [
            ((30, 2, 2024), (1, 30, 29)),
            ((29, 2, 2023), (1, 29, 28)),
            ((32, 1, 2024), (1, 32, 31)),
            ((31, 4, 2024), (1, 31, 30)),
            ((1, 13, 2024), (2, 13, 12)),
            ((1, 0, 2024), (2, 0, 12)),
            ((0, 1, 2024), (1, 0, 31)),
            ((-1, 1, 2024), (1, -1, 31)),
            ((1, -1, 2024), (2, -1, 12)),
            ((1, 1, -1), (3, -1, 9999)),
            ((1, 1, 10000), (3, 10000, 9999)),
            ((0, 5, 2024), (1, 0, 31)),
            ((1, 0, 0), (2, 0, 12)),
            // Several parts out of range: the month is checked first, then the year, then the day
            // (types/P133DateEvaluateFormat.al:608-629 P133_DMY2Date_ThreeArgs_SeveralPartsOutOfRange, :588-605).
            ((57, 52, 2076), (2, 52, 12)),
            ((32, 13, 2024), (2, 13, 12)),
            ((0, 0, 2024), (2, 0, 12)),
            ((57, 52, 0), (2, 52, 12)),
            ((1, 13, 10000), (2, 13, 12)),
            ((31, 2, 0), (3, 0, 9999)),
            ((32, 1, 0), (3, 0, 9999)),
            ((32, 1, 10000), (3, 10000, 9999)),
        ] {
            assert_eq!(text(d(a, b, c)), want(n, v, hi), "DMY2Date({a}, {b}, {c})");
        }
    }

    #[test]
    fn create_datetime_joins_date_and_time() {
        // json/TestJsonTypedGetters.al:351 `CreateDateTime(Today(), 120000T)`;
        // testpage/TestPageTemporalSetValue_Tests.al:85 pairs a date with 0T (midnight).
        let day = i64::from(day_from_civil(2024, 1, 15)) - 1;
        let noon = Value::Time(43_200_000 + 1);
        assert!(matches!(
            call("System.CreateDateTime#1", vec![date(2024, 1, 15), noon]),
            Ok(Value::DateTime(n)) if n == day * 86_400_000 + 43_200_000 + 1
        ));
        assert!(matches!(
            call("System.CreateDateTime#1", vec![date(2024, 1, 15), Value::Time(0)]),
            Ok(Value::DateTime(n)) if n == day * 86_400_000 + 1
        ));
        // types/TestDateRoundFormatStringContracts.al:CreateDateTime_MidnightBlankAndInvalid.
        assert!(matches!(
            call("System.CreateDateTime#1", vec![Value::Date(0, false), Value::Time(0)]),
            Ok(Value::DateTime(0))
        ));
        let Err(Signal::Error(e)) = call("System.CreateDateTime#1", vec![Value::Date(0, false), Value::Time(1)]) else {
            panic!("a blank date with a time must raise");
        };
        assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            ("The date is not valid.", "DateInvalid")
        );
        // A closing date reads as its day (types/TestProbeScalarContracts.al is silent; text/TestProbeTextDateFormatContracts.al:
        // closing dates are dates here), which this build reads as the plain date.
        assert!(matches!(
            call("System.CreateDateTime#1", vec![Value::Date(5, true), Value::Time(1)]),
            Ok(Value::DateTime(_))
        ));
    }

    #[test]
    fn dt2date_and_dt2time_split_a_datetime() {
        // testpage/TestPageTemporalSetValue_Tests.al:117-118.
        let dt = call(
            "System.CreateDateTime#1",
            vec![date(2024, 3, 17), Value::Time(52_200_000 + 1)],
        )
        .unwrap();
        is_date(call("System.DT2Date#1", vec![dt.clone()]), 2024, 3, 17);
        assert!(matches!(
            call("System.DT2Time#1", vec![dt]),
            Ok(Value::Time(52_200_001))
        ));
        // text/TestProbeTextDateFormatContracts.al:Dates_DateTime_DT2DateTime, `dt2date_of_0DT`, `dt2time_of_0DT`: both blank.
        assert!(matches!(
            call("System.DT2Date#1", vec![Value::DateTime(0)]),
            Ok(Value::Date(0, false))
        ));
        assert!(matches!(
            call("System.DT2Time#1", vec![Value::DateTime(0)]),
            Ok(Value::Time(0))
        ));
    }

    // The instance forms give the same as the functions (assumed, registry entry datetime-date-time-methods).
    #[test]
    fn the_datetime_instance_methods_split_like_dt2date_and_dt2time() {
        let dt = call(
            "System.CreateDateTime#1",
            vec![date(2024, 3, 17), Value::Time(52_200_000 + 1)],
        )
        .unwrap();
        is_date(on("DateTime.Date#1", dt.clone()), 2024, 3, 17);
        assert!(matches!(on("DateTime.Time#1", dt), Ok(Value::Time(52_200_001))));
        assert!(matches!(
            on("DateTime.Date#1", Value::DateTime(0)),
            Ok(Value::Date(0, false))
        ));
        assert!(matches!(on("DateTime.Time#1", Value::DateTime(0)), Ok(Value::Time(0))));
    }

    #[test]
    fn dati2variant_is_a_datetime() {
        // session-system/TestSystemExtended.al:168.
        assert!(matches!(
            call("System.DaTi2Variant#1", vec![date(2024, 1, 1), Value::Time(43_200_001)]),
            Ok(Value::DateTime(_))
        ));
    }

    #[test]
    fn date2dmy_picks_a_part() {
        // session-system/TestCalcDateNotificationContracts.al:61 `Date2DMY(D, 1)` is the day, :79 `..., 2)` the
        // month; :32 `..., 3)` the year.
        let part = |n: i32| int(call("System.Date2DMY#1", vec![date(2024, 3, 15), Value::Integer(n)]));
        assert_eq!((part(1), part(2), part(3)), (15, 3, 2024));
        // types/TestDateRoundFormatStringContracts.al:Date2DMY_Part4_Throws.
        let Err(Signal::Error(e)) = call("System.Date2DMY#1", vec![date(2024, 3, 15), Value::Integer(4)]) else {
            panic!("part 4 must raise");
        };
        assert_eq!(e.code, "OutsidePermittedRange");
        assert!(
            e.message
                .ends_with("The current value is: 4. The permitted range is: from 1 to 3.")
        );
        // Date2DMY of the undefined date raises for part 1 (errors/PnBDateFormat.al, undefinedDmy); the
        // other parts are not pinned.
        let Err(Signal::Error(e)) = call("System.Date2DMY#1", vec![Value::Date(0, false), Value::Integer(1)]) else {
            panic!("0D must raise");
        };
        assert_eq!(e.code, "DateInvalid");
    }

    #[test]
    fn date2dwy_is_iso_8601() {
        // types/TestDateRoundFormatStringContracts.al:Date2DWY_WeekNumbers_AreIso8601.
        let part = |y, m, d, n| int(call("System.Date2DWY#1", vec![date(y, m, d), Value::Integer(n)]));
        assert_eq!(part(2024, 1, 1, 3), 2024);
        assert_eq!(part(2024, 1, 1, 2), 1);
        assert_eq!(part(2024, 12, 29, 2), 52);
        assert_eq!(part(2024, 12, 30, 2), 1);
        assert_eq!(part(2021, 1, 1, 2), 53);
        assert_eq!(part(2021, 1, 3, 2), 53);
        assert_eq!(part(2020, 12, 31, 2), 53);
        assert_eq!(part(2024, 3, 5, 1), 2);
        // The year is the ISO week-year (text/TestProbeTextDateFormatContracts.al:Dates_Date2DWY_WeekAndYear: 2027-01-01 is
        // week 53 of 2026).
        assert_eq!(part(2021, 1, 1, 3), 2020);
        assert_eq!(part(2027, 1, 1, 3), 2026);
    }

    #[test]
    fn normal_date_and_closing_date() {
        // record/TestDateVirtualTable.al:188-190 and record/TestFilterContracts.al:56.
        assert!(matches!(
            call("System.ClosingDate#1", vec![date(2024, 12, 31)]),
            Ok(Value::Date(n, true)) if n == day_from_civil(2024, 12, 31)
        ));
        let closing = call("System.ClosingDate#1", vec![date(2026, 3, 31)]).unwrap();
        is_date(call("System.NormalDate#1", vec![closing]), 2026, 3, 31);
        is_date(call("System.NormalDate#1", vec![date(2026, 3, 1)]), 2026, 3, 1);
        assert!(matches!(
            call("System.ClosingDate#1", vec![Value::Date(0, false)]),
            Err(Signal::Error(e)) if e.code == "DateInvalid"
        ));
    }

    #[test]
    fn day_of_week_counts_from_monday() {
        // session-system/TestSystemExtended.al:278 `20240101D.DayOfWeek()` is 1 (Monday).
        assert_eq!(int(on("Date.DayOfWeek#1", date(2024, 1, 1))), 1);
        assert_eq!(int(on("Date.DayOfWeek#1", date(2024, 1, 7))), 7);
        assert_eq!(int(on("Date.DayOfWeek#1", date(2024, 1, 3))), 3);
        assert!(matches!(
            on("Date.DayOfWeek#1", Value::Date(0, false)),
            Err(Signal::Error(_))
        ));
        assert_eq!(
            int(on("Date.DayOfWeek#1", Value::Date(day_from_civil(2024, 12, 30), true))),
            1
        );
        refused(on("Date.WeekNo#1", Value::Date(0, false)));
    }

    #[test]
    fn date_day_month_year_methods() {
        // format-evaluate/P93Date.al:51-78 (normal dates, 9999-12-31, 1753, leap days).
        let parts = |v: Value| {
            (
                int(on("Date.Day#1", v.clone())),
                int(on("Date.Month#1", v.clone())),
                int(on("Date.Year#1", v)),
            )
        };
        assert_eq!(parts(date(2024, 3, 15)), (15, 3, 2024));
        assert_eq!(parts(date(2024, 2, 29)), (29, 2, 2024));
        assert_eq!(parts(date(9999, 12, 31)), (31, 12, 9999));
        assert_eq!(parts(date(1753, 1, 1)), (1, 1, 1753));
        // P93Date.al:98-121: a closing date gives the parts of its day.
        let closing = call("System.ClosingDate#1", vec![date(2023, 12, 31)]).unwrap();
        assert_eq!(parts(closing), (31, 12, 2023));
        // P93Date.al:59-60: 0001-01-01 raises outside the part; not pinned where, so refused.
        refused(on("Date.Year#1", date(1, 1, 1)));
        // P93Date.al:81-95: the undefined date raises.
        for id in ["Date.Day#1", "Date.Month#1", "Date.Year#1"] {
            assert!(matches!(
                on(id, Value::Date(0, false)),
                Err(Signal::Error(e)) if e.code == "DateInvalid" && e.message == "The date is not valid."
            ));
        }
    }

    #[test]
    fn week_no_is_the_iso_week() {
        // session-system/TestSystemExtended.al:285-300 pins the range 1..53 and the first week of a year.
        assert_eq!(int(on("Date.WeekNo#1", date(2024, 1, 1))), 1);
        assert_eq!(int(on("Date.WeekNo#1", date(2024, 1, 15))), 3);
        assert_eq!(int(on("Date.WeekNo#1", date(2021, 1, 3))), 53);
        assert_eq!(int(on("Date.WeekNo#1", date(2024, 12, 30))), 1);
    }

    #[test]
    fn time_parts() {
        // session-system/TestMiscTypes.al:22-42.
        let t = Value::Time(((12 * 60 + 30) * 60 + 5) * 1000 + 250 + 1);
        assert_eq!(int(on("Time.Hour#1", t.clone())), 12);
        assert_eq!(int(on("Time.Minute#1", t.clone())), 30);
        assert_eq!(int(on("Time.Second#1", t.clone())), 5);
        assert_eq!(int(on("Time.Millisecond#1", t)), 250);
        refused(on("Time.Hour#1", Value::Time(0)));
    }

    #[test]
    fn calcdate_day_month_year_steps() {
        // types/TestDate.al:28-38 (`<+1D>`, `<-1D>`), session-system/TestDateTimeContracts.al:29 (`<-1M>`),
        // :38 and :47 (`<1Y>` clamps Feb 29 to Feb 28).
        is_date(calc("<+1D>", date(2024, 1, 1)), 2024, 1, 2);
        is_date(calc("<-1D>", date(2024, 1, 2)), 2024, 1, 1);
        is_date(calc("<1D>", date(2024, 2, 28)), 2024, 2, 29);
        is_date(calc("<+1D>", date(2024, 2, 29)), 2024, 3, 1);
        is_date(calc("<-1M>", date(2024, 3, 1)), 2024, 2, 1);
        is_date(calc("<1M>", date(2024, 1, 1)), 2024, 2, 1);
        is_date(calc("<1Y>", date(2024, 2, 29)), 2025, 2, 28);
        is_date(calc("<1Y>", date(2024, 2, 28)), 2025, 2, 28);
        is_date(calc("<-1M>", date(2024, 1, 15)), 2023, 12, 15);
    }

    #[test]
    fn calcdate_ends_of_periods() {
        // session-system/TestCalcDateNotificationContracts.al:16-80.
        is_date(calc("<CM>", date(2024, 3, 15)), 2024, 3, 31);
        is_date(calc("<CM>", date(2024, 2, 15)), 2024, 2, 29);
        is_date(calc("<-CM>", date(2024, 3, 15)), 2024, 3, 1);
        is_date(calc("<CY>", date(2024, 3, 15)), 2024, 12, 31);
        is_date(calc("<-CY>", date(2024, 3, 15)), 2024, 1, 1);
        is_date(calc("<CQ>", date(2024, 2, 15)), 2024, 3, 31);
    }

    #[test]
    fn calcdate_terms_apply_left_to_right() {
        // session-system/TestCalcDateNotificationContracts.al:30 `<1M-1D>` from the first of a month stays in it.
        is_date(calc("<1M-1D>", date(2024, 3, 1)), 2024, 3, 31);
        is_date(calc("<CM+1D>", date(2024, 3, 10)), 2024, 4, 1);
    }

    #[test]
    fn calcdate_refuses_forms_the_corpus_does_not_pin() {
        let from = date(2024, 3, 15);
        for f in [
            "<MON>", "<1W+MON>", "<1XY>", "<1D>M", "<-2WD>", "<1D+1WD>", "<+M>", "<1<D>",
        ] {
            refused(calc(f, from.clone()));
        }
        // A result off the calendar raises DateInvalid (types/date-time/CalcDate.al:474-477; `range` has the cells).
        for r in [calc("<1Y>", date(9999, 6, 1)), calc("<-1Y>", date(1, 6, 1))] {
            assert!(matches!(&r, Err(Signal::Error(e)) if e.code == "DateInvalid"), "{r:?}");
        }
        refused(call("System.CalcDate#2", vec![t("<1D>")]));
    }

    // types/date-time/DateFormulaEdges.al: from Monday 2024-01-15.
    #[test]
    fn calcdate_edges_of_the_grammar() {
        let mon = date(2024, 1, 15);
        for (f, d) in [
            ("<+%2D>", 17),
            ("%1D", 16),
            ("<-%1D>", 14),
            ("<1D", 16),
            ("<<1D>", 16),
            ("<1D>>", 16),
            ("8D+<1D>", 24),
            ("<8D>+<1D>", 24),
            ("2WD", 17),
            ("<1WD+1D>", 17),
            ("<", 15),
        ] {
            is_date(calc(f, mon.clone()), 2024, 1, d);
        }
        for (f, msg) in [
            ("<D>", "The date formula 'D' should include a number."),
            ("<++1D>", "The date formula '++1D' should include a number."),
            ("<+%D>", "The date formula '+%D' should include a number."),
            ("<WD>", "The date formula 'WD' should include a number."),
            // With the opening bracket and no closing one, the text keeps the bracket
            // (text/text-methods/P136Text.al:P136_CalcDate_OneAngleBracket `noNumberOpen`).
            ("<W", "The date formula '<W' should include a number."),
            ("<+5>", "The date formula '+5' should include a time unit."),
            ("01/01/26", "The date formula '01/01/26' should include a time unit."),
            (
                "2026-10-16",
                "The date formula '2026-10-16' should include a time unit.",
            ),
            ("10-16-26", "The date formula '10-16-26' should include a time unit."),
        ] {
            match calc(f, mon.clone()) {
                Err(Signal::Error(e)) => assert!(e.message.starts_with(msg), "{f}: {}", e.message),
                other => panic!("{f}: {other:?}"),
            }
        }
    }

    #[test]
    fn two_bracketed_terms_without_a_sign() {
        // types/date-time/P131CalcDate.al:111 (CalcDate) and :147 (Evaluate with its result used).
        let Err(Signal::Error(e)) = calc("<1D><1D>", date(2024, 1, 15)) else {
            panic!("`<1D><1D>` must raise");
        };
        assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            (
                "The date formula '1D><1D' must include plus (+) or minus (-).",
                "Evaluate"
            )
        );
        assert!(matches!(
            super::evaluate_formula("<1D><1D>"),
            Ok(super::Evaluated::Rejected(Some(_)))
        ));
        // A sign at the end (P131CalcDate.al:80-81, 116-117, 152; P132DateFormula.al:142 `<+>`).
        for (f, shown) in [
            ("<+", "<+"),
            ("<->", "-"),
            ("<+>", "+"),
            ("<1D>+", "<1D>+"),
            ("<1D+>", "1D+"),
        ] {
            let Err(Signal::Error(e)) = calc(f, date(2024, 1, 15)) else {
                panic!("`{f}` must raise");
            };
            assert_eq!(
                e.message,
                format!("The date formula '{shown}' should include a number.")
            );
        }
        assert!(matches!(
            super::evaluate_formula("<1D>+"),
            Ok(super::Evaluated::Rejected(None))
        ));
        assert!(matches!(calc("1D+", date(2024, 1, 15)), Err(Signal::Unsupported(_))));
        // Two bracketed terms, the second with a sign, join (al-runner issue 6648, `brackets::between_terms`).
        is_date(calc("<1D><+1D>", date(2024, 1, 15)), 2024, 1, 17);
        // Other places of a bracket inside a term stay refused.
        assert!(matches!(
            calc("<1D> <1D>", date(2024, 1, 15)),
            Err(Signal::Unsupported(_))
        ));
        assert!(matches!(calc("1D<1D", date(2024, 1, 15)), Err(Signal::Unsupported(_))));
    }

    #[test]
    fn calcdate_probe_forms_from_the_text_probe() {
        // text/TestProbeTextDateFormatContracts.al:Dates_CalcDate_Formulas.
        let mon = date(2024, 1, 15);
        let wed = date(2024, 1, 17);
        // A weekday formula on a day that already is that weekday moves a week.
        is_date(calc("<WD1>", mon.clone()), 2024, 1, 22);
        is_date(calc("<-WD1>", mon.clone()), 2024, 1, 8);
        is_date(calc("<WD3>", wed.clone()), 2024, 1, 24);
        is_date(calc("<cm>", mon.clone()), 2024, 1, 31);
        is_date(calc("<1d>", mon.clone()), 2024, 1, 16);
        is_date(calc("<1D+1W>", mon.clone()), 2024, 1, 23);
        // Spaces between the terms are ignored (Microsoft's Tests-SCM Item.TestField("Next Counting Start Date",
        // CalcDate('<CM + 1D>', WorkDate())) and Base Application's `<CM + 1D>`; the corpus pins `< 1D >`).
        is_date(calc("<1D + 1W>", mon.clone()), 2024, 1, 23);
        // A second term without a sign.
        let Err(Signal::Error(e)) = calc("<1D1W>", mon.clone()) else {
            panic!("a second term without a sign must raise");
        };
        assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            (
                "The date formula '1D1W' must include plus (+) or minus (-).",
                "Evaluate"
            )
        );
        // A closing date as the base: `<1D>` from the closing date of 2023-12-31 is 2024-01-01.
        is_date(
            calc("<1D>", Value::Date(day_from_civil(2023, 12, 31), true)),
            2024,
            1,
            1,
        );
        is_date(
            calc("<CM>", Value::Date(day_from_civil(2023, 12, 31), true)),
            2023,
            12,
            31,
        );
        // `WD8` is too large.
        let Err(Signal::Error(e)) = calc("<WD8>", mon) else {
            panic!("WD8 must raise");
        };
        assert_eq!(e.code, "Evaluate");
        assert!(
            e.message
                .starts_with("The date formula 'WD8' contains a number that is too large.")
        );
    }

    #[test]
    fn calcdate_forms_the_probe_pins() {
        // types/TestDateRoundFormatStringContracts.al:CalcDate_MonthYearClamping and
        // CalcDate_WeeksQuartersWeekDaysAndCurrentPeriods.
        let tue = date(2024, 3, 5);
        is_date(calc("<1M>", date(2024, 1, 31)), 2024, 2, 29);
        is_date(calc("<1M>", date(2024, 11, 30)), 2024, 12, 30);
        is_date(calc("<-1M>", date(2024, 3, 31)), 2024, 2, 29);
        is_date(calc("<1Y>", date(2024, 2, 29)), 2025, 2, 28);
        is_date(calc("<1W>", tue.clone()), 2024, 3, 12);
        is_date(calc("<CW>", tue.clone()), 2024, 3, 10);
        is_date(calc("<-CW>", tue.clone()), 2024, 3, 4);
        is_date(calc("<CD>", tue.clone()), 2024, 3, 5);
        is_date(calc("<1Q>", tue.clone()), 2024, 6, 5);
        is_date(calc("<-CQ>", tue.clone()), 2024, 1, 1);
        is_date(calc("<CQ>", tue.clone()), 2024, 3, 31);
        is_date(calc("<WD1>", tue.clone()), 2024, 3, 11);
        is_date(calc("<-WD1>", tue.clone()), 2024, 3, 4);
        is_date(calc("<WD7>", tue.clone()), 2024, 3, 10);
        is_date(calc("1M", tue.clone()), 2024, 4, 5);
        is_date(calc("<1M+CM>", tue.clone()), 2024, 4, 30);
        is_date(calc("<CM+1D>", tue.clone()), 2024, 4, 1);
        is_date(calc("<-1D-1M>", tue), 2024, 2, 4);
    }

    #[test]
    fn calcdate_of_an_undefined_date_raises() {
        // types/TestDateRoundFormatStringContracts.al:CalcDate_UndefinedDate_Throws.
        let Err(Signal::Error(e)) = calc("<1D>", Value::Date(0, false)) else {
            panic!("must raise");
        };
        assert_eq!(
            e.message,
            "You cannot base a date calculation on an undefined date.\\\\Date: 0D\\Formula: 1D."
        );
        assert_eq!(e.code, "Argument");
    }

    #[test]
    fn calcdate_takes_a_date_formula() {
        // types/TestDateFormulaContracts.al:45-125.
        let c = |f: &str, d| call("System.CalcDate#1", vec![Value::DateFormula(f.into()), d]);
        is_date(c("<1D>", date(2024, 1, 1)), 2024, 1, 2);
        is_date(c("<1M>", date(2024, 1, 1)), 2024, 2, 1);
        is_date(c("<-1D>", date(2024, 1, 2)), 2024, 1, 1);
        is_date(c("<1Y>", date(2024, 1, 1)), 2025, 1, 1);
        // types/TestDateFormulaDurationFormatContracts.al:30-43: the empty formula moves nothing.
        is_date(c("", date(2024, 1, 1)), 2024, 1, 1);
        is_date(call("System.CalcDate#2", vec![t(""), date(2024, 3, 5)]), 2024, 3, 5);
        // The empty formula on the undefined date raises the same error as any other formula
        // (types/TestDateFormulaEvaluateLeftoversContracts.al:CalcDate_EmptyFormula).
        let Err(Signal::Error(e)) = c("", Value::Date(0, false)) else {
            panic!("expected an AL error")
        };
        assert!(
            e.message
                .starts_with("You cannot base a date calculation on an undefined date."),
            "{}",
            e.message
        );
    }

    // types/TestDateFormulaDurationFormatContracts.al:28-74.
    #[test]
    fn evaluate_reads_the_blank_like_formulas_and_rejects_garbage() {
        let f = lookup("System.Evaluate#1").unwrap();
        let eval = |old: &str, text: &str| {
            let mut args = vec![Value::DateFormula(old.into()), t(text)];
            let mut ctx = TestCtx::default();
            let r = f(&mut ctx, None, &mut args);
            let Value::DateFormula(now) = &args[0] else { panic!() };
            (matches!(r, Ok(Value::Boolean(true))), super::formula_text(now))
        };
        assert_eq!(eval("<1D>", ""), (true, String::new()));
        assert_eq!(eval("<1D>", "<>"), (true, String::new()));
        assert_eq!(eval("<1D>", " "), (true, String::new()));
        assert_eq!(eval("", "0D"), (true, "0D".into()));
        assert_eq!(eval("", "<0D>"), (true, "0D".into()));
        // A failed Evaluate leaves the value.
        assert_eq!(eval("<0D>", "xyz"), (false, "0D".into()));
    }

    // types/TestDateFormulaEvaluateLeftoversContracts.al:35-69 (BC 27.5): what Format prints back and what
    // CalcDate(formula, 2024-01-31) gives, for the inputs the other tests leave open.
    #[test]
    fn evaluate_reads_lower_case_brackets_white_space_signs_and_a_letter_before_its_number() {
        let f = lookup("System.Evaluate#1").unwrap();
        let eval = |text: &str| {
            let mut args = vec![Value::DateFormula("<9D>".into()), t(text)];
            let r = f(&mut TestCtx::default(), None, &mut args);
            let Value::DateFormula(now) = &args[0] else { panic!() };
            match r {
                Ok(Value::Boolean(true)) => {
                    let shown = super::formula_text(now);
                    let d = call("System.CalcDate#1", vec![args[0].clone(), date(2024, 1, 31)]);
                    Ok((shown, d))
                }
                Ok(Value::Boolean(false)) => Err(()),
                other => panic!("{text}: {other:?}"),
            }
        };
        let ok = |text: &str, shown: &str, y, m, d| {
            let (s, r) = eval(text).unwrap_or_else(|()| panic!("{text} was rejected"));
            assert_eq!(s, shown, "{text}");
            is_date(r, y, m, d);
        };
        ok("1d", "1D", 2024, 2, 1);
        ok("1D", "1D", 2024, 2, 1);
        ok("< >", "", 2024, 1, 31);
        ok("-1W", "-1W", 2024, 1, 24);
        ok("CM", "CM", 2024, 1, 31);
        ok("<1M+CM>", "1M+CM", 2024, 2, 29);
        ok("+1D", "+1D", 2024, 2, 1);
        ok("D1", "D1", 2024, 2, 1);
        ok("1Y-1D", "1Y-1D", 2025, 1, 30);
        ok("WD1", "WD1", 2024, 2, 5);
        ok("<-WD1>", "-WD1", 2024, 1, 29);
        ok("CQ", "CQ", 2024, 3, 31);
        ok(" 1D ", "1D", 2024, 2, 1);
        ok("1 D", "1D", 2024, 2, 1);
        for bad in ["1X", "D", "<D>", "1d1w", "1D1W", "1", "LW", "1.5D"] {
            assert!(eval(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn evaluate_reads_a_date_formula() {
        // types/TestDateFormulaContracts.al:21,31,43,139: Evaluate then Format, CalcDate.
        let mut args = vec![Value::DateFormula(String::new()), t("<1D>")];
        let f = lookup("System.Evaluate#1").unwrap();
        assert!(matches!(
            f(&mut TestCtx::default(), None, &mut args),
            Ok(Value::Boolean(true))
        ));
        assert!(matches!(&args[0], Value::DateFormula(s) if s == "<1D>"));
        // A text with letters no formula uses is rejected (types/TestDateFormulaDurationFormatContracts.al:68).
        let mut args = vec![Value::DateFormula(String::new()), t("not a formula")];
        assert!(matches!(
            f(&mut TestCtx::default(), None, &mut args),
            Ok(Value::Boolean(false))
        ));
        // A formula with one bracket missing is accepted (text/TestProbeTextDateFormatContracts.al:DateFormula_EvaluateForms, `<1D`).
        let mut args = vec![Value::DateFormula(String::new()), t("<1D")];
        assert!(matches!(
            f(&mut TestCtx::default(), None, &mut args),
            Ok(Value::Boolean(true))
        ));
    }

    fn dwy(d: i32, w: i32, y: i32) -> Result<Value, Signal> {
        call(
            "System.DWY2Date#1",
            vec![Value::Integer(d), Value::Integer(w), Value::Integer(y)],
        )
    }
    #[track_caller]
    fn raises(r: Result<Value, Signal>, code: &str) {
        match r {
            Err(Signal::Error(e)) => assert_eq!(e.code, code),
            other => panic!("{other:?}"),
        }
    }

    // types/date-time/DateFuncs.al:311-346 DWY2Date_Valid.
    #[test]
    fn dwy2date_valid_cases_of_the_corpus() {
        for ((d, w, y), want) in [
            ((1, 1, 2024), (2024, 1, 1)),
            ((7, 1, 2024), (2024, 1, 7)),
            ((3, 3, 2024), (2024, 1, 17)),
            ((1, 1, 2021), (2021, 1, 4)),
            ((1, 1, 2020), (2019, 12, 30)),
            ((1, 53, 2020), (2020, 12, 28)),
            ((7, 53, 2020), (2021, 1, 3)),
            ((1, 52, 2024), (2024, 12, 23)),
            ((7, 52, 2024), (2024, 12, 29)),
            ((1, 1, 1), (1, 1, 1)),
            ((4, 53, 2015), (2015, 12, 31)),
            ((1, 1, 9999), (9999, 1, 4)),
            ((5, 52, 9999), (9999, 12, 31)),
        ] {
            if (d, w, y) == (1, 1, 1) {
                assert!(
                    matches!(dwy(d, w, y), Ok(Value::Date(0, false))),
                    "w1 1 is the blank date"
                );
                continue;
            }
            is_date(dwy(d, w, y), want.0, want.1, want.2);
        }
    }

    // types/date-time/DateFuncs.al:349-399 DWY2Date_Invalid.
    #[test]
    fn dwy2date_invalid_cases_of_the_corpus() {
        for (d, w, y) in [
            (0, 1, 2024),
            (8, 1, 2024),
            (-1, 1, 2024),
            (1, 0, 2024),
            (1, 54, 2024),
            (1, 1, 0),
            (1, 1, 10000),
            (1, 1, -5),
        ] {
            raises(dwy(d, w, y), "OutsidePermittedRange");
        }
        raises(dwy(1, 53, 2024), "DateInvalid");
        raises(dwy(1, 53, 2023), "DateInvalid");
        raises(dwy(7, 52, 9999), "DateInvalid");
        match dwy(8, 1, 2024) {
            Err(Signal::Error(e)) => assert_eq!(
                e.message,
                "The value of DWY2Date parameter 1 is outside of the permitted range. The current value is: 8. The permitted range is: from 1 to 7."
            ),
            other => panic!("{other:?}"),
        }
    }

    // Two parts outside their range, a non-Integer part, and a missing Week or Year without a host clock are not pinned.
    #[test]
    fn dwy2date_refuses_what_the_corpus_does_not_pin() {
        refused(dwy(0, 0, 2024));
        refused(call("System.DWY2Date#1", vec![Value::Integer(3)]));
        refused(call("System.DWY2Date#1", vec![Value::Integer(3), Value::Integer(10)]));
        refused(call(
            "System.DWY2Date#1",
            vec![t("x"), Value::Integer(1), Value::Integer(2024)],
        ));
    }
    // types/date-time/DateFormulaSignedWd.al:41-136; types/date-time/CalcDate.al:100,117,188,204,463-480.
    #[test]
    fn calcdate_signed_working_day_percent_units_and_letter_errors() {
        let fri = date(2024, 1, 12);
        is_date(calc("+1WD", fri.clone()), 2024, 1, 13);
        is_date(calc("<-1WD>", fri.clone()), 2024, 1, 11);
        let err = |f: &str| match calc(f, fri.clone()) {
            Err(Signal::Error(e)) => (e.message, e.code),
            other => panic!("{f}: {other:?}"),
        };
        assert_eq!(
            err("<%Q>"),
            (
                "The date formula '%Q' should include a number.".to_string(),
                "Evaluate".to_string()
            )
        );
        assert_eq!(
            err("%Y"),
            (
                "The date formula '%Y' should include a number.".to_string(),
                "Evaluate".to_string()
            )
        );
        assert!(
            err("abc")
                .0
                .starts_with("The date formula 'ABC' should include a time unit.")
        );
        assert!(
            err("<1X>")
                .0
                .starts_with("The date formula '1X' should include a time unit.")
        );
        assert_eq!(err("<2CW>").1, crate::native::UNPINNED_ERROR);
        assert_eq!(err("<WD0>").1, crate::native::UNPINNED_ERROR);
    }

    // `Mn`: the first day of month n after the month of the date (`-Mn`: before it), corpus master
    // types/date-time/DateFormulaEdges.al:433-505 (TV_MonthNumber_*).
    #[test]
    fn a_month_letter_before_its_number_goes_to_the_first_of_that_month() {
        for (y, m, d, want) in [
            (2024, 1, 10, (2024, 2, 1)),
            (2024, 1, 31, (2024, 2, 1)),
            (2024, 2, 29, (2025, 2, 1)),
            (2023, 2, 28, (2024, 2, 1)),
            (2024, 4, 30, (2025, 2, 1)),
            (2024, 12, 31, (2025, 2, 1)),
        ] {
            is_date(calc("<M2>", date(y, m, d)), want.0, want.1, want.2);
        }
        is_date(calc("M2", date(2024, 1, 15)), 2024, 2, 1);
        is_date(calc("<M1>", date(2024, 1, 10)), 2025, 1, 1);
        is_date(calc("<M1>", date(2023, 2, 28)), 2024, 1, 1);
        is_date(calc("<M12>", date(2024, 1, 10)), 2024, 12, 1);
        is_date(calc("<M12>", date(2024, 12, 31)), 2025, 12, 1);
        is_date(calc("<-M2>", date(2024, 1, 15)), 2023, 2, 1);
        is_date(calc("<-M2>", date(2024, 4, 30)), 2024, 2, 1);
        is_date(calc("<-M2>", date(2024, 2, 29)), 2023, 2, 1);
        is_date(calc("<CM+M2>", date(2024, 1, 15)), 2024, 2, 1);
        is_date(calc("<1Y+M2>", date(2024, 1, 15)), 2025, 2, 1);
        for (f, shown) in [("<M0>", "M0"), ("<M13>", "M13")] {
            let Err(Signal::Error(e)) = calc(f, date(2024, 1, 15)) else {
                panic!("{f} must raise")
            };
            assert_eq!(
                e.message,
                format!(
                    "The date formula '{shown}' contains a number that is too large.\\\\The number must be in the range 1 - 12."
                )
            );
            assert_eq!(e.code, "Evaluate");
        }
        let Err(Signal::Error(e)) = calc("<2M2>", date(2024, 1, 15)) else {
            panic!("2M2 must raise")
        };
        assert_eq!(e.message, "The date formula '2M2' must include plus (+) or minus (-).");
        // Forward from the first day of month n: the next year's (corpus master, Np4DateFormulas.al NP4_CalcDate_M1); backward
        // is not pinned.
        is_date(calc("<M2>", date(2024, 2, 1)), 2025, 2, 1);
        refused(calc("<-M2>", date(2024, 2, 1)));
        refused(calc("<M14>", date(2024, 1, 15)));
    }
}
