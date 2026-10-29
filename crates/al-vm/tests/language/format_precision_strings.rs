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

/// TV_Format_Precision0To26_Values, _OtherValues and TV_Format_PrecisionAbove18_Forms (FormatCommaFormatString.al on corpus
/// master): `0:26` before standard formats 0, 2 and 9 writes the value as the format does, alone it writes nothing; `0:19`,
/// `0:28` and `26:26` before format 2; a precision does not change an Integer.
#[test]
fn precision_above_18_decimals_of_the_pinned_forms() {
    let wide = |value: &str| {
        let mut body = format!("procedure F(): Text var D: Decimal; R: Text; begin D := {value};");
        for f in [
            "<Precision,0:26><Standard Format,2>",
            "<Precision,0:26><Standard Format,0>",
            "<Precision,0:26><Standard Format,9>",
            "<Precision,0:26>",
            "<Precision,0:19><Standard Format,2>",
            "<Precision,0:28><Standard Format,2>",
        ] {
            body.push_str(&format!(" R += Format(D, 0, '{f}') + '|';"));
        }
        body.push_str(" exit(R); end;");
        run(&body)
    };
    assert_eq!(wide("1.5"), "1.5|1.5|1.5||1.5|1.5|");
    assert_eq!(wide("-2.25"), "-2.25|-2.25|-2.25||-2.25|-2.25|");
    assert_eq!(wide("0"), "0|0|0||0|0|");
    assert_eq!(
        wide("12345.6789"),
        "12345.6789|12,345.6789|12345.6789||12345.6789|12345.6789|"
    );
    assert_eq!(
        wide("1 / 3"),
        "0.333333333333333333|0.333333333333333333|0.333333333333333333||0.333333333333333333|0.333333333333333333|"
    );
    assert_eq!(wide("Round(2.346, 0.01)"), "2.35|2.35|2.35||2.35|2.35|");
    assert_eq!(
        run("procedure F(): Text begin exit(Format(1.5, 0, '<Precision,26:26><Standard Format,2>')); end;"),
        "1.50000000000000000000000000"
    );
    assert_eq!(
        run("procedure F(): Text begin exit(Format(15, 0, '<Precision,0:26><Standard Format,2>')); end;"),
        "15"
    );
}

/// The same precisions before another standard format, or first before `<Integer>`, are not pinned.
#[test]
fn precision_above_18_decimals_of_other_forms_is_refused() {
    for f in [
        "<Precision,0:26><Standard Format,1>",
        "<Precision,0:19><Standard Format,0>",
        "<Precision,0:26><Integer><Decimals>",
    ] {
        let src = format!(
            "codeunit 50100 C {{ procedure F(): Text var D: Decimal; begin D := 1.25; exit(Format(D, 0, '{f}')); end; }}"
        );
        let out = al_testkit::run_with(&src, "C", "F", vec![]);
        assert!(
            matches!(&out, Err(al_builtins::native::Signal::Unsupported(w)) if w.contains("not pinned")),
            "{f}: {out:?}"
        );
    }
}
