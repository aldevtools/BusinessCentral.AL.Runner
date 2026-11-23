// Neighbors just past the cue cells of GetFieldByControlId.al :2116-:2216 (corpus pull request 1538): a CardPart shown by a
// Document page instead of a Card, and a DateFormula field and a Boolean field in a wide cuegroup on a List instead of a Card.
// Each test asserts what Business Central 29.1 answered (3 runs). In a codeunit of its own: a refused test stops the rest of
// its codeunit.

page 97720 "LT CFN List"
{
    PageType = List;
    SourceTable = "LT CFN Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { }
            }
            cuegroup(Cues)
            {
                field(CueFormula; Rec.FormulaValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideFlag; Rec.FlagValue) { }
            }
        }
    }
}

page 97721 "LT CFN Doc Host"
{
    PageType = Document;
    SourceTable = "LT CFN Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            part(CuePart; "LT CFN Part")
            {
                SubPageLink = "No." = field("No.");
            }
        }
    }
}

codeunit 97722 "LT CFN More Tests"
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
    procedure DocHostPartCueCode_Value()
    var
        Doc: TestPage "LT CFN Doc Host";
    begin
        Initialize();
        Doc.OpenView();
        Note('value', Doc.CuePart.CueCode.Value());
        Doc.Close();
        Expect('value', '0');
    end;

    [Test]
    procedure ListCueDateFormula_Value()
    var
        Pg: TestPage "LT CFN List";
    begin
        Initialize();
        Pg.OpenView();
        Note('value', Pg.CueFormula.Value());
        Pg.Close();
        Expect('value', '0');
    end;

    [Test]
    procedure ListWideCueBoolean_Value()
    var
        Pg: TestPage "LT CFN List";
    begin
        Initialize();
        Pg.OpenView();
        Note('value', Pg.WideFlag.Value());
        Pg.Close();
        Expect('value', '0');
    end;
}
