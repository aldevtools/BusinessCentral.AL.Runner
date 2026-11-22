// Neighbors of al-runner issues 8643 and 8644: Evaluate into a Date or DateTime local or parameter of the texts corpus PR 1503
// measured (assumptions evaluate-date-variable-time-dropped, evaluate-short-date-rejected-last-error), and the targets,
// numbers, languages and texts next to them, which keep main's answer or refuse.
table 60950 "GK Date Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; D; Date) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 60950 "GK Evaluate Tests"
{
    Subtype = Test;

    var
        GlobalD: Date;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected [%1] but was [%2]', Expected, Actual);
    end;

    local procedure IntoParam(var V: Date; Txt: Text): Boolean
    begin
        exit(Evaluate(V, Txt));
    end;

    [Test]
    procedure Measured_DateLocal_TimeAfterDate()
    var
        V: Date;
    begin
        Check(Format(Evaluate(V, '01/04/25 12:00 PM')) + '|' + Format(V, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure Measured_DateParameter_TimeAfterDate()
    var
        V: Date;
    begin
        Check(Format(IntoParam(V, '01/04/25 12:00 PM')) + '|' + Format(V, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure Measured_DateLocal_OneDigitParts()
    var
        V: Date;
    begin
        Check(Format(Evaluate(V, '1/4/25 1:20 AM')) + '|' + Format(V, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure Measured_DateTimeLocal_TClock()
    var
        V: DateTime;
    begin
        Check(Format(Evaluate(V, '01/04/25T12:00')) + '|' + Format(V, 0, 9), 'Yes|2025-01-04T00:00:00Z');
    end;

    [Test]
    procedure Measured_DateLocal_RejectedLeavesLastErrorCode()
    var
        V: Date;
    begin
        ClearLastError();
        Check(Format(Evaluate(V, '13/01/25')) + '|' + GetLastErrorCode(), 'No|Evaluate');
    end;

    [Test]
    procedure MainKept_DateField_TimeAfterDate()
    var
        R: Record "GK Date Row";
    begin
        Check(Format(Evaluate(R.D, '01/04/25 12:00 PM')) + '|' + Format(R.D, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure MainKept_DateGlobal_TimeAfterDate()
    begin
        Check(Format(Evaluate(GlobalD, '01/04/25 12:00 PM')) + '|' + Format(GlobalD, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure MainKept_DateArrayElement_TimeAfterDate()
    var
        A: array[2] of Date;
    begin
        Check(Format(Evaluate(A[1], '01/04/25 12:00 PM')) + '|' + Format(A[1], 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure MainKept_DateLocal_Number1()
    var
        V: Date;
    begin
        Check(Format(Evaluate(V, '01/04/25 12:00 PM', 1)) + '|' + Format(V, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure MainKept_DateLocal_Number9()
    var
        V: Date;
    begin
        Check(Format(Evaluate(V, '01/04/25 12:00 PM', 9)), 'No');
    end;

    [Test]
    procedure MainKept_DateLocal_German()
    var
        V: Date;
    begin
        GlobalLanguage(1031);
        Check(Format(Evaluate(V, '01/04/25 12:00 PM')) + '|' + Format(V, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure MainKept_DateLocal_Hour14()
    var
        V: Date;
    begin
        Check(Format(Evaluate(V, '01/04/25 14:00 PM')) + '|' + Format(V, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure MainKept_DateLocal_OneDigitParts_Seconds()
    var
        V: Date;
    begin
        Check(Format(Evaluate(V, '1/4/25 23:59:59')) + '|' + Format(V, 0, 9), 'Yes|2025-01-04');
    end;

    [Test]
    procedure MainKept_DateLocal_DayZero()
    var
        V: Date;
    begin
        ClearLastError();
        Check(Format(Evaluate(V, '01/00/25')) + '|' + GetLastErrorCode(), 'No|');
    end;

    [Test]
    procedure MainKept_DateLocal_RejectedStatement()
    var
        V: Date;
    begin
        asserterror Evaluate(V, '13/01/25');
        Check(GetLastErrorCode(), 'Evaluate');
    end;

    [Test]
    procedure MainKept_DateTimeLocal_ShortDate()
    var
        V: DateTime;
    begin
        Check(Format(Evaluate(V, '02/01/25')) + '|' + Format(V, 0, 9), 'Yes|2025-02-01T00:00:00Z');
    end;
}

// One refused shape per codeunit: a refusal stops the rest of its codeunit.
codeunit 60962 "GK R60962"
{
    Subtype = Test;

    [Test]
    procedure Refused_DateLocal_RejectedLastErrorText()
    var
        V: Date;
    begin
        ClearLastError();
        if Evaluate(V, '13/01/25') then;
        Error('last error text [%1]', GetLastErrorText());
    end;
}

codeunit 60963 "GK R60963"
{
    Subtype = Test;

    [Test]
    procedure Refused_DateTimeLocal_Hour13Statement()
    var
        V: DateTime;
    begin
        Evaluate(V, '01/04/25 13:00 PM');
    end;
}
