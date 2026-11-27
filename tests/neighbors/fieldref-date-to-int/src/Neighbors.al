// Neighbors of al-runner issue 9265: FieldRef.Value() of a Date, Time or DateTime field converted straight into a number.
// BC 28.4 raises a conversion error for a Date into an Integer local where al-runner gave 0; no corpus case pins it, so
// head refuses a Date, Time or DateTime value converted into a number (a local, a global, a field, an argument, an exit
// value). A Variant in between keeps the pinned conversion; other field types and targets are unchanged.
table 50100 "NB FDI Row"
{
    fields
    {
        field(1; K; Integer) { }
        field(2; D; Date) { }
        field(3; T; Time) { }
        field(4; Dt; DateTime) { }
        field(5; N; Integer) { }
        field(6; Target; Integer) { }
        field(7; Dec; Decimal) { }
        field(8; Txt; Text[20]) { }
        field(9; Bool; Boolean) { }
        field(10; Opt; Option) { OptionMembers = Alpha,Beta,Gamma; OptionCaption = 'First,Second,Third'; }
        field(11; Big; BigInteger) { }
        field(12; Dur; Duration) { }
        field(13; Cod; Code[20]) { }
    }
    keys { key(PK; K) { Clustered = true; } }
}

codeunit 50101 "NB FDI Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        GlobalInt: Integer;

    local procedure Fr(FieldNo: Integer; var RecRef: RecordRef; var FR: FieldRef)
    begin
        RecRef.Open(Database::"NB FDI Row");
        FR := RecRef.Field(FieldNo);
    end;

    local procedure TakeInt(I: Integer): Integer
    begin
        exit(I);
    end;

    local procedure ReturnAsInteger(var FR: FieldRef): Integer
    begin
        exit(FR.Value());
    end;

    local procedure ReturnAsDecimal(var FR: FieldRef): Decimal
    begin
        exit(FR.Value());
    end;

    local procedure ReturnAsBoolean(var FR: FieldRef): Boolean
    begin
        exit(FR.Value());
    end;

    local procedure ReturnAsOption(var FR: FieldRef): Option Alpha,Beta,Gamma
    begin
        exit(FR.Value());
    end;

    local procedure ReturnAsBigInteger(var FR: FieldRef): BigInteger
    begin
        exit(FR.Value());
    end;

    local procedure ReturnAsDuration(var FR: FieldRef): Duration
    begin
        exit(FR.Value());
    end;

    [Test]
    procedure DateIntoIntegerLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Integer;
    begin
        Fr(2, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateIntoBigIntegerLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: BigInteger;
    begin
        Fr(2, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateIntoDecimalLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Decimal;
    begin
        Fr(2, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateIntoDurationLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Duration;
    begin
        Fr(2, RecRef, FR);
        X := FR.Value();
    end;

    // Option, Boolean, Byte and Char: main already refused an undefined Date, Time or DateTime into them (another text).
    [Test]
    procedure DateIntoOptionLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Option A,B;
    begin
        Fr(2, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateIntoBooleanLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Boolean;
    begin
        Fr(2, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateIntoByteLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Byte;
    begin
        Fr(2, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateIntoCharLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Char;
    begin
        Fr(2, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure TimeIntoIntegerLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Integer;
    begin
        Fr(3, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure TimeIntoBigIntegerLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: BigInteger;
    begin
        Fr(3, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure TimeIntoDecimalLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Decimal;
    begin
        Fr(3, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure TimeIntoDurationLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Duration;
    begin
        Fr(3, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure TimeIntoOptionLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Option A,B;
    begin
        Fr(3, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure TimeIntoBooleanLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Boolean;
    begin
        Fr(3, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure TimeIntoByteLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Byte;
    begin
        Fr(3, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure TimeIntoCharLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Char;
    begin
        Fr(3, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateTimeIntoIntegerLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Integer;
    begin
        Fr(4, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateTimeIntoBigIntegerLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: BigInteger;
    begin
        Fr(4, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateTimeIntoDecimalLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Decimal;
    begin
        Fr(4, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateTimeIntoDurationLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Duration;
    begin
        Fr(4, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateTimeIntoOptionLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Option A,B;
    begin
        Fr(4, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateTimeIntoBooleanLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Boolean;
    begin
        Fr(4, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateTimeIntoByteLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Byte;
    begin
        Fr(4, RecRef, FR);
        X := FR.Value();
    end;

    [Test]
    procedure DateTimeIntoCharLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        X: Char;
    begin
        Fr(4, RecRef, FR);
        X := FR.Value();
    end;

    // The issue's shape with BC 28.4's text: fails on main (no error, 0), refused on head.
    [Test]
    procedure DateIntoIntegerLocal_BcText()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        Fr(2, RecRef, FR);
        asserterror I := FR.Value();
        if GetLastErrorText() <> 'Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDate to System.Int32.' then
            Error('got [%1]', GetLastErrorText());
    end;

    [Test]
    procedure SetDateIntoIntegerLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        Fr(2, RecRef, FR);
        FR.Value := 20260115D;
        I := FR.Value();
    end;

    [Test]
    procedure DateIntoIntegerArgument()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(2, RecRef, FR);
        TakeInt(FR.Value());
    end;

    // `exit(FR.Value())` from an Integer procedure: refused as the assignment is.
    [Test]
    procedure DateIntoIntegerExitValue()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(2, RecRef, FR);
        ReturnAsInteger(FR);
    end;

    [Test]
    procedure DateIntoIntegerGlobal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(2, RecRef, FR);
        GlobalInt := FR.Value();
    end;

    [Test]
    procedure DateIntoIntegerRecordField()
    var
        Row: Record "NB FDI Row";
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(2, RecRef, FR);
        Row.Target := FR.Value();
    end;

    // Same on both: a Variant in between converts as types/date-time/Y62UndefinedDate.al:292,295 pins.
    [Test]
    procedure DateThroughVariantIntoInteger()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        V: Variant;
        I: Integer;
    begin
        Fr(2, RecRef, FR);
        FR.Value := 20260115D;
        V := FR.Value();
        I := V;
        if I <> 1479993 then
            Error('got %1', I);
    end;

    // Same on both: an Integer field into an Integer local.
    [Test]
    procedure IntegerFieldIntoIntegerLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        Fr(5, RecRef, FR);
        FR.Value := 7;
        I := FR.Value();
        if I <> 7 then
            Error('got %1', I);
    end;

    // Same on both: a Date field into a Date local.
    [Test]
    procedure DateFieldIntoDateLocal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        D: Date;
    begin
        Fr(2, RecRef, FR);
        FR.Value := 20260115D;
        D := FR.Value();
        if D <> 20260115D then
            Error('got %1', D);
    end;

    // Same on both: `exit(FR.Value())` of other field types into a number return converts as before.
    [Test]
    procedure IntegerFieldExitIntoInteger()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(5, RecRef, FR);
        FR.Value := 7;
        if ReturnAsInteger(FR) <> 7 then
            Error('got %1', ReturnAsInteger(FR));
    end;

    [Test]
    procedure DecimalFieldExitIntoInteger()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        Fr(7, RecRef, FR);
        FR.Value := 2.5;
        asserterror I := ReturnAsInteger(FR);
        Error('got [%1] %2', GetLastErrorText(), I);
    end;

    [Test]
    procedure TextFieldExitIntoInteger()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(8, RecRef, FR);
        FR.Value := '5';
        Error('got %1', ReturnAsInteger(FR));
    end;

    [Test]
    procedure IntegerFieldExitIntoDecimal()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(5, RecRef, FR);
        FR.Value := 7;
        if ReturnAsDecimal(FR) <> 7 then
            Error('got %1', ReturnAsDecimal(FR));
    end;

    // Same on both, asked in review: other field types returned through `exit(FR.Value())` from a procedure of their own or
    // another number type. Each test fails with the value it read, so a changed answer shows as another message.
    [Test]
    procedure BooleanFieldExitIntoBoolean()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(9, RecRef, FR);
        FR.Value := true;
        Error('got %1', ReturnAsBoolean(FR));
    end;

    [Test]
    procedure OptionFieldExitIntoOptionFormat()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(10, RecRef, FR);
        FR.Value := 1;
        Error('got %1', Format(ReturnAsOption(FR)));
    end;

    [Test]
    procedure BigIntegerFieldExitIntoIntegerInRange()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(11, RecRef, FR);
        FR.Value := 123456789L;
        Error('got %1', ReturnAsInteger(FR));
    end;

    [Test]
    procedure BigIntegerFieldExitIntoIntegerPastRange()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        Fr(11, RecRef, FR);
        FR.Value := 3000000000L;
        asserterror I := ReturnAsInteger(FR);
        Error('got [%1] %2', GetLastErrorText(), I);
    end;

    [Test]
    procedure IntegerFieldExitIntoBigInteger()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(5, RecRef, FR);
        FR.Value := 7;
        Error('got %1', ReturnAsBigInteger(FR));
    end;

    [Test]
    procedure IntegerFieldExitIntoDecimalValue()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(5, RecRef, FR);
        FR.Value := 7;
        Error('got %1', ReturnAsDecimal(FR));
    end;

    [Test]
    procedure DurationFieldExitIntoDuration()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        D: Duration;
    begin
        Fr(12, RecRef, FR);
        D := 65000;
        FR.Value := D;
        Error('got %1', Format(ReturnAsDuration(FR), 0, 9));
    end;

    [Test]
    procedure CodeFieldExitIntoInteger()
    var
        RecRef: RecordRef;
        FR: FieldRef;
    begin
        Fr(13, RecRef, FR);
        FR.Value := '5';
        Error('got %1', ReturnAsInteger(FR));
    end;
}
