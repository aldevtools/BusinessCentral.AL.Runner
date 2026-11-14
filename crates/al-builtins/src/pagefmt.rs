//! The text a page control shows for a value (ADR-012), en-US. This is not
//! `Format()`: a control spells Decimals with its own places and thousands
//! separators, Booleans as Yes/No, dates in the short forms, and so on.
//! Every rule cites the corpus test that measured it on real BC.

use crate::describe::Describe;
use crate::value::Value;
use rust_decimal::{Decimal, RoundingStrategy};

/// Which numbers a control blanks (`BlankNumbers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blank {
    #[default]
    DontBlank,
    Neg,
    NegAndZero,
    Zero,
    ZeroAndPos,
    Pos,
}

impl Blank {
    pub fn parse(s: &str) -> Option<Blank> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "dontblank" => Blank::DontBlank,
            "blankneg" => Blank::Neg,
            "blanknegandzero" => Blank::NegAndZero,
            "blankzero" => Blank::Zero,
            "blankzeroandpos" => Blank::ZeroAndPos,
            "blankpos" => Blank::Pos,
            _ => return None,
        })
    }
}

/// What the page and the source field say about how a value is shown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ControlFormat {
    /// `DecimalPlaces = min : max` (or the precision of an auto-format expression).
    pub decimal_places: Option<(u32, u32)>,
    pub blank_zero: bool,
    pub blank: Blank,
    /// `OptionCaption` of the control, else of the source field.
    pub option_captions: Option<Vec<String>>,
    /// The session's work date, which a Date control reads from the shorthand `w`
    /// (testpage/TestPageTemporalSetValue_Tests.al:272-286). `None` where the caller has none.
    pub work_date: Option<i32>,
    /// Today's date as a day number, for the shorthand `t` (session-system/W5WorkDateShorthand.al).
    pub today: Option<i32>,
    /// `ClosingDates = true` on the control or the source field: a closing date shows as `C12/31/2025`
    /// (record/table-properties/XTablePropsTests.al:523-524, pages/extensions/ZModifyControls.al:290-291).
    /// Without it a closing date shows its plain day.
    pub closing_dates: bool,
}

/// `2 : 2`, `0 : 5` or `3` as (min, max) places.
pub fn parse_decimal_places(p: &str) -> Option<(u32, u32)> {
    let mut it = p.split(':').map(|x| x.trim().parse::<u32>());
    let min = it.next()?.ok()?;
    let max = match it.next() {
        Some(m) => m.ok()?,
        None => min,
    };
    Some((min, max))
}

/// The precision of an auto-format expression: `<Precision,4:4><Standard Format,0>` is 4 : 4
/// (corpus format-evaluate/autoformat/TestAutoFormat.al, AutoFormat_CustomExpressionControl_HonoursTheFormatString).
pub fn parse_precision(expr: &str) -> Option<(u32, u32)> {
    let rest = expr.split("<Precision,").nth(1)?;
    parse_decimal_places(rest.split('>').next()?)
}

/// Why an `AutoFormatType` property (its text as written) is refused: it is none of the types the runtime models. BC resolves an
/// AutoFormat through the AutoFormatTranslate event of UI Helper Triggers and the subscribers of the loaded apps (al-runner issue
/// 8011), which the runtime does not raise, so a type outside the pinned ones has no known format. The pinned types, each by its
/// exact text:
/// - 0: pages/run-modal/P113AutoFormat.al:64, :71 (a page control), text/text-methods/RwDecimalFieldPlaces.al:63 (`Format`);
/// - 1, 2, 4, 5: format-evaluate/autoformat/PWAutoFormatTypes.al:19-26, :88-92 (a page control), text/text-methods/
///   SaUnitAmountSubst.al:138-145 (`Format`), :123-126 (StrSubstNo);
/// - 10: pages/run-modal/P113AutoFormat.al:67-70 (a page control), :311 (`Format`, P113_AF_TableFieldFormatIsPlain);
/// - 11: format-evaluate/autoformat/TestAutoFormat.al:78-90 (a page control), format-evaluate/Y43AfType11.al:221 (`Format`).
///
/// Each caller decides which shapes of a modelled type it runs; this answers only for the type.
pub fn unmodelled_auto_format_type(kind: &str) -> Option<String> {
    match kind.trim() {
        "0" | "1" | "2" | "4" | "5" | "10" | "11" => None,
        other => Some(format!(
            "AutoFormatType {other} (a type outside the pinned ones; BC resolves it through the AutoFormatTranslate event of UI Helper Triggers, which the runtime does not raise; only types 0, 1, 2, 4, 5, 10 and 11 are modelled)"
        )),
    }
}

