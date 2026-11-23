//! What a length does in `Format`, after the type rules of `format_value` and the format strings of `formatstr`: only the
//! shapes the corpus measured run, every other value and length refuses (al-runner issue 8389).

use super::{FormatCtx, FormatFail, format_value, pad};
use crate::describe::Describe;
use crate::value::Value;
use std::cmp::Ordering;

/// The text of `v` (`s`) in standard format `standard` with a length no type rule of `format_value` took. A length of 0
/// keeps the text.
pub(super) fn rest(v: &Value, standard: i32, s: String, length: i32) -> Result<String, FormatFail> {
    let fit = fit(s.chars().count(), length);
    if length == 0 || measured(v, standard, fit, length > 0) || measured_length(v, standard, fit, length) {
        return Ok(pad(s, length));
    }
    Err(FormatFail::Unsupported(format!(
        "Format of {} with length {length} in standard format {standard} (not pinned)",
        v.describe()
    )))
}

/// How a text of `chars` characters compares with the length.
fn fit(chars: usize, length: i32) -> Ordering {
    chars.cmp(&(length.unsigned_abs() as usize))
}

/// The pinned shapes, each by its value type, the standard format number, the sign of the length and how the text's length
/// compares with it. A cited line measures each whole tuple.
fn measured(v: &Value, standard: i32, fit: Ordering, positive: bool) -> bool {
    use Ordering::{Equal, Less};
    match (v, standard, positive, fit) {
        // A shorter number is right-aligned in a positive length: text/TestFormat.al:59 (`Format(42, 5)`),
        // format-evaluate/RandomFormat.al:161 (Integer, format 0); types/decimal-integer/ScT70050.al:182, :186-188 (Decimal,
        // formats 0, 1, 2 and 9).
        (Value::Integer(_), 0, true, Less) => true,
        (Value::Decimal(_), 0 | 1 | 2 | 9, true, Less) => true,
        // A defined Date in format 9: shorter is right-aligned, as long as the length stays (types/date-time/Format.al:531, :530).
        (Value::Date(day, false), 9, true, Less | Equal) => *day != 0,
        // A defined Date as long as the length in format 0, a closing one too, and in format 9 (format-evaluate/format-length/
        // FormatLengthStrings.al:341-342, :348-349, :345); a Duration in format 0 (:352-353). Both signs.
        (Value::Date(day, _), 0, _, Equal) => *day != 0,
        (Value::Date(day, false), 9, false, Equal) => *day != 0,
        (Value::Duration(_), 0, _, Equal) => true,
        // A defined DateTime in format 9, shorter: right-aligned (types/date-time/Format.al:535).
        (Value::DateTime(stamp), 9, true, Less) => *stamp != 0,
        // A Text as long as a positive length stays (types/date-time/P146FormatCalcDate.al:30, `'<+%1D>'` in 6).
        (Value::Text(_), 0, true, Equal) => true,
        // A negative length longer than the text keeps it, in format 0: format-evaluate/RandomFormat.al:162 (Integer), :315
        // (Text); types/decimal-integer/ScT70050.al:183 (Decimal); types/decimal-integer/ScT70053.al:75 (Boolean), :186
        // (Option); codeunits/interfaces/OjT71038.al:205 (Enum).
        (
            Value::Integer(_)
            | Value::Decimal(_)
            | Value::Text(_)
            | Value::Boolean(_)
            | Value::Option(..)
            | Value::Enum(..),
            0,
            false,
            Less,
        ) => true,
        _ => false,
    }
}

/// The pinned shapes where a cited line measures only some lengths, each by its value type, the standard format number and
/// the lengths (or the sign of the length and how the text compares with it).
fn measured_length(v: &Value, standard: i32, fit: Ordering, length: i32) -> bool {
    // The lengths the cells of 0D in formats 0 and 9 and of 0DT in format 0 measure, each with both signs.
    let measured_undefined = matches!(length.unsigned_abs(), 1 | 3 | 8 | 10 | 30);
    match (v, standard, length > 0, fit) {
        // 0D: blanks in a positive length, '' in a negative one; L = ±1, 3, 8, 10, 30 in formats 0 and 9, L = 10 in 1 and 2
        // (FormatLengthDates.al:62-72, :99-113, :162-168).
        (Value::Date(0, false), 0 | 9, _, _) => measured_undefined,
        (Value::Date(0, false), 1 | 2, true, _) => length == 10,
        // 0DT the same: L = ±1, 3, 8, 10, 30 in format 0; L = 3, 10, 30, -3, -30 in format 9 (FormatLengthDates.al:352-366,
        // :169-171).
        (Value::DateTime(0), 0, _, _) => measured_undefined,
        (Value::DateTime(0), 9, _, _) => matches!(length, 3 | 10 | 30 | -3 | -30),
        // A Text as long as a negative length stays (format-evaluate/format-length/FormatLengthTypes.al:888).
        (Value::Text(_), 0, false, Ordering::Equal) => true,
        _ => false,
    }
}

/// The format string of `R01` (pages/list-key-navigation/SgListKeyMovesTests.al:54).
const INTEGER_2_FILLER_0: &str = "<Integer,2><Filler Character,0>";
/// The format string of Format.al:855 (`0T <Hours,2>:<Minutes,2>:<Seconds,2>`).
const HOURS_MINUTES_SECONDS: &str = "<Hours,2>:<Minutes,2>:<Seconds,2>";
/// The format string of types/date-time/Format.al:537 (`date len 5 custom`).
const YEAR4_MONTH: &str = "<Year4>-<Month,2>";
/// The format string of types/date-time/Format.al:1281, :1286, :1298, :1315.
const SECOND_DEC: &str = "<Second dec.>";
/// The format string of format-evaluate/format-length/FormatLengthDates.al:178-223.
const YEAR4_MONTH_DAY: &str = "<Year4>-<Month,2>-<Day,2>";

