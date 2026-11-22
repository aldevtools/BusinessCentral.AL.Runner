codeunit 61630 "GK Fvt R61630"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_Global_DateTime()
    var
        FR: FieldRef;
        R: DateTime;

    begin
        Lib.Field(2, 1, FR);
        asserterror G := FR.Value;
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
}

codeunit 61631 "GK Fvt R61631"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_Field_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
        Row: Record "GK Fvt Row";
    begin
        Lib.Field(2, 1, FR);
        asserterror Row.DtF := FR.Value;
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
}

codeunit 61632 "GK Fvt R61632"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_Argument_DateTime()
    var
        FR: FieldRef;
        R: DateTime;

    begin
        Lib.Field(2, 1, FR);
        asserterror Take(FR.Value);
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
}

codeunit 61633 "GK Fvt R61633"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_Exit_DateTime()
    var
        FR: FieldRef;
        R: DateTime;

    begin
        Lib.Field(2, 1, FR);
        asserterror R := Give(FR);
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
}

codeunit 61634 "GK Fvt R61634"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_ArrayElement_DateTime()
    var
        FR: FieldRef;
        R: DateTime;
        A: array[2] of DateTime;
    begin
        Lib.Field(2, 1, FR);
        asserterror A[1] := FR.Value;
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
}

codeunit 61635 "GK Fvt R61635"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_WholeDecimal_DateTime()
    var
        FR: FieldRef;
        R: DateTime;

    begin
        Lib.Field(4, 2, FR);
        asserterror R := FR.Value;
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
}

codeunit 61636 "GK Fvt R61636"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_LargeDecimal_DateTime()
    var
        FR: FieldRef;
        R: DateTime;

    begin
        Lib.Field(4, 1500.5, FR);
        asserterror R := FR.Value;
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
}

codeunit 61637 "GK Fvt R61637"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_ZeroDuration_DateTime()
    var
        FR: FieldRef;
        R: DateTime;

    begin
        Lib.Field(5, 0, FR);
        asserterror R := FR.Value;
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
}

codeunit 61638 "GK Fvt R61638"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_DecLocal_Date()
    var
        FR: FieldRef;
        R: DateTime;
        D: Date;
    begin
        Lib.Field(4, 1.5, FR);
        asserterror D := FR.Value;
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
}

codeunit 61639 "GK Fvt R61639"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_BoolLocal_Time()
    var
        FR: FieldRef;
        R: DateTime;
        T: Time;
    begin
        Lib.Field(6, true, FR);
        asserterror T := FR.Value;
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
}

codeunit 61640 "GK Fvt R61640"
{
    Subtype = Test;

    var
        Lib: Codeunit "GK Fvt Lib";
        G: DateTime;

    [Test]
    procedure Refused_OptionField_DateTime()
    var
        FR: FieldRef;
        R: DateTime;

    begin
        Lib.Field(9, 1, FR);
        asserterror R := FR.Value;
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
}

