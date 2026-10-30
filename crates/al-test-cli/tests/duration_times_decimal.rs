//! Duration * Decimal and Decimal * Duration multiply the Duration's milliseconds into a Decimal that keeps the fraction; the
//! product assigns to a BigInteger or, when whole, to a Duration (corpus types/lt-ax-text-format-dates/AxDurationDateTime.al,
//! the Duration_Wb_* tests, corpus branch al-runtime-rs-staging-wb at a07787b2, not yet on master; al-runner issue 6285).

use al_testkit::outcomes_in;

const SRC: &str = r#"
codeunit 50100 T
{
    Subtype = Test;
    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then Error('expected %1, got %2', Expected, Actual);
    end;
    [Test] procedure DurationTimesDecimalVariable100()
    var D: Duration; Pct: Decimal; Result: Decimal;
    begin
        D := 28800000; Pct := 100; Result := D * Pct;
        Check('2880000000', Format(Result, 0, 9));
    end;
    [Test] procedure DecimalTimesDurationVariable()
    var D: Duration; Pct: Decimal; Result: Decimal;
    begin
        D := 28800000; Pct := 100; Result := Pct * D;
        Check('2880000000', Format(Result, 0, 9));
    end;
    [Test] procedure DurationTimesFractionVariable()
    var D: Duration; Pct: Decimal; Result: Decimal;
    begin
        D := 28800000; Pct := 0.5; Result := D * Pct;
        Check('14400000', Format(Result, 0, 9));
        Pct := 1.25; Result := D * Pct;
        Check('36000000', Format(Result, 0, 9));
    end;
    [Test] procedure DurationTimesPercentOverHundredIntoDuration()
    var D: Duration; Eff: Decimal; Taken: Duration; Back: Decimal;
    begin
        D := 28800000; Eff := 100; Taken := D * Eff / 100; Back := Taken;
        Check('28800000', Format(Back, 0, 9));
    end;
    [Test] procedure DurationTimesDecimalIntoBigInteger()
    var D: Duration; Pct: Decimal; Big: BigInteger;
    begin
        D := 28800000; Pct := 100; Big := D * Pct;
        Check('2880000000', Format(Big, 0, 9));
    end;
    [Test] procedure DurationTimesDecimalIntoDuration()
    var D: Duration; Pct: Decimal; R: Duration; Back: Decimal;
    begin
        D := 28800000; Pct := 0.5; R := D * Pct; Back := R;
        Check('14400000', Format(Back, 0, 9));
    end;
    [Test] procedure DurationTimesFractionPrintsValue()
    var D: Duration; Pct: Decimal;
    begin
        D := 7; Pct := 1.5;
        Check('10.5|10.5', Format(D * Pct, 0, 9) + '|' + Format(Pct * D, 0, 9));
    end;
}
"#;

#[test]
fn duration_times_decimal_is_a_decimal_product() {
    let o = outcomes_in(SRC, "T");
    assert_eq!(o.len(), 7);
    for (name, outcome) in &o {
        assert_eq!(format!("{outcome:?}"), "Pass", "{name}");
    }
}
