// Neighbors of al-runner issue 8832 just past the cells of GetFieldByControlId.al (corpus pull request 1538): a call on what
// GetField found that no cell measures, Caption of what a part control's id gives, and GetField on a Worksheet page. Each test asserts what Business Central 29.1 answered (3 runs).

codeunit 97711 "LT GFN Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Initialize()
    var
        Row: Record "LT GFI Row";
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Note := 'n1';
        Row.Insert();
        Commit();
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Actual <> Expected then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;

    // Activate on what GetField found: no cell calls it.
    [Test]
    procedure ActivateThroughGetField()
    var
        Card: TestPage "LT GFI Mod Card";
    begin
        Initialize();
        Card.OpenEdit();
        Card.GetField(1331314946).Activate();
        Check('value', 'Alpha', Card.Name.Value());
        Card.Close();
    end;

    // Caption of what the part control's id gives: the part page's caption.
    [Test]
    procedure PartIdCaption()
    var
        Card: TestPage "LT GFI Card";
    begin
        Initialize();
        Card.OpenView();
        Check('caption', 'LT GFI Lines Part', Card.GetField(2105193259).Caption());
        Card.Close();
    end;

    // GetField by the id of the Name control of a Worksheet page (97685): no cell opens a Worksheet.
    [Test]
    procedure WorksheetGetField()
    var
        Sheet: TestPage "LT GFN Sheet";
    begin
        Initialize();
        Sheet.OpenView();
        Sheet.First();
        Check('value', 'Alpha', Sheet.GetField(2044414625).Value());
        Sheet.Close();
    end;
}

page 97685 "LT GFN Sheet"
{
    PageType = Worksheet;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }
}
