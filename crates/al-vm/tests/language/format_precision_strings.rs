//! `Format` of a Decimal with `<Precision,2:26>` before a standard format, and `<Precision,2:2>` before `<Sign><Integer><Decimals>`
//! with `<Comma,.>` or `<Comma,,>`, before `<Integer Thousand><Decimals><Comma,,>`, and after `<Sign>` (corpus branch
//! al-runtime-rs-staging-pub-tp, types/lt-format-comma-format-string/FormatCommaFormatString.al TP_Format_PrecisionStrings,
//! TP_Format_PrecisionStrings_Small).

use al_testkit::text;

fn run(body: &str) -> String {
    let src = format!("codeunit 50100 C {{ {body} }}");
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

const FORMATS: [&str; 6] = [
    "<Precision,2:26><Standard Format,4>",
    "<Precision,2:26><Standard Format,2>",
    "<Precision,2:2><Sign><Integer><Decimals><Comma,.>",
    "<Precision,2:2><Sign><Integer><Decimals><Comma,,>",
    "<Precision,2:2><Integer Thousand><Decimals><Comma,,>",
    "<Sign><Precision,2:2><Integer><Decimals>",
];

fn formats(value: &str) -> String {
    let mut body = String::from("procedure F(): Text var D: Decimal; R: Text; begin D := ");
    body.push_str(value);
    body.push(';');
    for f in FORMATS {
        body.push_str(&format!(" R += Format(D, 0, '{f}') + '|';"));
    }
    body.push_str(" exit(R); end;");
    run(&body)
}

#[test]
fn precision_strings_of_the_pinned_values() {
    assert_eq!(formats("0"), "0.00 |0.00|0.00|0,00|0,00|0|");
    assert_eq!(formats("1234.5"), "1234.50 |1234.50|1234.50|1234,50|1,234,50|1234.5|");
    assert_eq!(
        formats("-1234.567"),
        "1234.567-|-1234.567|-1234.57|-1234,57|1,234,57|-1234.567|"
    );
    assert_eq!(formats("0.125"), "0.125 |0.125|0.13|0,13|0,13|0.125|");
    assert_eq!(
        formats("1234567.891"),
        "1234567.891 |1234567.891|1234567.89|1234567,89|1,234,567,89|1234567.891|"
    );
    assert_eq!(
        formats("0.1234567891234"),
        "0.1234567891234 |0.1234567891234|0.12|0,12|0,12|0.1234567891234|"
    );
    assert_eq!(formats("-0.005"), "0.005-|-0.005|-0.01|-0,01|0,01|-0.005|");
    assert_eq!(formats("12"), "12.00 |12.00|12.00|12,00|12,00|12|");
}

// FormatCommaFormatString.al TQ_Format_PrecisionComma (corpus branch al-runtime-rs-staging-pub-tq): `<Precision,0:0>`, `<Precision,1:3>`
// and `<Precision,3:3>` first, with `<Comma,,>` or `<Comma,.>`.
fn tq_comma(value: &str) -> String {
    let mut body = String::from("procedure F(): Text var D: Decimal; R: Text; begin D := ");
    body.push_str(value);
    body.push(';');
    for f in [
        "<Precision,0:0><Integer><Decimals><Comma,,>",
        "<Precision,1:3><Integer Thousand><Decimals><Comma,.>",
        "<Precision,3:3><Sign><Integer><Decimals><Comma,,>",
    ] {
        body.push_str(&format!(" R += Format(D, 0, '{f}') + '|';"));
    }
    body.push_str(" exit(R); end;");
    run(&body)
}

#[test]
fn precision_comma_strings_of_writer_tq() {
    assert_eq!(tq_comma("1.5"), "2|1.5|1,500|");
    assert_eq!(tq_comma("-2.5"), "3|2.5|-2,500|");
    assert_eq!(tq_comma("1234.56789"), "1235|1,234.568|1234,568|");
    assert_eq!(tq_comma("0.0004"), "0|0.0|0,000|");
    assert_eq!(tq_comma("-0.4"), "0|0.4|-0,400|");
}

// A leading `<Precision,A:B>` without a standard format is pinned for 0:0, 1:3, 2:2 and 3:3 only (TP and TQ): another A:B whose
// rounding or minimum decimals would show is refused.
#[test]
fn an_unpinned_leading_precision_that_would_show_is_refused() {
    let r = format!(
        "{:?}",
        al_testkit::run_with(
            "codeunit 50100 C { procedure F(): Text var D: Decimal; begin D := 1.23456; exit(Format(D, 0, '<Precision,2:4><Integer><Decimals>')); end; }",
            "C",
            "F",
            vec![]
        )
    );
    assert!(r.contains("Unsupported"), "{r}");
}
