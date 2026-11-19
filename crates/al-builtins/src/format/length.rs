//! What a length does in `Format`, after the type rules of `format_value` and the format strings of `formatstr`: only the
//! shapes the corpus measured run, every other value and length refuses (al-runner issue 8389).

use super::{FormatFail, pad};
use crate::describe::Describe;
use crate::value::Value;
use std::cmp::Ordering;

/// The text of `v` (`s`) in standard format `standard` with a length no type rule of `format_value` took. A length of 0
/// keeps the text.
pub(super) fn rest(v: &Value, standard: i32, s: String, length: i32) -> Result<String, FormatFail> {
    if length == 0 || measured(v, standard, fit(s.chars().count(), length), length > 0) {
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

/// `Format(v, length, FormatString)` gave a text of `chars` characters with length 0: whether the format string's text may
/// take the length. The pinned shapes, each by its value type, the sign of the length and how the text compares with it.
pub(crate) fn with_format_string(v: &Value, length: i32, chars: usize) -> Result<(), String> {
    use Ordering::{Equal, Greater, Less};
    let measured = match (v, length > 0, fit(chars, length)) {
        // A Decimal, shorter (right-aligned), as long, or longer (cut): format-evaluate/P86FormatDecimal.al:1078-1096
        // (Len_WithFormatString, `l1`-`l9`), format-evaluate/FormatCommaMore.al:601 (length 12); language/
        // R6901DecimalFieldFormat.al:87-88, :102-103, :129 (`0000.000` and `0001****` in 8).
        (Value::Decimal(_), true, Less | Equal | Greater) => true,
        // An Integer as long as the length: `R01` from `'R' + Format(1, 2, '<Integer,2><Filler Character,0>')`
        // (pages/list-key-navigation/SgListKeyMovesTests.al:54, :228; testpage/WdModalGoToKeyDescSecondary.al:138, :154).
        (Value::Integer(_), true, Equal) => true,
        // A defined Date, longer: cut (types/date-time/Format.al:537, `date len 5 custom`).
        (Value::Date(day, false), true, Greater) => *day != 0,
        // The undefined DateTime's empty text in a positive length: blanks (types/date-time/Format.al:1298, :1315).
        (Value::DateTime(0), true, Less) => true,
        // A Time, shorter, as long and longer, 0T too (types/date-time/Format.al:873, :885, :912, :917, :918,
        // TU_Time_Format_CodeNames; :1279-1280, :1284-1285); a negative length longer than the text keeps it (:1281, :1286).
        (Value::Time(_), true, Less | Equal | Greater) => true,
        (Value::Time(_), false, Less) => true,
        _ => false,
    };
    if length == 0 || measured {
        return Ok(());
    }
    Err(format!(
        "Format of {} with length {length} and a format string (not pinned)",
        v.describe()
    ))
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
            (Value::DateTime(0), 5, 0),
            (Value::Date(0, false), 10, 0),
            (Value::Date(0, false), 10, 9),
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
            (Value::Text("abc".into()), -3, 0),
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
            (Value::Date(0, false), 10, "<Standard Format,0>"),
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
        assert_eq!(ok(&time, -8, "<Hours24,2>"), " 9");
        assert_eq!(ok(&dec, 4, "<Integer>"), "1234");
        assert_eq!(ok(&Value::Integer(1), 2, "<Integer,2><Filler Character,0>"), "01");
        assert_eq!(ok(&Value::DateTime(0), 5, "<Second dec.>"), "     ");
        // A length of 0 keeps every text.
        assert_eq!(ok(&Value::Integer(42), 0, "<Integer>"), "42");
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
