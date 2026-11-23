codeunit 50809 "Fix Round Neighbors"
{
    Subtype = Test;

    var
        IsoTok: Label '<Year4>-<Month,2>-<Day,2>', Locked = true;
        Sf0Tok: Label '<Standard Format,0>', Locked = true;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected [%1], got [%2]', Expected, Actual);
    end;

    // A closing Date under the ISO string, as long as the length.
    [Test]
    procedure ClosingDate_Iso_AsLong()
    begin
        Check(Format(ClosingDate(20240115D), 10, IsoTok), '2024-01-15');
    end;

    // A closing Date cut by a negative length under a lone <Standard Format,0>.
    [Test]
    procedure StandardFormat0_ClosingDate_NegativeCut()
    begin
        Check(Format(ClosingDate(20240115D), -3, Sf0Tok), 'C01');
    end;

    // A defined Time in standard format 0: each caller form and fit.
    [Test]
    procedure Time_Plain_NegativeCut()
    begin
        Check(Format(134506T, -3), ' 1:');
    end;

    [Test]
    procedure Time_Number0_LongerPositive()
    begin
        Check(Format(134506T, 16, 0), '      1:45:06 PM');
    end;

    [Test]
    procedure Time_StandardFormat0_AsLong()
    begin
        Check(Format(134506T, 11, Sf0Tok), ' 1:45:06 PM');
    end;

    [Test]
    procedure Time_StandardFormat0_NegativeLonger()
    begin
        Check(Format(134506T, -16, Sf0Tok), ' 1:45:06 PM');
    end;

    // Length -10: measured only under the string; the number forms refuse.
    [Test]
    procedure Time_StandardFormat0_NegativeCut10()
    begin
        Check(Format(134506T, -10, Sf0Tok), ' 1:45:06 P');
    end;

    [Test]
    procedure Time_Number0_NegativeCut10()
    begin
        Check(Format(134506T, -10, 0), ' 1:45:06 P');
    end;

    [Test]
    procedure Time_Plain_NegativeCut10()
    begin
        Check(Format(134506T, -10), ' 1:45:06 P');
    end;

    // Neighbors no line measures: another length, the undefined Time, another standard format.
    [Test]
    procedure Time_Number0_Length12()
    begin
        Check(Format(134506T, 12, 0), '  1:45:06 PM');
    end;

    [Test]
    procedure Time_StandardFormat0_Length30()
    begin
        Check(Format(134506T, -30, Sf0Tok), ' 1:45:06 PM');
    end;

    [Test]
    procedure UndefinedTime_Number0_Length3()
    begin
        Check(Format(0T, 3, 0), '   ');
    end;

    [Test]
    procedure Time_Number1_Length16()
    begin
        Check(Format(134506T, 16, 1), '      1:45:06 PM');
    end;

    // Under CurrReport.FormatRegion: no format string with a length is measured.
    [Test]
    procedure Region_StandardFormat0_Length8()
    begin
        Check(RunRegion(1), '15.01.24');
    end;

    [Test]
    procedure Region_Iso_Length10()
    begin
        Check(RunRegion(2), '2024-01-15');
    end;

    [Test]
    procedure Region_Compact_Length8()
    begin
        Check(RunRegion(3), '20240115');
    end;

    [Test]
    procedure Region_Time_StandardFormat0_Length11()
    begin
        // Refusal only: no line measures the text.
        RunRegion(4);
    end;

    local procedure RunRegion(Mode: Integer): Text
    var
        Log: Codeunit "Fix Round Region Log";
    begin
        Log.SetMode(Mode);
        Report.Run(Report::"Fix Round Region");
        exit(Log.Take());
    end;
}