/// `Format(v, length, template)` gave a text of `chars` characters with length 0: whether the format string's text may take
/// the length. The pinned shapes, each by its value type, the sign of the length and how the text compares with it, and the
/// string where a cited line measures only one. A template that is one `<Standard Format,N>` code is not this function's:
/// `lone_standard_format`.
pub(crate) fn with_format_string(v: &Value, length: i32, template: &str, chars: usize) -> Result<(), String> {
    use Ordering::{Equal, Greater, Less};
    let fit = fit(chars, length);
    let measured = match (v, length > 0, fit) {
        // A Decimal, shorter (right-aligned), as long, or longer (cut): format-evaluate/P86FormatDecimal.al:1078-1096
        // (Len_WithFormatString, `l1`-`l9`), format-evaluate/FormatCommaMore.al:601 (length 12); language/
        // R6901DecimalFieldFormat.al:87-88, :102-103, :129 (`0000.000` and `0001****` in 8).
        (Value::Decimal(_), true, Less | Equal | Greater) => true,
        // An Integer as long as the length: `R01` from `'R' + Format(1, 2, '<Integer,2><Filler Character,0>')`
        // (pages/list-key-navigation/SgListKeyMovesTests.al:54, :228; testpage/WdModalGoToKeyDescSecondary.al:138, :154).
        (Value::Integer(_), true, Equal) => template == INTEGER_2_FILLER_0,
        // A defined Date, longer: cut, only under the measured string (types/date-time/Format.al:537, `date len 5 custom`).
        (Value::Date(day, false), true, Greater) => *day != 0 && template == YEAR4_MONTH,
        // The undefined DateTime's empty text in a positive length: blanks, only under the measured string (Format.al:1298, :1315).
        (Value::DateTime(0), true, Less) => template == SECOND_DEC,
        // A defined Time, shorter, as long and longer (types/date-time/Format.al:873, :885, :912, :918, TU_Time_Format_CodeNames;
        // :1279-1280, :1284-1285); a negative length longer than the text keeps it, only under the measured string (:1281, :1286).
        (Value::Time(t), true, Less | Equal | Greater) if *t != 0 => true,
        (Value::Time(t), false, Less) if *t != 0 => template == SECOND_DEC,
        // The undefined Time as long as a positive length, only under the measured string: Format.al:855, :917 (` 0:00:00` in 8).
        (Value::Time(0), true, Equal) => template == HOURS_MINUTES_SECONDS,
        _ => false,
    };
    if length == 0 || measured || string_length_cell(v, length, template, chars) {
        return Ok(());
    }
    if string_cell(integer_kind(v), INTEGER_STRING_CELLS, template, length > 0, fit) {
        crate::assumptions::assume(crate::assumptions::Assumption::FormatLengthIntegerString);
        return Ok(());
    }
    if string_cell(temporal_kind(v), DATE_STRING_CELLS, template, length > 0, fit) {
        crate::assumptions::assume(crate::assumptions::Assumption::FormatLengthDateString);
        return Ok(());
    }
    Err(format!(
        "Format of {} with length {length} and a format string (not pinned)",
        v.describe()
    ))
}

/// Whether a value is an Integer (`false`) or a BigInteger (`true`), for the cells of `INTEGER_STRING_CELLS`.
fn integer_kind(v: &Value) -> Option<bool> {
    match v {
        Value::Integer(_) => Some(false),
        Value::BigInteger(_) => Some(true),
        _ => None,
    }
}

/// The cells of corpus PR 1505 (open; format-evaluate/format-length/FormatLengthIntegerString.al, assumption
/// `format-length-integer-string`): BigInteger or not, the exact format string, a positive length or not, and how the text
/// without a length compares with it. The probe line of each cell follows it. A positive length pads a shorter text on the
/// left and cuts a longer one; a negative length keeps a shorter text and cuts a longer one (the cut: `formatstr::format_codes`).
const INTEGER_STRING_CELLS: &[(bool, &str, bool, Ordering)] = {
    use Ordering::{Equal, Greater, Less};
    &[
        (false, "<Integer>", true, Less),           // :26, :28, :38, :41, :68, :71-72, :77, :169
        (false, "<Integer>", true, Equal),          // :27, :33, :37, :69, :73
        (false, "<Integer>", true, Greater),        // :34, :36, :42, :76
        (false, "<Integer>", false, Less),          // :30-31, :40
        (false, "<Integer>", false, Greater),       // :39
        (false, "<Sign><Integer>", true, Less),     // :79, :81, :84
        (false, "<Sign><Integer>", true, Equal),    // :78, :82
        (false, INTEGER_2_FILLER_0, true, Less),    // :109
        (false, INTEGER_2_FILLER_0, true, Greater), // :110
        (false, "<Integer,4>", true, Less),         // :111
        (false, "<Integer,4>", true, Greater),      // :112
        (false, "<Integer,5><Filler Character,0>", true, Less), // :115
        (false, "<Integer,5><Filler Character,0>", true, Equal), // :118
        (false, "<Integer,5><Filler Character,0>", true, Greater), // :114, :117
        (false, "N<Integer>", true, Less),          // :139
        (false, "N<Integer>", true, Equal),         // :140
        (false, "No. <Integer> of 9", true, Less),  // :142
        (false, "No. <Integer> of 9", true, Greater), // :143
        (false, "<Integer>.<Integer>", true, Equal), // :144
        (false, "abc", true, Less),                 // :145
        (false, "", true, Less),                    // :146
        (false, "<Integer Thousand>", true, Less),  // :165, :171, :173
        (false, "<Integer Thousand>", true, Equal), // :166
        (false, "<Integer Thousand>", true, Greater), // :167, :174
        (true, "<Integer>", true, Less),            // :193, :199, :205
        (true, "<Integer>", true, Equal),           // :197
        (true, "<Integer>", true, Greater),         // :196, :204
        (true, INTEGER_2_FILLER_0, true, Equal),    // :201
        (true, INTEGER_2_FILLER_0, true, Less),     // :202
        (true, "<Integer Thousand>", true, Greater), // :206
    ]
};

/// Whether a result longer than `length` is cut to it, by the value type of `with_format_string`: a Decimal or BigInteger in a
/// positive length; an Integer (the `INTEGER_STRING_CELLS` rule), a Date, a Time and a DateTime (the `DATE_STRING_CELLS` rule) in
/// a positive or a negative one. A negative length cuts a Date and a Time under a lone `<Standard Format,0>` too
/// (format-evaluate/format-length/FormatLengthNeighbors.al:312, :369-370). Read by `formatstr::format_codes`; the gate decides
/// separately whether the call runs.
pub(crate) fn cuts_to_length(v: &Value, length: i32) -> bool {
    match v {
        Value::Decimal(_) | Value::BigInteger(_) => length > 0,
        Value::Integer(_) | Value::Date(..) | Value::Time(_) | Value::DateTime(_) => length != 0,
        _ => false,
    }
}

/// The kind of a Date, Time or DateTime that the cells of `DATE_STRING_CELLS` name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Temporal {
    Date,
    ClosingDate,
    UndefinedDate,
    Time,
    UndefinedTime,
    DateTime,
    UndefinedDateTime,
}

fn temporal_kind(v: &Value) -> Option<Temporal> {
    Some(match v {
        Value::Date(0, false) => Temporal::UndefinedDate,
        Value::Date(_, false) => Temporal::Date,
        Value::Date(0, true) => return None,
        Value::Date(_, true) => Temporal::ClosingDate,
        Value::Time(0) => Temporal::UndefinedTime,
        Value::Time(_) => Temporal::Time,
        Value::DateTime(0) => Temporal::UndefinedDateTime,
        Value::DateTime(_) => Temporal::DateTime,
        _ => return None,
    })
}

const DATE_COMPACT: &str = "<Year4><Month,2><Day,2>";
const DATE_DOTS: &str = "<Day,2>.<Month,2>.<Year4>";
const DATE_YEAR2: &str = "<Year><Month,2><Day,2>";
const TIME_COMPACT: &str = "<Hours24,2><Minutes,2><Seconds,2>";
const TIME_HOURS_MINUTES: &str = "<Hours24,2>:<Minutes,2>";
const DATETIME_COMPACT: &str = "<Year4><Month,2><Day,2><Hours24,2><Minutes,2><Seconds,2>";
const DATETIME_SEPARATED: &str = "<Year4>-<Month,2>-<Day,2> <Hours24,2>:<Minutes,2>";

