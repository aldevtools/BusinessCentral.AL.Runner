//! A JSON string read by `JsonValue.AsDecimal`: which texts are numbers, which raise the conversion error.
//!
//! Pinned before (json/JsonValueGetters.al NumericTextGetters, TextGetters; types/lt-aj-probes/AjScalars.al:150-209): digits,
//! `100.00`, `-3.5`, ` 7 ` read as the number; letters, `""` and `1e2` raise the conversion error. Measured by corpus writer
//! Y65 (json/JsonValueGetters.al Y65_NotANumberText_EveryGetter, Y65_DecimalShapedText_AsDecimal, _2; on its corpus branch,
//! assumption json-text-number-shapes until the pin): `4,5` reads 45 and `1,000.5` 1000.5 (a comma in the whole part is
//! dropped), `+3` 3, `.5` 0.5, ` 4.5 ` 4.5, `-0` 0; `n/a`, `NaN`, `Infinity`, `0x10`, `1e3`, `1.5.1` and `1 000` raise the
//! conversion error; `99999999999999999999` raises the Decimal range error.

use rust_decimal::Decimal;

/// What a JSON string reads as.
pub(super) enum TextNumber {
    /// The number; `measured`: the shape is pinned only by writer Y65's file.
    Number { value: Decimal, measured: bool },
    /// BC raises the conversion error; `measured` as above.
    NotANumber { measured: bool },
    /// The shape is not pinned.
    Unpinned,
}

pub(super) fn decimal_text(t: &str) -> TextNumber {
    let s = t.trim_matches(' ');
    if s.is_empty() {
        return TextNumber::NotANumber { measured: false };
    }
    // A letter or any sign outside the number alphabet: never a number (`n/a`, `NaN`, `Infinity`, `0x10`, `1e3`, `hello`).
    if s.chars()
        .any(|c| !(c.is_ascii_digit() || matches!(c, ',' | '.' | '+' | '-' | ' ')))
    {
        let pinned_before = s.bytes().all(|b| b.is_ascii_alphabetic())
            || s.split_once('e').is_some_and(|(a, b)| is_digits(a) && is_digits(b));
        return TextNumber::NotANumber {
            measured: !pinned_before,
        };
    }
    // Blanks between digits (`1 000`) and two points (`1.5.1`).
    if (s.contains(' ') && s.chars().all(|c| c.is_ascii_digit() || c == ' ')) || s.matches('.').count() > 1 {
        return TextNumber::NotANumber { measured: true };
    }
    let (negative, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (whole, fraction) = match body.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (body, None),
    };
    // The whole part: digits, a comma only between digits; the fraction: digits. Any other shape is not pinned.
    let whole_ok = whole.is_empty() && fraction.is_some()
        || (whole.starts_with(|c: char| c.is_ascii_digit())
            && whole.ends_with(|c: char| c.is_ascii_digit())
            && !whole.contains(",,")
            && whole.chars().all(|c| c.is_ascii_digit() || c == ','));
    if !whole_ok || fraction.is_some_and(|f| !is_digits(f)) {
        return TextNumber::Unpinned;
    }
    let digits = format!(
        "{}{}{}",
        if negative { "-" } else { "" },
        whole.replace(',', ""),
        fraction.map_or(String::new(), |f| format!(".{f}"))
    );
    let Ok(value) = digits.parse::<Decimal>() else {
        return TextNumber::Unpinned;
    };
    let pinned_before = super::signed_point_number(t) || super::blank_padded_digits(t);
    TextNumber::Number {
        value,
        measured: !pinned_before,
    }
}

/// The text holds a letter or a sign no number is written with (`n/a`, `hello`, `0x10`): every numeric getter raises its
/// conversion error (Y65_NotANumberText_EveryGetter: AsBigInteger, AsByte, AsOption, AsInteger and AsDecimal of `n/a`).
pub(super) fn outside_number_alphabet(t: &str) -> bool {
    t.chars()
        .any(|c| !(c.is_ascii_digit() || matches!(c, ',' | '.' | '+' | '-' | ' ')))
}

fn is_digits(t: &str) -> bool {
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}
