//! `<Precision,A:B>` first, without a standard format, for 2:5, 0:5, 1:5 and 4:6 (format-evaluate/FormatCommaFormatString.al:717-1105,
//! Vq_Precision25_IntegerDecimals and the other Vq_* tests). Unpinned pairs stay refused.

use al_testkit::text;

fn fmt(value: &str, f: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var D: Decimal; begin D := {value}; exit(Format(D, 0, '{f}')); end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn precision_25_integer_decimals() {
    let f = "<Precision,2:5><Integer><Decimals>";
    for (v, want) in [
        ("12", "12.00"),
        ("12.5", "12.50"),
        ("1.23456", "1.23456"),
        ("1.234565", "1.23457"),
        ("99999.999995", "100000.00"),
        ("-3.5", "3.50"),
        ("-0.000001", "0.00"),
        ("0", "0.00"),
    ] {
        assert_eq!(fmt(v, f), want, "{v}");
    }
    assert_eq!(
        fmt("99999.999995", "<Precision,2:5><Integer Thousand><Decimals>"),
        "100,000.00"
    );
}

#[test]
fn precision_05_15_46() {
    assert_eq!(fmt("12", "<Precision,0:5><Integer><Decimals>"), "12");
    assert_eq!(fmt("-0.000001", "<Precision,0:5><Integer><Decimals>"), "0");
    assert_eq!(fmt("12", "<Precision,1:5><Integer><Decimals>"), "12.0");
    assert_eq!(
        fmt("99999.999995", "<Precision,1:5><Integer Thousand><Decimals>"),
        "100,000.0"
    );
    assert_eq!(fmt("1.234565", "<Precision,4:6><Integer><Decimals>"), "1.234565");
    assert_eq!(fmt("-0.000001", "<Precision,4:6><Integer><Decimals>"), "0.000001");
    assert_eq!(fmt("0", "<Precision,4:6><Integer Thousand><Decimals>"), "0.0000");
}

#[test]
fn precision_25_sign_positions() {
    assert_eq!(fmt("-3.5", "<Sign><Precision,2:5><Integer><Decimals>"), "-3.5");
    assert_eq!(fmt("-3.5", "<Precision,2:5><Sign><Integer><Decimals>"), "-3.50");
    assert_eq!(fmt("-0.000001", "<Precision,2:5><Sign><Integer><Decimals>"), "-0.00");
    assert_eq!(fmt("-3.5", "<Precision,2:5><Integer><Decimals><Sign>"), "3.50-");
    assert_eq!(
        fmt("-0.000001", "<Precision,2:5><Integer Thousand><Decimals><Sign>"),
        "0.00-"
    );
    assert_eq!(fmt("0", "<Precision,2:5><Integer><Decimals><Sign>"), "0.00");
}

fn refused(value: &str, f: &str) -> bool {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var D: Decimal; begin D := {value}; exit(Format(D, 0, '{f}')); end; }}"
    );
    al_testkit::run_with(&src, "C", "F", vec![]).is_err()
}

#[test]
fn unpinned_pairs_stay_refused() {
    assert!(refused("1.2", "<Precision,2:7><Integer><Decimals>"));
    assert!(refused("-3.5", "<Precision,4:6><Sign><Integer><Decimals>"));
}
