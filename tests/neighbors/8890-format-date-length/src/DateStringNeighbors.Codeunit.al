codeunit 50806 "Date String Neighbors"
{
    Subtype = Test;

    var
        DateCompactTok: Label '<Year4><Month,2><Day,2>', Locked = true;
        TimeCompactTok: Label '<Hours24,2><Minutes,2><Seconds,2>', Locked = true;
        DateTimeCompactTok: Label '<Year4><Month,2><Day,2><Hours24,2><Minutes,2><Seconds,2>', Locked = true;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected [%1], got [%2]', Expected, Actual);
    end;

    local procedure Touch(Actual: Text)
    begin
    end;

    // Measured cells (corpus PR 1557): each runs through the assumption.
    [Test]
    procedure Date_Compact_LongerPositive()
    begin
        Check(Format(20240105D, 10, DateCompactTok), '  20240105');
    end;

    [Test]
    procedure Date_Compact_NegativeCut()
    begin
        Check(Format(20240105D, -3, DateCompactTok), '202');
    end;

    [Test]
    procedure Date_Compact_NegativeLonger()
    begin
        Check(Format(20240105D, -10, DateCompactTok), '20240105');
    end;

    [Test]
    procedure ClosingDate_Compact_AsLong()
    begin
        Check(Format(ClosingDate(20240105D), 8, DateCompactTok), '20240105');
    end;

    [Test]
    procedure UndefinedDate_Compact_Positive()
    begin
        Check(Format(0D, 8, DateCompactTok), '        ');
    end;

    [Test]
    procedure Date_Dots_Cut()
    begin
        Check(Format(20240105D, 8, '<Day,2>.<Month,2>.<Year4>'), '05.01.20');
    end;

    [Test]
    procedure Date_Iso_NegativeCut()
    begin
        Check(Format(20240105D, -9, '<Year4>-<Month,2>-<Day,2>'), '2024-01-0');
    end;

    [Test]
    procedure Time_Compact_NegativeCut()
    begin
        Check(Format(093005T, -4, TimeCompactTok), ' 930');
    end;

    [Test]
    procedure UndefinedTime_Compact_Cut()
    begin
        Check(Format(0T, 3, TimeCompactTok), ' 00');
    end;

    [Test]
    procedure DateTime_Compact_Cut()
    begin
        Check(Format(CreateDateTime(20240105D, 093005T), 8, DateTimeCompactTok), '20240105');
    end;

    [Test]
    procedure UndefinedDateTime_Compact_Negative()
    begin
        Check(Format(0DT, -3, DateTimeCompactTok), '');
    end;

    // Other readers of the same gate: a Variant, a table field and a FieldRef holding the Date.
    [Test]
    procedure Variant_Date_Compact()
    var
        V: Variant;
    begin
        V := 20240105D;
        Check(Format(V, 8, DateCompactTok), '20240105');
    end;

    [Test]
    procedure Field_Date_Compact()
    var
        Rec: Record "Repro Date Holder";
    begin
        Rec."Some Date" := 20240105D;
        Check(Format(Rec."Some Date", 8, DateCompactTok), '20240105');
    end;

    [Test]
    procedure FieldRef_Date_Compact()
    var
        Rec: Record "Repro Date Holder";
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        Rec."Some Date" := 20240105D;
        RecRef.GetTable(Rec);
        FldRef := RecRef.Field(Rec.FieldNo("Some Date"));
        Check(Format(FldRef, 8, DateCompactTok), '20240105');
    end;

    // The lone <Standard Format,0> takes the new negative cut (FormatLengthNeighbors.al:312).
    [Test]
    procedure StandardFormat0_Date_NegativeCut()
    begin
        Check(Format(20240115D, -3, '<Standard Format,0>'), '01/');
    end;

    [Test]
    procedure StandardFormat0_Time_NegativeCut()
    begin
        Check(Format(134501T, -3, '<Standard Format,0>'), ' 1:');
    end;

    // Neighbors the probe did not measure: refused on main and head. No expected text: none is measured.
    [Test]
    procedure Date_UnmeasuredString()
    begin
        Touch(Format(20240105D, 8, '<Day,2><Month,2><Year4>'));
    end;

    [Test]
    procedure ClosingDate_Compact_NegativeCut()
    begin
        Touch(Format(ClosingDate(20240105D), -3, DateCompactTok));
    end;

    [Test]
    procedure UndefinedDate_Dots()
    begin
        Touch(Format(0D, 8, '<Day,2>.<Month,2>.<Year4>'));
    end;

    [Test]
    procedure UndefinedTime_Compact_NegativeCut()
    begin
        Touch(Format(0T, -3, TimeCompactTok));
    end;

    [Test]
    procedure DateTime_DateOnlyString()
    begin
        Touch(Format(CreateDateTime(20240105D, 093005T), 8, DateCompactTok));
    end;

    [Test]
    procedure Time_SecondsColonString_Negative()
    begin
        Touch(Format(093005T, -3, '<Hours24,2>:<Minutes,2>:<Seconds,2>'));
    end;
}
