//! What a length does in `Format(Value, Length [, Number])` to the values the type rules of `format_value` leave:
//! only the shapes the corpus measured run, every other value and length refuses (al-runner issue 8389).

use super::{FormatFail, pad};
use crate::describe::Describe;
use crate::value::Value;
use std::cmp::Ordering;

/// The text of `v` (`s`) with a length no type rule of `format_value` took. A length of 0 keeps the text.
pub(super) fn rest(v: &Value, s: String, length: i32) -> Result<String, FormatFail> {
    if length == 0 || measured(v, s.chars().count(), length) {
        return Ok(pad(s, length));
    }
    Err(FormatFail::Unsupported(format!(
        "Format of {} with length {length} (not pinned)",
        v.describe()
    )))
}

/// The pinned shapes, each by its value type, the sign of the length and how the text's length compares with it.
fn measured(v: &Value, chars: usize, length: i32) -> bool {
    let fit = chars.cmp(&(length.unsigned_abs() as usize));
    match (v, length > 0, fit) {
        // An undefined Date or DateTime with a length: no corpus case measures one.
        (Value::Date(0, _) | Value::DateTime(0), _, _) => false,
        // A shorter number, date or DateTime is right-aligned in the length: text/TestFormat.al:59-60 (`Format(42, 5)`),
        // format-evaluate/RandomFormat.al:161; types/decimal-integer/ScT70050.al:182; types/date-time/Format.al:531, :535.
        (Value::Integer(_) | Value::Decimal(_), true, Ordering::Less) => true,
        (Value::Date(_, false) | Value::DateTime(_), true, Ordering::Less) => true,
        // A text exactly as long as the length stays as it is, where both the shorter and the longer text are measured for
        // the type and the sign (the pad and the cut leave it alike): types/date-time/Format.al:530 (`2024-03-05` in 10),
        // types/date-time/P146FormatCalcDate.al:30 (`'<+%1D>'` in 6). Measured both ways: Integer RandomFormat.al:160-161,
        // Decimal ScT70050.al:182, :184, DateTime Format.al:534-535, Code text/text-methods/TextFormatEvaluate.Codeunit.al:40-41,
        // a Text with a negative length RandomFormat.al:315 and ScT70053.al:212.
        (
            Value::Integer(_)
            | Value::Decimal(_)
            | Value::Date(_, false)
            | Value::DateTime(_)
            | Value::Text(_)
            | Value::Code(_),
            true,
            Ordering::Equal,
        ) => true,
        (Value::Text(_), false, Ordering::Equal) => true,
        // A negative length longer than the text keeps it: format-evaluate/RandomFormat.al:162 (Integer), :315 (Text);
        // types/decimal-integer/ScT70050.al:183 (Decimal); types/decimal-integer/ScT70053.al:75 (Boolean), :186 (Option);
        // codeunits/interfaces/OjT71038.al:205 (Enum).
        (
            Value::Integer(_)
            | Value::Decimal(_)
            | Value::Text(_)
            | Value::Boolean(_)
            | Value::Option(..)
            | Value::Enum(..),
            false,
            Ordering::Less,
        ) => true,
        _ => false,
    }
}

/// A length under a report's `CurrReport.FormatRegion`: no corpus case measures one for the values formatted in the
/// region (the texts and the Integer in the standard format go to `format_value` before this).
pub(super) fn in_region(v: &Value, text: String, length: i32) -> Result<String, FormatFail> {
    if length == 0 {
        return Ok(text);
    }
    Err(FormatFail::Unsupported(format!(
        "Format of {} with length {length} under CurrReport.FormatRegion (not pinned)",
        v.describe()
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

    /// Issue 8389: types and lengths no corpus case measures refuse instead of padding with the shared helper.
    #[test]
    fn a_length_no_case_measures_refuses() {
        let day = day_from_civil(2024, 3, 5);
        for (v, length) in [
            (Value::Duration(1000), 4),
            (Value::Duration(1000), 30),
            (Value::Duration(1000), -2),
            (Value::Char(97), 3),
            (Value::Char(97), -3),
            (Value::Byte(7), 3),
            (Value::BigInteger(5), 3),
            (Value::BigInteger(5), -3),
            (Value::Time(49_506_000 + 1), 30),
            (Value::Integer(12345), -2),
            (Value::Integer(12345), -5),
            (Value::Code("AB".into()), -2),
            (Value::Code("AB".into()), -5),
            (Value::Guid([0; 16]), -5),
            (Value::Date(day, true), 30),
            (Value::Date(day, false), -30),
            (Value::DateTime(0), 5),
            (Value::Date(0, false), 10),
            (Value::Date(day, true), 11),
            (Value::Null, 5),
        ] {
            assert!(
                refused(fmt(&v, length, 0)),
                "{v:?} with {length}: {:?}",
                fmt(&v, length, 0)
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
        assert_eq!(ok(&Value::Text("abc".into()), -3, 0), "abc");
        assert_eq!(ok(&Value::Integer(123), 3, 0), "123");
        assert_eq!(ok(&Value::Code("AB".into()), 2, 0), "AB");
        assert_eq!(ok(&Value::Boolean(true), -10, 0), "Yes");
        assert_eq!(ok(&Value::Enum(7, 1), -10, 0), "Blue");
        // A length of 0 keeps every text, also of a type with no length rule.
        assert_eq!(ok(&Value::Duration(1000), 0, 0), "1 second");
        // Lengths the type rules of `format_value` take are not this module's: a cut, a text padded on the right.
        assert_eq!(ok(&Value::Integer(12345), 3, 0), "123");
        assert_eq!(ok(&Value::Text("ab".into()), 5, 0), "ab   ");
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
