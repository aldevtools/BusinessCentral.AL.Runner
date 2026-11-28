// The shapes next to FieldRef.Value() of a Date, Time or DateTime field converted straight into a number (al-runner issues
// 9265 and 9305). Corpus PR 1629 (FvcTemporalIntoNumberPlaces.al) measured a blank and a set value of each kind into each number
// target and into an Integer in each place; head gives those readings, flagged with fieldref-value-temporal-into-number, and
// takes the rule for every value of the kind (Other_ tests: the values corpus PR 1641 measured, FvcTemporalIntoNumberMore.al
// :162-326). Same on both: the Variant route,
// a Date into a Text, an Integer field into an Integer, Abs of an Integer field and of a Variant holding a Date. Unmeasured_
// tests: places the probes did not measure, which refuse on both.
interface "FT Taker"
{
    procedure Take(I: Integer): Integer;
}

codeunit 50322 "FT Taker Impl" implements "FT Taker"
{
    procedure Take(I: Integer): Integer
    begin
        exit(I);
    end;
}

table 50320 "FT Row"
{
    fields
    {
        field(1; K; Integer) { }
        field(2; D; Date) { }
        field(3; T; Time) { }
        field(4; Dt; DateTime) { }
        field(5; IntT; Integer) { }
    }
    keys { key(PK; K) { Clustered = true; } }

    procedure CurrFieldNoFrom(var FR: FieldRef): Integer
    begin
        CurrFieldNo := FR.Value();
        exit(CurrFieldNo);
    end;
}

