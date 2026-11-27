// The shapes next to `exit(FR.Value())` from a procedure that returns a Code or Text (al-runner issue 9285). Corpus PR 1629
// (merged at 778b8fc1; FvcTimestampField.al:443, :466, :485, :214) measured the exit of an Integer, BigInteger, Decimal, Enum,
// Option, Boolean, Date and Duration field and the timestamp into Code[20], Text and Code[3]: head gives those, flagged with
// fieldref-value-exit-into-text. A Code or Text field, a number return, a Variant return, a Variant variable, assignment and
// argument answer the same on both. Paren_ tests: a parenthesized exit (issue 9305).
enum 50310 "FY Kind" { Extensible = false; value(0; Zero) { } value(1; One) { Caption = 'One Caption'; } }

table 50311 "FY Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; IntF; Integer) { }
        field(3; BigF; BigInteger) { }
        field(4; DecF; Decimal) { }
        field(5; EnumF; Enum "FY Kind") { }
        field(6; OptF; Option) { OptionMembers = Alpha,Beta,Gamma; }
        field(7; BoolF; Boolean) { }
        field(8; DateF; Date) { }
        field(9; DurF; Duration) { }
        field(10; CodeF; Code[20]) { }
        field(11; TextF; Text[30]) { }
        field(12; GuidF; Guid) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50312 "FY Helper"
{
    procedure Code20Of(var RecRef: RecordRef; FieldNo: Integer): Code[20]
    var
        FRef: FieldRef;
    begin
        FRef := RecRef.Field(FieldNo);
        exit(FRef.Value);
    end;

    procedure TextOf(var RecRef: RecordRef; FieldNo: Integer): Text
    begin
        exit(RecRef.Field(FieldNo).Value());
    end;

    procedure Code3Of(var RecRef: RecordRef; FieldNo: Integer): Code[3]
    begin
        exit(RecRef.Field(FieldNo).Value());
    end;

    procedure ParenCode20Of(var RecRef: RecordRef; FieldNo: Integer): Code[20]
    var
        FRef: FieldRef;
    begin
        FRef := RecRef.Field(FieldNo);
        exit((FRef.Value()));
    end;

    procedure ParenIntegerOf(var RecRef: RecordRef; FieldNo: Integer): Integer
    var
        FRef: FieldRef;
    begin
        FRef := RecRef.Field(FieldNo);
        exit((FRef.Value()));
    end;

    procedure IntegerOf(var RecRef: RecordRef; FieldNo: Integer): Integer
    begin
        exit(RecRef.Field(FieldNo).Value());
    end;

    procedure VariantOf(var RecRef: RecordRef; FieldNo: Integer): Variant
    begin
        exit(RecRef.Field(FieldNo).Value());
    end;

    procedure Code20OfVariant(V: Variant): Code[20]
    begin
        exit(V);
    end;

    procedure Echo(T: Text): Text
    begin
        exit(T);
    end;
}

