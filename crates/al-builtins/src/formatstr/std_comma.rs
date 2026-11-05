//! `<Standard Format,N>` with a `<Comma,x>` element on a Decimal (al-runner issue 6792). Real BC's values: corpus branch
//! al-runtime-rs-staging-r6792 0394701c, format-evaluate/FormatCommaMore.al:258-640 (BC 27.5, 28.5 and 29.0 gave the same
//! values; GlobalLanguage 1031 writes what 1033 writes).

use super::INVALID_FORMAT_MARK;
use crate::format::FormatCtx;
use crate::value::Value;

/// The `<Precision,A:B>` (or none) and `<Standard Format,N>` pairs the probe covers, with the `<Comma,x>` characters measured
/// after them. Standard Format 0, 1, 2 and 9 write the decimal point as `x` and keep the thousands commas (`1,234,567,9`,
/// :294-370, :408-540); 3 and 4 ignore the element (:484, :503).
const PINNED: &[(&str, u8, &[char])] = &[
    ("<Precision,1:1>", 0, &[',', '.', ';']),
    ("", 0, &[',', '.']),
    ("<Precision,2:2>", 0, &[',']),
    ("<Precision,0:2>", 0, &[',']),
    ("<Precision,1:1>", 1, &[',']),
    ("<Precision,1:1>", 2, &[',']),
    ("<Precision,1:1>", 3, &[',']),
    ("<Precision,1:1>", 4, &[',']),
    ("", 9, &[',']),
];

/// `Format(v, length, template)` for a template made of an optional `<Precision,A:B>`, a `<Standard Format,N>` and a
/// `<Comma,x>` element. `None`: not such a template, or a shape the probe does not pin (the caller refuses it).
pub(super) fn standard_comma(
    v: &Value,
    length: i32,
    template: &str,
    ctx: &dyn FormatCtx,
) -> Option<Result<String, String>> {
    const P11_STD0: &str = "<Precision,1:1><Standard Format,0>";
    match v {
        // An Integer and a BigInteger write their digits (:620 Format_Std0CommaComma_OtherTypes_En).
        Value::Integer(_) | Value::BigInteger(_) => {
            let pinned = [
                "<Precision,1:1><Standard Format,0><Comma,,>",
                "<Standard Format,0><Comma,,>",
            ];
            if length != 0 || !pinned.contains(&template) {
                return None;
            }
            return Some(Ok(match v {
                Value::Integer(i) => i.to_string(),
                Value::BigInteger(i) => i.to_string(),
                _ => unreachable!("integer"),
            }));
        }
        Value::Decimal(_) => {}
        _ => return None,
    }
    // `<Comma>` without a character is the invalid-format error; the element before or between the two codes changes nothing
    // (:541 `bare_`, :571 `first_`, `mid_`).
    let (prefix, point) = match template {
        "<Precision,1:1><Standard Format,0><Comma>" => return Some(Err(format!("{INVALID_FORMAT_MARK}{template}"))),
        "<Comma,,><Precision,1:1><Standard Format,0>" | "<Precision,1:1><Comma,,><Standard Format,0>" => {
            (P11_STD0, None)
        }
        t => {
            let body = t.strip_suffix('>')?;
            let (prefix, x) = body.rsplit_once("<Comma,")?;
            let mut chars = x.chars();
            let (Some(x), None) = (chars.next(), chars.next()) else {
                return None;
            };
            let (precision, std) = prefix.strip_suffix('>')?.rsplit_once("<Standard Format,")?;
            let std: u8 = std.parse().ok()?;
            let (_, _, xs) = PINNED.iter().find(|(p, n, _)| *p == precision && *n == std)?;
            if !xs.contains(&x) {
                return None;
            }
            (prefix, if std == 3 || std == 4 { None } else { Some(x) })
        }
    };
    let out = match super::format_with_string(v, 0, prefix, ctx) {
        Ok(out) => out,
        Err(e) => return Some(Err(e)),
    };
    // Only `.` separates the decimals here: the thousands separator of these Standard Formats is `,`.
    let out = match point {
        Some(x) => out.replacen('.', &x.to_string(), 1),
        None => out,
    };
    // A Length pads on the left (:601, Length 12); a negative one, or one shorter than the text, is not pinned.
    if length < 0 || (length > 0 && out.chars().count() > length as usize) {
        return None;
    }
    Some(Ok(crate::format::pad(out, length)))
}

#[cfg(test)]
mod tests {
    use crate::native::Signal;
    use crate::native::tests::{call, t};
    use crate::value::Value;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn fmt(v: Value, length: i32, template: &str) -> Result<Value, Signal> {
        call("System.Format#2", vec![v, Value::Integer(length), t(template)])
    }

    /// A format string, a Length, and the inputs with the text BC wrote for each.
    type Case = (&'static str, i32, &'static [(&'static str, &'static str)]);

