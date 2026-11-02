//! A Variant holding a defined Date, Time, DateTime or a number or text assigned to a number, Duration, temporal or Boolean
//! variable. Every literal is what real BC answered in the corpus: types/lt-y67-variant-numeric/Y67VariantNumeric.al (codeunit
//! 93629, writer Y67).

use al_testkit::text;

const SRC: &str = r#"
codeunit 50202 C
{
    [TryFunction]
    local procedure ToInteger(V: Variant; var Txt: Text)
    var
        T: Integer;
    begin
        T := V;
        Txt := Format(T, 0, 9);
    end;

    [TryFunction]
    local procedure ToBigInteger(V: Variant; var Txt: Text)
    var
        T: BigInteger;
    begin
        T := V;
        Txt := Format(T, 0, 9);
    end;

    [TryFunction]
    local procedure ToDuration(V: Variant; var Txt: Text)
    var
        T: Duration;
    begin
        T := V;
        Txt := Format(T, 0, 9);
    end;

    [TryFunction]
    local procedure ToDate(V: Variant; var Txt: Text)
    var
        T: Date;
    begin
        T := V;
        Txt := Format(T, 0, 9);
    end;

    [TryFunction]
    local procedure ToDateTime(V: Variant; var Txt: Text)
    var
        T: DateTime;
    begin
        T := V;
        Txt := Format(T, 0, 9);
    end;

    [TryFunction]
    local procedure ToBoolean(V: Variant; var Txt: Text)
    var
        T: Boolean;
    begin
        T := V;
        Txt := Format(T, 0, 9);
    end;

    local procedure All(V: Variant): Text
    var
        Txt: Text;
        Res: Text;
    begin
        if ToInteger(V, Txt) then Res += Txt else Res += GetLastErrorCode();
        Res += ';';
        if ToBigInteger(V, Txt) then Res += Txt else Res += GetLastErrorCode();
        Res += ';';
        if ToDuration(V, Txt) then Res += Txt else Res += GetLastErrorCode();
        Res += ';';
        if ToDate(V, Txt) then Res += Txt else Res += GetLastErrorCode();
        Res += ';';
        if ToDateTime(V, Txt) then Res += Txt else Res += GetLastErrorCode();
        Res += ';';
        if ToBoolean(V, Txt) then Res += Txt else Res += GetLastErrorCode();
        exit(Res + ' ');
    end;

    procedure Values(): Text
    var
        Big: BigInteger;
        Dec: Decimal;
    begin
        Evaluate(Big, '5000000000');
        Dec := 2.5;
        exit(All(20260115D) + All(ClosingDate(20260115D)) + All(120000T) + All(CreateDateTime(20000101D, 000000T)) + All(Big) + All(Dec) + All('12.5') + All('20260115'));
    end;
}
"#;

/// Variant_DefinedDates_C `20260115D`, _D `closing 20260115D`, Variant_Times_A `120000T`, Variant_DateTimes_A `dt 20000101 000000`,
/// Variant_Numbers_A `biginteger` and `decimal 2.5`, Variant_TextNumbers_A `text 12.5` and `text 20260115`.
#[test]
fn a_variant_into_numbers_temporal_values_and_boolean() {
    assert_eq!(
        text(al_testkit::run(SRC, "Values")),
        "1479993;1479993;P0DT0H24M39.993S;2026-01-15;2026-01-15T00:00:00Z;true \
         1479994;1479994;P0DT0H24M39.994S;2026-01-15;2026-01-15T23:59:59Z;true \
         43200001;43200001;P0DT12H0M0.001S;TypeConversion;TypeConversion;true \
         Conversion;63113904000001;P730485DT0H0M0.001S;2000-01-01;2000-01-01T00:00:00Z;Conversion \
         TypeConversion:Overflow;5000000000;P57DT20H53M20.0S;TypeConversion;TypeConversion;TypeConversion:Overflow \
         ConversionCast;ConversionCast;ConversionCast;TypeConversion;TypeConversion;ConversionCast \
         TypeConversion:Format;TypeConversion:Format;AL:Format;TypeConversion;TypeConversion;TypeConversion:Format \
         20260115;20260115;P0DT5H37M40.115S;TypeConversion;TypeConversion;true "
    );
}

const CHAR_BYTE: &str = r#"
codeunit 50204 D
{
    [TryFunction]
    local procedure ToChar(V: Variant)
    var
        T: Char;
    begin
        T := V;
    end;

    [TryFunction]
    local procedure ToByte(V: Variant)
    var
        T: Byte;
    begin
        T := V;
    end;

    procedure Errors(): Text
    var
        Big: BigInteger;
        Txt: Text;
    begin
        Big := 3000000000L;
        if not ToChar(Big) then Txt += GetLastErrorText() + '|' + GetLastErrorCode() + ' ';
        if not ToByte(300) then Txt += GetLastErrorText() + '|' + GetLastErrorCode() + ' ';
        if not ToChar(1.5) then Txt += GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Txt);
    end;
}
"#;

/// types/lt-y31-conversion-matrix/Y31ConvCv.al:9942 Variant_To_Char `assign.big` and `assign.dec1p5`, :10138 Variant_To_Byte
/// `assign.int300`.
#[test]
fn a_variant_number_beyond_a_char_or_a_byte() {
    assert_eq!(
        text(al_testkit::run(CHAR_BYTE, "Errors")),
        "The expression Variant cannot be type-converted to a Char value.|TypeConversion:Overflow \
         The expression Variant cannot be type-converted to a Byte value.|TypeConversion:Overflow \
         Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 1.5 to System.Int32.|ConversionCast"
    );
}