codeunit 50313 "FY Tests"
{
    Subtype = Test;

    local procedure Seed(var RecRef: RecordRef)
    var
        Row: Record "FY Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.IntF := 5;
        Row.BigF := 5000000000L;
        Row.DecF := 1.5;
        Row.EnumF := Row.EnumF::One;
        Row.OptF := Row.OptF::Beta;
        Row.BoolF := true;
        Row.DateF := 20260115D;
        Row.DurF := 1500;
        Row.CodeF := 'FY-1';
        Row.TextF := 'Mixed Case';
        Row.Insert();
        RecRef.Open(Database::"FY Row");
        RecRef.FindFirst();
    end;

    [Test]
    procedure Exit_Integer_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 2));
    end;

    [Test]
    procedure Exit_Integer_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 2));
    end;

    [Test]
    procedure Exit_Integer_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 2));
    end;

    [Test]
    procedure Exit_BigInteger_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 3));
    end;

    [Test]
    procedure Exit_BigInteger_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 3));
    end;

    [Test]
    procedure Exit_BigInteger_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 3));
    end;

    [Test]
    procedure Exit_Decimal_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 4));
    end;

    [Test]
    procedure Exit_Decimal_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 4));
    end;

    [Test]
    procedure Exit_Decimal_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 4));
    end;

    [Test]
    procedure Exit_Enum_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 5));
    end;

    [Test]
    procedure Exit_Enum_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 5));
    end;

    [Test]
    procedure Exit_Enum_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 5));
    end;

    [Test]
    procedure Exit_Option_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 6));
    end;

    [Test]
    procedure Exit_Option_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 6));
    end;

    [Test]
    procedure Exit_Option_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 6));
    end;

    [Test]
    procedure Exit_Boolean_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 7));
    end;

    [Test]
    procedure Exit_Boolean_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 7));
    end;

    [Test]
    procedure Exit_Boolean_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 7));
    end;

    [Test]
    procedure Exit_Date_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 8));
    end;

    [Test]
    procedure Exit_Date_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 8));
    end;

    [Test]
    procedure Exit_Date_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 8));
    end;

    [Test]
    procedure Exit_Duration_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 9));
    end;

    [Test]
    procedure Exit_Duration_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 9));
    end;

    [Test]
    procedure Exit_Duration_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 9));
    end;

    [Test]
    procedure Exit_Code_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 10));
    end;

    [Test]
    procedure Exit_Code_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 10));
    end;

    [Test]
    procedure Exit_Code_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 10));
    end;

    [Test]
    procedure Exit_Text_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 11));
    end;

    [Test]
    procedure Exit_Text_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 11));
    end;

    [Test]
    procedure Exit_Text_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 11));
    end;

    [Test]
    procedure Exit_Guid_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code20Of(RecRef, 12));
    end;

    [Test]
    procedure Exit_Guid_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.TextOf(RecRef, 12));
    end;

    [Test]
    procedure Exit_Guid_IntoCode3()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Code3Of(RecRef, 12));
    end;

    [Test]
    procedure Exit_Timestamp_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        if H.Code20Of(RecRef, 0) <> Format(RecRef.Field(0).Value()) then
            Error('timestamp differs from its Format');
    end;

    [Test]
    procedure Exit_Timestamp_IntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        if H.TextOf(RecRef, 0) <> Format(RecRef.Field(0).Value()) then
            Error('timestamp differs from its Format');
    end;

    [Test]
    procedure Paren_Integer_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.ParenCode20Of(RecRef, 2));
    end;

    [Test]
    procedure Paren_Code_IntoCode20()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.ParenCode20Of(RecRef, 10));
    end;

    [Test]
    procedure Paren_Date_IntoInteger()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.ParenIntegerOf(RecRef, 8));
    end;

    [Test]
    procedure Paren_Integer_IntoInteger()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.ParenIntegerOf(RecRef, 2));
    end;

    [Test]
    procedure Exit_Integer_IntoInteger()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.IntegerOf(RecRef, 2));
    end;

    [Test]
    procedure Exit_IntoVariantThenFormat()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', Format(H.VariantOf(RecRef, 2)));
    end;

    [Test]
    procedure VariantVariable_IntegerIntoCode20()
    var
        H: Codeunit "FY Helper";
        V: Variant;
    begin
        V := 7;
        Error('got [%1]', H.Code20OfVariant(V));
    end;

    [Test]
    procedure Assign_IntegerIntoCode20()
    var
        RecRef: RecordRef;
        C: Code[20];
    begin
        Seed(RecRef);
        C := RecRef.Field(2).Value();
        Error('got [%1]', C);
    end;

    [Test]
    procedure Arg_IntegerIntoText()
    var
        H: Codeunit "FY Helper";
        RecRef: RecordRef;
    begin
        Seed(RecRef);
        Error('got [%1]', H.Echo(RecRef.Field(2).Value()));
    end;
}