/// Whether AutoFormatType `kind` (its text as written) is one the runtime formats natively and BC resolves through the AutoFormat
/// events a subscriber can write (al-vm `auto_format_writer`): 1, 2, 4, 5, 10 and 11, by their exact text (the pins are those of
/// [`unmodelled_auto_format_type`]). Type 0 is not: Format of a plain field raises no event
/// (events/platform-triggers/P114GlobalTriggerEvents.al:341-347).
pub fn resolved_auto_format_type(kind: &str) -> bool {
    matches!(kind.trim(), "1" | "2" | "4" | "5" | "10" | "11")
}

/// `1234567.891` with (2, 2) as `1,234,567.89`: half away from zero, thousands separators.
pub fn decimal_text(d: &Decimal, places: (u32, u32)) -> String {
    let (min, max) = places;
    let rounded = d.round_dp_with_strategy(max, RoundingStrategy::MidpointAwayFromZero);
    let mut s = rounded.abs().to_string();
    if !s.contains('.') && min > 0 {
        s.push('.');
    }
    if let Some((_, frac)) = s.split_once('.') {
        let have = frac.len() as u32;
        // Trailing zeros beyond `min` places are dropped, missing ones up to `min` are added.
        let mut frac = frac.to_string();
        while (frac.len() as u32) > min && frac.ends_with('0') {
            frac.pop();
        }
        for _ in have.min(min)..min {
            if (frac.len() as u32) < min {
                frac.push('0');
            }
        }
        let int = s.split('.').next().unwrap_or("0").to_string();
        s = if frac.is_empty() { int } else { format!("{int}.{frac}") };
    }
    let (int, frac) = s.split_once('.').map_or((s.as_str(), None), |(i, f)| (i, Some(f)));
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if rounded.is_sign_negative() && !rounded.is_zero() {
        "-"
    } else {
        ""
    };
    match frac {
        Some(f) => format!("{sign}{grouped}.{f}"),
        None => format!("{sign}{grouped}"),
    }
}

fn blanked(f: &ControlFormat, sign: std::cmp::Ordering) -> bool {
    use std::cmp::Ordering::*;
    (f.blank_zero && sign == Equal)
        || match f.blank {
            Blank::DontBlank => false,
            Blank::Neg => sign == Less,
            Blank::NegAndZero => sign != Greater,
            Blank::Zero => sign == Equal,
            Blank::ZeroAndPos => sign != Less,
            Blank::Pos => sign == Greater,
        }
}

fn hour12(h: i64) -> (i64, &'static str) {
    (if h % 12 == 0 { 12 } else { h % 12 }, if h < 12 { "AM" } else { "PM" })
}

/// `12:34:56 PM` for a Time value (milliseconds since midnight plus one).
fn time_text(t: i64) -> String {
    let ms = t - 1;
    let (h, m, s) = (ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60);
    let (h12, ap) = hour12(h);
    format!("{h12}:{m:02}:{s:02} {ap}")
}

fn duration_text(ms: i64) -> String {
    crate::format::duration_words(ms)
}

/// The inverse of [`duration_text`]: exactly its own spelling, in milliseconds. Blank is zero
/// (corpus testpage/TestPageFieldFormats_Tests.al:1703 SetValue_Duration, :1724 _Text, :1745 _BlankText).
/// Any other spelling of a duration is not pinned.
fn parse_duration(t: &str) -> Option<i64> {
    if t.is_empty() {
        return Some(0);
    }
    // Every unit of a negative Duration carries the sign (testpage/TestPageRuntimeContracts.al:157).
    let neg = t.starts_with('-');
    let body = t;
    let units: [(&str, i64); 5] = [
        ("day", 86_400_000),
        ("hour", 3_600_000),
        ("minute", 60_000),
        ("second", 1000),
        ("millisecond", 1),
    ];
    let words: Vec<&str> = body.split(' ').collect();
    if words.len() % 2 != 0 {
        return None;
    }
    let mut total = 0i64;
    let mut at = 0;
    for pair in words.chunks(2) {
        let n: i64 = pair[0].parse().ok().filter(|n| *n != 0 && (*n < 0) == neg)?;
        let n = n.abs();
        let (i, size) = units.iter().enumerate().find_map(|(i, (name, size))| {
            let spelled = format!("{name}{}", if n == 1 { "" } else { "s" });
            (pair[1] == spelled).then_some((i, *size))
        })?;
        if i < at {
            return None;
        }
        at = i + 1;
        total = total.checked_add(n.checked_mul(size)?)?;
    }
    Some(if neg { -total } else { total })
}

