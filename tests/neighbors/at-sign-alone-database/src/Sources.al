// Neighbors of a CalcFormula's filter('@') over a source field that is neither Code nor Text, past the cells: AtSignAloneMethods.al
// measures a sum over a Date, Integer, Decimal, Boolean and an Option field without captions, read by any reader, in a session of
// language 1033 (:395, :430, :454, :657, :717). A count over a Date field, an Option field with an OptionCaption, a Time field and
// the Integer error in a session of language 1031 are not measured; the runner refuses them, so these tests assert nothing.
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
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 97538 "AN Source Neighbor Tests"
{
    Subtype = Test;

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
    end;

    [Test]
    procedure AtSignAlone_SumOverCaptionedOptionSource()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        Host.CalcFields("Captioned Opt At");
    end;

    [Test]
    procedure AtSignAlone_SumOverTimeSource()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        Host.CalcFields("Time At");
    end;

    // The error of a sum over an Integer source field is measured in a session of language 1033 only
    // (AtSignAloneMethods.al:430, :454); in another language the runner refuses it.
    [Test]
    procedure AtSignAlone_SumOverIntegerSource_OtherLanguage()
    var
        Host: Record "AN Source Host";
    begin
        NewHost(Host);
        GlobalLanguage(1031);
        asserterror Host.CalcFields("Num At");
        GlobalLanguage(1033);
    end;
}
