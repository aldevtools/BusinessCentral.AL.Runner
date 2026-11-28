// al-runner issue 9267: a Decimal at the limit of the type. MAX18 is 79228162514264337500000000000, the largest Decimal
// of 18 significant digits, the value BC holds for the literal 79228162514264337593543950335.0.
// Served: the overflow error of the measured shapes and the round-down of a literal and a sum at the limit.
// Kept: main's refusal of each result past the range that no cell measures, and main's value below the limit.
codeunit 61930 "DOV Decimal Limit Tests"
{
    Subtype = Test;

    var
        OverflowText: Label 'Value was either too large or too small for a Decimal.', Locked = true;

    local procedure Max18(): Decimal
    begin
        exit(79228162514264337500000000000.0);
    end;

    local procedure ExpectOverflow()
    begin
        if GetLastErrorText() <> OverflowText then
            Error('got [%1]', GetLastErrorText());
        if GetLastErrorCode() <> 'AL:Overflow' then
            Error('got code [%1]', GetLastErrorCode());
    end;

    local procedure ExpectMax18(D: Decimal)
    begin
        if D <> Max18() then
            Error('got %1', Format(D / 1000000000000000.0, 0, 9));
    end;

    [Test]
    procedure Overflow_SumOfTwoMax18()
    var
        D: Decimal;
        R: Decimal;
    begin
        D := Max18();
        asserterror R := D + D;
        ExpectOverflow();
    end;

    [Test]
    procedure Overflow_DifferenceOfNegativeMax18AndMax18()
    var
        D: Decimal;
        E: Decimal;
        R: Decimal;
    begin
        D := -Max18();
        E := Max18();
        asserterror R := D - E;
        ExpectOverflow();
    end;

    [Test]
    procedure Overflow_PlusAssignKeepsTarget()
    var
        D: Decimal;
    begin
        D := Max18();
        asserterror D += D;
        ExpectOverflow();
        ExpectMax18(D);
    end;

    [Test]
    procedure Overflow_AssignmentKeepsOtherTarget()
    var
        D: Decimal;
        R: Decimal;
    begin
        D := Max18();
        R := 1;
        asserterror R := D * 2;
        ExpectOverflow();
        if R <> 1 then
            Error('R changed');
    end;

    [Test]
    procedure Overflow_IntegerTimesMax18()
    var
        D: Decimal;
    begin
        D := Max18();
        asserterror D := 10 * D;
        ExpectOverflow();
    end;

    [Test]
    procedure Overflow_Max18TimesBigInteger()
    var
        D: Decimal;
        B: BigInteger;
    begin
        D := Max18();
        B := 10;
        asserterror D := D * B;
        ExpectOverflow();
    end;

    [Test]
    procedure Overflow_Max18DividedByHalf()
    var
        D: Decimal;
    begin
        D := Max18();
        asserterror D := D / 0.5;
        ExpectOverflow();
    end;

    // No cell measures a negative quotient: main's refusal.
    [Test]
    procedure Refused_NegativeQuotient()
    var
        D: Decimal;
    begin
        D := -Max18();
        asserterror D := D / 0.5;
        ExpectOverflow();
    end;

    // The pinned cell (errors/PnBErrorsCollections.al:564-566): its literal holds MAX18 (json/JsonValueSetters.al:193).
    [Test]
    procedure Overflow_LargestLiteralTimesTen()
    var
        D: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        asserterror D := D * 10;
        ExpectOverflow();
    end;

    [Test]
    procedure RoundDown_LiteralAtLimit()
    var
        D: Decimal;
    begin
        D := 79228162514264337560000000000.0;
        ExpectMax18(D);
    end;

    [Test]
    procedure RoundDown_LargestLiteral()
    var
        D: Decimal;
    begin
        D := 79228162514264337593543950335.0;
        ExpectMax18(D);
    end;

    [Test]
    procedure RoundDown_SumUpToTheDotNetMaximum()
    var
        D: Decimal;
    begin
        D := Max18();
        D := D + 93543950335.0;
        ExpectMax18(D);
    end;

    [Test]
    procedure RoundDown_NegativeDifference()
    var
        D: Decimal;
    begin
        D := -Max18();
        D := D - 50000000000.0;
        if D <> -Max18() then
            Error('changed');
    end;

    // Past the .NET maximum, below twice MAX18: no cell measures it.
    [Test]
    procedure Refused_SumJustPastTheDotNetMaximum()
    var
        D: Decimal;
    begin
        D := Max18();
        asserterror D := D + 93543950336.0;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_QuotientJustPastTheLimit()
    var
        D: Decimal;
    begin
        D := Max18();
        asserterror D := D / 0.999999999999999998;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_ProductJustPastTheDotNetMaximum()
    var
        D: Decimal;
    begin
        D := 138753349412021607000000000.0;
        asserterror D := D * 571;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_BigIntegerTimesMax18()
    var
        D: Decimal;
        B: BigInteger;
    begin
        D := Max18();
        B := 10;
        asserterror D := B * D;
        ExpectOverflow();
    end;

    // No cell measures these sign pairs: main's refusal.
    [Test]
    procedure Refused_SumOfTwoNegativeMax18()
    var
        D: Decimal;
    begin
        D := -Max18();
        asserterror D := D + D;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_PositiveDifference()
    var
        D: Decimal;
        E: Decimal;
    begin
        D := Max18();
        E := -Max18();
        asserterror D := D - E;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_NegativeMax18TimesDecimal()
    var
        D: Decimal;
        E: Decimal;
    begin
        D := -Max18();
        E := 10;
        asserterror D := D * E;
        ExpectOverflow();
    end;

    [Test]
    procedure Refused_NegativeIntegerTimesMax18()
    var
        D: Decimal;
    begin
        D := Max18();
        asserterror D := -10 * D;
        ExpectOverflow();
    end;

    // Measured for every sign pair: a negative Decimal times a negative Integer (`mul_negmax_neg10`).
    [Test]
    procedure Overflow_NegativeMax18TimesNegativeInteger()
    var
        D: Decimal;
    begin
        D := -Max18();
        asserterror D := D * -10;
        ExpectOverflow();
    end;

    // A 29-digit literal whose 18-digit round-up fits: the ordinary rounding.
    [Test]
    procedure Main_RoundUpThatFits()
    var
        D: Decimal;
    begin
        D := 79228162514264337450000000000.0;
        ExpectMax18(D);
    end;

    [Test]
    procedure Main_SumBelowTheLimit()
    var
        D: Decimal;
    begin
        D := Max18();
        D := D + 49999999999.0;
        ExpectMax18(D);
    end;

    [Test]
    procedure Main_ProductBelowTheLimit()
    var
        D: Decimal;
    begin
        D := 7922816251426433750000000000.0;
        D := D * 10;
        ExpectMax18(D);
    end;

    [Test]
    procedure FieldRange_AssignMax18Raises()
    var
        Row: Record "DOV Row";
    begin
        asserterror Row.Amount := Max18();
        if GetLastErrorCode() <> 'OutsidePermittedRange' then
            Error('got [%1]', GetLastErrorText());
    end;

    [Test]
    procedure FieldRange_AssignOnePastRaises()
    var
        Row: Record "DOV Row";
    begin
        asserterror Row.Amount := 1000000000000000.0;
        if GetLastErrorCode() <> 'OutsidePermittedRange' then
            Error('got [%1]', GetLastErrorText());
    end;

    [Test]
    procedure Main_FieldTakesTheLargestValueOfItsRange()
    var
        Row: Record "DOV Row";
    begin
        Row.Amount := 999999999999999.99;
        if Row.Amount <> 999999999999999.99 then
            Error('changed');
    end;

    // No cell measures a temporary record: main's store.
    [Test]
    procedure Main_TemporaryFieldTakesMax18()
    var
        Row: Record "DOV Row" temporary;
    begin
        Row.Amount := Max18();
        ExpectMax18(Row.Amount);
    end;

    // No cell measures Validate: main's store.
    [Test]
    procedure Main_ValidateTakesMax18()
    var
        Row: Record "DOV Row";
    begin
        Row.Validate(Amount, Max18());
        ExpectMax18(Row.Amount);
    end;
}

table 61931 "DOV Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}
