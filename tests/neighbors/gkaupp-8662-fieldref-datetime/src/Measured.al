// Neighbors of al-runner issue 8662: FieldRef.Value of an Integer, BigInteger, Decimal, Duration or Boolean field assigned to
// a local DateTime, Date or Time (corpus PR 1511, assumption fieldref-value-temporal-error), and the places, values, types
// and sources next to it, which keep main's answer or refuse. Error texts are the BC 27 one or the later one (corpus PR 1511).
table 61620 "GK Fvt Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; IntF; Integer) { }
        field(3; BigF; BigInteger) { }
        field(4; DecF; Decimal) { }
        field(5; DurF; Duration) { }
        field(6; BoolF; Boolean) { }
        field(7; DateF; Date) { }
        field(8; DtF; DateTime) { }
        field(9; OptF; Option) { OptionMembers = A,B; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 61620 "GK Fvt Lib"
{
    procedure Field(No: Integer; Val: Variant; var FR: FieldRef)
    var
        Row: Record "GK Fvt Row";
        RR: RecordRef;
    begin
        Row.DeleteAll();
        RR.GetTable(Row);
        FR := RR.Field(No);
        FR.Value := Val;
    end;

    procedure CheckError(Text27: Text; Text28: Text)
    begin
        if (GetLastErrorText() = Text27) and (GetLastErrorCode() = 'Conversion') then
            exit;
        if (GetLastErrorText() = Text28) and (GetLastErrorCode() = 'Evaluate:Conversion') then
            exit;
        Error('unexpected [%1] [%2]', GetLastErrorText(), GetLastErrorCode());
    end;

    procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected [%1] but was [%2]', Expected, Actual);
    end;
}

codeunit 61621 "GK Fvt Measured"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";

    [Test]
    procedure Measured_IntLocal_DateTime()
    var
        FR: FieldRef;
        Val: Integer;
        R: DateTime;
    begin
        Val := 1;
        Lib.Field(2, Val, FR);
        asserterror R := FR.Value;
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "1" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_IntLocal_DateTime_Max()
    var
        FR: FieldRef;
        Val: Integer;
        R: DateTime;
    begin
        Val := 2147483647;
        Lib.Field(2, Val, FR);
        asserterror R := FR.Value;
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "2147483647" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_BigLocal_Date()
    var
        FR: FieldRef;
        Val: BigInteger;
        R: Date;
    begin
        Val := 3000000000L;
        Lib.Field(3, Val, FR);
        asserterror R := FR.Value;
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime.', 'The format of the Date ''3000000000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''.');
    end;

    [Test]
    procedure Measured_IntLocal_Time()
    var
        FR: FieldRef;
        Val: Integer;
        R: Time;
    begin
        Val := 1000;
        Lib.Field(2, Val, FR);
        asserterror R := FR.Value;
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "1000" can''t be evaluated into type Time.');
    end;

    [Test]
    procedure Measured_DecLocal_DateTime()
    var
        FR: FieldRef;
        Val: Decimal;
        R: DateTime;
    begin
        Val := 1.5;
        Lib.Field(4, Val, FR);
        asserterror R := FR.Value;
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime.', 'The value "1.5" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_DurLocal_DateTime()
    var
        FR: FieldRef;
        Val: Duration;
        R: DateTime;
    begin
        Val := 65000;
        Lib.Field(5, Val, FR);
        asserterror R := FR.Value;
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime.', 'The value "1 minute 5 seconds" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_BoolLocal_DateTime()
    var
        FR: FieldRef;
        Val: Boolean;
        R: DateTime;
    begin
        Val := true;
        Lib.Field(6, Val, FR);
        asserterror R := FR.Value;
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime.', 'The value "Yes" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_IntVarParam_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(2, 1, FR);
        R := CreateDateTime(20260131D, 120000T);
        asserterror IntoVar(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "1" can''t be evaluated into type DateTime.');
        Lib.Check(Format(R, 0, 9), '2026-01-31T12:00:00Z');
    end;

    local procedure IntoVar(var FR: FieldRef; var R: DateTime)
    begin
        R := FR.Value;
    end;

    [Test]
    procedure Measured_PkFieldIndex_DateTime()
    var
        Row: Record "GK Fvt Row";
        RR: RecordRef;
        FR: FieldRef;
        R: DateTime;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        RR.GetTable(Row);
        FR := RR.FieldIndex(1);
        asserterror R := FR.Value;
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "1" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure MainKept_DateField_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(7, 20260131D, FR);
        R := FR.Value;
        Lib.Check(Format(R, 0, 9), '2026-01-31T00:00:00Z');
    end;

    [Test]
    procedure MainKept_DtField_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(8, CreateDateTime(20260131D, 120000T), FR);
        R := FR.Value;
        Lib.Check(Format(R, 0, 9), '2026-01-31T12:00:00Z');
    end;

    [Test]
    procedure MainKept_Variant_IntToDateTime()
    var
        FR: FieldRef;
        V: Variant;
        R: DateTime;
    begin
        Lib.Field(2, 1, FR);
        V := FR.Value;
        asserterror R := V;
        Lib.Check(GetLastErrorCode(), 'TypeConversion');
    end;

    [Test]
    procedure MainKept_IntLocal_Integer()
    var
        FR: FieldRef;
        R: Integer;
    begin
        Lib.Field(2, 7, FR);
        R := FR.Value;
        Lib.Check(Format(R), '7');
    end;
}