pub fn guid_text(g: &[u8; 16]) -> String {
    let h: String = g.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{{{}-{}-{}-{}-{}}}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// The text a control shows for `v`. `enum_caption` gives the caption of an enum value.
pub fn control_text(v: &Value, f: &ControlFormat, enum_caption: &dyn Fn(u32, i32) -> String) -> String {
    use std::cmp::Ordering;
    match v {
        Value::Null => String::new(),
        Value::Boolean(b) => if *b { "Yes" } else { "No" }.into(),
        Value::Integer(i) => {
            if blanked(f, i.cmp(&0)) {
                String::new()
            } else {
                i.to_string()
            }
        }
        Value::BigInteger(i) => {
            if blanked(f, i.cmp(&0)) {
                String::new()
            } else {
                i.to_string()
            }
        }
        Value::Byte(b) => b.to_string(),
        Value::Char(c) => char::from_u32(u32::from(*c)).map(String::from).unwrap_or_default(),
        Value::Decimal(d) => {
            let sign = d.cmp(&Decimal::ZERO);
            if blanked(f, sign) {
                String::new()
            } else {
                decimal_text(d, f.decimal_places.unwrap_or((2, 2)))
            }
        }
        Value::Text(t) | Value::Code(t) => t.to_string(),
        Value::Option(o, members) => match &f.option_captions {
            Some(c) if (*o as usize) < c.len() => c[*o as usize].clone(),
            _ => members.get(*o as usize).cloned().unwrap_or_else(|| o.to_string()),
        },
        Value::Enum(id, o) => enum_caption(*id, *o),
        Value::Date(0, _) | Value::Time(0) | Value::DateTime(0) => String::new(),
        Value::Date(day, closing) => {
            let (y, m, d) = crate::format::civil_from_day(*day);
            let mark = if *closing && f.closing_dates { "C" } else { "" };
            // The year has four digits, zeros in front (testpage/parts/PartFilterExpressionControl.al:493
            // Periods_FilterThenClearFilter_Read `cleared` `start=1/3/0001`).
            format!("{mark}{m}/{d}/{y:04}")
        }
        Value::Time(t) => time_text(*t),
        Value::DateTime(ms) => {
            let ms = ms - 1;
            let day = (ms.div_euclid(86_400_000)) as i32 + 1;
            let (y, mo, d) = crate::format::civil_from_day(day);
            let tod = ms.rem_euclid(86_400_000);
            let (h12, ap) = hour12(tod / 3_600_000);
            format!("{mo}/{d}/{y} {h12}:{:02} {ap}", tod / 60_000 % 60)
        }
        Value::Duration(ms) => duration_text(*ms),
        Value::Guid(g) => guid_text(g),
        Value::DateFormula(s) => crate::datefns::formula_text(s),
        other => {
            let _ = Ordering::Equal;
            other.describe()
        }
    }
}

/// What a control accepts as text for a value of type `ty`: the inverse of [`control_text`]
/// (corpus testpage/TestPageFieldFormats_Tests.al:1-40, "SetValue is spelled the same way and read
/// back by the client's own parser"). `None` = not an acceptable value
/// (testpage/TestPageBooleanSpelling_Tests.al: `Yes`/`No` only; `False` is refused).
/// `enum_by_text` finds an enum value's ordinal by caption or name.
pub fn parse_control_text(
    text: &str,
    ty: &crate::value::RtType,
    f: &ControlFormat,
    enum_by_text: &dyn Fn(u32, &str) -> Option<i32>,
) -> Result<Value, ParseError> {
    use crate::value::RtType as T;
    let t = text.trim();
    // Only the upper-case closing mark is pinned (pages/properties/P119ReportFields.al:174).
    if matches!(ty, T::Date) && f.closing_dates && t.strip_prefix('c').is_some_and(|r| r.contains('/')) {
        return Err(ParseError::Unsupported);
    }
    parse_typed(text, t, ty, f, enum_by_text).ok_or(match ty {
        T::Text(_)
        | T::Code(_)
        | T::Integer
        | T::BigInteger
        | T::Decimal
        | T::Boolean
        | T::Option(_)
        | T::Enum(_)
        | T::Date
        | T::Time
        | T::DateTime
        | T::Guid => ParseError::Invalid,
        _ => ParseError::Unsupported,
    })
}

/// Why a control text did not become a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// The client refuses it: "Your entry of '...' is not an acceptable value".
    Invalid,
    /// This type's spelling is not modelled yet.
    Unsupported,
}