codeunit 50321 "FT Tests"
{
    Subtype = Test;

    var
        GlobalInt: Integer;

    local procedure Src(var RR: RecordRef; FieldNo: Integer; V: Variant): FieldRef
    begin
        RR.Open(Database::"FT Row");
        RR.Field(FieldNo).Value := V;
        exit(RR.Field(FieldNo));
    end;

    local procedure Expect(Want: Text)
    begin
        if GetLastErrorText() <> Want then
            Error('got [%1], want [%2]', GetLastErrorText(), Want);
    end;

    local procedure Conv(Kind: Text; DotNet: Text): Text
    begin
        exit(StrSubstNo('Unable to convert from Microsoft.Dynamics.Nav.Runtime.Nav%1 to %2.', Kind, DotNet));
    end;

    local procedure TakeInt(I: Integer): Integer begin exit(I); end;
    [TryFunction]
    local procedure TryTakeInt(I: Integer) begin end;
    local procedure RetExit(var FR: FieldRef): Integer begin exit(FR.Value()); end;
    local procedure RetParen(var FR: FieldRef): Integer begin exit((FR.Value())); end;
    local procedure RetNamed(var FR: FieldRef) R: Integer begin R := FR.Value(); end;

    [Test]
    procedure Date1_IntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror I := FR.Value();
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date1_IntoDecimal()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Decimal;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror X := FR.Value();
        Expect(Conv('Date', 'Microsoft.Dynamics.Nav.Runtime.Decimal18'));
    end;

    [Test]
    procedure Time1_IntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 3, 120000T);
        asserterror I := FR.Value();
        Expect(Conv('Time', 'System.Int32'));
    end;

    [Test]
    procedure Time1_IntoDecimal()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Decimal;
    begin
        FR := Src(RR, 3, 120000T);
        asserterror X := FR.Value();
        Expect(Conv('Time', 'Microsoft.Dynamics.Nav.Runtime.Decimal18'));
    end;

    [Test]
    procedure DateTime1_IntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 4, CreateDateTime(20260115D, 120000T));
        asserterror I := FR.Value();
        Expect(Conv('DateTime', 'System.Int32'));
    end;

    [Test]
    procedure DateTime1_IntoDecimal()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Decimal;
    begin
        FR := Src(RR, 4, CreateDateTime(20260115D, 120000T));
        asserterror X := FR.Value();
        Expect(Conv('DateTime', 'Microsoft.Dynamics.Nav.Runtime.Decimal18'));
    end;

    [Test]
    procedure Date0_IntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, 0D);
        asserterror I := FR.Value();
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date0_IntoDecimal()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Decimal;
    begin
        FR := Src(RR, 2, 0D);
        asserterror X := FR.Value();
        Expect(Conv('Date', 'Microsoft.Dynamics.Nav.Runtime.Decimal18'));
    end;

    [Test]
    procedure Date1_IntoBigInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: BigInteger;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror X := FR.Value();
        Expect(Conv('Date', 'System.Int64'));
    end;

    [Test]
    procedure Time1_IntoByte()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Byte;
    begin
        FR := Src(RR, 3, 120000T);
        asserterror X := FR.Value();
        Expect(Conv('Time', 'System.Byte'));
    end;

    [Test]
    procedure Date1_IntoChar()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Char;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror X := FR.Value();
        Expect(Conv('Date', 'System.Char'));
    end;

    [Test]
    procedure Date1_IntoOption()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Option A,B,C;
    begin
        FR := Src(RR, 2, 20260115D);
        X := FR.Value();
        if Format(X, 0, 9) <> '1479993' then Error('got %1', Format(X, 0, 9));
    end;

    [Test]
    procedure Time1_IntoBoolean()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Boolean;
    begin
        FR := Src(RR, 3, 120000T);
        X := FR.Value();
        if not X then Error('got false');
    end;

    [Test]
    procedure Date1_IntoDuration()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Duration;
    begin
        FR := Src(RR, 2, 20260115D);
        X := FR.Value();
        if Format(X, 0, 9) <> 'P0DT0H24M39.993S' then Error('got %1', Format(X, 0, 9));
    end;

    [Test]
    procedure DateTime1_IntoBoolean()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Boolean;
    begin
        FR := Src(RR, 4, CreateDateTime(20260115D, 120000T));
        asserterror X := FR.Value();
        Expect(Conv('DateTime', 'System.Int32'));
    end;

    [Test]
    procedure DateTime1_IntoOption()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Option A,B,C;
    begin
        FR := Src(RR, 4, CreateDateTime(20260115D, 120000T));
        asserterror X := FR.Value();
        Error('got [%1]', GetLastErrorText());
    end;

    [Test]
    procedure Date1_Exit()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror I := RetExit(FR);
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date1_ParenExit()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror I := RetParen(FR);
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date1_NamedReturn()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror I := RetNamed(FR);
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date1_Argument()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror I := TakeInt(FR.Value());
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date1_Global()
    var
        RR: RecordRef;
        FR: FieldRef;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror GlobalInt := FR.Value();
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date1_RecordField()
    var
        RR: RecordRef;
        FR: FieldRef;
        Row: Record "FT Row";
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror Row.IntT := FR.Value();
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date1_ArrayElement()
    var
        RR: RecordRef;
        FR: FieldRef;
        A: array[2] of Integer;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror A[1] := FR.Value();
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Date1_ListOfDecimal()
    var
        RR: RecordRef;
        FR: FieldRef;
        L: List of [Decimal];
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror L.Add(FR.Value());
        Expect(Conv('Date', 'Microsoft.Dynamics.Nav.Runtime.Decimal18'));
    end;

    [Test]
    procedure Date1_ListOfBigInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        L: List of [BigInteger];
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror L.Add(FR.Value());
        Expect(Conv('Date', 'System.Int64'));
    end;

    [Test]
    procedure Date1_Abs()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, 20260115D);
        asserterror I := Abs(FR.Value());
        Expect(Conv('Date', 'Microsoft.Dynamics.Nav.Runtime.Decimal18'));
    end;

    [Test]
    procedure Other_EarlyDate_IntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, 17530101D);
        asserterror I := FR.Value();
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Other_LateDate_IntoDecimal()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Decimal;
    begin
        FR := Src(RR, 2, 99991231D);
        asserterror X := FR.Value();
        Expect(Conv('Date', 'Microsoft.Dynamics.Nav.Runtime.Decimal18'));
    end;

    [Test]
    procedure Other_Time_IntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 3, 083015T);
        asserterror I := FR.Value();
        Expect(Conv('Time', 'System.Int32'));
    end;

    [Test]
    procedure Other_Time_IntoOption()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Option A,B,C;
    begin
        FR := Src(RR, 3, 000001T);
        X := FR.Value();
        Error('got [%1]', Format(X, 0, 9));
    end;

    [Test]
    procedure Other_Date_IntoDuration()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Duration;
    begin
        FR := Src(RR, 2, 17530101D);
        X := FR.Value();
        Error('got [%1]', Format(X, 0, 9));
    end;

    [Test]
    procedure Other_DateTime_IntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 4, CreateDateTime(17530101D, 000001T));
        asserterror I := FR.Value();
        Expect(Conv('DateTime', 'System.Int32'));
    end;

    [Test]
    procedure Same_VariantRoute_DateIntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
        V: Variant;
    begin
        FR := Src(RR, 2, 20260115D);
        V := FR.Value();
        I := V;
        Error('got [%1]', I);
    end;

    [Test]
    procedure Same_DateIntoText()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Text;
    begin
        FR := Src(RR, 2, 20260115D);
        X := FR.Value();
        Error('got [%1]', X);
    end;

    [Test]
    procedure Same_IntegerFieldIntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 5, 7);
        I := FR.Value();
        Error('got [%1]', I);
    end;

    [Test]
    procedure Same_AbsOfIntegerField()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 5, -7);
        I := Abs(FR.Value());
        Error('got [%1]', I);
    end;

    [Test]
    procedure Same_AbsOfVariantHoldingDate()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
        V: Variant;
    begin
        FR := Src(RR, 2, 20260115D);
        V := FR.Value();
        I := Abs(V);
        Error('got [%1]', I);
    end;

    [Test]
    procedure Unmeasured_ListContains()
    var
        RR: RecordRef;
        FR: FieldRef;
        L: List of [Integer];
    begin
        FR := Src(RR, 2, 20260115D);
        Error('got [%1]', L.Contains(FR.Value()));
    end;

    [Test]
    procedure Unmeasured_ListIndexOf()
    var
        RR: RecordRef;
        FR: FieldRef;
        L: List of [Integer];
    begin
        FR := Src(RR, 2, 20260115D);
        Error('got [%1]', L.IndexOf(FR.Value()));
    end;

    [Test]
    procedure Unmeasured_ListInsert()
    var
        RR: RecordRef;
        FR: FieldRef;
        L: List of [Integer];
    begin
        FR := Src(RR, 2, 20260115D);
        L.Insert(1, FR.Value());
        Error('got [%1]', L.Get(1));
    end;

    [Test]
    procedure Unmeasured_DictionaryAdd()
    var
        RR: RecordRef;
        FR: FieldRef;
        D: Dictionary of [Integer, Integer];
    begin
        FR := Src(RR, 2, 20260115D);
        D.Add(FR.Value(), 1);
        Error('got [%1]', D.Count());
    end;

    [Test]
    procedure Unmeasured_DictionaryContainsKey()
    var
        RR: RecordRef;
        FR: FieldRef;
        D: Dictionary of [Integer, Integer];
    begin
        FR := Src(RR, 2, 20260115D);
        Error('got [%1]', D.ContainsKey(FR.Value()));
    end;

    [Test]
    procedure Unmeasured_TryFunctionArgument()
    var
        RR: RecordRef;
        FR: FieldRef;
    begin
        FR := Src(RR, 2, 20260115D);
        Error('got [%1]', TryTakeInt(FR.Value()));
    end;

    [Test]
    procedure Unmeasured_InterfaceArgument()
    var
        RR: RecordRef;
        FR: FieldRef;
        Impl: Codeunit "FT Taker Impl";
        Taker: Interface "FT Taker";
    begin
        FR := Src(RR, 2, 20260115D);
        Taker := Impl;
        Error('got [%1]', Taker.Take(FR.Value()));
    end;

    [Test]
    procedure Unmeasured_CurrFieldNo()
    var
        RR: RecordRef;
        FR: FieldRef;
        Row: Record "FT Row";
    begin
        FR := Src(RR, 2, 20260115D);
        Error('got [%1]', Row.CurrFieldNoFrom(FR));
    end;

    [Test]
    procedure Other_ClosingDate_IntoOption()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Option A,B,C;
    begin
        FR := Src(RR, 2, ClosingDate(20260115D));
        X := FR.Value();
        Error('got [%1]', Format(X, 0, 9));
    end;

    [Test]
    procedure Other_ClosingDate_IntoInteger()
    var
        RR: RecordRef;
        FR: FieldRef;
        I: Integer;
    begin
        FR := Src(RR, 2, ClosingDate(20260115D));
        asserterror I := FR.Value();
        Expect(Conv('Date', 'System.Int32'));
    end;

    [Test]
    procedure Other_EarlyDateTime_IntoOption()
    var
        RR: RecordRef;
        FR: FieldRef;
        X: Option A,B,C;
    begin
        FR := Src(RR, 4, CreateDateTime(17530101D, 000001T));
        asserterror X := FR.Value();
        Error('got [%1]', GetLastErrorText());
    end;
}
