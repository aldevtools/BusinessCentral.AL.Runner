codeunit 61630 "GK Fvt Refused"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_Global_DateTime()
    var
        FR: FieldRef;
    begin
        Lib.Field(2, 1, FR);
        asserterror G := FR.Value;
        Refused();
    end;

    [Test]
    procedure Refused_Field_DateTime()
    var
        FR: FieldRef;
        Row: Record "GK Fvt Row";
    begin
        Lib.Field(2, 1, FR);
        asserterror Row.DtF := FR.Value;
        Refused();
    end;

    [Test]
    procedure Refused_Argument_DateTime()
    var
        FR: FieldRef;
    begin
        Lib.Field(2, 1, FR);
        asserterror Take(FR.Value);
        Refused();
    end;

    [Test]
    procedure Refused_Exit_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(2, 1, FR);
        asserterror R := Give(FR);
        Refused();
    end;

    [Test]
    procedure Refused_ArrayElement_DateTime()
    var
        FR: FieldRef;
        A: array[2] of DateTime;
    begin
        Lib.Field(2, 1, FR);
        asserterror A[1] := FR.Value;
        Refused();
    end;

    [Test]
    procedure Refused_WholeDecimal_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(4, 2, FR);
        asserterror IntoVar(FR, R);
        Refused();
    end;

    [Test]
    procedure Refused_LargeDecimal_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(4, 1500.5, FR);
        asserterror IntoVar(FR, R);
        Refused();
    end;

    [Test]
    procedure Refused_BoolVarParam_Time()
    var
        FR: FieldRef;
        T: Time;
    begin
        Lib.Field(6, true, FR);
        asserterror IntoVar(FR, T);
        Refused();
    end;

    [Test]
    procedure Refused_OptionField_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(9, 1, FR);
        asserterror IntoVar(FR, R);
        Refused();
    end;

    [Test]
    procedure Refused_IntLocal_OtherValue()
    var
        FR: FieldRef;
    begin
        Lib.Field(2, 2, FR);
        asserterror LocalDateTime(FR);
        Refused();
    end;

    [Test]
    procedure Refused_BigLocal_Date()
    var
        FR: FieldRef;
    begin
        Lib.Field(3, 1, FR);
        asserterror LocalDate(FR);
        Refused();
    end;

    [Test]
    procedure Refused_BoolLocal_Date()
    var
        FR: FieldRef;
    begin
        Lib.Field(6, true, FR);
        asserterror LocalDate(FR);
        Refused();
    end;

    [Test]
    procedure Refused_TextLocal_OtherDigits()
    var
        FR: FieldRef;
    begin
        Lib.Field(10, '2026', FR);
        asserterror LocalDateTime(FR);
        Refused();
    end;

    [Test]
    procedure Refused_DecLocal_OtherValue_Date()
    var
        FR: FieldRef;
    begin
        Lib.Field(4, 0.5, FR);
        asserterror LocalDate(FR);
        Refused();
    end;

    [Test]
    procedure Refused_DurLocal_OtherValue_Time()
    var
        FR: FieldRef;
    begin
        Lib.Field(5, 2000, FR);
        asserterror LocalTime(FR);
        Refused();
    end;

    [Test]
    procedure Refused_IntLocal_Language1031()
    var
        FR: FieldRef;
    begin
        Lib.Field(2, 1, FR);
        GlobalLanguage(1031);
        asserterror LocalDateTime(FR);
        GlobalLanguage(1033);
        Refused();
    end;

    [Test]
    procedure Refused_DecLocal_Language1031()
    var
        FR: FieldRef;
    begin
        Lib.Field(4, 1.5, FR);
        GlobalLanguage(1031);
        asserterror LocalDateTime(FR);
        GlobalLanguage(1033);
        Refused();
    end;

    [Test]
    procedure Refused_IntVarParam_Language1031()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(2, 1, FR);
        GlobalLanguage(1031);
        asserterror IntoVar(FR, R);
        GlobalLanguage(1033);
        Refused();
    end;

    [Test]
    procedure Refused_BigVarParam_Language1031()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(3, 1, FR);
        GlobalLanguage(1031);
        asserterror IntoVar(FR, R);
        GlobalLanguage(1033);
        Refused();
    end;

    [Test]
    procedure Refused_DecVarParam_Language1031()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(4, 1.5, FR);
        GlobalLanguage(1031);
        asserterror IntoVar(FR, R);
        GlobalLanguage(1033);
        Refused();
    end;

    [Test]
    procedure Refused_BoolVarParam_Language1031()
    var
        FR: FieldRef;
        R: DateTime;
    begin
        Lib.Field(6, true, FR);
        GlobalLanguage(1031);
        asserterror IntoVar(FR, R);
        GlobalLanguage(1033);
        Refused();
    end;

    [Test]
    procedure Refused_TriggerLocal_DateTime()
    var
        FR: FieldRef;
    begin
        Lib.Field(2, 1, FR);
        asserterror Codeunit.Run(Codeunit::"GK Fvt Trigger Local");
        Refused();
    end;

    local procedure Refused()
    begin
        if GetLastErrorCode() = '' then
            Error('no error');
    end;

    local procedure Take(P: DateTime)
    begin
        G := P;
    end;

    local procedure Give(var FR: FieldRef): DateTime
    begin
        exit(FR.Value);
    end;

    local procedure IntoVar(var FR: FieldRef; var R: DateTime)
    begin
        R := FR.Value;
    end;

    local procedure IntoVar(var FR: FieldRef; var R: Date)
    begin
        R := FR.Value;
    end;

    local procedure IntoVar(var FR: FieldRef; var R: Time)
    begin
        R := FR.Value;
    end;

    local procedure LocalDateTime(var FR: FieldRef)
    var
        L: DateTime;
    begin
        L := FR.Value;
    end;

    local procedure LocalDate(var FR: FieldRef)
    var
        L: Date;
    begin
        L := FR.Value;
    end;

    local procedure LocalTime(var FR: FieldRef)
    var
        L: Time;
    begin
        L := FR.Value;
    end;
}

// A plain local of a trigger: the probes measured procedures only.
codeunit 61631 "GK Fvt Trigger Local"
{
    trigger OnRun()
    var
        FR: FieldRef;
        Row: Record "GK Fvt Row";
        RR: RecordRef;
        L: DateTime;
    begin
        RR.GetTable(Row);
        FR := RR.Field(2);
        L := FR.Value;
    end;
}