fn parse_typed(
    text: &str,
    t: &str,
    ty: &crate::value::RtType,
    f: &ControlFormat,
    enum_by_text: &dyn Fn(u32, &str) -> Option<i32>,
) -> Option<Value> {
    use crate::value::RtType as T;
    Some(match ty {
        T::Text(_) => Value::Text(text.to_string().into()),
        T::Code(_) => Value::Code(text.to_string().into()),
        T::Integer => Value::Integer(if t.is_empty() { 0 } else { t.parse().ok()? }),
        T::BigInteger => Value::BigInteger(if t.is_empty() { 0 } else { t.parse().ok()? }),
        T::Decimal => Value::Decimal(if t.is_empty() {
            Decimal::ZERO
        } else {
            t.replace(',', "").parse().ok()?
        }),
        T::Boolean => match t.to_ascii_lowercase().as_str() {
            "yes" => Value::Boolean(true),
            "no" => Value::Boolean(false),
            _ => return None,
        },
        T::Option(members) => {
            let by_caption = f
                .option_captions
                .as_ref()
                .and_then(|c| c.iter().position(|x| x.eq_ignore_ascii_case(t)));
            let i = by_caption.or_else(|| members.iter().position(|x| x.eq_ignore_ascii_case(t)));
            // A blank text of spaces selects the first member whose caption is blank; the empty text does not, it is refused
            // (testpage/TestPageAssertEqualsOption.al:SetValue_BlankOption_AsSpaceText,
            // SetValue_BlankOption_AsEmptyText). Only a control with an OptionCaption is pinned; a control whose blank member
            // has no caption stays an assumption (testpage-setvalue-blank-option-member).
            let i = match i {
                Some(i) => i,
                None if t.is_empty() && !text.is_empty() => match &f.option_captions {
                    Some(c) => c.iter().position(|x| x.trim().is_empty())?,
                    None => {
                        let i = members.iter().position(|x| x.trim().is_empty())?;
                        crate::assumptions::assume(crate::assumptions::Assumption::TestPageSetValueBlankOptionMember);
                        i
                    }
                },
                None => return None,
            };
            Value::Option(i as i32, members.clone())
        }
        T::Enum(id) => Value::Enum(*id, enum_by_text(*id, t)?),
        T::Date => {
            if t.is_empty() {
                Value::Date(0, false)
            } else if matches!(t.to_ascii_lowercase().as_str(), "w" | "workdate") {
                // The shorthands, in any case: w and workdate are the work date, t and today the system date
                // (session-system/W5WorkDateShorthand.al, BC 27.5 and 29.0).
                Value::Date(f.work_date?, false)
            } else if matches!(t.to_ascii_lowercase().as_str(), "t" | "today") {
                Value::Date(f.today?, false)
            } else if let Some(rest) = t.strip_prefix('C').filter(|r| f.closing_dates && r.contains('/')) {
                // A ClosingDates control takes back the closing date it shows, `C12/31/2025`
                // (pages/properties/P119ReportFields.al:174, :257-258).
                match parse_typed(rest, rest, ty, f, enum_by_text)? {
                    Value::Date(d, false) if d != 0 => Value::Date(d, true),
                    _ => return None,
                }
            } else {
                // The ISO spelling, else `M/D/Y` as the control shows it; a two-digit year is
                // read as 1930-2029, as Evaluate does.
                if let Some(d) = crate::iso::parse_date(t).filter(|_| t.len() == 10) {
                    return Some(Value::Date(d, false));
                }
                let mut p = t.split('/');
                let (m, d, y) = (p.next()?, p.next()?, p.next()?);
                let (m, d, mut yy): (u32, u32, i32) = (m.parse().ok()?, d.parse().ok()?, y.parse().ok()?);
                if y.len() <= 2 {
                    // 00 to 30 are 2000 to 2030, 31 to 99 are 1931 to 1999 (pages/controls/W2ValueControls.al:
                    // Date_SessionFormatTwoDigitYear_Later, `06/15/30` is 2030-06-15).
                    yy += if yy <= 30 { 2000 } else { 1900 };
                }
                if p.next().is_some()
                    || !(1..=12).contains(&m)
                    || !(1..=9999).contains(&yy)
                    || d < 1
                    || d > crate::iso::days_in_month(yy, m)
                {
                    return None;
                }
                Value::Date(crate::format::day_from_civil(yy, m, d), false)
            }
        }
        T::Duration => Value::Duration(parse_duration(t)?),
        // A formula reads with or without its angle brackets and is kept in the bracketed form
        // (pages/controls/TestDateFormulaControl.al:63-100). BC reads a leading token and
        // stops, so it accepts texts the runtime's formula parser refuses; those are not pinned and
        // end as `Unsupported`, never as an invalid entry.
        T::DateFormula => {
            if t.is_empty() {
                Value::DateFormula(String::new())
            } else {
                let bracketed = if t.starts_with('<') {
                    t.to_string()
                } else {
                    format!("<{t}>")
                };
                crate::datefns::check_formula(&bracketed).ok()?;
                Value::DateFormula(bracketed)
            }
        }
        T::Time => {
            if t.is_empty() {
                Value::Time(0)
            } else {
                Value::Time(parse_time(t)? + 1)
            }
        }
        T::DateTime => {
            if t.is_empty() {
                Value::DateTime(0)
            } else {
                let (date, time) = t.split_once(' ')?;
                let mut p = date.split('/');
                let (m, d, y): (u32, u32, i32) = (
                    p.next()?.parse().ok()?,
                    p.next()?.parse().ok()?,
                    p.next()?.parse().ok()?,
                );
                if p.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
                    return None;
                }
                let day = i64::from(crate::format::day_from_civil(y, m, d) - 1);
                Value::DateTime(day * 86_400_000 + parse_time(time.trim())? + 1)
            }
        }
        T::Guid => {
            let h: String = t
                .trim_matches(|c| c == '{' || c == '}')
                .chars()
                .filter(|c| *c != '-')
                .collect();
            if h.len() != 32 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            let mut g = [0u8; 16];
            for (i, b) in g.iter_mut().enumerate() {
                *b = u8::from_str_radix(&h[2 * i..2 * i + 2], 16).ok()?;
            }
            Value::Guid(g)
        }
        _ => return None,
    })
}