    #[rustfmt::skip]
    const CASES: &[Case] = &[
        // Format_Prec11Std0CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,0><Comma,,>", 0, &[("0", "0,0"), ("0.04", "0,0"), ("-0.04", "-0,0"), ("0.05", "0,1"), ("-1.45", "-1,5"), ("6.38", "6,4"), ("999.94", "999,9"), ("999.95", "1,000,0"), ("1234.5", "1,234,5"), ("1234567.89", "1,234,567,9")]),
        // Format_Std0CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Standard Format,0><Comma,,>", 0, &[("0", "0"), ("0.04", "0,04"), ("-0.04", "-0,04"), ("0.05", "0,05"), ("-1.45", "-1,45"), ("6.38", "6,38"), ("999.94", "999,94"), ("999.95", "999,95"), ("1234.5", "1,234,5"), ("1234567.89", "1,234,567,89")]),
        // Format_Prec11Std0CommaComma_De (FormatCommaMore.al, GlobalLanguage 1031)
        ("<Precision,1:1><Standard Format,0><Comma,,>", 0, &[("0", "0,0"), ("0.04", "0,0"), ("-0.04", "-0,0"), ("0.05", "0,1"), ("-1.45", "-1,5"), ("6.38", "6,4"), ("999.94", "999,9"), ("999.95", "1,000,0"), ("1234.5", "1,234,5"), ("1234567.89", "1,234,567,9")]),
        // Format_Std0CommaComma_De (FormatCommaMore.al, GlobalLanguage 1031)
        ("<Standard Format,0><Comma,,>", 0, &[("0", "0"), ("0.04", "0,04"), ("-0.04", "-0,04"), ("0.05", "0,05"), ("-1.45", "-1,45"), ("6.38", "6,38"), ("999.94", "999,94"), ("999.95", "999,95"), ("1234.5", "1,234,5"), ("1234567.89", "1,234,567,89")]),
        // Format_Prec11Std0CommaDot_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,0><Comma,.>", 0, &[("0", "0.0"), ("0.04", "0.0"), ("-0.04", "-0.0"), ("0.05", "0.1"), ("-1.45", "-1.5"), ("6.38", "6.4"), ("999.94", "999.9"), ("999.95", "1,000.0"), ("1234.5", "1,234.5"), ("1234567.89", "1,234,567.9")]),
        // Format_Std0CommaDot_De (FormatCommaMore.al, GlobalLanguage 1031)
        ("<Standard Format,0><Comma,.>", 0, &[("0", "0"), ("0.04", "0.04"), ("-0.04", "-0.04"), ("0.05", "0.05"), ("-1.45", "-1.45"), ("6.38", "6.38"), ("999.94", "999.94"), ("999.95", "999.95"), ("1234.5", "1,234.5"), ("1234567.89", "1,234,567.89")]),
        // Format_Prec22Std0CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,2:2><Standard Format,0><Comma,,>", 0, &[("0", "0,00"), ("0.04", "0,04"), ("-0.04", "-0,04"), ("0.05", "0,05"), ("-1.45", "-1,45"), ("6.38", "6,38"), ("999.94", "999,94"), ("999.95", "999,95"), ("1234.5", "1,234,50"), ("1234567.89", "1,234,567,89")]),
        // Format_Prec02Std0CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,0:2><Standard Format,0><Comma,,>", 0, &[("0", "0"), ("0.04", "0,04"), ("-0.04", "-0,04"), ("0.05", "0,05"), ("-1.45", "-1,45"), ("6.38", "6,38"), ("999.94", "999,94"), ("999.95", "999,95"), ("1234.5", "1,234,5"), ("1234567.89", "1,234,567,89")]),
        // Format_Prec11Std1CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,1><Comma,,>", 0, &[("0", "0,0"), ("0.04", "0,0"), ("-0.04", "-0,0"), ("0.05", "0,1"), ("-1.45", "-1,5"), ("6.38", "6,4"), ("999.94", "999,9"), ("999.95", "1000,0"), ("1234.5", "1234,5"), ("1234567.89", "1234567,9")]),
        // Format_Prec11Std2CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,2><Comma,,>", 0, &[("0", "0,0"), ("0.04", "0,0"), ("-0.04", "-0,0"), ("0.05", "0,1"), ("-1.45", "-1,5"), ("6.38", "6,4"), ("999.94", "999,9"), ("999.95", "1000,0"), ("1234.5", "1234,5"), ("1234567.89", "1234567,9")]),
        // Format_Prec11Std3CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,3><Comma,,>", 0, &[("0", "0.0 "), ("0.04", "0.0 "), ("-0.04", "0.0-"), ("0.05", "0.1 "), ("-1.45", "1.5-"), ("6.38", "6.4 "), ("999.94", "999.9 "), ("999.95", "1,000.0 "), ("1234.5", "1,234.5 "), ("1234567.89", "1,234,567.9 ")]),
        // Format_Prec11Std4CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,4><Comma,,>", 0, &[("0", "0.0 "), ("0.04", "0.0 "), ("-0.04", "0.0-"), ("0.05", "0.1 "), ("-1.45", "1.5-"), ("6.38", "6.4 "), ("999.94", "999.9 "), ("999.95", "1000.0 "), ("1234.5", "1234.5 "), ("1234567.89", "1234567.9 ")]),
        // Format_Std9CommaComma_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Standard Format,9><Comma,,>", 0, &[("0", "0"), ("0.04", "0,04"), ("-0.04", "-0,04"), ("0.05", "0,05"), ("-1.45", "-1,45"), ("6.38", "6,38"), ("999.94", "999,94"), ("999.95", "999,95"), ("1234.5", "1234,5"), ("1234567.89", "1234567,89")]),
        // Format_Prec11Std0CommaOther_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,0><Comma,;>", 0, &[("0", "0;0"), ("0.04", "0;0"), ("-0.04", "-0;0"), ("0.05", "0;1"), ("-1.45", "-1;5"), ("6.38", "6;4"), ("999.94", "999;9"), ("999.95", "1,000;0"), ("1234.5", "1,234;5"), ("1234567.89", "1,234,567;9")]),
        // Format_Prec11Std0CommaOther_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,0><Comma>", 0, &[("0", "error"), ("0.04", "error"), ("-0.04", "error"), ("0.05", "error"), ("-1.45", "error"), ("6.38", "error"), ("999.94", "error"), ("999.95", "error"), ("1234.5", "error"), ("1234567.89", "error")]),
        // Format_CommaFirstPrec11Std0_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Comma,,><Precision,1:1><Standard Format,0>", 0, &[("0", "0.0"), ("0.04", "0.0"), ("-0.04", "-0.0"), ("0.05", "0.1"), ("-1.45", "-1.5"), ("6.38", "6.4"), ("999.94", "999.9"), ("999.95", "1,000.0"), ("1234.5", "1,234.5"), ("1234567.89", "1,234,567.9")]),
        // Format_CommaFirstPrec11Std0_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Comma,,><Standard Format,0>", 0, &[("0", "0.0"), ("0.04", "0.0"), ("-0.04", "-0.0"), ("0.05", "0.1"), ("-1.45", "-1.5"), ("6.38", "6.4"), ("999.94", "999.9"), ("999.95", "1,000.0"), ("1234.5", "1,234.5"), ("1234567.89", "1,234,567.9")]),
        // Format_Prec11Std0CommaComma_Length12_En (FormatCommaMore.al, GlobalLanguage 1033)
        ("<Precision,1:1><Standard Format,0><Comma,,>", 12, &[("0", "         0,0"), ("0.04", "         0,0"), ("-0.04", "        -0,0"), ("0.05", "         0,1"), ("-1.45", "        -1,5"), ("6.38", "         6,4"), ("999.94", "       999,9"), ("999.95", "     1,000,0"), ("1234.5", "     1,234,5"), ("1234567.89", " 1,234,567,9")]),
    ];

