// Review round 1: the place of the null. The probe passed the null of Convert.ToString(Byte), DateTimeOffset and
// Regex(.., null TimeSpan) as a local of the test, and the null of every other call as a by-value parameter of a
// [TryFunction]. Measured_* use the measured place; Neighbor_* put the null of a measured call in another place, or
// give DateTime.Compare an AL DateTime where the probe used a DotNet DateTime variable.
dotnet
{
    assembly("mscorlib")
    {
        type("System.Byte"; NVPByte) { }
        type("System.Int32"; NVPInt32) { }
        type("System.Boolean"; NVPBoolean) { }
        type("System.DateTime"; NVPDateTime) { }
        type("System.TimeSpan"; NVPTimeSpan) { }
        type("System.Convert"; NVPConvert) { }
        type("System.Text.StringBuilder"; NVPStringBuilder) { }
    }
}

codeunit 50470 "NVP Tests"
{
    Subtype = Test;

    var
        ResultText: Text;

    local procedure Expect(Actual: Text; Wanted: Text)
    begin
        if Actual <> Wanted then
            Error('got ''%1'', wanted ''%2''', Actual, Wanted);
    end;

    [TryFunction]
    local procedure TryDateTimeAdd(TS: DotNet NVPTimeSpan)
    var
        D: DotNet NVPDateTime;
        D2: DotNet NVPDateTime;
    begin
        D := D.UtcNow();
        D2 := D.Add(TS);
        ResultText := Format(D2.Equals(D));
    end;

    [TryFunction]
    local procedure TryToStringLocalByte()
    var
        B: DotNet NVPByte;
        Cv: DotNet NVPConvert;
    begin
        ResultText := Cv.ToString(B, 2);
    end;

    [TryFunction]
    local procedure TryCompareAlDateTime(DN: DotNet NVPDateTime)
    var
        DMin: DotNet NVPDateTime;
    begin
        ResultText := Format(DMin.Compare(DN, CreateDateTime(DMY2Date(1, 1, 2000), 0T)));
    end;

    local procedure ToStringByValuePlain(B: DotNet NVPByte): Text
    var
        Cv: DotNet NVPConvert;
    begin
        exit(Cv.ToString(B, 2));
    end;

    local procedure ToStringLocalOfHelper(): Text
    var
        B: DotNet NVPByte;
        Cv: DotNet NVPConvert;
    begin
        exit(Cv.ToString(B, 2));
    end;

    [Test]
    procedure Measured_DateTimeAddNullTimeSpanTryParam()
    var
        TS: DotNet NVPTimeSpan;
    begin
        if not TryDateTimeAdd(TS) then
            Error('TryDateTimeAdd failed: %1', GetLastErrorText());
        Expect(ResultText, 'Yes');
    end;

    [Test]
    procedure Neighbor_DateTimeAddNullTimeSpanLocalDirect()
    var
        D: DotNet NVPDateTime;
        D2: DotNet NVPDateTime;
        TS: DotNet NVPTimeSpan;
    begin
        D := D.UtcNow();
        D2 := D.Add(TS);
        Expect(Format(D2.Equals(D)), 'Yes');
    end;

    [Test]
    procedure Neighbor_ConvertToStringNullByteByValuePlainProc()
    var
        B: DotNet NVPByte;
    begin
        Expect(ToStringByValuePlain(B), '0');
    end;

    [Test]
    procedure Neighbor_ConvertToStringNullByteLocalOfTryFunction()
    begin
        if not TryToStringLocalByte() then
            Error('TryToStringLocalByte failed: %1', GetLastErrorText());
        Expect(ResultText, '0');
    end;

    [Test]
    procedure Neighbor_ConvertToStringNullByteLocalOfHelper()
    begin
        // A local of a plain procedure, not of the test: refused (the probe measured the direct cells only as locals of a [Test] method).
        Expect(ToStringLocalOfHelper(), '0');
    end;

    [Test]
    procedure Neighbor_ConvertToInt32NullBooleanLocalDirect()
    var
        Bo: DotNet NVPBoolean;
        Cv: DotNet NVPConvert;
    begin
        Expect(Format(Cv.ToInt32(Bo)), '0');
    end;

    [Test]
    procedure Neighbor_StringBuilderCtorNullInt32LocalDirect()
    var
        I: DotNet NVPInt32;
        Sb: DotNet NVPStringBuilder;
    begin
        asserterror Sb := Sb.StringBuilder(I, 5);
        Expect(GetLastErrorCode(), 'DotNetCreate:NullReference');
    end;

    [Test]
    procedure Neighbor_DateTimeCompareAlDateTimeSecond()
    var
        DN: DotNet NVPDateTime;
    begin
        if not TryCompareAlDateTime(DN) then
            Error('TryCompareAlDateTime failed: %1', GetLastErrorText());
        Expect(ResultText, '-1');
    end;
}
