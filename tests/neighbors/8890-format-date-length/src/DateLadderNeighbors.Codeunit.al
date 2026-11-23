codeunit 50808 "Date Ladder Neighbors"
{
    Subtype = Test;

    var
        DayMonthYear2Tok: Label '<Day,2><Month,2><Year,2>', Locked = true;
        HoursMinutesTok: Label '<Hours24,2>:<Minutes,2>', Locked = true;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected [%1], got [%2]', Expected, Actual);
    end;

    local procedure Touch(Actual: Text)
    begin
    end;

    // Cells of the pinned codeunit 96391 (FormatLengthStrings.al): clean on head.
    [Test]
    procedure Date_DayMonthYear2_AsLong()
    begin
        Check(Format(20240115D, 6, DayMonthYear2Tok), '150124');
    end;

    [Test]
    procedure ClosingDate_DayMonthYear2_NegativeCut()
    begin
        Check(Format(ClosingDate(20240115D), -4, DayMonthYear2Tok), '1501');
    end;

    [Test]
    procedure Date_StandardFormat0_One()
    begin
        Check(Format(20240115D, 1, '<Standard Format,0>'), '0');
    end;

    [Test]
    procedure Date_StandardFormat9_LongerPositive()
    begin
        Check(Format(20240115D, 13, '<Standard Format,9>'), '   2024-01-15');
    end;

    [Test]
    procedure UndefinedDate_StandardFormat0_Positive()
    begin
        Check(Format(0D, 8, '<Standard Format,0>'), '        ');
    end;

    [Test]
    procedure UndefinedDateTime_DayMonthYear2_Negative()
    begin
        Check(Format(0DT, -30, DayMonthYear2Tok), '');
    end;

    [Test]
    procedure Date_Number0_AsLong_Negative()
    begin
        Check(Format(20240115D, -8, 0), '01/15/24');
    end;

    [Test]
    procedure Duration_StandardFormat0_AsLong()
    var
        Dur: Duration;
    begin
        Dur := 3723004;
        Check(Format(Dur, StrLen(Format(Dur, 0, 0)), '<Standard Format,0>'), Format(Dur, 0, 0));
    end;

    [Test]
    procedure UndefinedDate_HoursMinutes_Error()
    begin
        asserterror Touch(Format(0D, 5, HoursMinutesTok));
        Check(GetLastErrorCode(), 'InvalidFormatString');
    end;

    // Neighbors no line measures: refused on main and head. No expected text: none is measured.
    [Test]
    procedure Date_DayMonthYear2_UnmeasuredLength()
    begin
        Touch(Format(20240115D, 2, DayMonthYear2Tok));
    end;

    [Test]
    procedure Date_StandardFormat0_UnmeasuredLength()
    begin
        Touch(Format(20240115D, 4, '<Standard Format,0>'));
    end;

    [Test]
    procedure UndefinedDateTime_DayMonthYear2_UnmeasuredLength()
    begin
        Touch(Format(0DT, 2, DayMonthYear2Tok));
    end;

    [Test]
    procedure ClosingDay_OneDigitDay()
    begin
        Touch(Format(20240105D, 1, '<Closing><Day>'));
    end;

    [Test]
    procedure ClosingDate_Year4Month_Cut()
    begin
        Touch(Format(ClosingDate(20240115D), 5, '<Year4>-<Month,2>'));
    end;

    [Test]
    procedure DateTime_DayMonthYear2()
    begin
        Touch(Format(CreateDateTime(20240115D, 093005T), 6, DayMonthYear2Tok));
    end;

    [Test]
    procedure Date_HoursMinutes()
    begin
        Touch(Format(20240115D, 5, HoursMinutesTok));
    end;

    [Test]
    procedure UndefinedDate_Hours24Alone()
    begin
        Touch(Format(0D, 0, '<Hours24,2>'));
    end;

    [Test]
    procedure Date_Number1_AsLong()
    begin
        Touch(Format(20240115D, 8, 1));
    end;
}
