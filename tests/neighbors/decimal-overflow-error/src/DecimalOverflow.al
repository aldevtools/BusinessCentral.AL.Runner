// al-runner issue 9267: a positive Decimal times a positive Integer past the largest Decimal raises BC's overflow error
// (the pinned shape). Each test below flips one fact of that shape; every flipped overflow stays refused.
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
    procedure Pinned_PastMaxIntoOtherVariable()
    var
        D: Decimal;
        R: Decimal;
    begin
        D := 7922816251426433759354395033.5;
        asserterror R := D * 100;
        ExpectOverflow();
    end;

    [Test]
    procedure Pinned_IntegerVariableOperand()
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
    procedure CompoundStarAssign()
    var
        D: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        asserterror D *= 10;
        ExpectOverflow();
    end;

    // A product that may land on the largest Decimal (not measured): the runner refuses it, as main does.
    [Test]
    procedure Refused_ProductLandsOnMax()
    var
        D: Decimal;
    begin
        D := 7922816251426433759354395033.5;
        D := D * 10;
        if Format(D, 0, 9) <> '79228162514264337593543950335' then
            Error('got %1', Format(D, 0, 9));
    end;

    [Test]
    procedure Unchanged_MaxDecimalLiteralFormats()
    var
        D: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        if Format(D, 0, 9) <> '79228162514264337593543950335' then
            Error('got %1', Format(D, 0, 9));
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