/// The cells of corpus PR 1557 (open; format-evaluate/format-length/FormatLengthDateString.al, assumption
/// `format-length-date-string`): the kind of value, the exact format string, a positive length or not, and how the text without
/// a length compares with it. The probe lines of each cell follow it. A length shorter than the text cuts it (either sign), a
/// longer positive length pads it on the left, a longer negative length keeps it (the cut: `cuts_to_length`). A defined Time
/// in a positive length has no row: the pinned arm of `with_format_string` runs it under any string (Format.al:873, :885). The
/// lengths of `STRING_LENGTH_CELLS` run clean before these rows.
const DATE_STRING_CELLS: &[(Temporal, &str, bool, Ordering)] = {
    use Ordering::{Equal, Greater, Less};
    use Temporal::*;
    &[
        (Date, DATE_COMPACT, true, Equal),                  // :27, :55
        (Date, DATE_COMPACT, true, Greater),                // :29, :56-58
        (Date, DATE_COMPACT, true, Less),                   // :30, :59-61
        (Date, DATE_COMPACT, false, Equal),                 // :31, :62
        (Date, DATE_COMPACT, false, Greater),               // :32, :63-65
        (Date, DATE_COMPACT, false, Less),                  // :33, :66-68
        (ClosingDate, DATE_COMPACT, true, Equal),           // :69
        (ClosingDate, DATE_COMPACT, true, Less),            // :70
        (ClosingDate, DATE_COMPACT, false, Equal),          // :71
        (UndefinedDate, DATE_COMPACT, true, Less),          // :73-75
        (UndefinedDate, DATE_COMPACT, false, Less),         // :76-77
        (Date, DATE_DOTS, true, Equal),                     // :117
        (Date, DATE_DOTS, true, Greater),                   // :118-119
        (Date, DATE_DOTS, true, Less),                      // :120
        (Date, DATE_DOTS, false, Equal),                    // :121
        (Date, DATE_DOTS, false, Greater),                  // :122
        (Date, DATE_DOTS, false, Less),                     // :123
        (Date, DATE_YEAR2, true, Equal),                    // :126
        (Date, DATE_YEAR2, true, Greater),                  // :127
        (Date, DATE_YEAR2, true, Less),                     // :128
        (Date, DATE_YEAR2, false, Equal),                   // :129
        (Date, DATE_YEAR2, false, Greater),                 // :130
        (Date, DATE_YEAR2, false, Less),                    // :131
        (Date, YEAR4_MONTH_DAY, true, Greater),             // :133
        (Date, YEAR4_MONTH_DAY, true, Less),                // :134
        (Date, YEAR4_MONTH_DAY, false, Greater),            // :135-137
        (Date, YEAR4_MONTH_DAY, false, Less),               // :138-139
        (Time, TIME_COMPACT, false, Equal),                 // :181
        (Time, TIME_COMPACT, false, Greater),               // :182
        (Time, TIME_COMPACT, false, Less),                  // :183
        (UndefinedTime, TIME_COMPACT, true, Equal),         // :185
        (UndefinedTime, TIME_COMPACT, true, Greater),       // :186
        (UndefinedTime, TIME_COMPACT, true, Less),          // :187
        (UndefinedTime, TIME_COMPACT, false, Equal),        // :188
        (UndefinedTime, TIME_COMPACT, false, Less),         // :189
        (Time, TIME_HOURS_MINUTES, false, Equal),           // :195
        (Time, TIME_HOURS_MINUTES, false, Greater),         // :196
        (Time, TIME_HOURS_MINUTES, false, Less),            // :197
        (DateTime, DATETIME_COMPACT, true, Equal),          // :234
        (DateTime, DATETIME_COMPACT, true, Greater),        // :235-236
        (DateTime, DATETIME_COMPACT, true, Less),           // :237-238
        (DateTime, DATETIME_COMPACT, false, Equal),         // :239
        (DateTime, DATETIME_COMPACT, false, Greater),       // :240
        (DateTime, DATETIME_COMPACT, false, Less),          // :241
        (UndefinedDateTime, DATETIME_COMPACT, true, Less),  // :243-245
        (UndefinedDateTime, DATETIME_COMPACT, false, Less), // :246-247
        (DateTime, DATETIME_SEPARATED, true, Equal),        // :277
        (DateTime, DATETIME_SEPARATED, true, Greater),      // :278
        (DateTime, DATETIME_SEPARATED, true, Less),         // :279
        (DateTime, DATETIME_SEPARATED, false, Equal),       // :280
        (DateTime, DATETIME_SEPARATED, false, Greater),     // :281
        (DateTime, DATETIME_SEPARATED, false, Less),        // :282
    ]
};

