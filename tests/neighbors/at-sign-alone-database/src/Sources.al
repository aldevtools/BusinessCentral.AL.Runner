// Neighbors of a CalcFormula's filter('@') over a source field that is neither Code nor Text, next to the cells of
// AtSignAloneTypes.al (every method over a Date, Integer, Decimal, Boolean, Option, Time, DateTime, Duration, BigInteger, Guid and
// Enum field, with and without captions, in languages 1033, 1031 and 1030: :651-:974). A count over a Date field, an Option field
// with an OptionCaption, a Time field and the Integer error in language 1031 are answered there and run as BC answered them. A
// DateFormula field and the error in language 1036 are not measured; the runner refuses them, so those tests assert nothing.
table 97508 "AN Source Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "Group"; Code[20]) { }
        field(2; Id; Integer) { }
        field(3; Day; Date) { }
        field(4; Opt; Option) { OptionMembers = A,B; OptionCaption = 'First,Second'; }
        field(5; At; Time) { }
        field(6; Amount; Decimal) { }
        field(7; Num; Integer) { }
        field(8; Formula; DateFormula) { }
    }

    keys
    {
        key(PK; "Group", Id) { Clustered = true; }
    }
}

table 97509 "AN Source Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Count Day At"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("AN Source Row" where("Group" = field("No."), Day = filter('@')));
        }
        field(3; "Captioned Opt At"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AN Source Row".Amount where("Group" = field("No."), Opt = filter('@')));
        }
        field(4; "Time At"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AN Source Row".Amount where("Group" = field("No."), At = filter('@')));
        }
        field(5; "Num At"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AN Source Row".Amount where("Group" = field("No."), Num = filter('@')));
        }
        field(6; "Formula At"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("AN Source Row".Amount where("Group" = field("No."), Formula = filter('@')));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 97538 "AN Source Neighbor Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewHost(var Host: Record "AN Source Host")
    var
        Row: Record "AN Source Row";
    begin
        Row."Group" := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Row.Id := 1;
        Row.Day := 20260101D;
        Row.Amount := 10;
        Row.Insert();
        Host."No." := Row."Group";
        Host.Insert();
    end;

    [Test]
    procedure AtSignAlone_CountOverDateSource()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        Host.CalcFields("Count Day At");
        Assert.AreEqual(0, Host."Count Day At", 'count @ over a Date field (AtSignAloneTypes.al:651)');
    end;

    [Test]
    procedure AtSignAlone_SumOverCaptionedOptionSource()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Captioned Opt At");
        Assert.AreEqual('''@'' is not an option. The existing options are: First,Second', GetLastErrorText(), 'captions (AtSignAloneTypes.al:761)');
    end;

    [Test]
    procedure AtSignAlone_SumOverTimeSource()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Time At");
        Assert.AreEqual('The value "@" can''t be evaluated into type Time.', GetLastErrorText(), 'time (AtSignAloneTypes.al:785)');
    end;

    // The error of a sum over an Integer source field in a session of language 1031 is the text of 1033 (AtSignAloneTypes.al:935).
    [Test]
    procedure AtSignAlone_SumOverIntegerSource_OtherLanguage()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        GlobalLanguage(1031);
        asserterror Host.CalcFields("Num At");
        GlobalLanguage(1033);
        Assert.AreEqual('The value "@" can''t be evaluated into type Integer.', GetLastErrorText(), 'language 1031');
    end;

    // Language 1036 is not measured (AtSignAloneTypes.al measures 1033, 1031 and 1030); the runner refuses the error.
    [Test]
    procedure AtSignAlone_SumOverIntegerSource_Language1036()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        GlobalLanguage(1036);
        asserterror Host.CalcFields("Num At");
        GlobalLanguage(1033);
    end;

    // No cell measures a DateFormula source field; the lowering refuses the FlowField.
    [Test]
    procedure AtSignAlone_SumOverDateFormulaSource()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        Host.CalcFields("Formula At");
    end;
}
