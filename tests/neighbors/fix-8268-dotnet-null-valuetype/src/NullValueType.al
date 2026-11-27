// A DotNet variable that holds no object, passed where its declared type is a .NET value type: the measured calls
// (Measured_*), and the shapes next to them that the probe left out (Neighbor_*). Control_* keep main's handling.
dotnet
{
    assembly("mscorlib")
    {
        type("System.Byte"; NVTByte) { }
        type("System.Int32"; NVTInt32) { }
        type("System.Int64"; NVTInt64) { }
        type("System.Boolean"; NVTBoolean) { }
        type("System.DateTime"; NVTDateTime) { }
        type("System.TimeSpan"; NVTTimeSpan) { }
        type("System.DateTimeOffset"; NVTDateTimeOffset) { }
        type("System.Convert"; NVTConvert) { }
        type("System.String"; NVTString) { }
        type("System.Text.StringBuilder"; NVTStringBuilder) { }
    }
}

codeunit 50460 "NVT Tests"
{
    Subtype = Test;

    var
        GlobalInt32: DotNet NVTInt32;
        ResultText: Text;

    [TryFunction]
    local procedure TryToInt32(B: DotNet NVTBoolean)
    var
        Cv: DotNet NVTConvert;
    begin
        ResultText := Format(Cv.ToInt32(B));
    end;

    [TryFunction]
    local procedure TrySbCtor(I: DotNet NVTInt32)
    var
        Sb: DotNet NVTStringBuilder;
    begin
        Sb := Sb.StringBuilder(I, 5);
    end;

    local procedure ToStringByVar(var I: DotNet NVTInt32): Text
    var
        Cv: DotNet NVTConvert;
    begin
        exit(Cv.ToString(I, 2));
    end;

    local procedure Expect(Actual: Text; Wanted: Text)
    begin
        if Actual <> Wanted then
            Error('got ''%1'', wanted ''%2''', Actual, Wanted);
    end;

    [Test]
    procedure Measured_ConvertToStringNullByte()
    var
        B: DotNet NVTByte;
        Cv: DotNet NVTConvert;
    begin
        Expect(Cv.ToString(B, 2), '0');
    end;

    [Test]
    procedure Measured_ConvertToInt32NullBooleanByValueParam()
    var
        B: DotNet NVTBoolean;
    begin
        if not TryToInt32(B) then
            Error('TryToInt32 failed: %1', GetLastErrorText());
        Expect(ResultText, '0');
    end;

    [Test]
    procedure Measured_DateTimeOffsetNullTimeSpanRaises()
    var
        D: DotNet NVTDateTime;
        Ts: DotNet NVTTimeSpan;
        Dto: DotNet NVTDateTimeOffset;
    begin
        D := D.UtcNow();
        asserterror Dto := Dto.DateTimeOffset(D, Ts);
        Expect(GetLastErrorCode(), 'DotNetCreate:NullReference');
    end;

    [Test]
    procedure Measured_StringBuilderNullInt32Raises()
    var
        I: DotNet NVTInt32;
    begin
        if TrySbCtor(I) then
            Error('TrySbCtor did not fail');
        Expect(GetLastErrorCode(), 'DotNetCreate:NullReference');
    end;

    [Test]
    procedure Neighbor_NullInt64UnmeasuredType()
    var
        L: DotNet NVTInt64;
        Cv: DotNet NVTConvert;
    begin
        Expect(Cv.ToString(L, 2), '0');
    end;

    [Test]
    procedure Neighbor_NullInt32OneArgumentOverload()
    var
        I: DotNet NVTInt32;
        Cv: DotNet NVTConvert;
    begin
        Expect(Cv.ToString(I), '0');
    end;

    [Test]
    procedure Neighbor_NullInt32PropertySetter()
    var
        I: DotNet NVTInt32;
        Sb: DotNet NVTStringBuilder;
    begin
        Sb := Sb.StringBuilder();
        Sb.Capacity := I;
    end;

    [Test]
    procedure Neighbor_NullInt32VarParam()
    var
        I: DotNet NVTInt32;
    begin
        Expect(ToStringByVar(I), '0');
    end;

    [Test]
    procedure Neighbor_NullInt32Global()
    var
        Cv: DotNet NVTConvert;
    begin
        Expect(Cv.ToString(GlobalInt32, 2), '0');
    end;

    [Test]
    procedure Neighbor_NullInt32IntoAppendOverloads()
    var
        I: DotNet NVTInt32;
        Sb: DotNet NVTStringBuilder;
    begin
        Sb := Sb.StringBuilder();
        Sb.Append(I);
        Expect(Sb.ToString(), '');
    end;

    [Test]
    procedure Neighbor_DateTimeCompareNullSecond()
    var
        D: DotNet NVTDateTime;
        Min: DotNet NVTDateTime;
    begin
        Min := Min.DateTime(1, 1, 1);
        Expect(Format(D.Compare(Min, D)), '0');
    end;

    [Test]
    procedure Neighbor_StringBuilderNullInt32OnBc29Shape()
    var
        I: DotNet NVTInt32;
        Sb: DotNet NVTStringBuilder;
    begin
        // The constructor with a null Int32 and a Text second argument: not measured.
        asserterror Sb := Sb.StringBuilder(I, 'x');
        Expect(GetLastErrorCode(), 'DotNetCreate:NullReference');
    end;

    [Test]
    procedure Control_ConvertToStringInt32Value()
    var
        I: DotNet NVTInt32;
        Cv: DotNet NVTConvert;
    begin
        I := 5;
        Expect(Cv.ToString(I, 2), '101');
    end;

    [Test]
    procedure Control_NullStringReferenceType()
    var
        S: DotNet NVTString;
        Sb: DotNet NVTStringBuilder;
    begin
        Sb := Sb.StringBuilder(S);
        Expect(Sb.ToString(), '');
    end;
}