/// Whether a value of `kind` under `template`, by the sign of the length and the fit, is one of `cells` (`INTEGER_STRING_CELLS`
/// with `integer_kind`, `DATE_STRING_CELLS` with `temporal_kind`).
fn string_cell<'a, K: PartialEq>(
    kind: Option<K>,
    cells: &[(K, &'a str, bool, Ordering)],
    template: &'a str,
    positive: bool,
    fit: Ordering,
) -> bool {
    kind.is_some_and(|kind| cells.contains(&(kind, template, positive, fit)))
}

/// The format string of format-evaluate/format-length/FormatLengthStrings.al:47 (`DayMonthYear2Txt`).
const DAY_MONTH_YEAR2: &str = "<Day,2><Month,2><Year,2>";
/// The format string of FormatLengthStrings.al:743 and format-evaluate/format-length/FormatLengthDates.al:130.
const CLOSING_DAY: &str = "<Closing><Day>";
const STANDARD_FORMAT_0: &str = "<Standard Format,0>";
const STANDARD_FORMAT_9: &str = "<Standard Format,9>";

/// The pinned cells under a format string where a cited line measures only some lengths: the kind of value, the exact format
/// string, the length of the text the string gives without a length, and the lengths measured. A shorter length cuts the text
/// (either sign), a longer positive one pads it on the left, a longer negative one keeps it; an undefined value's text is empty
/// (the cut: `cuts_to_length`). FormatLengthStrings.al (codeunit 96391): `DateLadder` (:1065-1084) measures, for the text's
/// length x, 1, x, x + 3, x - 1 and x - 2 with both signs (only 1 when x is below 3); `UndefinedLadder` (:1086-1100) 1, 5 and
/// 30 on 0DT and 5 on 0D, with both signs.
const STRING_LENGTH_CELLS: &[(Temporal, &str, usize, &[i32])] = {
    use Temporal::*;
    const UNDEFINED_LADDER: &[i32] = &[1, 5, 30, -1, -5, -30];
    &[
        // FormatLengthStrings.al:639-657.
        (Date, YEAR4_MONTH, 7, &[1, 5, 6, 7, 10, -1, -5, -6, -7, -10]),
        // FormatLengthDates.al:206-214, :219-223, :445-447 (WorkDate and 2024-01-15); FormatLengthStrings.al:659-677.
        (
            Date,
            YEAR4_MONTH_DAY,
            10,
            &[1, 3, 8, 9, 10, 11, 12, 13, -1, -3, -8, -9, -10, -12, -13],
        ),
        // FormatLengthStrings.al:82-103 (two dates), :171-175 (a Date field), :679-697.
        (
            Date,
            DAY_MONTH_YEAR2,
            6,
            &[1, 4, 5, 6, 7, 8, 9, 12, -1, -4, -5, -6, -7, -8, -9, -12],
        ),
        // FormatLengthStrings.al:699-717, :338-339.
        (Date, STANDARD_FORMAT_0, 8, &[1, 6, 7, 8, 11, -1, -6, -7, -8, -11]),
        // FormatLengthStrings.al:719-737, :343-344.
        (Date, STANDARD_FORMAT_9, 10, &[1, 8, 9, 10, 13, -1, -8, -9, -10, -13]),
        // FormatLengthStrings.al:739-749.
        (Date, CLOSING_DAY, 2, &[1, -1]),
        // A closing Date: FormatLengthStrings.al:142-148 (no C under the string), :346-347.
        (ClosingDate, DAY_MONTH_YEAR2, 6, &[4, 6, 8, -4, -6, -8]),
        (ClosingDate, STANDARD_FORMAT_0, 9, &[9, -9]),
        // 0D: FormatLengthStrings.al:132-141, :176-177 (a Date field), :789-806.
        (UndefinedDate, DAY_MONTH_YEAR2, 0, &[1, 4, 5, 6, 8, -1, -4, -5, -6, -8]),
        // FormatLengthDates.al:132-136; FormatLengthStrings.al:770-787.
        (UndefinedDate, YEAR4_MONTH_DAY, 0, &[3, 5, 10, 30, -3, -5, -10]),
        // FormatLengthStrings.al:1003-1010 (`UndefinedDate_Templates_Length`), :808-844 (L = 5); FormatLengthDates.al:137.
        (UndefinedDate, YEAR4_MONTH, 0, &[7, -7]),
        (UndefinedDate, STANDARD_FORMAT_0, 0, &[5, 8, -5, -8]),
        (UndefinedDate, STANDARD_FORMAT_9, 0, &[5, 10, -5, -10]),
        (UndefinedDate, CLOSING_DAY, 0, &[1, 10, -3]),
        // 0DT: FormatLengthStrings.al:751-863, :970-988 (a DateTime field).
        (UndefinedDateTime, SECOND_DEC, 0, UNDEFINED_LADDER),
        (UndefinedDateTime, YEAR4_MONTH_DAY, 0, UNDEFINED_LADDER),
        (UndefinedDateTime, DAY_MONTH_YEAR2, 0, UNDEFINED_LADDER),
        (UndefinedDateTime, STANDARD_FORMAT_0, 0, UNDEFINED_LADDER),
        (
            UndefinedDateTime,
            STANDARD_FORMAT_9,
            0,
            &[1, 5, 10, 30, -1, -5, -10, -30],
        ),
        (UndefinedDateTime, TIME_HOURS_MINUTES, 0, UNDEFINED_LADDER),
    ]
};

/// Whether `Format(v, length, template)`, whose text without a length has `chars` characters, is a cell of `STRING_LENGTH_CELLS`.
fn string_length_cell(v: &Value, length: i32, template: &str, chars: usize) -> bool {
    temporal_kind(v).is_some_and(|kind| {
        STRING_LENGTH_CELLS
            .iter()
            .any(|&(k, t, x, lengths)| k == kind && t == template && x == chars && lengths.contains(&length))
    })
}

/// `Format(v, length, '<Standard Format,N>')`, a template of that one code, gave `out`: `None` for any other template. The
/// string takes the gate of `Format(v, length, N)` and runs only where that call gives the same text: pinned for a Decimal
/// shorter than a positive length in formats 0, 1 and 9 (format-evaluate/P86FormatDecimal.al:1083, :1084, :1086), and for the
/// types, numbers and lengths of format-evaluate/format-length/FormatLengthNeighbors.al with a text shorter or longer than the
/// length, and for a Duration as long as it. The cells of `STRING_LENGTH_CELLS` under the string run as `format_codes` gives
/// them, without `Format(v, length, N)`.
pub(crate) fn lone_standard_format(
    v: &Value,
    length: i32,
    template: &str,
    out: &str,
    ctx: &dyn FormatCtx,
) -> Option<Result<(), String>> {
    let standard: i32 = template
        .strip_prefix("<Standard Format,")?
        .strip_suffix('>')?
        .parse()
        .ok()?;
    let refused = || {
        format!(
            "Format of {} with length {length} and the format string {template} (not pinned)",
            v.describe()
        )
    };
    let Ok(bare) = format_value(v, 0, standard, ctx) else {
        return Some(Err(refused()));
    };
    if string_length_cell(v, length, template, bare.chars().count()) {
        return Some(Ok(()));
    }
    let plain = match format_value(v, length, standard, ctx) {
        Ok(plain) => plain,
        Err(FormatFail::Unsupported(why)) => return Some(Err(why)),
        Err(FormatFail::Error { .. }) => return Some(Err(refused())),
    };
    let fit = fit(bare.chars().count(), length);
    let decimal = matches!(
        (v, standard, length > 0, fit),
        (Value::Decimal(_), 0 | 1 | 9, true, Ordering::Less)
    );
    // format-evaluate/format-length/FormatLengthNeighbors.al:300-318, :357-375, :407-420, :452-465 (format 0, lengths 10, 3,
    // -3, -10), :486-492 (format 9 on a Date, lengths 10, 3, 12, -12, -10).
    let neighbors = matches!(fit, Ordering::Less | Ordering::Greater)
        && matches!(
            (v, standard, length),
            (
                Value::Date(..)
                    | Value::DateTime(0)
                    | Value::Duration(_)
                    | Value::Char(_)
                    | Value::Time(_)
                    | Value::Boolean(_)
                    | Value::Guid(_)
                    | Value::BigInteger(_)
                    | Value::Byte(_)
                    | Value::Integer(_)
                    | Value::Decimal(_)
                    | Value::Code(_),
                0,
                10 | 3 | -3 | -10,
            ) | (Value::Date(_, false), 9, 10 | 3 | 12 | -12 | -10)
        );
    // A Duration as long as the length, both signs (format-evaluate/format-length/FormatLengthStrings.al:350-351).
    let duration = matches!((v, standard, fit), (Value::Duration(_), 0, Ordering::Equal));
    if plain == out && (decimal || neighbors || duration) {
        return Some(Ok(()));
    }
    Some(Err(refused()))
}

/// A length no cited line measures where `what` is formatted: `text` with a length of 0, else refused.
pub(crate) fn none_measured(what: &str, text: String, length: i32) -> Result<String, FormatFail> {
    if length == 0 {
        return Ok(text);
    }
    Err(FormatFail::Unsupported(format!(
        "Format of {what} with length {length} (not pinned)"
    )))
}

#[cfg(test)]
mod tests {
    use super::super::{FormatCtx, day_from_civil, format_value};
    use super::*;
    use rust_decimal::Decimal;

    struct Ctx(Option<&'static str>);
    impl FormatCtx for Ctx {
        fn enum_name(&self, _: u32, ordinal: i32) -> String {
            ["Red", "Blue"][ordinal as usize].to_string()
        }
        fn region(&self) -> Option<String> {
            self.0.map(str::to_string)
        }
    }

    fn fmt(v: &Value, length: i32, standard: i32) -> Result<String, FormatFail> {
        format_value(v, length, standard, &Ctx(None))
    }

    fn refused(r: Result<String, FormatFail>) -> bool {
        matches!(r, Err(FormatFail::Unsupported(_)))
    }

    /// Issue 8389: types, lengths and standard formats no corpus case measures refuse instead of padding with the shared helper.
    #[test]
    fn a_length_no_case_measures_refuses() {
        let day = day_from_civil(2024, 3, 5);
        let stamp = Value::DateTime((i64::from(day) - 1) * 86_400_000 + 49_506_000 + 1);
        let dec = Value::Decimal(Decimal::new(15, 1));
        for (v, length, standard) in [
            (Value::Duration(1000), 4, 0),
            (Value::Duration(1000), 30, 0),
            (Value::Duration(1000), -2, 0),
            (Value::Char(97), 3, 0),
            (Value::Char(97), -3, 0),
            (Value::Byte(7), 3, 0),
            (Value::BigInteger(5), 3, 0),
            (Value::BigInteger(5), -3, 0),
            (Value::Time(49_506_000 + 1), 30, 0),
            (Value::Integer(12345), -2, 0),
            (Value::Integer(12345), -5, 0),
            (Value::Code("AB".into()), -2, 0),
            (Value::Code("AB".into()), -5, 0),
            (Value::Guid([0; 16]), -5, 0),
            (Value::Date(day, true), 30, 0),
            (Value::Date(day, false), -30, 0),
            (Value::Date(day, true), 11, 0),
            (Value::Null, 5, 0),
            // A measured type and sign in a standard format no case measures it in.
            (Value::Integer(42), 5, 9),
            (Value::Integer(42), 5, 2),
            (Value::Integer(42), 5, 1),
            (Value::Date(day, false), 20, 0),
            (Value::Date(day, false), 30, 4),
            (stamp.clone(), 30, 0),
            (dec.clone(), -10, 9),
            (Value::Boolean(true), -10, 2),
            // A text exactly as long as the length, where no case measures that cell.
            (Value::Integer(123), 3, 0),
            (dec, 3, 0),
            (stamp, 20, 9),
            (Value::Code("AB".into()), 2, 0),
        ] {
            assert!(
                refused(fmt(&v, length, standard)),
                "{v:?} with {length} in {standard}: {:?}",
                fmt(&v, length, standard)
            );
        }
    }

    /// The measured neighbors keep main's text.
    #[test]
    fn the_measured_lengths_keep_their_text() {
        let day = Value::Date(day_from_civil(2024, 3, 5), false);
        let stamp = Value::DateTime((i64::from(day_from_civil(2024, 3, 5)) - 1) * 86_400_000 + 49_506_000 + 1);
        let ok = |v: &Value, length: i32, standard: i32| fmt(v, length, standard).unwrap();
        assert_eq!(ok(&Value::Integer(42), 5, 0), "   42");
        assert_eq!(ok(&Value::Integer(42), -10, 0), "42");
        assert_eq!(ok(&Value::Decimal(Decimal::new(15, 1)), 5, 0), "  1.5");
        assert_eq!(ok(&Value::Decimal(Decimal::new(15, 1)), -10, 0), "1.5");
        assert_eq!(ok(&day, 11, 9), " 2024-03-05");
        assert_eq!(ok(&day, 10, 9), "2024-03-05");
        assert_eq!(ok(&stamp, 30, 9), "          2024-03-05T13:45:06Z");
        assert_eq!(ok(&Value::Text("<+%1D>".into()), 6, 0), "<+%1D>");
        assert_eq!(ok(&Value::Text("abc".into()), -5, 0), "abc");
        for standard in [1, 2, 9] {
            assert_eq!(ok(&Value::Decimal(Decimal::new(15, 1)), 5, standard), "  1.5");
        }
        assert_eq!(ok(&Value::Boolean(true), -10, 0), "Yes");
        assert_eq!(ok(&Value::Enum(7, 1), -10, 0), "Blue");
        // A length of 0 keeps every text, also of a type with no length rule.
        assert_eq!(ok(&Value::Duration(1000), 0, 0), "1 second");
        // Lengths the type rules of `format_value` take are not this module's: a cut, a text padded on the right.
        assert_eq!(ok(&Value::Integer(12345), 3, 0), "123");
        assert_eq!(ok(&Value::Text("ab".into()), 5, 0), "ab   ");
    }

    fn fmt_string(v: &Value, length: i32, template: &str) -> Result<String, String> {
        crate::formatstr::format_with_string(v, length, template, &Ctx(None))
    }

    /// Issue 8389: a length under a format string runs only in the measured cells; a refusal is an error without a mark.
    #[test]
    fn a_length_under_a_format_string_runs_only_where_measured() {
        let date = Value::Date(day_from_civil(2024, 3, 5), false);
        let time = Value::Time((9 * 3600 + 5 * 60 + 3) * 1000 + 1);
        let dec = Value::Decimal(Decimal::new(12345, 1));
        let refused_string = |r: Result<String, String>| matches!(&r, Err(e) if !e.starts_with(crate::formatstr::INVALID_FORMAT_MARK) && !e.starts_with(crate::formatstr::RAISE_MARK));
        for (v, length, template) in [
            (Value::Duration(1000), 30, "<Standard Format,0>"),
            (Value::Char(97), 3, "<Standard Format,0>"),
            (Value::Boolean(true), -2, "<Standard Format,0>"),
            (Value::Integer(42), 5, "<Integer,3>"),
            (Value::Integer(12345), 3, "<Integer,3>"),
            (dec.clone(), -10, "<Integer>"),
            (Value::Integer(42), 1, "<Integer,3>"),
            (Value::DateTime(0), -6, "<Second dec.>"),
            (date.clone(), 20, "<Year4>-<Month,2>"),
            (time.clone(), -1, "<Hours24,2>"),
            (time.clone(), -2, "<Hours24,2>"),
            // Only the cited cell's string runs (Format.al:1298, :1315; :1281, :1286; :537): any other string refuses.
            (Value::DateTime(0), 10, "<Year4>-<Month,2>-<Day,2>"),
            (Value::DateTime(0), 5, "<Year4>"),
            (time.clone(), -8, "<Hours24,2>"),
            (date.clone(), 3, "<Year4>"),
        ] {
            let r = fmt_string(&v, length, template);
            assert!(refused_string(r.clone()), "{v:?} {length} {template}: {r:?}");
        }
        let ok = |v: &Value, length: i32, template: &str| fmt_string(v, length, template).unwrap();
        assert_eq!(ok(&dec, 10, "<Precision,2:2>"), "          ");
        assert_eq!(ok(&dec, 1, "<Integer Thousand>"), "1");
        assert_eq!(ok(&dec, 10, "<Standard Format,0>"), "   1,234.5");
        assert_eq!(ok(&date, 5, "<Year4>-<Month,2>"), "2024-");
        assert_eq!(ok(&time, 8, "<Hours24,2>"), "       9");
        assert_eq!(ok(&time, 8, "<Hours24,2>:<Minutes,2>:<Seconds,2>"), " 9:05:03");
        assert_eq!(ok(&time, 5, "<Hours24,2>:<Minutes,2>:<Seconds,2>"), " 9:05");
        // The cited cell of a negative length longer than the text (Format.al:1281, :1286): '<Second dec.>' only.
        assert!(fmt_string(&time, -12, "<Second dec.>").is_ok());
        assert_eq!(ok(&dec, 4, "<Integer>"), "1234");
        assert_eq!(ok(&Value::Integer(1), 2, "<Integer,2><Filler Character,0>"), "01");
        assert_eq!(ok(&Value::DateTime(0), 5, "<Second dec.>"), "     ");
        // A length of 0 keeps every text.
        assert_eq!(ok(&Value::Integer(42), 0, "<Integer>"), "42");
    }

    fn fmt_string_hits(
        v: &Value,
        length: i32,
        template: &str,
    ) -> (Result<String, String>, Vec<crate::assumptions::Assumption>) {
        crate::assumptions::clear_hits();
        let r = fmt_string(v, length, template);
        (r, crate::assumptions::take_hits())
    }

    fn fmt_hits(
        v: &Value,
        length: i32,
        standard: i32,
    ) -> (Result<String, FormatFail>, Vec<crate::assumptions::Assumption>) {
        crate::assumptions::clear_hits();
        let r = fmt(v, length, standard);
        (r, crate::assumptions::take_hits())
    }

    fn refused_by_string(r: &Result<String, String>) -> bool {
        matches!(r, Err(e) if !e.starts_with(crate::formatstr::INVALID_FORMAT_MARK) && !e.starts_with(crate::formatstr::RAISE_MARK))
    }

    /// A lone `<Standard Format,N>` takes the gate of `Format(v, L, N)`: what the plain call refuses, the string refuses too.
    #[test]
    fn a_lone_standard_format_string_takes_the_gate_of_its_standard_format() {
        let time = Value::Time((13 * 3600 + 45 * 60 + 6) * 1000 + 1);
        let half = Value::Decimal(Decimal::new(15, 1));
        let dec = Value::Decimal(Decimal::new(12345, 1));
        for (v, length, template) in [
            (Value::Integer(123), 3, "<Standard Format,0>"),
            (Value::Integer(1234), 4, "<Standard Format,0>"),
            (half.clone(), 3, "<Standard Format,0>"),
            (dec.clone(), 6, "<Standard Format,2>"),
            (time.clone(), 30, "<Standard Format,0>"),
            (Value::Time(0), 5, "<Standard Format,0>"),
        ] {
            let r = fmt_string(&v, length, template);
            assert!(refused_by_string(&r), "{v:?} {length} {template}: {r:?}");
        }
        assert!(refused(fmt(&Value::Integer(123), 3, 0)));
        assert!(refused(fmt(&time, 30, 0)));
        // The pinned cells: P86FormatDecimal.al:1083, :1084, :1086, no mark.
        for (length, template, want) in [
            (10, "<Standard Format,0>", "   1,234.5"),
            (10, "<Standard Format,1>", "    1234.5"),
            (20, "<Standard Format,9>", "              1234.5"),
        ] {
            assert_eq!(fmt_string_hits(&dec, length, template), (Ok(want.to_string()), vec![]));
        }
        // A Decimal longer than the length is cut as by `Format(v, 3, 0)` (FormatLengthNeighbors.al:458), no mark.
        assert_eq!(
            fmt_string_hits(&dec, 3, "<Standard Format,0>"),
            (Ok("1,2".to_string()), vec![])
        );
        // A negative length cuts a longer Date as `Format(v, -3, 0)` does (FormatLengthNeighbors.al:312), no mark.
        let date = Value::Date(day_from_civil(2024, 1, 15), false);
        assert_eq!(
            fmt_string_hits(&date, -3, "<Standard Format,0>"),
            (Ok("01/".to_string()), vec![])
        );
        // Only the measured lengths: `Format(42, 5, 0)` runs, the string at 5 does not; at 10 it runs (FormatLengthNeighbors.al:452).
        assert_eq!(fmt(&Value::Integer(42), 5, 0).unwrap(), "   42");
        let r = fmt_string(&Value::Integer(42), 5, "<Standard Format,0>");
        assert!(refused_by_string(&r), "{r:?}");
        assert_eq!(
            fmt_string_hits(&Value::Integer(42), 10, "<Standard Format,0>"),
            (Ok("        42".to_string()), vec![])
        );
    }

    /// The undefined Time under a format string runs only in the one measured cell (types/date-time/Format.al:855, :917).
    #[test]
    fn an_undefined_time_under_a_format_string_runs_only_in_the_measured_cell() {
        for (length, template) in [
            (4, "<Hours24,2>:<Minutes,2>:<Seconds,2>"),
            (12, "<Hours24,2>:<Minutes,2>:<Seconds,2>"),
            (-12, "<Second dec.>"),
            (12, "<Hours,2>:<Minutes,2>:<Seconds,2>"),
            (-8, "<Hours,2>:<Minutes,2>:<Seconds,2>"),
        ] {
            let r = fmt_string(&Value::Time(0), length, template);
            assert!(refused_by_string(&r), "0T {length} {template}: {r:?}");
        }
        assert_eq!(
            fmt_string(&Value::Time(0), 8, "<Hours,2>:<Minutes,2>:<Seconds,2>").unwrap(),
            " 0:00:00"
        );
    }

    /// An Integer as long as the length runs unmarked only under the cited string (SgListKeyMovesTests.al:54); under
    /// `<Integer>` it runs through the assumption of corpus PR 1505 (FormatLengthIntegerString.al:33).
    #[test]
    fn an_integer_as_long_as_the_length_runs_unmarked_only_under_the_cited_string() {
        let (r, hits) = fmt_string_hits(&Value::Integer(12), 2, "<Integer,2><Filler Character,0>");
        assert_eq!((r.unwrap().as_str(), hits.is_empty()), ("12", true));
        let (r, hits) = fmt_string_hits(&Value::Integer(12), 2, "<Integer>");
        assert_eq!(r.unwrap(), "12");
        assert_eq!(hits, [crate::assumptions::Assumption::FormatLengthIntegerString]);
    }

    /// Issue 8652: the cells of corpus PR 1505 (FormatLengthIntegerString.al) give the probe's text through the assumption.
    #[test]
    fn the_integer_string_cells_run_flagged() {
        let int = Value::Integer;
        let big = Value::BigInteger;
        for (v, length, template, want) in [
            (int(8), 2, "<Integer>", " 8"),                                     // :26, :43
            (int(123), 2, "<Integer>", "12"),                                   // :36, :51
            (int(123), -2, "<Integer>", "12"),                                  // :39, :54
            (int(8), -5, "<Integer>", "8"),                                     // :31, :48
            (int(-5), 4, "<Integer>", "   5"),                                  // :71, :87
            (int(-123), 8, "<Sign><Integer>", "    -123"),                      // :79, :94
            (int(8), 1, "<Integer,2><Filler Character,0>", "0"),                // :110, :121
            (int(8), 2, "<Integer,4>", "  "),                                   // :112, :123
            (int(8), 6, "No. <Integer> of 9", "No. 8 "),                        // :143, :151
            (int(8), 5, "", "     "),                                           // :146, :154
            (int(-1234), 3, "<Integer Thousand>", "1,2"),                       // :174, :182
            (big(1234567890123), 8, "<Integer>", "12345678"),                   // :204, :214
            (big(8), 5, "<Integer,2><Filler Character,0>", "   08"),            // :202, :213
            (big(1234567890123), 16, "<Integer Thousand>", "1,234,567,890,12"), // :206, :216
        ] {
            let (r, hits) = fmt_string_hits(&v, length, template);
            assert_eq!(r.as_deref(), Ok(want), "{v:?} {length} {template}");
            assert_eq!(
                hits,
                [crate::assumptions::Assumption::FormatLengthIntegerString],
                "{v:?} {length} {template}"
            );
        }
    }

    /// Issue 8652: the neighbors of those cells refuse: an unmeasured string, an unmeasured fit of a measured string, a negative
    /// length on a BigInteger or as long as the text, and the other whole-number types.
    #[test]
    fn the_neighbors_of_the_integer_string_cells_refuse() {
        for (v, length, template) in [
            (Value::Integer(8), 2, "<Integer,3>"),
            (Value::Integer(8), 2, "M<Integer>"),
            (Value::Integer(-1234), 3, "<Sign><Integer>"),
            (Value::Integer(8), 2, "<Integer>.<Integer>"),
            (Value::Integer(12), -2, "<Integer>"),
            (Value::Integer(123), -2, "<Integer Thousand>"),
            (Value::Integer(8), -5, "N<Integer>"),
            (Value::BigInteger(123), -2, "<Integer>"),
            (Value::BigInteger(8), 5, "<Integer,4>"),
            (Value::BigInteger(8), 5, ""),
            (Value::Byte(8), 2, "<Integer>"),
        ] {
            let (r, hits) = fmt_string_hits(&v, length, template);
            assert!(refused_by_string(&r), "{v:?} {length} {template}: {r:?}");
            assert!(hits.is_empty(), "{v:?} {length} {template}");
        }
    }

    /// The cells of format-evaluate/format-length/FormatLengthDates.al and FormatLengthTypes.al:888 run, unmarked.
    #[test]
    fn the_pinned_format_length_cells_run_unmarked() {
        let date = Value::Date(day_from_civil(2024, 1, 15), false);
        let undefined = Value::Date(0, false);
        for (v, length, standard, want) in [
            (&undefined, 10, 9, "          "),
            (&undefined, 1, 9, " "),
            (&undefined, 30, 0, "                              "),
            (&undefined, -10, 9, ""),
            (&undefined, -30, 0, ""),
            (&undefined, 10, 1, "          "),
            (&undefined, 10, 2, "          "),
            (&Value::DateTime(0), 10, 0, "          "),
            (&Value::DateTime(0), -1, 0, ""),
            (&Value::DateTime(0), 30, 9, "                              "),
            (&Value::DateTime(0), -3, 9, ""),
        ] {
            assert_eq!(
                fmt_hits(v, length, standard),
                (Ok(want.to_string()), vec![]),
                "{v:?} {length} {standard}"
            );
        }
        assert_eq!(
            fmt_hits(&Value::Text("abcde".into()), -5, 0),
            (Ok("abcde".to_string()), vec![])
        );
        const YEAR4: &str = "<Year4>-<Month,2>-<Day,2>";
        for (length, want) in [
            (10, "2024-01-15"),
            (11, " 2024-01-15"),
            (12, "  2024-01-15"),
            (-10, "2024-01-15"),
            (-12, "2024-01-15"),
        ] {
            assert_eq!(
                fmt_string_hits(&date, length, YEAR4),
                (Ok(want.to_string()), vec![]),
                "{length}"
            );
        }
        // Longer than a positive length under '<Year4>-<Month,2>': cut, pinned (types/date-time/Format.al:537), no mark.
        assert_eq!(
            fmt_string_hits(&date, 5, "<Year4>-<Month,2>"),
            (Ok("2024-".to_string()), vec![])
        );
        // Under '<Year4>-<Month,2>-<Day,2>' the cut is pinned at FormatLengthDates.al:207, :209, :445.
        for (length, want) in [(8, "2024-01-"), (9, "2024-01-1"), (3, "202")] {
            assert_eq!(
                fmt_string_hits(&date, length, YEAR4),
                (Ok(want.to_string()), vec![]),
                "{length}"
            );
        }
        for (length, template, want) in [
            (10, YEAR4, "          "),
            (3, YEAR4, "   "),
            (-10, YEAR4, ""),
            (10, "<Closing><Day>", "          "),
        ] {
            assert_eq!(
                fmt_string_hits(&undefined, length, template),
                (Ok(want.to_string()), vec![]),
                "{length} {template}"
            );
        }
    }

    /// The neighbors of the pinned format-length cells that no line measures keep refusing.
    #[test]
    fn the_neighbors_of_the_pinned_format_length_cells_refuse() {
        let date = Value::Date(day_from_civil(2024, 1, 15), false);
        let undefined = Value::Date(0, false);
        for (v, length, standard) in [
            (&undefined, 31, 9),
            (&undefined, -31, 0),
            // Lengths between the pinned ±1, 3, 8, 10 and 30 (FormatLengthDates.al:62-72, :84-93, :337-361).
            (&undefined, 2, 9),
            (&undefined, 20, 0),
            (&undefined, -5, 9),
            (&Value::DateTime(0), 2, 0),
            (&Value::DateTime(0), -20, 0),
            (&undefined, 3, 1),
            (&undefined, 10, 4),
            (&Value::DateTime(0), 8, 9),
            (&Value::DateTime(0), 31, 0),
            (&Value::Code("AB".into()), -2, 0),
            (&Value::Text("abcde".into()), -5, 9),
        ] {
            assert!(
                refused(fmt(v, length, standard)),
                "{v:?} {length} {standard}: {:?}",
                fmt(v, length, standard)
            );
        }
        for (v, length, template) in [
            (
                &Value::Date(day_from_civil(2024, 1, 15), true),
                10,
                "<Year4>-<Month,2>-<Day,2>",
            ),
            (&date, 10, "<Day,2><Month,2><Year,2>"),
            (&undefined, 8, "<Year4>-<Month,2>-<Day,2>"),
            (&undefined, 10, "<Year4>"),
        ] {
            let r = fmt_string(v, length, template);
            assert!(refused_by_string(&r), "{v:?} {length} {template}: {r:?}");
        }
    }

    /// The probe's values (FormatLengthDateString.al): 2024-01-05, its closing date, 09:30:05.000.
    fn date() -> Value {
        Value::Date(day_from_civil(2024, 1, 5), false)
    }

    fn closing() -> Value {
        Value::Date(day_from_civil(2024, 1, 5), true)
    }

    const TIME_MS: i64 = (9 * 3600 + 30 * 60 + 5) * 1000 + 1;

    fn time() -> Value {
        Value::Time(TIME_MS)
    }

    /// `CreateDateTime(20240105D, 093005T)`: a DateTime holds its milliseconds plus one (`datefns::create_datetime`).
    fn datetime() -> Value {
        let day = i64::from(day_from_civil(2024, 1, 5)) - 1;
        Value::DateTime(day * 86_400_000 + TIME_MS)
    }

    /// `Format(v, length, template)` gives `want` through the assumption `format-length-date-string`.
    fn assert_date_string_flagged(v: &Value, length: i32, template: &str, want: &str) {
        let (r, hits) = fmt_string_hits(v, length, template);
        assert_eq!(r.as_deref(), Ok(want), "{v:?} {length} {template}");
        assert_eq!(
            hits,
            [crate::assumptions::Assumption::FormatLengthDateString],
            "{v:?} {length} {template}"
        );
    }

    /// Issue 8890: the cells of corpus PR 1557 (FormatLengthDateString.al) give the probe's text through the assumption, the
    /// cut by a negative length included; the pinned cells of the same strings stay unmarked.
    #[test]
    fn the_date_string_cells_run_flagged() {
        let (date, closing, time) = (date(), closing(), time());
        for (v, length, template, want) in [
            (&date, 8, DATE_COMPACT, "20240105"),                           // :55
            (&date, 7, DATE_COMPACT, "2024010"),                            // :56
            (&date, 10, DATE_COMPACT, "  20240105"),                        // :60
            (&date, -8, DATE_COMPACT, "20240105"),                          // :62
            (&date, -3, DATE_COMPACT, "202"),                               // :64
            (&date, -10, DATE_COMPACT, "20240105"),                         // :67
            (&closing, 8, DATE_COMPACT, "20240105"),                        // :69
            (&closing, 10, DATE_COMPACT, "  20240105"),                     // :70
            (&closing, -8, DATE_COMPACT, "20240105"),                       // :71
            (&Value::Date(0, false), 8, DATE_COMPACT, "        "),          // :73
            (&Value::Date(0, false), -10, DATE_COMPACT, ""),                // :77
            (&date, 8, DATE_DOTS, "05.01.20"),                              // :118
            (&date, -12, DATE_DOTS, "05.01.2024"),                          // :123
            (&date, -4, DATE_YEAR2, "2401"),                                // :130
            (&date, 8, DATE_YEAR2, "  240105"),                             // :128
            (&date, 30, YEAR4_MONTH_DAY, "                    2024-01-05"), // :134
            (&date, -11, YEAR4_MONTH_DAY, "2024-01-05"),                    // :138
            (&time, -6, TIME_COMPACT, " 93005"),                            // :181
            (&time, -4, TIME_COMPACT, " 930"),                              // :182
            (&time, -8, TIME_COMPACT, " 93005"),                            // :183
            (&Value::Time(0), 6, TIME_COMPACT, " 00000"),                   // :185
            (&Value::Time(0), 3, TIME_COMPACT, " 00"),                      // :186
            (&Value::Time(0), 8, TIME_COMPACT, "   00000"),                 // :187
            (&Value::Time(0), -8, TIME_COMPACT, " 00000"),                  // :189
            (&time, -3, TIME_HOURS_MINUTES, " 9:"),                         // :196
            (&time, -7, TIME_HOURS_MINUTES, " 9:30"),                       // :197
        ] {
            assert_date_string_flagged(v, length, template, want);
        }
        // The cells a cited line pins keep running unmarked (FormatLengthDates.al:206-214; FormatLengthStrings.al:663, :667;
        // Format.al:873, :885).
        assert_eq!(
            fmt_string_hits(&date, 10, YEAR4_MONTH_DAY),
            (Ok("2024-01-05".to_string()), vec![])
        );
        assert_eq!(
            fmt_string_hits(&date, 1, YEAR4_MONTH_DAY),
            (Ok("2".to_string()), vec![])
        );
        assert_eq!(
            fmt_string_hits(&date, -9, YEAR4_MONTH_DAY),
            (Ok("2024-01-0".to_string()), vec![])
        );
        assert_eq!(
            fmt_string_hits(&time, 4, TIME_COMPACT),
            (Ok(" 930".to_string()), vec![])
        );
    }

    /// Issue 8890: the DateTime cells of corpus PR 1557 (FormatLengthDateString.al:233-282) run flagged.
    #[test]
    fn the_datetime_string_cells_run_flagged() {
        let dt = datetime();
        assert_eq!(fmt_string(&dt, 0, DATETIME_COMPACT).unwrap(), "20240105093005");
        for (v, length, template, want) in [
            (&dt, 14, DATETIME_COMPACT, "20240105093005"),                 // :234
            (&dt, 8, DATETIME_COMPACT, "20240105"),                        // :235
            (&dt, 16, DATETIME_COMPACT, "  20240105093005"),               // :237
            (&dt, -14, DATETIME_COMPACT, "20240105093005"),                // :239
            (&dt, -8, DATETIME_COMPACT, "20240105"),                       // :240
            (&dt, -16, DATETIME_COMPACT, "20240105093005"),                // :241
            (&Value::DateTime(0), 14, DATETIME_COMPACT, "              "), // :243
            (&Value::DateTime(0), -3, DATETIME_COMPACT, ""),               // :247
            (&dt, 16, DATETIME_SEPARATED, "2024-01-05 09:30"),             // :277
            (&dt, 10, DATETIME_SEPARATED, "2024-01-05"),                   // :278
            (&dt, 18, DATETIME_SEPARATED, "  2024-01-05 09:30"),           // :279
            (&dt, -10, DATETIME_SEPARATED, "2024-01-05"),                  // :281
            (&dt, -18, DATETIME_SEPARATED, "2024-01-05 09:30"),            // :282
        ] {
            assert_date_string_flagged(v, length, template, want);
        }
    }

    /// Issue 8890: the neighbors of the date string cells refuse: an unmeasured string, an unmeasured fit or sign of a
    /// measured string, and a value kind the probe did not measure under that string.
    #[test]
    fn the_neighbors_of_the_date_string_cells_refuse() {
        for (v, length, template) in [
            (date(), 8, "<Day,2><Month,2><Year4>"),
            (date(), 3, "<Year4>"),
            (closing(), 3, DATE_COMPACT),
            (closing(), -10, DATE_COMPACT),
            (closing(), -3, DATE_COMPACT),
            (Value::Date(0, false), 8, DATE_DOTS),
            (Value::Date(0, false), 8, YEAR4_MONTH_DAY),
            (Value::Time(0), -3, TIME_COMPACT),
            (Value::Time(0), -5, TIME_HOURS_MINUTES),
            (time(), -3, "<Hours24,2>:<Minutes,2>:<Seconds,2>"),
            (time(), -3, DATE_COMPACT),
            (datetime(), 8, DATE_COMPACT),
            (datetime(), -3, TIME_COMPACT),
            (Value::DateTime(0), 3, DATETIME_SEPARATED),
        ] {
            let (r, hits) = fmt_string_hits(&v, length, template);
            assert!(refused_by_string(&r), "{v:?} {length} {template}: {r:?}");
            assert!(hits.is_empty(), "{v:?} {length} {template}");
        }
    }

    /// Under `CurrReport.FormatRegion` a length refuses, except for the values formatted as without the region.
    #[test]
    fn a_length_under_a_format_region_refuses() {
        let de = Ctx(Some("de-DE"));
        let dec = Value::Decimal(Decimal::new(15, 1));
        assert!(refused(format_value(&dec, 10, 0, &de)));
        assert_eq!(format_value(&dec, 0, 0, &de).unwrap(), "1,5");
        assert_eq!(format_value(&Value::Integer(42), 5, 0, &de).unwrap(), "   42");
    }
}
