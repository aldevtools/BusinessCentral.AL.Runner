//! `<1000Character,x>` and `<Comma,y>` in a format string of a number (al-runner issue 7369). Pinned by corpus master
//! format-evaluate/FormatCommaMore.al (corpus PR 1247): Format_CommaAfterIntegerAndThousandChar_IssueStrings_En, _Variants1_En,
//! _Variants2_En, _Variants3_En and _De; and format-evaluate/format-matrix/FMNumeric.al (corpus PR 1249) `<Integer><1000Character,.>`
//! on an Integer, a BigInteger and a Decimal.
//!
//! What the cells show, the same under GlobalLanguage 1033 and 1031:
//! - `<1000Character,x>` groups the digits of `<Integer>` in threes with `x` (`1.234.567`).
//! - `<Comma,y>` after a decimals code (`<Decimals>`, `<Decimals,N>`, `<Decimal,N>`) is the character before the fraction; before
//!   it the point stays (`<Sign><Integer><Comma,,><Decimals,3>` is `1234.50`).
//! - A whole Decimal shows its decimals code as zeros (`0,00`, `1.000,00`, FMNumeric.al:1300, :2126).
//! - `<Decimals,N>`: N counts the separator; the fraction is padded with zeros to N - 1 digits, and a longer one is N stars (not
//!   rounded). `<Decimals>`: every digit of the fraction.
//! - `<Sign>` writes the minus of a negative value; without it no sign is written.

use rust_decimal::Decimal;

/// One code of a template of this family.
enum Code {
    Sign,
    Integer,
    IntegerThousand,
    /// `<Decimals>` (`None`) or `<Decimals,N>`.
    Decimals(Option<usize>),
}

/// The text of `value` for `template`, or `None` when the template is not one of this family (the caller refuses or reads it
/// otherwise). `Some(Err(()))` is the invalid-format error.
pub(super) fn format(value: Decimal, whole_number_type: bool, template: &str) -> Option<Result<String, ()>> {
    let mut codes = Vec::new();
    let mut thousand: Option<char> = None;
    let mut comma: Option<(char, bool)> = None;
    let mut rest = template;
    while !rest.is_empty() {
        let inner = rest.strip_prefix('<')?;
        let close = inner.find('>')?;
        let code = &inner[..close];
        rest = &inner[close + 1..];
        let seen_decimals = codes.iter().any(|c| matches!(c, Code::Decimals(_)));
        match code {
            "Sign" => codes.push(Code::Sign),
            "Integer" => codes.push(Code::Integer),
            "Integer Thousand" => codes.push(Code::IntegerThousand),
            "Decimals" => codes.push(Code::Decimals(None)),
            c if c.starts_with("1000Character,") && thousand.is_none() => {
                let mut ch = c["1000Character,".len()..].chars();
                thousand = Some(ch.next()?);
                if ch.next().is_some() {
                    return None;
                }
            }
            c if c.starts_with("Comma,") && comma.is_none() => {
                let mut ch = c["Comma,".len()..].chars();
                comma = Some((ch.next()?, seen_decimals));
                if ch.next().is_some() {
                    return None;
                }
            }
            // `<Comma>` without its character is the invalid-format error on every number (FMNumeric.al:122, :240, :830).
            "Comma" => return Some(Err(())),
            c => {
                let width = c.strip_prefix("Decimals,").or_else(|| c.strip_prefix("Decimal,"))?;
                codes.push(Code::Decimals(Some(width.parse().ok().filter(|w| *w > 0)?)));
            }
        }
    }
    if thousand.is_none() && comma.is_none() {
        return None;
    }
    let integers = codes
        .iter()
        .filter(|c| matches!(c, Code::Integer | Code::IntegerThousand))
        .count();
    let decimals = codes.iter().filter(|c| matches!(c, Code::Decimals(_))).count();
    // One integer code; `<1000Character>` only with `<Integer>`; decimals codes only on a Decimal. The other combinations are not
    // pinned.
    if integers != 1 || decimals > 1 || (thousand.is_some() && codes.iter().any(|c| matches!(c, Code::IntegerThousand)))
    {
        return None;
    }
    // On an Integer or a BigInteger a decimals code with `<Comma>` or a width is the invalid-format error (FormatCommaMore.al:
    // the cells `_0` and `_1000`, whose values are Integer literals; FMNumeric.al:76-125, :783-833); `<Decimals>` alone there
    // with a `<1000Character>` is not pinned.
    if whole_number_type && decimals > 0 {
        let width = codes.iter().any(|c| matches!(c, Code::Decimals(Some(_))));
        return (comma.is_some() || width).then_some(Err(()));
    }
    let abs = value.abs();
    let point = match comma {
        Some((c, true)) => c,
        _ => '.',
    };
    let mut out = String::new();
    for c in &codes {
        match c {
            Code::Sign if value.is_sign_negative() && !value.is_zero() => out.push('-'),
            Code::Sign => {}
            Code::Integer => out.push_str(&grouped(abs.trunc(), thousand)),
            Code::IntegerThousand => out.push_str(&grouped(abs.trunc(), Some(','))),
            Code::Decimals(width) => {
                let digits = abs.fract().normalize().to_string();
                let digits = digits.strip_prefix("0.").unwrap_or("");
                match width {
                    None if !digits.is_empty() => {
                        out.push(point);
                        out.push_str(digits);
                    }
                    None => {}
                    Some(w) if digits.len() > w - 1 => out.push_str(&"*".repeat(*w)),
                    Some(w) => {
                        out.push(point);
                        out.push_str(&format!("{digits:0<width$}", width = w - 1));
                    }
                }
            }
        }
    }
    Some(Ok(out))
}

/// The whole number `n` (not negative) in digits, grouped in threes with `sep` when there is one.
fn grouped(n: Decimal, sep: Option<char>) -> String {
    let digits = n.normalize().to_string();
    let Some(sep) = sep else {
        return digits;
    };
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(ch);
    }
    out
}