/// `2:30:00 PM` / `2:30 PM` as milliseconds since midnight.
fn parse_time(t: &str) -> Option<i64> {
    let (hms, ap) = t.rsplit_once([' ', '\u{202f}'])?;
    let pm = match ap.to_ascii_uppercase().as_str() {
        "AM" => false,
        "PM" => true,
        _ => return None,
    };
    let mut p = hms.split(':');
    let h: i64 = p.next()?.parse().ok()?;
    let m: i64 = p.next()?.parse().ok()?;
    let s: i64 = match p.next() {
        Some(s) => s.parse().ok()?,
        None => 0,
    };
    if p.next().is_some() || !(1..=12).contains(&h) || m > 59 || s > 59 {
        return None;
    }
    Some((((h % 12) + if pm { 12 } else { 0 }) * 3600 + m * 60 + s) * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Value {
        Value::Decimal(Decimal::from_str(s).unwrap())
    }
    fn t(v: &Value, f: &ControlFormat) -> String {
        control_text(v, f, &|_, o| format!("enum{o}"))
    }
    fn places(min: u32, max: u32) -> ControlFormat {
        ControlFormat {
            decimal_places: Some((min, max)),
            ..Default::default()
        }
    }

    // pages/properties/P119ReportFields.al:174, :257-258: a closing date set on a ClosingDates control
    // reads back as `C12/31/2025`.
    #[test]
    fn a_closing_dates_control_reads_back_the_closing_date_it_shows() {
        let f = ControlFormat {
            closing_dates: true,
            ..Default::default()
        };
        let d = crate::format::day_from_civil(2025, 12, 31);
        let v = parse_control_text("C12/31/2025", &crate::value::RtType::Date, &f, &|_, _| None);
        assert!(matches!(v, Ok(Value::Date(x, true)) if x == d), "{v:?}");
        assert_eq!(t(&Value::Date(d, true), &f), "C12/31/2025");
    }

    // Only the upper-case mark is pinned (P119ReportFields.al:174): a lower-case `c` is refused, not rejected.
    #[test]
    fn a_lower_case_closing_mark_is_refused() {
        let f = ControlFormat {
            closing_dates: true,
            ..Default::default()
        };
        let v = parse_control_text("c12/31/2025", &crate::value::RtType::Date, &f, &|_, _| None);
        assert!(matches!(v, Err(ParseError::Unsupported)), "{v:?}");
    }

    // testpage/TestPageFieldFormats_Tests.al Value_Decimal_TwoPlaces / NoDecimalPlaces
    #[test]
    fn decimals_default_to_two_places_with_thousands_and_half_away_from_zero_rounding() {
        let f = ControlFormat::default();
        for (v, want) in [
            ("0", "0.00"),
            ("999.5", "999.50"),
            ("1000", "1,000.00"),
            ("1234567.89", "1,234,567.89"),
            ("-1234567.89", "-1,234,567.89"),
            ("0.005", "0.01"),
            ("0.015", "0.02"),
            ("99999999999999.99", "99,999,999,999,999.99"),
            ("3.14159", "3.14"),
            ("1234567.891", "1,234,567.89"),
        ] {
            assert_eq!(t(&d(v), &f), want, "{v}");
        }
    }

    // Value_Decimal_OnePlace / FivePlaces / ZeroToFivePlaces
    #[test]
    fn decimal_places_set_minimum_and_maximum() {
        assert_eq!(t(&d("3.25"), &places(1, 1)), "3.3");
        assert_eq!(t(&d("1234.56"), &places(1, 1)), "1,234.6");
        assert_eq!(t(&d("0"), &places(1, 1)), "0.0");
        assert_eq!(t(&d("1.5"), &places(5, 5)), "1.50000");
        assert_eq!(t(&d("1234.12345"), &places(5, 5)), "1,234.12345");
        for (v, want) in [
            ("1000", "1,000"),
            ("1234.5", "1,234.5"),
            ("1.123456", "1.12346"),
            ("0", "0"),
        ] {
            assert_eq!(t(&d(v), &places(0, 5)), want, "{v}");
        }
    }

    // Value_Decimal_BlankZero / BlankZeroAndPositive / BlankNegative, Value_Integer_BlankZero
    #[test]
    fn blank_rules() {
        let bz = ControlFormat {
            blank_zero: true,
            ..Default::default()
        };
        assert_eq!(t(&d("0"), &bz), "");
        assert_eq!(t(&d("5"), &bz), "5.00");
        assert_eq!(t(&Value::Integer(0), &bz), "");
        assert_eq!(t(&Value::Integer(7), &bz), "7");
        let zp = ControlFormat {
            blank: Blank::ZeroAndPos,
            ..Default::default()
        };
        assert_eq!(
            (t(&d("-5"), &zp), t(&d("0"), &zp), t(&d("5"), &zp)),
            ("-5.00".into(), "".into(), "".into())
        );
        let neg = ControlFormat {
            blank: Blank::Neg,
            ..Default::default()
        };
        assert_eq!(
            (t(&d("-5"), &neg), t(&d("0"), &neg), t(&d("5"), &neg)),
            ("".into(), "0.00".into(), "5.00".into())
        );
    }

    // Value_Integer / BigInteger: plain digits, no thousands separator.
    #[test]
    fn integers_have_no_separator() {
        let f = ControlFormat::default();
        assert_eq!(t(&Value::Integer(1234567), &f), "1234567");
        assert_eq!(t(&Value::Integer(-1234567), &f), "-1234567");
        assert_eq!(t(&Value::BigInteger(9_000_000_000), &f), "9000000000");
    }

    // Value_Boolean, Value_Option (captions), Value_Guid
    #[test]
    fn booleans_options_guids() {
        let f = ControlFormat::default();
        assert_eq!(t(&Value::Boolean(true), &f), "Yes");
        assert_eq!(t(&Value::Boolean(false), &f), "No");
        let members: std::sync::Arc<[String]> = std::sync::Arc::from(vec!["Alpha".to_string(), "Beta".to_string()]);
        let cap = ControlFormat {
            option_captions: Some(vec!["Eins".into(), "Zwei".into()]),
            ..Default::default()
        };
        assert_eq!(t(&Value::Option(1, members.clone()), &cap), "Zwei");
        assert_eq!(t(&Value::Option(1, members), &f), "Beta");
        let g = [
            0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
        ];
        assert_eq!(t(&Value::Guid(g), &f), "{11111111-2222-3333-4444-555555555555}");
        assert_eq!(t(&Value::Guid([0; 16]), &f), "{00000000-0000-0000-0000-000000000000}");
    }

    // Value_Date / Value_Time / Value_DateTime: the short forms, blank when undefined.
    #[test]
    fn dates_and_times_use_the_short_forms() {
        let f = ControlFormat::default();
        let day = crate::format::day_from_civil(2024, 3, 2);
        assert_eq!(t(&Value::Date(day, false), &f), "3/2/2024");
        assert_eq!(t(&Value::Date(0, false), &f), "");
        let tm = |h: i64, m: i64, s: i64| Value::Time(((h * 60 + m) * 60 + s) * 1000 + 1);
        assert_eq!(t(&tm(12, 34, 56), &f), "12:34:56 PM");
        assert_eq!(t(&tm(23, 59, 59), &f), "11:59:59 PM");
        assert_eq!(t(&tm(0, 0, 1), &f), "12:00:01 AM");
        assert_eq!(t(&Value::Time(0), &f), "");
        let dt = |y, m, d, h: i64, mi: i64| {
            Value::DateTime(
                i64::from(crate::format::day_from_civil(y, m, d) - 1) * 86_400_000 + (h * 60 + mi) * 60_000 + 1,
            )
        };
        assert_eq!(t(&dt(2024, 3, 2, 12, 34), &f), "3/2/2024 12:34 PM");
        assert_eq!(t(&dt(2024, 3, 2, 0, 0), &f), "3/2/2024 12:00 AM");
        assert_eq!(t(&dt(2024, 12, 31, 23, 59), &f), "12/31/2024 11:59 PM");
        assert_eq!(t(&Value::DateTime(0), &f), "");
    }

    // Value_Duration
    #[test]
    fn durations_are_spelled_in_words() {
        let f = ControlFormat::default();
        for (ms, want) in [
            (0, ""),
            (5000, "5 seconds"),
            (3_723_000, "1 hour 2 minutes 3 seconds"),
            (90_000_000, "1 day 1 hour"),
            (86_400_000, "1 day"),
            (1, "1 millisecond"),
            // testpage/TestPageRuntimeContracts.al:157: every unit of a negative Duration is negative.
            (-90_061_001, "-1 day -1 hour -1 minute -1 second -1 millisecond"),
            (-5000, "-5 seconds"),
        ] {
            assert_eq!(t(&Value::Duration(ms), &f), want, "{ms}");
        }
    }

    // testpage/TestPageBooleanSpelling_Tests.al: SetValue accepts Yes/No and refuses True/False;
    // testpage/TestPageFieldFormats_Tests.al:1-40: "a blank text written to a numeric control is zero".
    #[test]
    fn control_text_parses_back() {
        use crate::value::RtType as T;
        let f = ControlFormat::default();
        let none = |_: u32, _: &str| None;
        let p = |text: &str, ty: &T| parse_control_text(text, ty, &f, &none);
        assert!(matches!(p("Yes", &T::Boolean), Ok(Value::Boolean(true))));
        assert!(matches!(p("no", &T::Boolean), Ok(Value::Boolean(false))));
        assert_eq!(p("False", &T::Boolean).unwrap_err(), ParseError::Invalid);
        assert!(matches!(p("", &T::Integer), Ok(Value::Integer(0))));
        assert!(matches!(p("12", &T::Integer), Ok(Value::Integer(12))));
        assert_eq!(p("abc", &T::Integer).unwrap_err(), ParseError::Invalid);
        // testpage/TestPageFieldFormats_Tests.al:1724 SetValue_Duration_Text, :1745 _BlankText, :1703 SetValue_Duration
        // (the control's own spelling reads back); other spellings are not pinned.
        assert!(matches!(p("5 seconds", &T::Duration), Ok(Value::Duration(5000))));
        assert!(matches!(p("", &T::Duration), Ok(Value::Duration(0))));
        assert!(matches!(
            p("1 hour 2 minutes 3 seconds", &T::Duration),
            Ok(Value::Duration(3_723_000))
        ));
        assert!(matches!(
            p("-1 day -1 millisecond", &T::Duration),
            Ok(Value::Duration(-86_400_001))
        ));
        // A sign on the first unit only is not BC's spelling (testpage/TestPageRuntimeContracts.al:157).
        assert_eq!(
            p("-1 day 1 millisecond", &T::Duration).unwrap_err(),
            ParseError::Unsupported
        );
        assert_eq!(p("5s", &T::Duration).unwrap_err(), ParseError::Unsupported);
        assert_eq!(
            p("1 second 2 hours", &T::Duration).unwrap_err(),
            ParseError::Unsupported
        );
        // pages/controls/TestDateFormulaControl.al: '<1D>' and the unbracketed '3D' name the same formula.
        assert!(matches!(p("<1D>", &T::DateFormula), Ok(Value::DateFormula(s)) if s == "<1D>"));
        assert!(matches!(p("3D", &T::DateFormula), Ok(Value::DateFormula(s)) if s == "<3D>"));
        assert!(matches!(p("", &T::DateFormula), Ok(Value::DateFormula(s)) if s.is_empty()));
        assert!(
            matches!(p("1,234.50", &T::Decimal), Ok(Value::Decimal(d)) if d == Decimal::from_str("1234.50").unwrap())
        );
        assert!(
            matches!(p("3/2/2024", &T::Date), Ok(Value::Date(d, false)) if d == crate::format::day_from_civil(2024, 3, 2))
        );
        // The spelling Value() answers for a Time and a DateTime reads back.
        assert!(matches!(p("2:30:00 PM", &T::Time), Ok(Value::Time(t)) if t == (14 * 3600 + 30 * 60) * 1000 + 1));
        assert!(matches!(p("12:00:01 AM", &T::Time), Ok(Value::Time(t)) if t == 1001));
        assert!(matches!(p("3/2/2024 12:34 PM", &T::DateTime), Ok(Value::DateTime(_))));
        let cap = ControlFormat {
            option_captions: Some(vec!["Eins".into(), "Zwei".into()]),
            ..Default::default()
        };
        let o = T::Option(vec!["Alpha".into(), "Beta".into()].into());
        assert!(matches!(
            parse_control_text("Zwei", &o, &cap, &none),
            Ok(Value::Option(1, _))
        ));
        assert!(matches!(
            parse_control_text("Alpha", &o, &cap, &none),
            Ok(Value::Option(0, _))
        ));
    }

    // testpage/TestPageTemporalSetValue_Tests.al:TemporalSetValue_PageVariableDateControlTakesIsoText
    // and ...TakesFormattedText: a Date control takes the ISO spelling and the `Format(Date)` text
    // (`01/01/24`, a two-digit year read as in Evaluate: 1930-2029).
    #[test]
    fn a_date_control_takes_iso_text_and_a_two_digit_year() {
        use crate::value::RtType as T;
        let f = ControlFormat::default();
        let none = |_: u32, _: &str| None;
        let date = |text: &str| match parse_control_text(text, &T::Date, &f, &none) {
            Ok(Value::Date(d, false)) => Some(d),
            _ => None,
        };
        let jan1 = crate::format::day_from_civil(2024, 1, 1);
        assert_eq!(date("2024-01-01"), Some(jan1));
        assert_eq!(date("01/01/24"), Some(jan1));
        assert_eq!(date("1/1/2024"), Some(jan1));
        assert_eq!(date("12/31/99"), Some(crate::format::day_from_civil(1999, 12, 31)));
        assert_eq!(date("2024-13-01"), None);
        assert_eq!(date("not-a-date"), None);
    }

    #[test]
    fn precision_and_decimal_places_parse() {
        assert_eq!(parse_decimal_places("2 : 2"), Some((2, 2)));
        assert_eq!(parse_decimal_places("0:5"), Some((0, 5)));
        assert_eq!(parse_decimal_places("3"), Some((3, 3)));
        assert_eq!(parse_precision("<Precision,4:4><Standard Format,0>"), Some((4, 4)));
        assert_eq!(parse_precision("EUR"), None);
    }
}
