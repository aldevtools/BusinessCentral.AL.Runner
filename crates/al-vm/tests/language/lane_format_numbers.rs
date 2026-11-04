//! Lane T23: a Byte from 0x80 up assigned to a character of a text, and the 18 significant digits of large `Power`
//! results.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const BYTE_CHAR: &str = r#"
codeunit 69911 BC
{
    procedure Codes(): Text
    var
        Values: List of [Integer];
        Value: Integer;
        B: Byte;
        One: Text[1];
        Code: Integer;
        Result: Text;
    begin
        Values.Add(127);
        Values.Add(128);
        Values.Add(159);
        Values.Add(160);
        Values.Add(239);
        Values.Add(187);
        Values.Add(191);
        Values.Add(255);
        foreach Value in Values do begin
            B := Value;
            One[1] := B;
            Code := One[1];
            Result += Format(Code) + ',';
        end;
        exit(Result);
    end;

    procedure Unpinned(): Text
    var
        B: Byte;
        One: Text[1];
        Code: Integer;
    begin
        B := 176;
        One[1] := B;
        Code := One[1];
        exit(Format(Code));
    end;
}
"#;

// al-language-onprem/types/ByteFormat.al:13-31 and :44-79: a Byte assigned to a character of a Text[1] reads back as the
// same code Format of the byte shows (128 is 199, 239 is 217).
#[test]
fn a_byte_from_0x80_assigned_to_a_text_character_maps_as_format_does() {
    assert_eq!(
        text(run_in(BYTE_CHAR, "BC", "Codes")),
        "127,199,402,225,217,8249,143,160,"
    );
}

// Byte 0xB0 (code page 437 `░`) is not pinned for a text character: refused, not run.
#[test]
fn a_byte_whose_character_is_not_pinned_is_refused() {
    let r = run_in(BYTE_CHAR, "BC", "Unpinned");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

const POWER: &str = r#"
codeunit 69912 PW
{
    procedure Item40(): Text
    var
        V1: Decimal;
        V11: Decimal;
        V22: Decimal;
    begin
        V1 := Power(2, 63);
        V11 := V1 - 1.0;
        V22 := V1 - 5.0;
        exit(Format(V11 = V22) + '|' + Format(V11 <> V22) + '|' + Format(V11 - V22) + '|'
            + Format(Power(2, 53) + 1 - Power(2, 53)) + '|'
            + Format(Power(2, 60) - 1152921504606846976.0) + '|'
            + Format(V1 - 9223372036854775808.0) + '|'
            + Format(Power(3, 40) - 12157665459056928801.0) + '|'
            + Format(Power(2, 62) = Power(2, 62) + 1));
    end;
}
"#;

// types/CqOddLeads.al:419-446 (Item40_PowerPrecision).
#[test]
fn power_results_keep_18_significant_digits() {
    assert_eq!(text(run_in(POWER, "PW", "Item40")), "Yes|No|0|1|0|0|0|Yes");
}

const POWER_BIG: &str = r#"
codeunit 69913 PB
{
    procedure Round(): Text
    var
        V1: Decimal;
        V11: Decimal;
        V22: Decimal;
        B: BigInteger;
        R: Text;
    begin
        V1 := Power(2, 63);
        V11 := V1 - 1.0;
        V22 := V1 - 5.0;
        R := Format(V11 = V22) + '|' + Format(V1 - V11, 0, 9) + '|' + Format(V1 - V22, 0, 9) + '|';
        R += Format((Power(2, 53) + 1) - Power(2, 53), 0, 9) + '|';
        R += Format(Power(2, 60) = 1152921504606846976L) + '|';
        R += Format(Power(2, 60) - (Power(2, 60) - 1), 0, 9) + '|';
        R += Format(Power(3, 40) - (Power(3, 40) - 1), 0, 9) + '|';
        B := Power(2, 62);
        R += Format(B) + '|';
        B := Power(2, 62) - 1;
        R += Format(B) + '|';
        B := Power(2, 53) + 1;
        R += Format(B);
        exit(R);
    end;
}
"#;

// types/decimal-integer/Round.al:370-409 (LT_BU_Power_LargeResultPrecision).
#[test]
fn power_results_keep_18_significant_digits_into_big_integer() {
    assert_eq!(
        text(run_in(POWER_BIG, "PB", "Round")),
        "Yes|0|0|1|Yes|0|0|4611686018427387900|4611686018427387900|9007199254740993"
    );
}

const SIGNED_DIGITS: &str = r#"
codeunit 69914 UD
{
    procedure NegativeSum(): Text
    var
        V: Decimal;
    begin
        V := -Power(2, 63);
        V := V + 5;
        exit(Format(V = -Power(2, 63)));
    end;

    procedure NegativeBigCompare(): Text
    begin
        exit(Format(-Power(2, 60) = -1152921504606846976L));
    end;

    procedure ShortSum(): Text
    begin
        exit(Format(-1.5 + 0.25) + '|' + Format(2.5 * 4));
    end;
}
"#;

// Negative sums and comparisons of more than 18 significant digits round as the non-negative ones do
// (types/decimal-integer/DecimalDigits.al:179-346, QE_Decimal_SumDifference_SignsAndTies, QE_Decimal_BigIntegerOperands).
#[test]
fn negative_results_of_more_than_18_digits_keep_18() {
    assert_eq!(text(run_in(SIGNED_DIGITS, "UD", "NegativeSum")), "Yes");
    assert_eq!(text(run_in(SIGNED_DIGITS, "UD", "NegativeBigCompare")), "Yes");
    assert_eq!(text(run_in(SIGNED_DIGITS, "UD", "ShortSum")), "-1.25|10");
}