    #[test]
    fn decimal_values_of_the_probe() {
        for (template, length, rows) in CASES {
            for (input, want) in *rows {
                let got = fmt(Value::Decimal(Decimal::from_str(input).unwrap()), *length, template);
                match (*want, got) {
                    ("error", Err(Signal::Error(e))) => assert_eq!(e.code, "InvalidFormatString", "{template} {input}"),
                    (w, Ok(Value::Text(s))) => assert_eq!(s.as_str(), w, "{template} {input}"),
                    (w, other) => panic!("{template} {input}: want {w}, got {other:?}"),
                }
            }
        }
    }

    /// FormatCommaMore.al:620 (Format_Std0CommaComma_OtherTypes_En): an Integer and a BigInteger write their digits, without
    /// grouping and without decimals.
    #[test]
    fn integer_and_biginteger_values_of_the_probe() {
        let cases = [
            (
                Value::Integer(1234),
                "<Precision,1:1><Standard Format,0><Comma,,>",
                "1234",
            ),
            (Value::Integer(1234), "<Standard Format,0><Comma,,>", "1234"),
            (Value::Integer(0), "<Precision,1:1><Standard Format,0><Comma,,>", "0"),
            (
                Value::BigInteger(1234567),
                "<Precision,1:1><Standard Format,0><Comma,,>",
                "1234567",
            ),
            (Value::BigInteger(1234567), "<Standard Format,0><Comma,,>", "1234567"),
        ];
        for (v, template, want) in cases {
            match fmt(v, 0, template) {
                Ok(Value::Text(s)) => assert_eq!(s.as_str(), want, "{template}"),
                other => panic!("{template}: {other:?}"),
            }
        }
    }

    /// A Precision or a Standard Format number the probe does not cover stays refused.
    #[test]
    fn unpinned_shapes_are_refused() {
        for template in [
            "<Precision,3:3><Standard Format,0><Comma,,>",
            "<Standard Format,5><Comma,,>",
            "<Standard Format,1><Comma,,>",
        ] {
            match fmt(Value::Decimal(Decimal::from_str("1.5").unwrap()), 0, template) {
                Err(Signal::Unsupported(_)) => {}
                other => panic!("{template}: {other:?}"),
            }
        }
    }
}
