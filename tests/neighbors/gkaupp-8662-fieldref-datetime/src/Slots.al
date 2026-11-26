// Cells of corpus PR 1522 (assumption fieldref-value-temporal-error): FieldRef.Value into a DateTime, Date or Time in a plain local, a
// by-value parameter, a named return value and a var parameter. Each raises; CheckError takes the earliest text or the later text.
codeunit 61622 "GK Fvt Slots"
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
        asserterror LocalDateTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "1" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_IntValueParam_DateTime()
    var
        FR: FieldRef;
        Val: Integer;
        R: DateTime;
    begin
        Val := 1;
        Lib.Field(2, Val, FR);
        asserterror ByValueDateTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "1" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_IntNamedReturn_DateTime()
    var
        FR: FieldRef;
        Val: Integer;
        R: DateTime;
    begin
        Val := 1;
        Lib.Field(2, Val, FR);
        asserterror NamedDateTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "1" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_IntLocal_Date()
    var
        FR: FieldRef;
        Val: Integer;
        R: Date;
    begin
        Val := 1000;
        Lib.Field(2, Val, FR);
        asserterror LocalDate(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The format of the Date ''1000'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''.');
    end;

    [Test]
    procedure Measured_BigNamedReturn_DateTime()
    var
        FR: FieldRef;
        Val: BigInteger;
        R: DateTime;
    begin
        Val := 3000000000L;
        Lib.Field(3, Val, FR);
        asserterror NamedDateTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBigInteger to System.DateTime.', 'The value "3000000000" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_NegDecVarParam_DateTime()
    var
        FR: FieldRef;
        Val: Decimal;
        R: DateTime;
    begin
        Val := -0.5;
        Lib.Field(4, Val, FR);
        asserterror VarDateTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime.', 'The value "-0.5" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_ManyDecLocal_DateTime()
    var
        FR: FieldRef;
        Val: Decimal;
        R: DateTime;
    begin
        Val := 0.123456789;
        Lib.Field(4, Val, FR);
        asserterror LocalDateTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime.', 'The value "0.123456789" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_LargeDecValueParam_DateTime()
    var
        FR: FieldRef;
        Val: Decimal;
        R: DateTime;
    begin
        Val := 1000.5;
        Lib.Field(4, Val, FR);
        asserterror ByValueDateTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime.', 'The value "1,000.5" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_ZeroDecNamedReturn_DateTime()
    var
        FR: FieldRef;
        Val: Decimal;
        R: DateTime;
    begin
        Val := 0;
        Lib.Field(4, Val, FR);
        asserterror NamedDateTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime.', 'The value "0" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_DecLocal_Date()
    var
        FR: FieldRef;
        Val: Decimal;
        R: Date;
    begin
        Val := 1.5;
        Lib.Field(4, Val, FR);
        asserterror LocalDate(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime.', 'The format of the Date ''1.5'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''.');
    end;

    [Test]
    procedure Measured_DecVarParam_Time()
    var
        FR: FieldRef;
        Val: Decimal;
        R: Time;
    begin
        Val := 999.99;
        Lib.Field(4, Val, FR);
        asserterror VarTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDecimal to System.DateTime.', 'The value "999.99" can''t be evaluated into type Time.');
    end;

    [Test]
    procedure Measured_DayDurVarParam_DateTime()
    var
        FR: FieldRef;
        Val: Duration;
        R: DateTime;
    begin
        Val := 93780000;
        Lib.Field(5, Val, FR);
        asserterror VarDateTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime.', 'The value "1 day 2 hours 3 minutes" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_MsDurVarParam_DateTime()
    var
        FR: FieldRef;
        Val: Duration;
        R: DateTime;
    begin
        Val := 65001;
        Lib.Field(5, Val, FR);
        asserterror VarDateTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime.', 'The value "1 minute 5 seconds 1 millisecond" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_ZeroDurVarParam_DateTime()
    var
        FR: FieldRef;
        Val: Duration;
        R: DateTime;
    begin
        Val := 0;
        Lib.Field(5, Val, FR);
        asserterror VarDateTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime.', 'The value "" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_DurLocal_Time()
    var
        FR: FieldRef;
        Val: Duration;
        R: Time;
    begin
        Val := 65000;
        Lib.Field(5, Val, FR);
        asserterror LocalTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDuration to System.DateTime.', 'The value "1 minute 5 seconds" can''t be evaluated into type Time.');
    end;

    [Test]
    procedure Measured_BoolNamedReturn_DateTime()
    var
        FR: FieldRef;
        Val: Boolean;
        R: DateTime;
    begin
        Val := false;
        Lib.Field(6, Val, FR);
        asserterror NamedDateTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavBoolean to System.DateTime.', 'The value "No" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_TextLocal_DateTime()
    var
        FR: FieldRef;
        Val: Text;
        R: DateTime;
    begin
        Val := '20260115';
        Lib.Field(10, Val, FR);
        asserterror LocalDateTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime.', 'The value "20260115" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_TextValueParam_DateTime()
    var
        FR: FieldRef;
        Val: Text;
        R: DateTime;
    begin
        Val := '202601151200';
        Lib.Field(10, Val, FR);
        asserterror ByValueDateTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime.', 'The value "202601151200" can''t be evaluated into type DateTime.');
    end;

    [Test]
    procedure Measured_TextLocal_Date()
    var
        FR: FieldRef;
        Val: Text;
        R: Date;
    begin
        Val := '01152026';
        Lib.Field(10, Val, FR);
        asserterror LocalDate(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime.', 'The format of the Date ''01152026'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''.');
    end;

    [Test]
    procedure Measured_CodeVarParam_Time()
    var
        FR: FieldRef;
        Val: Code[30];
        R: Time;
    begin
        Val := '120000';
        Lib.Field(11, Val, FR);
        asserterror VarTime(FR, R);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime.', 'The value "120000" can''t be evaluated into type Time.');
    end;

    [Test]
    procedure Measured_CodeNamedReturn_DateTime()
    var
        FR: FieldRef;
        Val: Code[30];
        R: DateTime;
    begin
        Val := '0';
        Lib.Field(11, Val, FR);
        asserterror NamedDateTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavCode to System.DateTime.', 'The value "0" can''t be evaluated into type DateTime.');
    end;

    local procedure ByValueDateTime(var FR: FieldRef; R: DateTime)
    begin
        R := FR.Value;
    end;

    local procedure LocalDate(var FR: FieldRef)
    var
        L: Date;
    begin
        L := FR.Value;
    end;

    local procedure LocalDateTime(var FR: FieldRef)
    var
        L: DateTime;
    begin
        L := FR.Value;
    end;

    local procedure LocalTime(var FR: FieldRef)
    var
        L: Time;
    begin
        L := FR.Value;
    end;

    local procedure NamedDateTime(var FR: FieldRef) R: DateTime
    begin
        R := FR.Value;
    end;

    local procedure VarDateTime(var FR: FieldRef; var R: DateTime)
    begin
        R := FR.Value;
    end;

    local procedure VarTime(var FR: FieldRef; var R: Time)
    begin
        R := FR.Value;
    end;

    [Test]
    procedure Measured_IntValueParam_Date()
    var
        FR: FieldRef;
        Val: Integer;
        D: Date;
    begin
        Val := 1;
        Lib.Field(2, Val, FR);
        asserterror ByValueDate(FR, D);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The format of the Date ''1'' does not match your device''s date settings.\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format ''yyyy-mm-dd''.');
    end;

    [Test]
    procedure Measured_IntNamedReturn_Time()
    var
        FR: FieldRef;
        Val: Integer;
    begin
        Val := 1;
        Lib.Field(2, Val, FR);
        asserterror NamedTime(FR);
        Lib.CheckError('Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to System.DateTime.', 'The value "1" can''t be evaluated into type Time.');
    end;

    local procedure ByValueDate(var FR: FieldRef; R: Date)
    begin
        R := FR.Value;
    end;

    local procedure NamedTime(var FR: FieldRef) R: Time
    begin
        R := FR.Value;
    end;
}
