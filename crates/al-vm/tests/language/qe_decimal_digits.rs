//! Corpus bump 35, writer QE: every Decimal result keeps 18 significant digits, rounded half away from zero, for any
//! sign and for BigInteger operands (types/decimal-integer/DecimalDigits.al, codeunit 73232 QE_Decimal_*).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 69931 QD
{
    local procedure D(Value: Text): Decimal
    var
        Result: Decimal;
    begin
        Evaluate(Result, Value, 9);
        exit(Result);
    end;

    local procedure F(Value: Decimal): Text
    begin
        exit(Format(Value, 0, 9));
    end;

    procedure Sums(): Text
    var
        P62: Decimal;
        N62: Decimal;
    begin
        P62 := Power(2, 62);
        N62 := -P62;
        exit(F((N62 - 11) - N62) + '|' + F((N62 - 6) - N62) + '|' + Format(N62 - 1 = N62) + '|'
            + F(D('-0.5') - D('0.999999999999999985')) + '|' + F(D('-0.999999999999999999') + D('0.5')) + '|'
            + F(D('-1234.56') - (D('1') / D('3'))) + '|' + F(D('-0.123456789012345678') - D('0.000000000000000001')));
    end;

    procedure BigIntegers(): Text
    var
        P62: Decimal;
        B1: BigInteger;
        B62p1: BigInteger;
        BI: BigInteger;
    begin
        P62 := Power(2, 62);
        B1 := 1;
        B62p1 := 4611686018427387905L;
        BI := -B62p1 - 0.5;
        exit(F((-P62 - 11 * B1) + P62) + '|' + F(B62p1 - P62) + '|' + F(-B62p1 + P62) + '|' + Format(-B62p1 < -P62) + '|'
            + Format(-B62p1 = -P62) + '|' + F((-B62p1 - 0.5) + P62) + '|' + Format(BI) + '|'
            + F(D('-0.123456789012345678') - 1000 * B1) + '|' + F((D('1') / D('3')) * (B1 * -1234567)));
    end;

    procedure AssignBigInteger(): Text
    var
        P62: Decimal;
        Dec: Decimal;
        B62p1: BigInteger;
    begin
        P62 := Power(2, 62);
        B62p1 := 4611686018427387905L;
        Dec := B62p1;
        exit(F(Dec - P62));
    end;

    procedure Products(): Text
    var
        T: Decimal;
        Factor: Decimal;
    begin
        T := D('1') / D('3');
        Factor := D('100') / D('64.8824');
        exit(F(D('1234.56') * T) + '|' + Format(D('1234.56') * T = 411.52) + '|' + F(D('-6.015') * T) + '|'
            + F(Round(D('6.015') * T, 0.01)) + '|' + F(Round(D('-6.015') * T, 0.01)) + '|' + F(D('-1234.56') * Factor) + '|'
            + F(D('0.333333333333333329') * D('0.5')) + '|' + F(D('-0.333333333333333329') * D('0.5')) + '|'
            + F(-(D('2') / D('3')) * (D('2') / D('3'))) + '|' + F(D('-3') * T));
    end;

    procedure Small(): Text
    var
        Q: Decimal;
    begin
        Q := D('-0.000000000000000001') / D('3');
        exit(F(Q) + '|' + Format(Q = 0) + '|' + Format(Q < 0) + '|' + Format(Q) + '|' + F(Q + 1) + '|' + F(Q * 1000) + '|'
            + F(D('-0.000000000000000001') * D('0.4')) + '|' + F(D('-0.000000000000000001') * D('0.5')) + '|'
            + Format(D('0.0000000001') * D('0.000000001') = 0));
    end;
}
"#;

// DecimalDigits.al:200-235 (QE_Decimal_SumDifference_SignsAndTies): negative sums and differences round at 18 digits too.
#[test]
fn negative_sums_and_differences_keep_18_digits() {
    assert_eq!(
        text(run_in(SRC, "QD", "Sums")),
        "-10|-10|Yes|-1.49999999999999999|-0.499999999999999999|-1234.89333333333333|-0.123456789012345679"
    );
}

// DecimalDigits.al:267-337 (QE_Decimal_BigIntegerOperands): a BigInteger operand becomes a Decimal of 18 digits first
// (4611686018427387905 -> ...910), and the result rounds at 18 digits.
#[test]
fn big_integer_operands_round_to_18_digits() {
    assert_eq!(
        text(run_in(SRC, "QD", "BigIntegers")),
        "-10|10|-10|Yes|No|-10|-4611686018427387910|-1000.12345678901235|-411522.333333333333"
    );
}

// DecimalDigits.al:287-288, :327 (`dec := b62p1, back`): a BigInteger assigned to a Decimal keeps 18 digits.
#[test]
fn a_big_integer_assigned_to_a_decimal_keeps_18_digits() {
    assert_eq!(text(run_in(SRC, "QD", "AssignBigInteger")), "10");
}

// DecimalDigits.al:349-440 (QE_Decimal_Products_SignsAndRound): every product rounds at 18 digits, half away from zero.
#[test]
fn products_keep_18_digits() {
    assert_eq!(
        text(run_in(SRC, "QD", "Products")),
        "411.52|Yes|-2.005|2.01|-2.01|-1902.76561902765619|0.166666666666666665|-0.166666666666666665|-0.444444444444444445|-0.999999999999999999"
    );
}

// DecimalDigits.al:480-541 (QE_Decimal_SmallResults_DecimalPlaces): below 1E-18 a value keeps its digits; Format shows at
// most 18 decimals and '-0' for a negative value that rounds to zero, in the XML and in the default format.
#[test]
fn small_results_keep_their_digits_and_format_as_minus_zero() {
    assert_eq!(
        text(run_in(SRC, "QD", "Small")),
        "-0|No|Yes|-0|1|-0.000000000000000333|-0|-0.000000000000000001|No"
    );
}
