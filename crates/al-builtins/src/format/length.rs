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
    // The file's results (format-evaluate/format-length/FormatLengthDates.al:8-9) name every length; the cells measure lengths
    // up to 30.
    let within_30 = (1..=30).contains(&length.unsigned_abs());
    match (v, standard, length > 0, fit) {
        // 0D: blanks in a positive length, '' in a negative one; L = ±1, 3, 8, 10, 30 in formats 0 and 9, L = 10 in 1 and 2
        // (FormatLengthDates.al:62-72, :99-113, :162-168).
        (Value::Date(0, false), 0 | 9, _, _) => within_30,
        (Value::Date(0, false), 1 | 2, true, _) => length == 10,
        // 0DT the same: L = ±1, 3, 8, 10, 30 in format 0; L = 3, 10, 30, -3, -30 in format 9 (FormatLengthDates.al:352-366,
        // :169-171).
        (Value::DateTime(0), 0, _, _) => within_30,
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
    if length == 0 || measured || measured_string_length(v, length, template) {
        return Ok(());
    }
    Err(format!(
        "Format of {} with length {length} and a format string (not pinned)",
        v.describe()
    ))
}

/// The pinned cells under a format string where a cited line measures only some lengths of one string.
fn measured_string_length(v: &Value, length: i32, template: &str) -> bool {
    match v {
        // 0D: blanks / '' (format-evaluate/format-length/FormatLengthDates.al:132-137, :216-217).
        Value::Date(0, false) => {
            (template == YEAR4_MONTH_DAY && matches!(length, 3 | 10 | 30 | -3 | -10))
                || (template == "<Closing><Day>" && length == 10)
        }
        // A defined Date: as long as the length, padded on the left, kept in a longer negative length, or cut to 3, 8 or 9
        // (FormatLengthDates.al:206-214, :219-223; the cuts :207, :209, :445).
        Value::Date(_, false) => template == YEAR4_MONTH_DAY && matches!(length, 3 | 8 | 9 | 10 | 11 | 12 | -10 | -12),
        _ => false,
    }
}

/// `Format(v, length, '<Standard Format,N>')`, a template of that one code, gave `out`: `None` for any other template. The
/// string takes the gate of `Format(v, length, N)` and runs only where that call gives the same text: pinned for a Decimal
/// shorter than a positive length in formats 0, 1 and 9 (format-evaluate/P86FormatDecimal.al:1083, :1084, :1086), and for the
/// types, numbers and lengths of format-evaluate/format-length/FormatLengthNeighbors.al with a text shorter or longer than the
/// length.
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
    let plain = match format_value(v, length, standard, ctx) {
        Ok(plain) => plain,
        Err(FormatFail::Unsupported(why)) => return Some(Err(why)),
        Err(FormatFail::Error { .. }) => return Some(Err(refused())),
    };
    let Ok(bare) = format_value(v, 0, standard, ctx) else {
        return Some(Err(refused()));
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
    if plain == out && (decimal || neighbors) {
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
            (Value::Integer(42), 5, "<Integer>"),
            (Value::Integer(12345), 3, "<Integer>"),
            (dec.clone(), -10, "<Integer>"),
            (Value::Integer(42), 1, "<Integer>"),
            (Value::DateTime(0), -5, "<Second dec.>"),
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

    /// An Integer as long as the length runs only under the cited string.
    #[test]
    fn an_integer_as_long_as_the_length_runs_only_under_the_cited_string() {
        let r = fmt_string(&Value::Integer(12), 2, "<Integer>");
        assert!(refused_by_string(&r), "{r:?}");
        assert_eq!(
            fmt_string(&Value::Integer(12), 2, "<Integer,2><Filler Character,0>").unwrap(),
            "12"
        );
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
            (&date, -8, "<Year4>-<Month,2>-<Day,2>"),
            (&date, -11, "<Year4>-<Month,2>-<Day,2>"),
            (&date, 13, "<Year4>-<Month,2>-<Day,2>"),
            (
                &Value::Date(day_from_civil(2024, 1, 15), true),
                10,
                "<Year4>-<Month,2>-<Day,2>",
            ),
            (&date, 6, "<Day,2><Month,2><Year,2>"),
            (&undefined, 8, "<Year4>-<Month,2>-<Day,2>"),
            (&undefined, 10, "<Year4>"),
        ] {
            let r = fmt_string(v, length, template);
            assert!(refused_by_string(&r), "{v:?} {length} {template}: {r:?}");
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
