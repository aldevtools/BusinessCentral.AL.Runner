enum 97714 "LT CFN Kind"
{
    Extensible = false;

    value(0; Alpha) { }
    value(1; Beta) { }
}

table 97713 "LT CFN Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; GuidValue; Guid) { }
        field(3; KindValue; Enum "LT CFN Kind") { }
        field(4; SpanValue; Duration) { }
        field(5; CodeValue; Code[20]) { }
        field(6; IntValue; Integer) { }
        field(7; FormulaValue; DateFormula) { }
        field(8; FlagValue; Boolean) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 97715 "LT CFN Card"
{
    PageType = Card;
    SourceTable = "LT CFN Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            cuegroup(Cues)
            {
                field(CueGuid; Rec.GuidValue) { }
                field(CueKind; Rec.KindValue) { }
                field(CueSpan; Rec.SpanValue) { }
                field(CueFormula; Rec.FormulaValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideCode; Rec.CodeValue) { }
                field(WideInt; Rec.IntValue) { }
                field(WideFlag; Rec.FlagValue) { }
            }
        }
    }
}

// Neighbors of al-runner issue 8845 just past the cells of GetFieldByControlId.al (corpus pull request 1538, CueTypes_ValueByNameAndById):
// a DateFormula field in a cuegroup, and a Boolean field in a cuegroup with CuegroupLayout = Wide.
// Each test asserts what Business Central 29.1 answered (3 runs).
codeunit 97713 "LT CFN Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Row: Record "LT CFN Row";
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.GuidValue := '{0F2E7C52-6E0B-4F51-9C3A-1B2C3D4E5F60}';
        Row.KindValue := Row.KindValue::Beta;
        Row.SpanValue := 90000;
        Row.CodeValue := 'R1';
        Row.IntValue := 5;
        Evaluate(Row.FormulaValue, '<1D>');
        Row.FlagValue := true;
        Row.Insert();
        Clear(Seen);
    end;

    local procedure Note(What: Text; Actual: Text)
    begin
        Seen.Set(What, Actual);
    end;

    local procedure Expect(What: Text; Expected: Text)
    begin
        Assert.AreEqual(Expected, Seen.Get(What), What);
    end;

    [Test]
    procedure CueDateFormula_Value()
    var
        Card: TestPage "LT CFN Card";
    begin
        Initialize();
        Card.OpenView();
        Note('value', Card.CueFormula.Value());
        Card.Close();
        Expect('value', '0');
    end;

    [Test]
    procedure WideCueBoolean_Value()
    var
        Card: TestPage "LT CFN Card";
    begin
        Initialize();
        Card.OpenView();
        Note('value', Card.WideFlag.Value());
        Card.Close();
        Expect('value', '0');
    end;
}
