// ToString() of a DotNet System.DateTime, with no argument: BC writes a plain space before AM or PM (al-runner issue 9395,
// corpus pull request 1653, DotNetDateTimeKind.al, the tostring step of every origin). Served_* run the measured shapes.
// Neighbor_* are the members and shapes next to them that the probe did not measure; they keep main's result. Their
// expected text is a plain space too, so a run shows the text it got in the failure message.
dotnet
{
    assembly("mscorlib")
    {
        type("System.DateTime"; DtTxDateTime) { }
        type("System.DateTimeKind"; DtTxKind) { }
        type("System.DateTimeOffset"; DtTxOffset) { }
        type("System.String"; DtTxString) { }
        type("System.Convert"; DtTxConvert) { }
        type("System.Globalization.CultureInfo"; DtTxCulture) { }
    }
}

codeunit 50480 "DtText Neighbors"
{
    Subtype = Test;

    local procedure Utc(Moment: Text): DateTime
    var
        Dt: DateTime;
    begin
        Evaluate(Dt, Moment, 9);
        exit(Dt);
    end;

    local procedure FromParam(Dt: DateTime; var Dn: DotNet DtTxDateTime)
    begin
        Dn := Dt;
    end;

    local procedure Equal(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected <%1>, got <%2>', Expected, Actual);
    end;

    // DotNetDateTimeKind.al:377 (Origin_AlParam, tostring).
    [Test]
    procedure Served_AlParam_ToString()
    var
        Dn: DotNet DtTxDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('1/15/2023 12:00:00 PM', Dn.ToString());
    end;

    // DotNetDateTimeKind.al:521 (Origin_ParseOffset, tostring): AM.
    [Test]
    procedure Served_ParseOffset_ToString_AM()
    var
        Dn: DotNet DtTxDateTime;
    begin
        Dn := Dn.Parse('2023-01-15T12:00:00+02:00');
        Equal('1/15/2023 10:00:00 AM', Dn.ToString());
    end;

    // DotNetDateTimeKind.al:701 (Origin_AlSumTwoHoursParam, tostring): a one-digit hour.
    [Test]
    procedure Served_AlSumTwoHours_ToString()
    var
        Dn: DotNet DtTxDateTime;
        Off: Duration;
    begin
        Off := 2 * 60 * 60 * 1000;
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Dn := Dn.ToUniversalTime() + Off;
        Equal('1/15/2023 2:00:00 PM', Dn.ToString());
    end;

    // DotNetDateTimeKind.al:431 (Origin_CtorUnspecified, tostring).
    [Test]
    procedure Served_CtorUnspecified_ToString()
    var
        Dn: DotNet DtTxDateTime;
    begin
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0);
        Equal('1/15/2023 12:00:00 PM', Dn.ToString());
    end;

    // DotNetDateTimeKind.al:467 (Origin_CtorLocal, tostring).
    [Test]
    procedure Served_CtorLocal_ToString()
    var
        Dn: DotNet DtTxDateTime;
        Kind: DotNet DtTxKind;
    begin
        Kind := Kind.Local;
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0, Kind);
        Equal('1/15/2023 12:00:00 PM', Dn.ToString());
    end;

    // DotNetDateTimeKind.al:377 (the eval part): Evaluate reads the text back. Passes on main too.
    [Test]
    procedure Served_AlParam_ToString_Evaluate()
    var
        Dn: DotNet DtTxDateTime;
        Al: DateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        if not Evaluate(Al, Dn.ToString()) then
            Error('Evaluate failed');
        Equal('2023-01-15T12:00:00Z', Format(Al, 0, 9));
    end;

    // The moment midnight (12 AM) is not measured; the probe measured 10 AM, 12 PM and 2 PM.
    [Test]
    procedure Neighbor_Midnight_ToString()
    var
        Dn: DotNet DtTxDateTime;
    begin
        Dn := Dn.DateTime(2023, 1, 15, 0, 0, 0);
        Equal('1/15/2023 12:00:00 AM', Dn.ToString());
    end;

    // The form without parentheses is not measured.
    [Test]
    procedure Neighbor_ToString_NoParentheses()
    var
        Dn: DotNet DtTxDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('1/15/2023 12:00:00 PM', Dn.ToString);
    end;

    [Test]
    procedure Neighbor_ToString_FormatG()
    var
        Dn: DotNet DtTxDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('1/15/2023 12:00:00 PM', Dn.ToString('G'));
    end;

    [Test]
    procedure Neighbor_ToString_FormatShortTime()
    var
        Dn: DotNet DtTxDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('12:00 PM', Dn.ToString('t'));
    end;

    [Test]
    procedure Neighbor_ToString_Culture()
    var
        Dn: DotNet DtTxDateTime;
        Culture: DotNet DtTxCulture;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Culture := Culture.CultureInfo('en-US');
        Equal('1/15/2023 12:00:00 PM', Dn.ToString(Culture));
    end;

    [Test]
    procedure Neighbor_ToLongTimeString()
    var
        Dn: DotNet DtTxDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('12:00:00 PM', Dn.ToLongTimeString());
    end;

    [Test]
    procedure Neighbor_ToShortTimeString()
    var
        Dn: DotNet DtTxDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('12:00 PM', Dn.ToShortTimeString());
    end;

    [Test]
    procedure Neighbor_StringFormat()
    var
        Dn: DotNet DtTxDateTime;
        Str: DotNet DtTxString;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('1/15/2023 12:00:00 PM', Str.Format('{0}', Dn));
    end;

    [Test]
    procedure Neighbor_ConvertToString()
    var
        Dn: DotNet DtTxDateTime;
        Conv: DotNet DtTxConvert;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('1/15/2023 12:00:00 PM', Conv.ToString(Dn));
    end;

    [Test]
    procedure Neighbor_DateTimeOffset_ToString()
    var
        Dn: DotNet DtTxDateTime;
        Offset: DotNet DtTxOffset;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Offset := Offset.DateTimeOffset(Dn);
        Equal('1/15/2023 12:00:00 PM +00:00', Offset.ToString());
    end;

    // AL Format of the DotNet variable: AL's DateTime format, not .NET's ToString.
    [Test]
    procedure Neighbor_AlFormat()
    var
        Dn: DotNet DtTxDateTime;
    begin
        FromParam(Utc('2023-01-15T12:00:00Z'), Dn);
        Equal('01/15/23 12:00 PM', Format(Dn));
    end;
}
