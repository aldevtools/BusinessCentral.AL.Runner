// Review round 1: direct CalcFields of every new type/method cell, and the other readers AtSignAloneMethods.al:657 names.
enum 97601 "AS Review Kind"
{
    Extensible = false;
    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}

table 97601 "AS Review Reader Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal) { }
        field(3; Tm; Time) { }
        field(4; Uid; Guid) { }
        field(5; Opt; Option) { OptionMembers = A,B; OptionCaption = 'Alpha,Beta'; }
        field(6; Kind; Enum "AS Review Kind") { }
    }
    keys { key(PK; "No.") { } }
}

table 97602 "AS Review Reader Host"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Time Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Reader Row" where("No." = field("No."), Tm = filter('@'))); }
        field(3; "Time Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Reader Row" where("No." = field("No."), Tm = filter('<>@'))); }
        field(4; "Time Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AS Review Reader Row".Amount where("No." = field("No."), Tm = filter('@'))); }
        field(5; "Guid Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Reader Row" where("No." = field("No."), Uid = filter('@'))); }
        field(6; "Option Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Reader Row" where("No." = field("No."), Opt = filter('@'))); }
        field(7; "Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Reader Row" where("No." = field("No."), Kind = filter('@'))); }
    }
    keys { key(PK; "No.") { } }
}

query 97601 "AS Review Reader Time"
{
    elements
    {
        dataitem(Host; "AS Review Reader Host") { column("No"; "No.") { } column(TimeAt; "Time Count At") { } }
    }
}

codeunit 97601 "AS Review Reader Tests"
{
    Subtype = Test;
    TestIsolation = Function;

    local procedure NewHost(var Host: Record "AS Review Reader Host"): Code[20]
    var
        Row: Record "AS Review Reader Row";
    begin
        Row."No." := 'R' + CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Row.Amount := 10;
        Row.Insert();
        Host."No." := Row."No.";
        Host.Insert();
        exit(Row."No.");
    end;

    [Test]
    procedure Readers_CalcFields_NewShapes()
    var
        Host: Record "AS Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Time Count At");
        asserterror Host.CalcFields("Guid Count At");
        asserterror Host.CalcFields("Option Count At");
        asserterror Host.CalcFields("Enum Count At");
    end;

    [Test]
    procedure Readers_SetAutoCalcFields_TimeCount()
    var
        Host: Record "AS Review Reader Host";
        G: Code[20];
    begin
        G := NewHost(Host);
        Host.SetAutoCalcFields("Time Count At");
        Host.SetRange("No.", G);
        Host.FindFirst();
    end;

    [Test]
    procedure Readers_FilterOnFlowField_TimeCount()
    var
        Host: Record "AS Review Reader Host";
        G: Code[20];
    begin
        G := NewHost(Host);
        Host.SetRange("No.", G);
        Host.SetRange("Time Count At", 0);
        Host.Count();
    end;

    [Test]
    procedure Readers_Query_TimeCount()
    var
        Host: Record "AS Review Reader Host";
        Q: Query "AS Review Reader Time";
    begin
        NewHost(Host);
        Q.Open();
        Q.Read();
        Q.Close();
    end;

    [Test]
    procedure Readers_TemporaryHost_TimeCount()
    var
        Temp: Record "AS Review Reader Host" temporary;
    begin
        Temp."No." := 'TEMP';
        Temp.Insert();
        asserterror Temp.CalcFields("Time Count At");
    end;

    [Test]
    procedure Readers_Language1031_SumAt()
    var
        Host: Record "AS Review Reader Host";
    begin
        NewHost(Host);
        GlobalLanguage(1031);
        asserterror Host.CalcFields("Time Sum At");
    end;

    [Test]
    procedure Readers_Language1031_CountNotAt()
    var
        Host: Record "AS Review Reader Host";
    begin
        NewHost(Host);
        GlobalLanguage(1031);
        asserterror Host.CalcFields("Time Count Not At");
    end;

    [Test]
    procedure Readers_Language1030_SumAt()
    var
        Host: Record "AS Review Reader Host";
    begin
        NewHost(Host);
        GlobalLanguage(1030);
        asserterror Host.CalcFields("Time Sum At");
    end;

    [Test]
    procedure Readers_Language1030_CountNotAt()
    var
        Host: Record "AS Review Reader Host";
    begin
        NewHost(Host);
        GlobalLanguage(1030);
        asserterror Host.CalcFields("Time Count Not At");
    end;
}
