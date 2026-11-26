// Neighbors of al-runner issues 8832 and 8887 just past the cells of GetFieldByControlId.al (corpus pull request 1538): a call on
// what GetField found that no cell measures, Caption of what a part control's id gives, GetField by a control's own id on a
// Worksheet, a ListPlus and a CardPart behind a TestPart, and on a Worksheet the id of another page's control. Each test asserts
// what Business Central 29.1 answered (3 runs).

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

    // GetField by the id of the Name control of a Worksheet page (97692): no cell finds a control on a Worksheet.
    [Test]
    procedure WorksheetGetField()
    var
        Sheet: TestPage "LT GFN Sheet";
    begin
        Initialize();
        Sheet.OpenView();
        Sheet.First();
        Check('value', 'Alpha', Sheet.GetField(525577793).Value());
        Sheet.Close();
    end;

    // GetField by the id of the ListPlus page's own Name control (97686, GetFieldByControlId.al:1648 the missing id there).
    [Test]
    procedure ListPlusOwnId()
    var
        Pg: TestPage "LT GFI ListPlus";
    begin
        Initialize();
        Pg.OpenEdit();
        Check('value', 'Alpha', Pg.GetField(1633031500).Value());
        Pg.Close();
    end;

    // GetField on the host's TestPart by the id of the CardPart's own Name control (97687, :1665 the missing id there).
    [Test]
    procedure CardPartOwnId()
    var
        Pg: TestPage "LT GFI Part Host";
    begin
        Initialize();
        Pg.OpenEdit();
        Check('value', 'Alpha', Pg.Part.GetField(1480767443).Value());
        Pg.Close();
    end;
}

// GetField on a Worksheet by the id of another page's control: in a codeunit of its own, as the tests above refuse.
codeunit 97693 "LT GFN Missing Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // The id of the Name control of the List 97702 on the Worksheet 97685 (:1631 an id on no page).
    [Test]
    procedure WorksheetOtherPageId()
    var
        Row: Record "LT GFI Row";
        Sheet: TestPage "LT GFI Worksheet";
        Shown: Text;
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Insert();
        Commit();
        Sheet.OpenEdit();
        asserterror Shown := Sheet.GetField(2021824417).Value();
        if GetLastErrorText() <> 'The field with ID = 2021824417 is not found on the page.' then
            Error('error: found <%1>.', GetLastErrorText());
        Sheet.Close();
    end;
}

page 97692 "LT GFN Sheet"
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

// Just past the cells of :2559-:2702: an action's id on a NavigatePage (the cells measure an action's id on a Worksheet only),
// and GetField(1) on the CardPart a Document page shows (the cells measure it behind a Card and a RoleCenter). In a codeunit of
// its own, as the runner refuses both.
page 97743 "LT GFN Navigate Act"
{
    PageType = NavigatePage;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(NextStep)
            {
                ApplicationArea = All;
                InFooterBar = true;

                trigger OnAction()
                begin
                end;
            }
        }
    }
}

page 97744 "LT GFN Doc Host"
{
    PageType = Document;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(Name; Rec.Name) { ApplicationArea = All; }
            part(Part; "LT GFI Card Part")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
        }
    }
}

// Just past the cells of :3015 and :3035 (the review of #8902 at 22:31Z): an actionref a second page extension adds to the
// Worksheet 97749, and a field a second page extension adds to the Card 97688, read through its CardPart, each by its extension's id.
pageextension 97741 "LT GFN Ext Sheet Ext2" extends "LT GFI Ext Sheet"
{
    actions
    {
        addlast(Processing)
        {
            action(NbAct)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                end;
            }
        }
        addlast(Promoted)
        {
            actionref(NbActRef; NbAct) { }
        }
    }
}

pageextension 97742 "LT GFN Part Host Ext2" extends "LT GFI Part Host"
{
    layout
    {
        addlast(General)
        {
            field(NbHostExt; Rec.Name) { ApplicationArea = All; }
        }
    }
}

codeunit 97745 "LT GFN Kind Tests"
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
        Row.Insert();
        Commit();
    end;

    // The id of the NavigatePage's own action.
    [Test]
    procedure NavigatePageActionId()
    var
        Pg: TestPage "LT GFN Navigate Act";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        asserterror Shown := Pg.GetField(461634918).Value();
        if GetLastErrorText() <> 'The field with ID = 461634918 is not found on the page.' then
            Error('error: found <%1>.', GetLastErrorText());
        Pg.Close();
    end;

    // GetField(1) on the TestPart of the CardPart a Document page shows.
    [Test]
    procedure DocumentHostPartMissingId()
    var
        Pg: TestPage "LT GFN Doc Host";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        asserterror Shown := Pg.Part.GetField(1).Value();
        if GetLastErrorText() <> 'The field with ID = 1 is not found on the page.' then
            Error('error: found <%1>.', GetLastErrorText());
        Pg.Close();
    end;

    // An actionref of the second extension on the Worksheet 97749, by its extension's id.
    [Test]
    procedure WorksheetExtActionRefId()
    var
        Pg: TestPage "LT GFI Ext Sheet";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        Shown := Pg.GetField(399114906).Value();
        Pg.Close();
        if Shown <> '' then
            Error('found <%1>.', Shown);
    end;

    // A field of the second extension on the Card 97688, by its extension's id, through the CardPart's TestPart.
    [Test]
    procedure CardPartHostExtFieldId()
    var
        Pg: TestPage "LT GFI Part Host";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        asserterror Shown := Pg.Part.GetField(1319895369).Value();
        if GetLastErrorText() <> 'The field with ID = 1319895369 is not found on the page.' then
            Error('error: found <%1>.', GetLastErrorText());
        Pg.Close();
    end;
}
