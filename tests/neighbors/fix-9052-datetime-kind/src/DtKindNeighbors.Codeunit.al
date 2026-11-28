// The members of a DotNet System.DateTime whose answer does not depend on the kind when the service tier's zone is UTC
// (ToUniversalTime, ToLocalTime, ToFileTime): Served_* run them on values of each kind. Neighbor_* are the members and shapes
// next to them that read the kind or a moment the probe left out; they keep main's result.
dotnet
{
    assembly("mscorlib")
    {
        type("System.DateTime"; DkNbDateTime) { }
        type("System.DateTimeKind"; DkNbKind) { }
        type("System.TimeSpan"; DkNbSpan) { }
        type("System.DateTimeOffset"; DkNbOffset) { }
    }
}

codeunit 50470 "DtKind Neighbors"
{
    Subtype = Test;

    var
        Assert: Codeunit "DtKind Assert";

    local procedure Utc(Moment: Text): DateTime
    var
        Dt: DateTime;
    begin
        Evaluate(Dt, Moment, 9);
        exit(Dt);
    end;

    local procedure FromParam(Dt: DateTime; var Dn: DotNet DkNbDateTime)
    begin
        Dn := Dt;
    end;

    [Test]
    procedure Served_ToUniversalTime_AlParam()
    var
        Dn: DotNet DkNbDateTime;
        Al: DateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Dn := Dn.ToUniversalTime();
        Al := Dn;
        Assert.Equal('2023-01-15T12:00:00Z', Format(Al, 0, 9));
    end;

    [Test]
    procedure Served_ToLocalTime_AlParam()
    var
        Dn: DotNet DkNbDateTime;
        Al: DateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Dn := Dn.ToLocalTime();
        Al := Dn;
        Assert.Equal('2023-01-15T12:00:00Z', Format(Al, 0, 9));
    end;

    [Test]
    procedure Served_ToFileTime_AlParam()
    var
        Dn: DotNet DkNbDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Assert.Equal('133182576000000000', Format(Dn.ToFileTime()));
    end;

    [Test]
    procedure Served_ToUniversalTime_CtorUnspecified()
    var
        Dn: DotNet DkNbDateTime;
        Al: DateTime;
    begin
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0);
        Al := Dn.ToUniversalTime();
        Assert.Equal('2023-01-15T12:00:00Z', Format(Al, 0, 9));
    end;

    [Test]
    procedure Served_ToLocalTime_CtorLocal()
    var
        Dn: DotNet DkNbDateTime;
        K: DotNet DkNbKind;
        Al: DateTime;
    begin
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0, K.Local);
        Al := Dn.ToLocalTime();
        Assert.Equal('2023-01-15T12:00:00Z', Format(Al, 0, 9));
    end;

    [Test]
    procedure Served_ToFileTime_CtorLocal()
    var
        Dn: DotNet DkNbDateTime;
        K: DotNet DkNbKind;
    begin
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0, K.Local);
        Assert.Equal('133182576000000000', Format(Dn.ToFileTime()));
    end;

    [Test]
    procedure Served_ParseOffset_ToUniversalTime()
    var
        Dn: DotNet DkNbDateTime;
        Al: DateTime;
    begin
        Dn := Dn.Parse('2023-01-15T12:00:00+02:00');
        Al := Dn.ToUniversalTime();
        Assert.Equal('2023-01-15T10:00:00Z', Format(Al, 0, 9));
    end;

    [Test]
    procedure Served_ParseFormat9_ToUniversalTime_R()
    var
        Dn: DotNet DkNbDateTime;
    begin
        Dn := Dn.Parse(Format(Utc('2023-01-15T12:00:00Z'), 0, 9));
        Assert.Equal('Sun, 15 Jan 2023 12:00:00 GMT', Dn.ToUniversalTime().ToString('R'));
    end;

    [Test]
    procedure Served_EpochAdd_ToLocalTime()
    var
        Dn: DotNet DkNbDateTime;
        K: DotNet DkNbKind;
        Span: DotNet DkNbSpan;
        Al: DateTime;
    begin
        Dn := Dn.DateTime(1970, 1, 1, 0, 0, 0, K.Utc);
        Span := Span.FromSeconds(1673784000);
        Dn := Dn.Add(Span);
        Al := Dn.ToLocalTime();
        Assert.Equal('2023-01-15T12:00:00Z', Format(Al, 0, 9));
    end;

    [Test]
    procedure Served_NowToUniversalTimePlusZero_EvaluateToString()
    var
        Dn: DotNet DkNbDateTime;
        Result: DateTime;
        Off: Duration;
    begin
        Dn := Dn.Now;
        Dn := Dn.ToUniversalTime() + Off;
        Assert.IsTrue(Evaluate(Result, Dn.ToString()));
        Assert.IsTrue(Abs(Result - CurrentDateTime) < 5000);
    end;

    [Test]
    procedure Served_NowToUniversalTimePlusTwoHours_EvaluateToString()
    var
        Dn: DotNet DkNbDateTime;
        Result: DateTime;
        Off: Duration;
    begin
        Off := 2 * 60 * 60 * 1000;
        Dn := Dn.Now;
        Dn := Dn.ToUniversalTime() + Off;
        Assert.IsTrue(Evaluate(Result, Dn.ToString()));
        Assert.IsTrue(Abs(Result - CurrentDateTime - Off) < 5000);
    end;

    [Test]
    procedure Served_AlParamPlusTwoHours_ToString()
    var
        Dn: DotNet DkNbDateTime;
        Result: DateTime;
        Off: Duration;
    begin
        Off := 2 * 60 * 60 * 1000;
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Dn := Dn.ToUniversalTime() + Off;
        Assert.IsTrue(Evaluate(Result, Dn.ToString()));
        Assert.Equal('2023-01-15T14:00:00Z', Format(Result, 0, 9));
    end;

    [Test]
    procedure Served_UtcNowToLocalTime()
    var
        Dn: DotNet DkNbDateTime;
        Al: DateTime;
    begin
        Dn := Dn.UtcNow;
        Al := Dn.ToLocalTime();
        Assert.IsTrue(Abs(Al - CurrentDateTime) < 5000);
    end;

    [Test]
    procedure Neighbor_Kind_AlParam()
    var
        Dn: DotNet DkNbDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Assert.Equal('Utc', Format(Dn.Kind));
    end;

    [Test]
    procedure Neighbor_Kind_OfToUniversalTimeResult()
    var
        Dn: DotNet DkNbDateTime;
    begin
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0);
        Dn := Dn.ToUniversalTime();
        Assert.Equal('Utc', Format(Dn.Kind));
    end;

    [Test]
    procedure Neighbor_ToStringRoundTrip_AlParam()
    var
        Dn: DotNet DkNbDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Assert.Equal('2023-01-15T12:00:00.0000000Z', Dn.ToString('o'));
    end;

    [Test]
    procedure Neighbor_ToStringK_AlParam()
    var
        Dn: DotNet DkNbDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Assert.Equal('2023-01-15T12:00:00Z', Dn.ToString('yyyy-MM-ddTHH:mm:ssK'));
    end;

    [Test]
    procedure Neighbor_ToStringZzz_AlParam()
    var
        Dn: DotNet DkNbDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Assert.Equal('12:00 +00:00', Dn.ToString('HH:mm zzz'));
    end;

    [Test]
    procedure Neighbor_ToBinary_AlParam()
    var
        Dn: DotNet DkNbDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Assert.Equal('5249779826427387904', Format(Dn.ToBinary()));
    end;

    [Test]
    procedure Neighbor_ToFileTime_Before1601()
    var
        Dn: DotNet DkNbDateTime;
    begin
        FromParam(CreateDateTime(DMY2Date(1, 1, 1600), 0T), Dn);
        asserterror Dn.ToFileTime();
    end;

    [Test]
    procedure Neighbor_DateTimeOffset_ToUniversalTime()
    var
        Dto: DotNet DkNbOffset;
        Span: DotNet DkNbSpan;
        Dn: DotNet DkNbDateTime;
        Al: DateTime;
    begin
        Dto := Dto.DateTimeOffset(2023, 1, 15, 12, 0, 0, Span.FromHours(2));
        Dn := Dto.UtcDateTime;
        Al := Dn;
        Assert.Equal('2023-01-15T10:00:00Z', Format(Al, 0, 9));
    end;
}

codeunit 50471 "DtKind Assert"
{
    procedure Equal(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected <%1>, got <%2>', Expected, Actual);
    end;

    procedure IsTrue(Condition: Boolean)
    begin
        if not Condition then
            Error('Expected true');
    end;
}
