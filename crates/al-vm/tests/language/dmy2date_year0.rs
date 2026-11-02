//! DMY2Date with year 0 and with arguments that are not whole Integers (writer Y81, types/lt-y81-values/Y81DMY2Date.al
//! :41-:64 year 0, :183-:215 Decimal and BigInteger arguments; needs corpus bump). Only (1, 1, 0) is a valid year 0 and
//! equals (2, 1, 1); any other year 0 with a day and month in range raises the parameter 3 range error; a Decimal with a
//! fraction and a BigInteger beyond Integer raise ConversionCast.

use al_testkit::{run, text};

const SRC: &str = r#"
codeunit 69971 C
{
    [TryFunction]
    local procedure TryInt(D: Integer; M: Integer; Y: Integer; var R: Date)
    begin
        R := DMY2Date(D, M, Y);
    end;

    [TryFunction]
    local procedure TryDec(D: Decimal; M: Decimal; Y: Decimal; var R: Date)
    begin
        R := DMY2Date(D, M, Y);
    end;

    [TryFunction]
    local procedure TryBig(D: BigInteger; M: BigInteger; Y: BigInteger; var R: Date)
    begin
        R := DMY2Date(D, M, Y);
    end;

    local procedure Show(R: Date): Text
    begin
        exit(Format(R, 0, 9) + '|' + Format(Date2DMY(R, 1)) + '.' + Format(Date2DMY(R, 2)) + '.' + Format(Date2DMY(R, 3)));
    end;

    procedure Int(D: Integer; M: Integer; Y: Integer): Text
    var R: Date;
    begin
        ClearLastError();
        if TryInt(D, M, Y, R) then exit(Show(R));
        exit('E ' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure Dec(D: Decimal; M: Decimal; Y: Decimal): Text
    var R: Date;
    begin
        ClearLastError();
        if TryDec(D, M, Y, R) then exit(Show(R));
        exit('E ' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure Big(D: BigInteger; M: BigInteger; Y: BigInteger): Text
    var R: Date;
    begin
        ClearLastError();
        if TryBig(D, M, Y, R) then exit(Show(R));
        exit('E ' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure Long(): Text
    begin
        exit(Format(DMY2Date(15, 6, 1), 0, 4) + '#' + Format(DMY2Date(15, 6, 1)));
    end;

    procedure Year0(): Text
    begin
        exit(Int(1, 1, 0) + '#' + Int(2, 1, 0) + '#' + Int(31, 12, 0) + '#' + Int(30, 2, 0) + '#' + Int(2, 1, 1));
    end;

    procedure Decimals(): Text
    begin
        exit(Dec(15, 3, 2024) + '#' + Dec(15.4, 3, 2024) + '#' + Dec(15, 3.5, 2024) + '#' + Dec(1, 1, 0.5) + '#' + Dec(-0.4, 3, 2024));
    end;

    procedure BigIntegers(): Text
    begin
        exit(Big(1, 1, 0) + '#' + Big(1, 1, 2147483648L) + '#' + Big(1, 4294967297L, 2024) + '#' + Big(1, 1, -2147483649L));
    end;
}
"#;

const RANGE3: &str = "E The value of DMY2Date parameter 3 is outside of the permitted range. The current value is: 0. The permitted range is: from 1 to 9999.|OutsidePermittedRange";

#[test]
fn year_zero_is_valid_only_for_the_first_of_january() {
    assert_eq!(
        text(run(SRC, "Year0")),
        format!("0001-01-02|1.1.0#{RANGE3}#{RANGE3}#{RANGE3}#0001-01-02|1.1.0")
    );
}

fn conv(v: &str, ty: &str) -> String {
    format!("E Overflow under conversion of {ty} value {v} to System.Int32.|ConversionCast")
}

#[test]
fn a_decimal_with_a_fraction_raises_conversion_cast() {
    let d = "Microsoft.Dynamics.Nav.Runtime.Decimal18";
    assert_eq!(
        text(run(SRC, "Decimals")),
        format!(
            "2024-03-15|15.3.2024#{}#{}#{}#{}",
            conv("15.4", d),
            conv("3.5", d),
            conv("0.5", d),
            conv("-0.4", d)
        )
    );
}

#[test]
fn a_big_integer_beyond_integer_raises_conversion_cast() {
    let i = "System.Int64";
    assert_eq!(
        text(run(SRC, "BigIntegers")),
        format!(
            "0001-01-02|1.1.0#{}#{}#{}",
            conv("2147483648", i),
            conv("4294967297", i),
            conv("-2147483649", i)
        )
    );
}

#[test]
fn the_long_date_of_year_one_has_four_digits() {
    // types/lt-y81-values/Y81DMY2Date.al:94-114 Y81_DMY2Date_Year1_Arithmetic `format 4`, `format 0`.
    assert_eq!(text(run(SRC, "Long")), "June 15, 0001#06/15/0001");
}
