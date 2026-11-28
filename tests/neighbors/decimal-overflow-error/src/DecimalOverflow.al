// al-runner issue 9267: the largest Decimal times the Integer 10 raises BC's overflow error (the pinned cell). The rule
// reads the operator and the two values only, so the target variable, a compound assignment and an Integer variable
// holding 10 run it too. Every other value and every flipped fact stays refused.
codeunit 61920 "DO Decimal Overflow Tests"
{
    Subtype = Test;

    var
        OverflowText: Label 'Value was either too large or too small for a Decimal.', Locked = true;

    local procedure ExpectOverflow()
    begin
        if GetLastErrorText() <> OverflowText then
            Error('got [%1]', GetLastErrorText());
        if GetLastErrorCode() <> 'AL:Overflow' then
            Error('got code [%1]', GetLastErrorCode());
    end;

    [Test]
    procedure Pinned_MaxDecimalTimesIntegerTen()
    var
        D: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        asserterror D := D * 10;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_NearMaxTimesHundredIntoOtherVariable()
    var
        D: Decimal;
        R: Decimal;
    begin
        D := 7922816251426433759354395033.5;
        asserterror R := D * 100;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_MaxTimesIntegerVariableTwo()
    var
        D: Decimal;
        N: Integer;
    begin
        D := 79228162514264337593543950335.0;
        N := 2;
        asserterror D := D * N;
        ExpectOverflow();
    end;

    [Test]
    procedure MaxTimesTenIntoOtherVariable()
    var
        D: Decimal;
        R: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        R := 1;
        asserterror R := D * 10;
        ExpectOverflow();
    end;

    [Test]
    procedure MaxTimesIntegerVariableTen()
    var
        D: Decimal;
        N: Integer;
    begin
        D := 79228162514264337593543950335.0;
        N := 10;
        asserterror D := D * N;
        ExpectOverflow();
    end;

    [Test]
    procedure CompoundStarAssign()
    var
        D: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        asserterror D *= 10;
        ExpectOverflow();
    end;

    // Both operands have at most 18 significant digits, so no literal is rounded: the product is past the largest Decimal,
    // and it is not the pinned value.
    [Test]
    procedure Refused_EighteenDigitDecimalTimesHundred()
    var
        D: Decimal;
    begin
        D := 792281625142643376000000000.0;
        asserterror D := D * 100;
        ExpectOverflow();
    end;

    [Test]
    procedure Unchanged_ProductBelowMax()
    var
        D: Decimal;
    begin
        D := 792281625142643375000000000.0;
        D := D * 10;
        if D <> 7922816251426433750000000000.0 then
            Error('the product changed');
    end;

    [Test]
    procedure Refused_DecimalTimesDecimal()
    var
        D: Decimal;
        Ten: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        Ten := 10;
        asserterror D := D * Ten;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_IntegerTimesDecimal()
    var
        D: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        asserterror D := 10 * D;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_NegativeDecimalTimesInteger()
    var
        D: Decimal;
    begin
        D := -79228162514264337593543950335.0;
        asserterror D := D * 10;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_DecimalTimesNegativeInteger()
    var
        D: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        asserterror D := D * -10;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_PlusPastMax()
    var
        D: Decimal;
        R: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        asserterror R := D + D;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_MinusPastMin()
    var
        D: Decimal;
        R: Decimal;
    begin
        D := -79228162514264337593543950335.0;
        asserterror R := D - 79228162514264337593543950335.0;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_DecimalTimesBigInteger()
    var
        D: Decimal;
        B: BigInteger;
    begin
        D := 79228162514264337593543950335.0;
        B := 10;
        asserterror D := D * B;
        ExpectOverflow();
    end;

    [Test]
    procedure Unchanged_IntegerOverflow()
    var
        I: Integer;
    begin
        I := 2147483647;
        asserterror I := I * 2;
        if GetLastErrorText() <> 'Arithmetic operation resulted in an overflow.' then
            Error('got [%1]', GetLastErrorText());
    end;
}
