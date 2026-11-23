// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpage-getfield-method
// Scope: in-scope
// local object: the pages' controls and the ids the compiler gives them are what is under test
// Fixtures used: shared Assert (60021); own tables 97701, 97711 and 97735, enum 97713, pages 97680-97690, 97695, 97696, 97699, 97700, 97702-97705, 97708, 97709, 97712, 97734-97737 and 97739-97742, page extensions 97706, 97710 and 97738, tests 97707, 97691, 97734 and 97737
// Probe (al-runner issue 8832): TestPage.GetField and TestPart.GetField take the id the compiler gives a page control. The ids in
//   this file are FNV-1a (32 bit) over the UTF-16LE text of the declaring object's id followed by the control name as declared,
//   read as a signed number and made positive; the compiled app's SymbolReference.json lists the same ids. The issue's page (a List
//   with "No." and Name, page 97702), a second List with the same control names (97703), a Card with field controls of several
//   kinds, a label, groups, a part and an action (97704), the part's ListPart (97705), a page extension that adds a field (97706)
//   a Card with a cuegroup, a fixed, a grid and a FactBox part (97708), and a Card (97709) with a field the application area
//   hides, an action group and a field whose caption a page extension (97710) modifies.
//   Readings: the control's Value (and Caption) through GetField on the row the page stands on, a SetValue through GetField, and the
//   error text and code where GetField finds no field.
// BC versions: 27, 28, 29

table 97701 "LT GFI Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
        field(3; "Unit Price"; Decimal) { }
        field(4; Note; Text[30]) { }
        field(5; Extra; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 97702 "LT GFI List"
{
    PageType = List;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }
}

page 97703 "LT GFI Other List"
{
    PageType = List;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }
}

page 97704 "LT GFI Card"
{
    PageType = Card;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
                field("Unit Price"; Rec."Unit Price") { }
                field(MixedCaseNote; Rec.Note) { }
                field(GlobalText; GlobalText)
                {
                    Caption = 'Global Text';
                }
                field(HiddenName; Rec.Name)
                {
                    Visible = false;
                }
                label(InfoLabel)
                {
                    Caption = 'Info';
                }
                group(Inner)
                {
                    field(InnerName; Rec.Name) { }
                }
            }
            group(HiddenGroup)
            {
                Visible = false;
                field(InHiddenGroup; Rec.Name) { }
            }
            part(Lines; "LT GFI Lines Part") { }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkIt)
            {
                trigger OnAction()
                begin
                    Rec.Note := 'marked';
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        GlobalText := 'G-' + Rec."No.";
    end;

    var
        GlobalText: Text[30];
}

page 97705 "LT GFI Lines Part"
{
    PageType = ListPart;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(PartName; Rec.Name) { }
                field(PartNo; Rec."No.") { }
            }
        }
    }
}

pageextension 97706 "LT GFI Card Ext" extends "LT GFI Card"
{
    layout
    {
        addlast(General)
        {
            field(ExtExtra; Rec.Extra) { }
        }
    }
}

page 97708 "LT GFI Cue Card"
{
    PageType = Card;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            cuegroup(Cues)
            {
                field(CueName; Rec.Name) { }
            }
            fixed(FixedPart)
            {
                group(FixedGroup)
                {
                    field(FixedName; Rec.Name) { }
                }
            }
            grid(GridPart)
            {
                field(GridName; Rec.Name) { }
            }
        }
        area(FactBoxes)
        {
            part(Facts; "LT GFI Lines Part") { }
        }
    }
}

page 97709 "LT GFI Mod Card"
{
    PageType = Card;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Name; Rec.Name) { }
                field(SuiteNote; Rec.Note)
                {
                    ApplicationArea = Suite;
                }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            group(Grp)
            {
                action(InnerAction)
                {
                    trigger OnAction()
                    begin
                        Rec.Note := 'acted';
                    end;
                }
            }
        }
    }
}

pageextension 97710 "LT GFI Mod Card Ext" extends "LT GFI Mod Card"
{
    layout
    {
        modify(Name)
        {
            Caption = 'Changed Name';
        }
    }
}

codeunit 97707 "LT GFI Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Row: Record "LT GFI Row";
    begin
        Row.DeleteAll();
        AddRow('R1', 'Alpha', 1.5, 'n1', 'x1');
        AddRow('R2', 'Beta', 2.25, 'n2', 'x2');
        Commit();
        Clear(Seen);
    end;

    local procedure AddRow(No: Code[20]; Name: Text[30]; UnitPrice: Decimal; NoteText: Text[30]; ExtraText: Text[30])
    var
        Row: Record "LT GFI Row";
    begin
        Row."No." := No;
        Row.Name := Name;
        Row."Unit Price" := UnitPrice;
        Row.Note := NoteText;
        Row.Extra := ExtraText;
        Row.Insert();
    end;

    local procedure Note(What: Text; Actual: Text)
    begin
        Seen.Set(What, Actual);
    end;

    local procedure Expect(What: Text; Expected: Text)
    begin
        Assert.AreEqual(Expected, Seen.Get(What), What);
    end;

    local procedure Failure(): Text
    begin
        exit('error ' + GetLastErrorCode() + ': ' + GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryList(var Rows: TestPage "LT GFI List"; Id: Integer; var Shown: Text)
    begin
        Shown := Rows.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryOther(var Rows: TestPage "LT GFI Other List"; Id: Integer; var Shown: Text)
    begin
        Shown := Rows.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryCard(var Card: TestPage "LT GFI Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Card.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryPart(var Card: TestPage "LT GFI Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Card.Lines.GetField(Id).Value();
    end;

    local procedure ReadList(var Rows: TestPage "LT GFI List"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryList(Rows, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    local procedure ReadOther(var Rows: TestPage "LT GFI Other List"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryOther(Rows, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    local procedure ReadCard(var Card: TestPage "LT GFI Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCard(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    local procedure ReadPart(var Card: TestPage "LT GFI Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryPart(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryCue(var Card: TestPage "LT GFI Cue Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Card.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryFacts(var Card: TestPage "LT GFI Cue Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Card.Facts.GetField(Id).Value();
    end;

    local procedure ReadCue(var Card: TestPage "LT GFI Cue Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCue(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    local procedure ReadFacts(var Card: TestPage "LT GFI Cue Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryFacts(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryCardCaption(var Card: TestPage "LT GFI Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Card.GetField(Id).Caption();
    end;

    [TryFunction]
    local procedure TryCardVisible(var Card: TestPage "LT GFI Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Format(Card.GetField(Id).Visible());
    end;

    local procedure CardCaption(var Card: TestPage "LT GFI Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCardCaption(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    local procedure CardVisible(var Card: TestPage "LT GFI Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCardVisible(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryMod(var Card: TestPage "LT GFI Mod Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Card.GetField(Id).Value();
    end;

    local procedure ReadMod(var Card: TestPage "LT GFI Mod Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryMod(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // The issue's shape: the List stands on R1, then on R2; GetField by the id of Name and of "No." reads the row it stands on.
    [Test]
    procedure List_GetFieldByControlId_ReadsTheRow()
    var
        Rows: TestPage "LT GFI List";
    begin
        Initialize();
        Rows.OpenView();
        Rows.First();
        Note('name', ReadList(Rows, 2021824417));
        Note('no', ReadList(Rows, 153425479));
        Rows.Next();
        Note('next', ReadList(Rows, 2021824417) + '/' + ReadList(Rows, 153425479));
        Rows.Close();
        Expect('name', 'Alpha');
        Expect('no', 'R1');
        Expect('next', 'Beta/R2');
    end;

    // The same control names on another page have ids of their own: the other page's id of Name finds nothing.
    [Test]
    procedure OtherList_SameNames_IdsOfTheirOwnPage()
    var
        Rows: TestPage "LT GFI Other List";
    begin
        Initialize();
        Rows.OpenView();
        Rows.First();
        Note('own', ReadOther(Rows, 109586264) + '/' + ReadOther(Rows, 96040840));
        Note('other', ReadOther(Rows, 2021824417));
        Rows.Close();
        Expect('own', 'Alpha/R1');
        Expect('other', 'error TestFieldNotFound: The field with ID = 2021824417 is not found on the page.');
    end;

    // Ids no field of the page has: a name the page does not hold, the name in lower case, the repeater's name.
    [Test]
    procedure List_IdsOfNoField()
    var
        Rows: TestPage "LT GFI List";
    begin
        Initialize();
        Rows.OpenView();
        Rows.First();
        Note('missing', ReadList(Rows, 1537008046));
        Note('lower', ReadList(Rows, 1717459071));
        Note('repeater', ReadList(Rows, 1683664403));
        Rows.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1537008046 is not found on the page.');
        Expect('lower', 'error TestFieldNotFound: The field with ID = 1717459071 is not found on the page.');
        Expect('repeater', 'error TestFieldNotFound: The field with ID = 1683664403 is not found on the page.');
    end;

    // The Card's field controls: bound to a field (a quoted name, a name in mixed case, one in a nested group), to a page variable,
    // and a label.
    [Test]
    procedure Card_FieldControls()
    var
        Card: TestPage "LT GFI Card";
    begin
        Initialize();
        Card.OpenView();
        Note('no', ReadCard(Card, 558084351));
        Note('name', ReadCard(Card, 201861513));
        Note('price', ReadCard(Card, 669412059));
        Note('note', ReadCard(Card, 526571967));
        Note('global', ReadCard(Card, 1921904910));
        Note('inner', ReadCard(Card, 1812390415));
        Note('label', ReadCard(Card, 1397552416));
        Note('captions', Card.GetField(669412059).Caption() + '/' + Card.GetField(1921904910).Caption());
        Card.Close();
        Expect('no', 'R1');
        Expect('name', 'Alpha');
        Expect('price', '1.50');
        Expect('note', 'n1');
        Expect('global', 'G-R1');
        Expect('inner', 'Alpha');
        Expect('label', 'Info');
        Expect('captions', 'Unit Price/Global Text');
    end;

    // A field control with Visible = false and one in a group with Visible = false.
    [Test]
    procedure Card_HiddenControls()
    var
        Card: TestPage "LT GFI Card";
    begin
        Initialize();
        Card.OpenView();
        Note('hidden', ReadCard(Card, 1811926571));
        Note('ingroup', ReadCard(Card, 621122018));
        Card.Close();
        Expect('hidden', 'error TestFieldNotFound: The field with ID = 1811926571 is not found on the page.');
        Expect('ingroup', 'error TestFieldNotFound: The field with ID = 621122018 is not found on the page.');
    end;

    // The ids of controls that are no field: the groups, the part, the action, the areas.
    [Test]
    procedure Card_OtherControlKinds()
    var
        Card: TestPage "LT GFI Card";
    begin
        Initialize();
        Card.OpenView();
        Note('group', ReadCard(Card, 45333458));
        Note('inner', ReadCard(Card, 182751126));
        Note('hiddengroup', ReadCard(Card, 830125693));
        Note('part', ReadCard(Card, 2105193259));
        Note('action', ReadCard(Card, 1170108248));
        Note('content', ReadCard(Card, 1462267929));
        Note('processing', ReadCard(Card, 356260187));
        Card.Close();
        Expect('group', 'error TestFieldNotFound: The field with ID = 45333458 is not found on the page.');
        Expect('inner', 'error TestFieldNotFound: The field with ID = 182751126 is not found on the page.');
        Expect('hiddengroup', 'error TestFieldNotFound: The field with ID = 830125693 is not found on the page.');
        Expect('part', '');
        Expect('action', 'error TestFieldNotFound: The field with ID = 1170108248 is not found on the page.');
        Expect('content', 'error TestFieldNotFound: The field with ID = 1462267929 is not found on the page.');
        Expect('processing', 'error TestFieldNotFound: The field with ID = 356260187 is not found on the page.');
    end;

    // SetValue through GetField: the Name control shows the value, and Close saves it.
    [Test]
    procedure Card_SetValueThroughGetField()
    var
        Card: TestPage "LT GFI Card";
        Row: Record "LT GFI Row";
    begin
        Initialize();
        Card.OpenEdit();
        Card.GetField(201861513).SetValue('Zed');
        Note('shown', Card.Name.Value() + '/' + Card.GetField(1812390415).Value());
        Card.Close();
        Row.Get('R1');
        Note('stored', Row.Name);
        Expect('shown', 'Zed/Zed');
        Expect('stored', 'Zed');
    end;

    // A field a page extension adds has the id of the extension and its name (the extension's SymbolReference.json lists it). From
    // BC 28 GetField finds it by neither that id nor the base page's id with its name; BC 27 finds it by the extension's id.
    [Test]
    procedure Card_ExtensionControl()
    var
        Card: TestPage "LT GFI Card";
    begin
        Initialize();
        Card.OpenView();
        Note('ext', ReadCard(Card, 1335360081));
        Note('base', ReadCard(Card, 766046563));
        Card.Close();
#if BC28PLUS
        // BC 28 and later: GetField does not find a field a page extension adds.
        Expect('ext', 'error TestFieldNotFound: The field with ID = 1335360081 is not found on the page.');
#else
        // BC 27: GetField finds it by the extension's id.
        Expect('ext', 'x1');
#endif
        Expect('base', 'error TestFieldNotFound: The field with ID = 766046563 is not found on the page.');
    end;

    // The part's controls: TestPart.GetField by the ids of the ListPart's controls reads the part's row; the host's GetField with
    // those ids, and the part's GetField with the host's ids and the part's repeater, find nothing.
    [Test]
    procedure Card_PartControls()
    var
        Card: TestPage "LT GFI Card";
    begin
        Initialize();
        Card.OpenView();
        Note('part', ReadPart(Card, 1064400143) + '/' + ReadPart(Card, 134725455));
        Note('hostid', ReadPart(Card, 201861513));
        Note('repeater', ReadPart(Card, 992006240));
        Note('onhost', ReadCard(Card, 1064400143));
        Card.Close();
        Expect('part', 'Alpha/R1');
        Expect('hostid', 'error TestFieldNotFound: The field with ID = 201861513 is not found on the page.');
        Expect('repeater', 'error TestFieldNotFound: The field with ID = 992006240 is not found on the page.');
        Expect('onhost', 'error TestFieldNotFound: The field with ID = 1064400143 is not found on the page.');
    end;
    // Fields in a fixed and in a grid, the ids of a cuegroup, a fixed and a grid, and a FactBox part: its id on the host, and
    // TestPart.GetField with the ListPart's ids.
    [Test]
    procedure CueCard_OtherContainers()
    var
        Card: TestPage "LT GFI Cue Card";
    begin
        Initialize();
        Card.OpenView();
        Note('fields', ReadCue(Card, 710836701) + '/' + ReadCue(Card, 373308387));
        Note('cues', ReadCue(Card, 518652348));
        Note('fixed', ReadCue(Card, 923386085));
        Note('grid', ReadCue(Card, 393395371));
        Note('factbox', ReadCue(Card, 345689153));
        Note('facts', ReadFacts(Card, 1064400143) + '/' + ReadFacts(Card, 134725455));
        Card.Close();
        Expect('fields', 'Alpha/Alpha');
        Expect('cues', '');
        Expect('fixed', '');
        Expect('grid', '');
        Expect('factbox', '');
        Expect('facts', 'Alpha/R1');
    end;

    // A Text field in a cuegroup, through GetField and by its name.
    [Test]
    procedure CueCard_CueField()
    var
        Card: TestPage "LT GFI Cue Card";
    begin
        Initialize();
        Card.OpenView();
        Note('byid', ReadCue(Card, 1246435368));
        Note('direct', Card.CueName.Value());
        Card.Close();
        Expect('byid', '0');
        Expect('direct', '0');
    end;
    // The other read-only calls through GetField answer as on the control by its name: the Decimal control, the label and the
    // control bound to a page variable.
    [Test]
    procedure Card_ReadsThroughGetField()
    var
        Card: TestPage "LT GFI Card";
    begin
        Initialize();
        Card.OpenEdit();
        Note('byid', Format(Card.GetField(669412059).Visible()) + '/' + Format(Card.GetField(669412059).Enabled()) + '/' +
          Format(Card.GetField(669412059).Editable()) + '/' + Format(Card.GetField(669412059).AsDecimal()) + '/' +
          Format(Card.GetField(669412059).HideValue()) + '/' + Format(Card.GetField(669412059).ShowMandatory()) + '/' +
          Format(Card.GetField(669412059).ValidationErrorCount()) + '/' + Format(Card.GetField(1397552416).Visible()) + '/' +
          Format(Card.GetField(1921904910).Editable()));
        Note('direct', Format(Card."Unit Price".Visible()) + '/' + Format(Card."Unit Price".Enabled()) + '/' +
          Format(Card."Unit Price".Editable()) + '/' + Format(Card."Unit Price".AsDecimal()) + '/' +
          Format(Card."Unit Price".HideValue()) + '/' + Format(Card."Unit Price".ShowMandatory()) + '/' +
          Format(Card."Unit Price".ValidationErrorCount()) + '/' + Format(Card.InfoLabel.Visible()) + '/' +
          Format(Card.GlobalText.Editable()));
        Card.GetField(669412059).AssertEquals(1.5);
        Card.Close();
        Expect('byid', 'Yes/Yes/Yes/1.5/No/No/0/Yes/Yes');
        Expect('direct', 'Yes/Yes/Yes/1.5/No/No/0/Yes/Yes');
    end;

    // Caption and Visible of what the part control's id gives.
    [Test]
    procedure Card_PartIdCaptionAndVisible()
    var
        Card: TestPage "LT GFI Card";
    begin
        Initialize();
        Card.OpenView();
        Note('part', CardCaption(Card, 2105193259) + '|' + CardVisible(Card, 2105193259));
        Card.Close();
        Expect('part', 'LT GFI Lines Part|Yes');
    end;
    // A base field whose caption a page extension modifies keeps the base page's id; a field the application area hides
    // (#Basic, the field Suite) and an action group are not found.
    [Test]
    procedure ModCard_ModifiedHiddenAndActionGroup()
    var
        Card: TestPage "LT GFI Mod Card";
        Areas: Text;
    begin
        Initialize();
        Areas := ApplicationArea();
        ApplicationArea('#Basic');
        Card.OpenView();
        Note('modified', ReadMod(Card, 1331314946) + '/' + Card.GetField(1331314946).Caption());
        Note('hidden', ReadMod(Card, 651490805));
        Note('group', ReadMod(Card, 298019664));
        Card.Close();
        ApplicationArea(Areas);
        Expect('modified', 'Alpha/Changed Name');
        Expect('hidden', 'error TestFieldNotFound: The field with ID = 651490805 is not found on the page.');
        Expect('group', 'error TestFieldNotFound: The field with ID = 298019664 is not found on the page.');
    end;
    [TryFunction]
    local procedure TryCueTypes(var Card: TestPage "LT GFI Cue Types"; Id: Integer; var Shown: Text)
    begin
        Shown := Card.GetField(Id).Value();
    end;

    local procedure ReadCueTypes(var Card: TestPage "LT GFI Cue Types"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCueTypes(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // A field control in a cuegroup for a field of each type, and a Code and an Integer field in a cuegroup with CuegroupLayout = Wide:
    // its Value by its name, then through GetField by its id.
    [Test]
    procedure CueTypes_ValueByNameAndById()
    var
        Row: Record "LT GFI Cue Row";
        Card: TestPage "LT GFI Cue Types";
    begin
        Initialize();
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.TextWord := 'Alpha';
        Row.TextNumber := '42';
        Row.CodeWord := 'R1';
        Row.CodeNumber := '7';
        Row.IntValue := 5;
        Row.IntZero := 0;
        Row.DecValue := 1.5;
        Row.DecBig := 1234.5;
        Row.BoolValue := true;
        Row.DayValue := DMY2Date(3, 3, 2026);
        Row.BigValue := 77;
        Row.OptValue := Row.OptValue::Beta;
        Row.AtValue := 103000T;
        Row.StampValue := CreateDateTime(DMY2Date(3, 3, 2026), 103000T);
        Row.GuidValue := '{0F2E7C52-6E0B-4F51-9C3A-1B2C3D4E5F60}';
        Row.KindValue := Row.KindValue::Beta;
        Row.SpanValue := 90000;
        Row.Insert();
        Card.OpenView();
        Note('TextWord', Card.CueTextWord.Value() + '|' + ReadCueTypes(Card, 674324257));
        Note('TextNumber', Card.CueTextNumber.Value() + '|' + ReadCueTypes(Card, 702274306));
        Note('CodeWord', Card.CueCodeWord.Value() + '|' + ReadCueTypes(Card, 541430713));
        Note('CodeNumber', Card.CueCodeNumber.Value() + '|' + ReadCueTypes(Card, 843379546));
        Note('IntValue', Card.CueIntValue.Value() + '|' + ReadCueTypes(Card, 1094229972));
        Note('IntZero', Card.CueIntZero.Value() + '|' + ReadCueTypes(Card, 1411875605));
        Note('DecValue', Card.CueDecValue.Value() + '|' + ReadCueTypes(Card, 1957540459));
        Note('DecBig', Card.CueDecBig.Value() + '|' + ReadCueTypes(Card, 2039052834));
        Note('BoolValue', Card.CueBoolValue.Value() + '|' + ReadCueTypes(Card, 1490279559));
        Note('DayValue', Card.CueDayValue.Value() + '|' + ReadCueTypes(Card, 772286949));
        Note('BigValue', Card.CueBigValue.Value() + '|' + ReadCueTypes(Card, 166715205));
        Note('OptValue', Card.CueOptValue.Value() + '|' + ReadCueTypes(Card, 1575506972));
        Note('AtValue', Card.CueAtValue.Value() + '|' + ReadCueTypes(Card, 1819569266));
        Note('StampValue', Card.CueStampValue.Value() + '|' + ReadCueTypes(Card, 339305132));
        Note('GuidValue', Card.CueGuidValue.Value() + '|' + ReadCueTypes(Card, 1928044720));
        Note('KindValue', Card.CueKindValue.Value() + '|' + ReadCueTypes(Card, 2059022367));
        Note('SpanValue', Card.CueSpanValue.Value() + '|' + ReadCueTypes(Card, 1714720931));
        Note('WideCodeWord', Card.WideCodeWord.Value() + '|' + ReadCueTypes(Card, 1127485133));
        Note('WideIntValue', Card.WideIntValue.Value() + '|' + ReadCueTypes(Card, 585839752));
        Card.Close();
        Expect('TextWord', '0|0');
        Expect('TextNumber', '0|0');
        Expect('CodeWord', '0|0');
        Expect('CodeNumber', '0|0');
        Expect('IntValue', '5|5');
        Expect('IntZero', '0|0');
        Expect('DecValue', '1.50|1.50');
        Expect('DecBig', '1,234.50|1,234.50');
        Expect('BoolValue', '0|0');
        Expect('DayValue', '0|0');
        Expect('BigValue', '0|0');
        Expect('OptValue', '1|1');
        Expect('AtValue', '0|0');
        Expect('StampValue', '0|0');
        Expect('GuidValue', '0|0');
        Expect('KindValue', '1|1');
        Expect('SpanValue', '0|0');
        Expect('WideCodeWord', '0|0');
        Expect('WideIntValue', '5|5');
    end;

    // The one row of table 97711 the cue tests on the other page kinds read: the values of CueTypes_ValueByNameAndById.
    local procedure SeedCueRow()
    var
        Row: Record "LT GFI Cue Row";
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.TextWord := 'Alpha';
        Row.TextNumber := '42';
        Row.CodeWord := 'R1';
        Row.CodeNumber := '7';
        Row.IntValue := 5;
        Row.IntZero := 0;
        Row.DecValue := 1.5;
        Row.DecBig := 1234.5;
        Row.BoolValue := true;
        Row.DayValue := DMY2Date(3, 3, 2026);
        Row.BigValue := 77;
        Row.OptValue := Row.OptValue::Beta;
        Row.AtValue := 103000T;
        Row.StampValue := CreateDateTime(DMY2Date(3, 3, 2026), 103000T);
        Row.GuidValue := '{0F2E7C52-6E0B-4F51-9C3A-1B2C3D4E5F60}';
        Row.KindValue := Row.KindValue::Beta;
        Row.SpanValue := 90000;
        Row.Insert();
    end;

    [TryFunction]
    local procedure TryCueCardPart(var Pg: TestPage "LT GFI Cue Part"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadCueCardPart(var Pg: TestPage "LT GFI Cue Part"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCueCardPart(Pg, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // The cuegroups of CueTypes_ValueByNameAndById on a CardPart page (97680): each field's Value by its name, then through GetField
    // by its id.
    [Test]
    procedure CueTypesCardPart_ValueByNameAndById()
    var
        Pg: TestPage "LT GFI Cue Part";
    begin
        Initialize();
        SeedCueRow();
        Pg.OpenView();
        Note('TextWord', Pg.CueTextWord.Value() + '|' + ReadCueCardPart(Pg, 26977565));
        Note('TextNumber', Pg.CueTextNumber.Value() + '|' + ReadCueCardPart(Pg, 1688402688));
        Note('CodeWord', Pg.CueCodeWord.Value() + '|' + ReadCueCardPart(Pg, 1587224453));
        Note('CodeNumber', Pg.CueCodeNumber.Value() + '|' + ReadCueCardPart(Pg, 717132888));
        Note('IntValue', Pg.CueIntValue.Value() + '|' + ReadCueCardPart(Pg, 1897462670));
        Note('IntZero', Pg.CueIntZero.Value() + '|' + ReadCueCardPart(Pg, 258298535));
        Note('DecValue', Pg.CueDecValue.Value() + '|' + ReadCueCardPart(Pg, 1218698967));
        Note('DecBig', Pg.CueDecBig.Value() + '|' + ReadCueCardPart(Pg, 737049792));
        Note('BoolValue', Pg.CueBoolValue.Value() + '|' + ReadCueCardPart(Pg, 1213531979));
        Note('DayValue', Pg.CueDayValue.Value() + '|' + ReadCueCardPart(Pg, 865399911));
        Note('BigValue', Pg.CueBigValue.Value() + '|' + ReadCueCardPart(Pg, 1009768583));
        Note('OptValue', Pg.CueOptValue.Value() + '|' + ReadCueCardPart(Pg, 1863129850));
        Note('AtValue', Pg.CueAtValue.Value() + '|' + ReadCueCardPart(Pg, 62956608));
        Note('StampValue', Pg.CueStampValue.Value() + '|' + ReadCueCardPart(Pg, 1525930642));
        Note('GuidValue', Pg.CueGuidValue.Value() + '|' + ReadCueCardPart(Pg, 1580321858));
        Note('KindValue', Pg.CueKindValue.Value() + '|' + ReadCueCardPart(Pg, 1381938349));
        Note('SpanValue', Pg.CueSpanValue.Value() + '|' + ReadCueCardPart(Pg, 1624429455));
        Note('WideCodeWord', Pg.WideCodeWord.Value() + '|' + ReadCueCardPart(Pg, 275062239));
        Note('WideIntValue', Pg.WideIntValue.Value() + '|' + ReadCueCardPart(Pg, 2143231722));
        Pg.Close();
        Expect('TextWord', '0|0');
        Expect('TextNumber', '0|0');
        Expect('CodeWord', '0|0');
        Expect('CodeNumber', '0|0');
        Expect('IntValue', '5|5');
        Expect('IntZero', '0|0');
        Expect('DecValue', '1.50|1.50');
        Expect('DecBig', '1,234.50|1,234.50');
        Expect('BoolValue', '0|0');
        Expect('DayValue', '0|0');
        Expect('BigValue', '0|0');
        Expect('OptValue', '1|1');
        Expect('AtValue', '0|0');
        Expect('StampValue', '0|0');
        Expect('GuidValue', '0|0');
        Expect('KindValue', '1|1');
        Expect('SpanValue', '0|0');
        Expect('WideCodeWord', '0|0');
        Expect('WideIntValue', '5|5');
    end;

    [TryFunction]
    local procedure TryCueListPart(var Pg: TestPage "LT GFI Cue List Part"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadCueListPart(var Pg: TestPage "LT GFI Cue List Part"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCueListPart(Pg, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // The cuegroups of CueTypes_ValueByNameAndById on a ListPart page (97681): each field's Value by its name, then through GetField
    // by its id.
    [Test]
    procedure CueTypesListPart_ValueByNameAndById()
    var
        Pg: TestPage "LT GFI Cue List Part";
    begin
        Initialize();
        SeedCueRow();
        Pg.OpenView();
        Note('TextWord', Pg.CueTextWord.Value() + '|' + ReadCueListPart(Pg, 1125578228));
        Note('TextNumber', Pg.CueTextNumber.Value() + '|' + ReadCueListPart(Pg, 1123672561));
        Note('CodeWord', Pg.CueCodeWord.Value() + '|' + ReadCueListPart(Pg, 1186723660));
        Note('CodeNumber', Pg.CueCodeNumber.Value() + '|' + ReadCueListPart(Pg, 334073687));
        Note('IntValue', Pg.CueIntValue.Value() + '|' + ReadCueListPart(Pg, 1903534943));
        Note('IntZero', Pg.CueIntZero.Value() + '|' + ReadCueListPart(Pg, 373523854));
        Note('DecValue', Pg.CueDecValue.Value() + '|' + ReadCueListPart(Pg, 139652282));
        Note('DecBig', Pg.CueDecBig.Value() + '|' + ReadCueListPart(Pg, 337577135));
        Note('BoolValue', Pg.CueBoolValue.Value() + '|' + ReadCueListPart(Pg, 1292040146));
        Note('DayValue', Pg.CueDayValue.Value() + '|' + ReadCueListPart(Pg, 1460232392));
        Note('BigValue', Pg.CueBigValue.Value() + '|' + ReadCueListPart(Pg, 633992840));
        Note('OptValue', Pg.CueOptValue.Value() + '|' + ReadCueListPart(Pg, 1194663655));
        Note('AtValue', Pg.CueAtValue.Value() + '|' + ReadCueListPart(Pg, 1530822439));
        Note('StampValue', Pg.CueStampValue.Value() + '|' + ReadCueListPart(Pg, 859439775));
        Note('GuidValue', Pg.CueGuidValue.Value() + '|' + ReadCueListPart(Pg, 1989317605));
        Note('KindValue', Pg.CueKindValue.Value() + '|' + ReadCueListPart(Pg, 817846844));
        Note('SpanValue', Pg.CueSpanValue.Value() + '|' + ReadCueListPart(Pg, 945837176));
        Note('WideCodeWord', Pg.WideCodeWord.Value() + '|' + ReadCueListPart(Pg, 1297950792));
        Note('WideIntValue', Pg.WideIntValue.Value() + '|' + ReadCueListPart(Pg, 259042531));
        Pg.Close();
        Expect('TextWord', '0|0');
        Expect('TextNumber', '0|0');
        Expect('CodeWord', '0|0');
        Expect('CodeNumber', '0|0');
        Expect('IntValue', '5|5');
        Expect('IntZero', '0|0');
        Expect('DecValue', '1.50|1.50');
        Expect('DecBig', '1,234.50|1,234.50');
        Expect('BoolValue', '0|0');
        Expect('DayValue', '0|0');
        Expect('BigValue', '0|0');
        Expect('OptValue', '1|1');
        Expect('AtValue', '0|0');
        Expect('StampValue', '0|0');
        Expect('GuidValue', '0|0');
        Expect('KindValue', '1|1');
        Expect('SpanValue', '0|0');
        Expect('WideCodeWord', '0|0');
        Expect('WideIntValue', '5|5');
    end;

    [TryFunction]
    local procedure TryCueList(var Pg: TestPage "LT GFI Cue List"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadCueList(var Pg: TestPage "LT GFI Cue List"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCueList(Pg, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // The cuegroups of CueTypes_ValueByNameAndById on a List page (97682): each field's Value by its name, then through GetField
    // by its id.
    [Test]
    procedure CueTypesList_ValueByNameAndById()
    var
        Pg: TestPage "LT GFI Cue List";
    begin
        Initialize();
        SeedCueRow();
        Pg.OpenView();
        Note('TextWord', Pg.CueTextWord.Value() + '|' + ReadCueList(Pg, 1282578369));
        Note('TextNumber', Pg.CueTextNumber.Value() + '|' + ReadCueList(Pg, 764471650));
        Note('CodeWord', Pg.CueCodeWord.Value() + '|' + ReadCueList(Pg, 2082555225));
        Note('CodeNumber', Pg.CueCodeNumber.Value() + '|' + ReadCueList(Pg, 883022266));
        Note('IntValue', Pg.CueIntValue.Value() + '|' + ReadCueList(Pg, 1610916340));
        Note('IntZero', Pg.CueIntZero.Value() + '|' + ReadCueList(Pg, 1841691339));
        Note('DecValue', Pg.CueDecValue.Value() + '|' + ReadCueList(Pg, 1277752971));
        Note('DecBig', Pg.CueDecBig.Value() + '|' + ReadCueList(Pg, 589548286));
        Note('BoolValue', Pg.CueBoolValue.Value() + '|' + ReadCueList(Pg, 525456793));
        Note('DayValue', Pg.CueDayValue.Value() + '|' + ReadCueList(Pg, 730154107));
        Note('BigValue', Pg.CueBigValue.Value() + '|' + ReadCueList(Pg, 554648987));
        Note('OptValue', Pg.CueOptValue.Value() + '|' + ReadCueList(Pg, 1956933316));
        Note('AtValue', Pg.CueAtValue.Value() + '|' + ReadCueList(Pg, 809468946));
        Note('StampValue', Pg.CueStampValue.Value() + '|' + ReadCueList(Pg, 2026829964));
        Note('GuidValue', Pg.CueGuidValue.Value() + '|' + ReadCueList(Pg, 1833458896));
        Note('KindValue', Pg.CueKindValue.Value() + '|' + ReadCueList(Pg, 83400383));
        Note('SpanValue', Pg.CueSpanValue.Value() + '|' + ReadCueList(Pg, 244366781));
        Note('WideCodeWord', Pg.WideCodeWord.Value() + '|' + ReadCueList(Pg, 66349907));
        Note('WideIntValue', Pg.WideIntValue.Value() + '|' + ReadCueList(Pg, 1309032344));
        Pg.Close();
        Expect('TextWord', '0|0');
        Expect('TextNumber', '0|0');
        Expect('CodeWord', '0|0');
        Expect('CodeNumber', '0|0');
        Expect('IntValue', '5|5');
        Expect('IntZero', '0|0');
        Expect('DecValue', '1.50|1.50');
        Expect('DecBig', '1,234.50|1,234.50');
        Expect('BoolValue', '0|0');
        Expect('DayValue', '0|0');
        Expect('BigValue', '0|0');
        Expect('OptValue', '1|1');
        Expect('AtValue', '0|0');
        Expect('StampValue', '0|0');
        Expect('GuidValue', '0|0');
        Expect('KindValue', '1|1');
        Expect('SpanValue', '0|0');
        Expect('WideCodeWord', '0|0');
        Expect('WideIntValue', '5|5');
    end;

    [TryFunction]
    local procedure TryCueRoleCenter(var Pg: TestPage "LT GFI Cue Role"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadCueRoleCenter(var Pg: TestPage "LT GFI Cue Role"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCueRoleCenter(Pg, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryCueRoleByName(var Pg: TestPage "LT GFI Cue Role"; Wide: Boolean; var Shown: Text)
    begin
        if Wide then
            Shown := Pg.WideIntValue.Value()
        else
            Shown := Pg.CueTextWord.Value();
    end;

    local procedure ReadCueRoleByName(var Pg: TestPage "LT GFI Cue Role"; Wide: Boolean): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryCueRoleByName(Pg, Wide, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // The cuegroups of CueTypes_ValueByNameAndById on a RoleCenter page (97683): the Text field of the cuegroup and the Integer
    // field of the wide one, by its name and through GetField by its id.
    [Test]
    procedure CueTypesRoleCenter_ValueByNameAndById()
    var
        Pg: TestPage "LT GFI Cue Role";
    begin
        Initialize();
        SeedCueRow();
        Pg.OpenView();
        Note('TextWord', ReadCueRoleByName(Pg, false) + '|' + ReadCueRoleCenter(Pg, 110307410));
        Note('WideIntValue', ReadCueRoleByName(Pg, true) + '|' + ReadCueRoleCenter(Pg, 1211919791));
        Pg.Close();
        Expect('TextWord', 'error TestFieldNotFound: The field with ID = 110307410 is not found on the page.|error TestFieldNotFound: The field with ID = 110307410 is not found on the page.');
        Expect('WideIntValue', 'error TestFieldNotFound: The field with ID = 1211919791 is not found on the page.|error TestFieldNotFound: The field with ID = 1211919791 is not found on the page.');
    end;

    [TryFunction]
    local procedure TryGetCardField(var Card: TestPage "LT GFI Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Format(Card.GetField(Id).Visible()) + '/' + Card.GetField(Id).Caption();
    end;

    [TryFunction]
    local procedure TryGetModField(var Card: TestPage "LT GFI Mod Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Format(Card.GetField(Id).Visible()) + '/' + Card.GetField(Id).Caption();
    end;

    local procedure GetCardFieldAlone(var Card: TestPage "LT GFI Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryGetCardField(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    local procedure GetModFieldAlone(var Card: TestPage "LT GFI Mod Card"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryGetModField(Card, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // GetField with Visible() and Caption() instead of Value() on what it gives (the compiler wants its return value used), on the
    // ids of the hidden fields of Card_HiddenControls (Visible = false, and in a group with Visible = false), of the field the
    // application area hides on page 97709, and of a visible field: which call raises.
    [Test]
    procedure Card_HiddenControls_VisibleAndCaption()
    var
        Card: TestPage "LT GFI Card";
        ModCard: TestPage "LT GFI Mod Card";
        Areas: Text;
    begin
        Initialize();
        Card.OpenView();
        Note('hidden', GetCardFieldAlone(Card, 1811926571));
        Note('ingroup', GetCardFieldAlone(Card, 621122018));
        Note('visible', GetCardFieldAlone(Card, 201861513));
        Card.Close();
        Areas := ApplicationArea();
        ApplicationArea('#Basic');
        ModCard.OpenView();
        Note('appareahidden', GetModFieldAlone(ModCard, 651490805));
        ModCard.Close();
        ApplicationArea(Areas);
        Expect('hidden', 'error TestFieldNotFound: The field with ID = 1811926571 is not found on the page.');
        Expect('ingroup', 'error TestFieldNotFound: The field with ID = 621122018 is not found on the page.');
        Expect('visible', 'Yes/Name');
        Expect('appareahidden', 'error TestFieldNotFound: The field with ID = 651490805 is not found on the page.');
    end;

    [TryFunction]
    local procedure TryDoc(var Doc: TestPage "LT GFI Doc"; Id: Integer; var Shown: Text)
    begin
        Shown := Doc.GetField(Id).Value();
    end;

    local procedure ReadDoc(var Doc: TestPage "LT GFI Doc"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryDoc(Doc, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // A Document page (97684): GetField by the ids of its field controls, on R1 then R2.
    [Test]
    procedure Document_GetFieldByControlId()
    var
        Doc: TestPage "LT GFI Doc";
    begin
        Initialize();
        Doc.OpenView();
        Doc.First();
        Note('first', ReadDoc(Doc, 723944874) + '/' + ReadDoc(Doc, 2139865318));
        Doc.Next();
        Note('next', ReadDoc(Doc, 723944874) + '/' + ReadDoc(Doc, 2139865318));
        Doc.Close();
        Expect('first', 'R1/Alpha');
        Expect('next', 'R2/Beta');
    end;
}

table 97711 "LT GFI Cue Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; TextWord; Text[30]) { }
        field(3; TextNumber; Text[30]) { }
        field(4; CodeWord; Code[20]) { }
        field(5; CodeNumber; Code[20]) { }
        field(6; IntValue; Integer) { }
        field(7; IntZero; Integer) { }
        field(8; DecValue; Decimal) { }
        field(9; DecBig; Decimal) { }
        field(10; BoolValue; Boolean) { }
        field(11; DayValue; Date) { }
        field(12; BigValue; BigInteger) { }
        field(13; OptValue; Option) { OptionMembers = Alpha,Beta,Gamma; }
        field(14; AtValue; Time) { }
        field(15; StampValue; DateTime) { }
        field(16; GuidValue; Guid) { }
        field(17; KindValue; Enum "LT GFI Cue Kind") { }
        field(18; SpanValue; Duration) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 97712 "LT GFI Cue Types"
{
    PageType = Card;
    SourceTable = "LT GFI Cue Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            cuegroup(Cues)
            {
                field(CueTextWord; Rec.TextWord) { }
                field(CueTextNumber; Rec.TextNumber) { }
                field(CueCodeWord; Rec.CodeWord) { }
                field(CueCodeNumber; Rec.CodeNumber) { }
                field(CueIntValue; Rec.IntValue) { }
                field(CueIntZero; Rec.IntZero) { }
                field(CueDecValue; Rec.DecValue) { }
                field(CueDecBig; Rec.DecBig) { }
                field(CueBoolValue; Rec.BoolValue) { }
                field(CueDayValue; Rec.DayValue) { }
                field(CueBigValue; Rec.BigValue) { }
                field(CueOptValue; Rec.OptValue) { }
                field(CueAtValue; Rec.AtValue) { }
                field(CueStampValue; Rec.StampValue) { }
                field(CueGuidValue; Rec.GuidValue) { }
                field(CueKindValue; Rec.KindValue) { }
                field(CueSpanValue; Rec.SpanValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideCodeWord; Rec.CodeWord) { }
                field(WideIntValue; Rec.IntValue) { }
            }
        }
    }
}

enum 97713 "LT GFI Cue Kind"
{
    Extensible = false;

    value(0; Alpha) { }
    value(1; Beta) { }
}

// The cuegroups of page 97712 on a CardPart page.
page 97680 "LT GFI Cue Part"
{
    PageType = CardPart;
    SourceTable = "LT GFI Cue Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            cuegroup(Cues)
            {
                field(CueTextWord; Rec.TextWord) { }
                field(CueTextNumber; Rec.TextNumber) { }
                field(CueCodeWord; Rec.CodeWord) { }
                field(CueCodeNumber; Rec.CodeNumber) { }
                field(CueIntValue; Rec.IntValue) { }
                field(CueIntZero; Rec.IntZero) { }
                field(CueDecValue; Rec.DecValue) { }
                field(CueDecBig; Rec.DecBig) { }
                field(CueBoolValue; Rec.BoolValue) { }
                field(CueDayValue; Rec.DayValue) { }
                field(CueBigValue; Rec.BigValue) { }
                field(CueOptValue; Rec.OptValue) { }
                field(CueAtValue; Rec.AtValue) { }
                field(CueStampValue; Rec.StampValue) { }
                field(CueGuidValue; Rec.GuidValue) { }
                field(CueKindValue; Rec.KindValue) { }
                field(CueSpanValue; Rec.SpanValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideCodeWord; Rec.CodeWord) { }
                field(WideIntValue; Rec.IntValue) { }
            }
        }
    }
}

// The cuegroups of page 97712 on a ListPart page, after a repeater over the rows.
page 97681 "LT GFI Cue List Part"
{
    PageType = ListPart;
    SourceTable = "LT GFI Cue Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(RowNo; Rec."No.") { }
            }
            cuegroup(Cues)
            {
                field(CueTextWord; Rec.TextWord) { }
                field(CueTextNumber; Rec.TextNumber) { }
                field(CueCodeWord; Rec.CodeWord) { }
                field(CueCodeNumber; Rec.CodeNumber) { }
                field(CueIntValue; Rec.IntValue) { }
                field(CueIntZero; Rec.IntZero) { }
                field(CueDecValue; Rec.DecValue) { }
                field(CueDecBig; Rec.DecBig) { }
                field(CueBoolValue; Rec.BoolValue) { }
                field(CueDayValue; Rec.DayValue) { }
                field(CueBigValue; Rec.BigValue) { }
                field(CueOptValue; Rec.OptValue) { }
                field(CueAtValue; Rec.AtValue) { }
                field(CueStampValue; Rec.StampValue) { }
                field(CueGuidValue; Rec.GuidValue) { }
                field(CueKindValue; Rec.KindValue) { }
                field(CueSpanValue; Rec.SpanValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideCodeWord; Rec.CodeWord) { }
                field(WideIntValue; Rec.IntValue) { }
            }
        }
    }
}

// The cuegroups of page 97712 on a List page, after a repeater over the rows.
page 97682 "LT GFI Cue List"
{
    PageType = List;
    SourceTable = "LT GFI Cue Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(RowNo; Rec."No.") { }
            }
            cuegroup(Cues)
            {
                field(CueTextWord; Rec.TextWord) { }
                field(CueTextNumber; Rec.TextNumber) { }
                field(CueCodeWord; Rec.CodeWord) { }
                field(CueCodeNumber; Rec.CodeNumber) { }
                field(CueIntValue; Rec.IntValue) { }
                field(CueIntZero; Rec.IntZero) { }
                field(CueDecValue; Rec.DecValue) { }
                field(CueDecBig; Rec.DecBig) { }
                field(CueBoolValue; Rec.BoolValue) { }
                field(CueDayValue; Rec.DayValue) { }
                field(CueBigValue; Rec.BigValue) { }
                field(CueOptValue; Rec.OptValue) { }
                field(CueAtValue; Rec.AtValue) { }
                field(CueStampValue; Rec.StampValue) { }
                field(CueGuidValue; Rec.GuidValue) { }
                field(CueKindValue; Rec.KindValue) { }
                field(CueSpanValue; Rec.SpanValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideCodeWord; Rec.CodeWord) { }
                field(WideIntValue; Rec.IntValue) { }
            }
        }
    }
}

// The cuegroups of page 97712 on a RoleCenter page.
page 97683 "LT GFI Cue Role"
{
    PageType = RoleCenter;
    SourceTable = "LT GFI Cue Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            cuegroup(Cues)
            {
                field(CueTextWord; Rec.TextWord) { }
                field(CueTextNumber; Rec.TextNumber) { }
                field(CueCodeWord; Rec.CodeWord) { }
                field(CueCodeNumber; Rec.CodeNumber) { }
                field(CueIntValue; Rec.IntValue) { }
                field(CueIntZero; Rec.IntZero) { }
                field(CueDecValue; Rec.DecValue) { }
                field(CueDecBig; Rec.DecBig) { }
                field(CueBoolValue; Rec.BoolValue) { }
                field(CueDayValue; Rec.DayValue) { }
                field(CueBigValue; Rec.BigValue) { }
                field(CueOptValue; Rec.OptValue) { }
                field(CueAtValue; Rec.AtValue) { }
                field(CueStampValue; Rec.StampValue) { }
                field(CueGuidValue; Rec.GuidValue) { }
                field(CueKindValue; Rec.KindValue) { }
                field(CueSpanValue; Rec.SpanValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideCodeWord; Rec.CodeWord) { }
                field(WideIntValue; Rec.IntValue) { }
            }
        }
    }
}

// A Document page with the issue's two fields.
page 97684 "LT GFI Doc"
{
    PageType = Document;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }
}

// GetField with an id that is on no page, on the page kinds the cells above do not open (al-runner issue 8887): a Worksheet, a
// ListPlus, a CardPart a TestPart shows, and a page without PageType.

page 97685 "LT GFI Worksheet"
{
    PageType = Worksheet;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }
}

page 97686 "LT GFI ListPlus"
{
    PageType = ListPlus;
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
}

page 97687 "LT GFI Card Part"
{
    PageType = CardPart;
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
}

page 97688 "LT GFI Part Host"
{
    PageType = Card;
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
            part(Part; "LT GFI Card Part")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
        }
    }
}

page 97689 "LT GFI No Type"
{
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
}

codeunit 97691 "LT GFI Missing Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Row: Record "LT GFI Row";
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Insert();
        Commit();
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

    local procedure Failure(): Text
    begin
        exit('error ' + GetLastErrorCode() + ': ' + GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryWorksheet(var Pg: TestPage "LT GFI Worksheet"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryListPlus(var Pg: TestPage "LT GFI ListPlus"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryPart(var Pg: TestPage "LT GFI Part Host"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.Part.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryNoType(var Pg: TestPage "LT GFI No Type"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    // A Worksheet: GetField(1), an id on no page.
    [Test]
    procedure Worksheet_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI Worksheet";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        if TryWorksheet(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A ListPlus.
    [Test]
    procedure ListPlus_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI ListPlus";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        if TryListPlus(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A CardPart the host's TestPart shows: GetField on the TestPart.
    [Test]
    procedure CardPartOfHost_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI Part Host";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        if TryPart(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A page without PageType.
    [Test]
    procedure NoPageType_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI No Type";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        if TryNoType(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    [TryFunction]
    local procedure TryRoleCenter(var Pg: TestPage "LT GFI Role Center"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryNavigatePage(var Pg: TestPage "LT GFI Navigate"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryStandardDialog(var Pg: TestPage "LT GFI Dialog"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryConfirmationDialog(var Pg: TestPage "LT GFI Confirm"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryHeadlinePart(var Pg: TestPage "LT GFI Headline"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    // A RoleCenter.
    [Test]
    procedure RoleCenter_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI Role Center";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenView();
        if TryRoleCenter(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A NavigatePage.
    [Test]
    procedure NavigatePage_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI Navigate";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        if TryNavigatePage(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A StandardDialog.
    [Test]
    procedure StandardDialog_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI Dialog";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        if TryStandardDialog(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A ConfirmationDialog.
    [Test]
    procedure ConfirmationDialog_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI Confirm";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenEdit();
        if TryConfirmationDialog(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A HeadlinePart.
    [Test]
    procedure HeadlinePart_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI Headline";
        Shown: Text;
    begin
        Initialize();
        Pg.OpenView();
        if TryHeadlinePart(Pg, 1, Shown) then
            Note('missing', 'found ' + Shown)
        else
            Note('missing', Failure());
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;
}

// The other page kinds a TestPage opens, for GetField with an id that is on no page (al-runner issue 8887).

page 97690 "LT GFI Role Center"
{
    PageType = RoleCenter;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(RoleCenter)
        {
            part(Part; "LT GFI Card Part") { ApplicationArea = All; }
        }
    }
}

page 97695 "LT GFI Navigate"
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
}

page 97696 "LT GFI Dialog"
{
    PageType = StandardDialog;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Title; Title) { ApplicationArea = All; }
            }
        }
    }

    var
        Title: Text;
}

page 97700 "LT GFI Confirm"
{
    PageType = ConfirmationDialog;
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
}

page 97699 "LT GFI Headline"
{
    PageType = HeadlinePart;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Title; Title) { ApplicationArea = All; }
            }
        }
    }

    var
        Title: Text;
}

// The cuegroups of page 97712 on a Document page (97734), and of the CardPart 97680 read through a Card that shows it as a part
// (97735); a DateFormula field in a cuegroup and a Boolean field in a cuegroup with CuegroupLayout = Wide on a Card (97736).
page 97734 "LT GFI Cue Doc"
{
    PageType = Document;
    SourceTable = "LT GFI Cue Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            cuegroup(Cues)
            {
                field(CueTextWord; Rec.TextWord) { }
                field(CueTextNumber; Rec.TextNumber) { }
                field(CueCodeWord; Rec.CodeWord) { }
                field(CueCodeNumber; Rec.CodeNumber) { }
                field(CueIntValue; Rec.IntValue) { }
                field(CueIntZero; Rec.IntZero) { }
                field(CueDecValue; Rec.DecValue) { }
                field(CueDecBig; Rec.DecBig) { }
                field(CueBoolValue; Rec.BoolValue) { }
                field(CueDayValue; Rec.DayValue) { }
                field(CueBigValue; Rec.BigValue) { }
                field(CueOptValue; Rec.OptValue) { }
                field(CueAtValue; Rec.AtValue) { }
                field(CueStampValue; Rec.StampValue) { }
                field(CueGuidValue; Rec.GuidValue) { }
                field(CueKindValue; Rec.KindValue) { }
                field(CueSpanValue; Rec.SpanValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideCodeWord; Rec.CodeWord) { }
                field(WideIntValue; Rec.IntValue) { }
            }
        }
    }
}

page 97735 "LT GFI Cue Part Host"
{
    PageType = Card;
    SourceTable = "LT GFI Cue Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            part(CuePart; "LT GFI Cue Part")
            {
                SubPageLink = "No." = field("No.");
            }
        }
    }
}

table 97735 "LT GFI Cue Extra Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; FormulaValue; DateFormula) { }
        field(3; FlagValue; Boolean) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 97736 "LT GFI Cue Extra"
{
    PageType = Card;
    SourceTable = "LT GFI Cue Extra Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            cuegroup(Cues)
            {
                field(CueFormulaValue; Rec.FormulaValue) { }
            }
            cuegroup(WideCues)
            {
                CuegroupLayout = Wide;

                field(WideFlagValue; Rec.FlagValue) { }
            }
        }
    }
}

codeunit 97734 "LT GFI Cue More Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Seen: Dictionary of [Text, Text];

    local procedure Initialize()
    var
        Row: Record "LT GFI Cue Row";
        Extra: Record "LT GFI Cue Extra Row";
    begin
        Row.DeleteAll();
        Extra.DeleteAll();
        Extra."No." := 'R1';
        Evaluate(Extra.FormulaValue, '<1D>');
        Extra.FlagValue := true;
        Extra.Insert();
        Commit();
        Clear(Seen);
    end;

    local procedure SeedCueRow()
    var
        Row: Record "LT GFI Cue Row";
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.TextWord := 'Alpha';
        Row.TextNumber := '42';
        Row.CodeWord := 'R1';
        Row.CodeNumber := '7';
        Row.IntValue := 5;
        Row.IntZero := 0;
        Row.DecValue := 1.5;
        Row.DecBig := 1234.5;
        Row.BoolValue := true;
        Row.DayValue := DMY2Date(3, 3, 2026);
        Row.BigValue := 77;
        Row.OptValue := Row.OptValue::Beta;
        Row.AtValue := 103000T;
        Row.StampValue := CreateDateTime(DMY2Date(3, 3, 2026), 103000T);
        Row.GuidValue := '{0F2E7C52-6E0B-4F51-9C3A-1B2C3D4E5F60}';
        Row.KindValue := Row.KindValue::Beta;
        Row.SpanValue := 90000;
        Row.Insert();
    end;

    local procedure Note(What: Text; Actual: Text)
    begin
        Seen.Set(What, Actual);
    end;

    local procedure Expect(What: Text; Expected: Text)
    begin
        Assert.AreEqual(Expected, Seen.Get(What), What);
    end;

    local procedure Failure(): Text
    begin
        exit('error ' + GetLastErrorCode() + ': ' + GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryDoc(var Pg: TestPage "LT GFI Cue Doc"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadDoc(var Pg: TestPage "LT GFI Cue Doc"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryDoc(Pg, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryExtra(var Pg: TestPage "LT GFI Cue Extra"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadExtra(var Pg: TestPage "LT GFI Cue Extra"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryExtra(Pg, Id, Shown) then
            exit(Shown);
        exit(Failure());
    end;

    // The cuegroups of CueTypes_ValueByNameAndById on a Document page: each field's Value by its name, then through GetField by
    // its id.
    [Test]
    procedure CueTypesDocument_ValueByNameAndById()
    var
        Pg: TestPage "LT GFI Cue Doc";
    begin
        Initialize();
        SeedCueRow();
        Pg.OpenView();
        Note('CueTextWord', Pg.CueTextWord.Value() + '|' + ReadDoc(Pg, 774306315));
        Note('CueTextNumber', Pg.CueTextNumber.Value() + '|' + ReadDoc(Pg, 1173125942));
        Note('CueCodeWord', Pg.CueCodeWord.Value() + '|' + ReadDoc(Pg, 1605813843));
        Note('CueCodeNumber', Pg.CueCodeNumber.Value() + '|' + ReadDoc(Pg, 1682521106));
        Note('CueIntValue', Pg.CueIntValue.Value() + '|' + ReadDoc(Pg, 1446471016));
        Note('CueIntZero', Pg.CueIntZero.Value() + '|' + ReadDoc(Pg, 1555023967));
        Note('CueDecValue', Pg.CueDecValue.Value() + '|' + ReadDoc(Pg, 928580449));
        Note('CueDecBig', Pg.CueDecBig.Value() + '|' + ReadDoc(Pg, 382669622));
        Note('CueBoolValue', Pg.CueBoolValue.Value() + '|' + ReadDoc(Pg, 671714541));
        Note('CueDayValue', Pg.CueDayValue.Value() + '|' + ReadDoc(Pg, 1733444401));
        Note('CueBigValue', Pg.CueBigValue.Value() + '|' + ReadDoc(Pg, 2012819407));
        Note('CueOptValue', Pg.CueOptValue.Value() + '|' + ReadDoc(Pg, 1432903760));
        Note('CueAtValue', Pg.CueAtValue.Value() + '|' + ReadDoc(Pg, 1712542342));
        Note('CueStampValue', Pg.CueStampValue.Value() + '|' + ReadDoc(Pg, 761028584));
        Note('CueGuidValue', Pg.CueGuidValue.Value() + '|' + ReadDoc(Pg, 25729340));
        Note('CueKindValue', Pg.CueKindValue.Value() + '|' + ReadDoc(Pg, 1342154251));
        Note('CueSpanValue', Pg.CueSpanValue.Value() + '|' + ReadDoc(Pg, 227380503));
        Note('WideCodeWord', Pg.WideCodeWord.Value() + '|' + ReadDoc(Pg, 140578137));
        Note('WideIntValue', Pg.WideIntValue.Value() + '|' + ReadDoc(Pg, 331060748));
        Pg.Close();
        Expect('CueTextWord', '0|0');
        Expect('CueTextNumber', '0|0');
        Expect('CueCodeWord', '0|0');
        Expect('CueCodeNumber', '0|0');
        Expect('CueIntValue', '5|5');
        Expect('CueIntZero', '0|0');
        Expect('CueDecValue', '1.50|1.50');
        Expect('CueDecBig', '1,234.50|1,234.50');
        Expect('CueBoolValue', '0|0');
        Expect('CueDayValue', '0|0');
        Expect('CueBigValue', '0|0');
        Expect('CueOptValue', '1|1');
        Expect('CueAtValue', '0|0');
        Expect('CueStampValue', '0|0');
        Expect('CueGuidValue', '0|0');
        Expect('CueKindValue', '1|1');
        Expect('CueSpanValue', '0|0');
        Expect('WideCodeWord', '0|0');
        Expect('WideIntValue', '5|5');
    end;

    // The same cuegroups on the CardPart 97680, read through the Card 97735 that shows it: each field's Value by its name.
    [Test]
    procedure CueTypesPartThroughHost_ValueByName()
    var
        Host: TestPage "LT GFI Cue Part Host";
    begin
        Initialize();
        SeedCueRow();
        Host.OpenView();
        Note('CueTextWord', Host.CuePart.CueTextWord.Value());
        Note('CueTextNumber', Host.CuePart.CueTextNumber.Value());
        Note('CueCodeWord', Host.CuePart.CueCodeWord.Value());
        Note('CueCodeNumber', Host.CuePart.CueCodeNumber.Value());
        Note('CueIntValue', Host.CuePart.CueIntValue.Value());
        Note('CueIntZero', Host.CuePart.CueIntZero.Value());
        Note('CueDecValue', Host.CuePart.CueDecValue.Value());
        Note('CueDecBig', Host.CuePart.CueDecBig.Value());
        Note('CueBoolValue', Host.CuePart.CueBoolValue.Value());
        Note('CueDayValue', Host.CuePart.CueDayValue.Value());
        Note('CueBigValue', Host.CuePart.CueBigValue.Value());
        Note('CueOptValue', Host.CuePart.CueOptValue.Value());
        Note('CueAtValue', Host.CuePart.CueAtValue.Value());
        Note('CueStampValue', Host.CuePart.CueStampValue.Value());
        Note('CueGuidValue', Host.CuePart.CueGuidValue.Value());
        Note('CueKindValue', Host.CuePart.CueKindValue.Value());
        Note('CueSpanValue', Host.CuePart.CueSpanValue.Value());
        Note('WideCodeWord', Host.CuePart.WideCodeWord.Value());
        Note('WideIntValue', Host.CuePart.WideIntValue.Value());
        Host.Close();
        Expect('CueTextWord', '0');
        Expect('CueTextNumber', '0');
        Expect('CueCodeWord', '0');
        Expect('CueCodeNumber', '0');
        Expect('CueIntValue', '5');
        Expect('CueIntZero', '0');
        Expect('CueDecValue', '1.50');
        Expect('CueDecBig', '1,234.50');
        Expect('CueBoolValue', '0');
        Expect('CueDayValue', '0');
        Expect('CueBigValue', '0');
        Expect('CueOptValue', '1');
        Expect('CueAtValue', '0');
        Expect('CueStampValue', '0');
        Expect('CueGuidValue', '0');
        Expect('CueKindValue', '1');
        Expect('CueSpanValue', '0');
        Expect('WideCodeWord', '0');
        Expect('WideIntValue', '5');
    end;

    // A DateFormula field in a cuegroup and a Boolean field in a wide cuegroup on a Card: by name, then through GetField by id.
    [Test]
    procedure CueDateFormulaAndWideBoolean_ValueByNameAndById()
    var
        Pg: TestPage "LT GFI Cue Extra";
    begin
        Initialize();
        Pg.OpenView();
        Note('FormulaValue', Pg.CueFormulaValue.Value() + '|' + ReadExtra(Pg, 1631619223));
        Note('WideFlagValue', Pg.WideFlagValue.Value() + '|' + ReadExtra(Pg, 1182527305));
        Pg.Close();
        Expect('FormulaValue', '0|0');
        Expect('WideFlagValue', '0|0');
    end;
}

// The ids around the other page kinds (al-runner issue 8887, review of the runner's pull request 8902): an action, an action
// group and a page extension's field on a Worksheet, a host control's id through the TestPart of a CardPart, the CardPart a
// RoleCenter shows, a missing id on an API page, a PromptDialog, a ReportPreview and a ReportProcessingOnly page, and a control's
// own id on a Worksheet, a ListPlus and a CardPart a TestPart shows.
page 97737 "LT GFI Action Sheet"
{
    PageType = Worksheet;
    SourceTable = "LT GFI Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Act1)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                end;
            }
            group(ActGroup)
            {
                action(Act2)
                {
                    ApplicationArea = All;

                    trigger OnAction()
                    begin
                    end;
                }
            }
        }
    }
}

pageextension 97738 "LT GFI Action Sheet Ext" extends "LT GFI Action Sheet"
{
    layout
    {
        addlast(Lines)
        {
            field(ExtNote; Rec.Note) { ApplicationArea = All; }
        }
    }
}

page 97739 "LT GFI Api"
{
    PageType = API;
    APIPublisher = 'ltcorpus';
    APIGroup = 'gfi';
    APIVersion = 'v1.0';
    EntityName = 'gfiRow';
    EntitySetName = 'gfiRows';
    SourceTable = "LT GFI Row";
    DelayedInsert = true;
    ODataKeyFields = SystemId;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(name; Rec.Name) { }
            }
        }
    }
}

page 97740 "LT GFI Prompt"
{
    PageType = PromptDialog;
    Extensible = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Prompt)
        {
            field(Input; InputText) { ApplicationArea = All; }
        }
        area(Content)
        {
            field(Output; OutputText) { ApplicationArea = All; }
        }
    }

    var
        InputText: Text;
        OutputText: Text;
}

page 97741 "LT GFI Report Preview"
{
    PageType = ReportPreview;
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
}

page 97742 "LT GFI Processing Only"
{
    PageType = ReportProcessingOnly;
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
}

codeunit 97737 "LT GFI Kinds Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        Seen: Dictionary of [Text, Text];

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

    local procedure Failure(): Text
    begin
        exit('error ' + GetLastErrorCode() + ': ' + GetLastErrorText());
    end;

    [TryFunction]
    local procedure TrySheet(var Pg: TestPage "LT GFI Action Sheet"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadSheet(var Pg: TestPage "LT GFI Action Sheet"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TrySheet(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryApi(var Pg: TestPage "LT GFI Api"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadApi(var Pg: TestPage "LT GFI Api"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryApi(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryPrompt(var Pg: TestPage "LT GFI Prompt"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadPrompt(var Pg: TestPage "LT GFI Prompt"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryPrompt(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryPreview(var Pg: TestPage "LT GFI Report Preview"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadPreview(var Pg: TestPage "LT GFI Report Preview"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryPreview(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryOnly(var Pg: TestPage "LT GFI Processing Only"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadOnly(var Pg: TestPage "LT GFI Processing Only"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryOnly(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryHost(var Pg: TestPage "LT GFI Part Host"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.Part.GetField(Id).Value();
    end;

    local procedure ReadHost(var Pg: TestPage "LT GFI Part Host"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryHost(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryRole(var Pg: TestPage "LT GFI Role Center"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.Part.GetField(Id).Value();
    end;

    local procedure ReadRole(var Pg: TestPage "LT GFI Role Center"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryRole(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryWork(var Pg: TestPage "LT GFI Worksheet"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadWork(var Pg: TestPage "LT GFI Worksheet"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryWork(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    [TryFunction]
    local procedure TryPlus(var Pg: TestPage "LT GFI ListPlus"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    local procedure ReadPlus(var Pg: TestPage "LT GFI ListPlus"; Id: Integer): Text
    var
        Shown: Text;
    begin
        ClearLastError();
        if TryPlus(Pg, Id, Shown) then
            exit('found ' + Shown);
        exit(Failure());
    end;

    // A Worksheet: the ids of an action, an action group and an action in it.
    [Test]
    procedure Worksheet_GetFieldActionIds_NotFound()
    var
        Pg: TestPage "LT GFI Action Sheet";
    begin
        Initialize();
        Pg.OpenEdit();
        Note('action', ReadSheet(Pg, 1754972975));
        Note('group', ReadSheet(Pg, 1979372073));
        Note('inner', ReadSheet(Pg, 158116876));
        Pg.Close();
        Expect('action', 'error TestFieldNotFound: The field with ID = 1754972975 is not found on the page.');
        Expect('group', 'error TestFieldNotFound: The field with ID = 1979372073 is not found on the page.');
        Expect('inner', 'error TestFieldNotFound: The field with ID = 158116876 is not found on the page.');
    end;

    // A Worksheet: a page extension's field by the base page's id and by the extension's id.
    [Test]
    procedure Worksheet_GetFieldExtensionFieldIds()
    var
        Pg: TestPage "LT GFI Action Sheet";
    begin
        Initialize();
        Pg.OpenEdit();
        Note('base', ReadSheet(Pg, 2109999191));
        Note('ext', ReadSheet(Pg, 853341002));
        Pg.Close();
        Expect('base', 'error TestFieldNotFound: The field with ID = 2109999191 is not found on the page.');
        Expect('ext', 'found n1');
    end;

    // The CardPart a Card shows: GetField on the TestPart with the id of the host's own Name control.
    [Test]
    procedure CardPartOfHost_GetFieldHostControlId_NotFound()
    var
        Pg: TestPage "LT GFI Part Host";
    begin
        Initialize();
        Pg.OpenEdit();
        Note('host', ReadHost(Pg, 1524274214));
        Pg.Close();
        Expect('host', 'error TestFieldNotFound: The field with ID = 1524274214 is not found on the page.');
    end;

    // The CardPart a RoleCenter shows: GetField(1) on the TestPart.
    [Test]
    procedure RoleCenterPart_GetFieldMissingId_NotFound()
    var
        Pg: TestPage "LT GFI Role Center";
    begin
        Initialize();
        Pg.OpenView();
        Note('missing', ReadRole(Pg, 1));
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // An API page.
    [Test]
    procedure Api_GetFieldMissingId()
    var
        Pg: TestPage "LT GFI Api";
    begin
        Initialize();
        Pg.OpenView();
        Note('missing', ReadApi(Pg, 1));
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A PromptDialog.
    [Test]
    procedure PromptDialog_GetFieldMissingId()
    var
        Pg: TestPage "LT GFI Prompt";
    begin
        Initialize();
        Pg.OpenEdit();
        Note('missing', ReadPrompt(Pg, 1));
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A ReportPreview page. BC 27 and 28 raise on its open; BC 29 opens it.
    [Test]
    procedure ReportPreview_GetFieldMissingId()
    var
        Pg: TestPage "LT GFI Report Preview";
    begin
        Initialize();
#if BC29PLUS
        Pg.OpenEdit();
        Note('missing', ReadPreview(Pg, 1));
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
#else
        // BC 27 and 28: the open raises, so GetField is never reached.
        asserterror Pg.OpenEdit();
        Note('open', GetLastErrorText());
        Expect('open', 'The identifier ''ReportLayoutFormat'' could not be found.');
#endif
    end;

    // A ReportProcessingOnly page.
    [Test]
    procedure ReportProcessingOnly_GetFieldMissingId()
    var
        Pg: TestPage "LT GFI Processing Only";
    begin
        Initialize();
        Pg.OpenEdit();
        Note('missing', ReadOnly(Pg, 1));
        Pg.Close();
        Expect('missing', 'error TestFieldNotFound: The field with ID = 1 is not found on the page.');
    end;

    // A control's own id: the Name control of the Worksheet 97685.
    [Test]
    procedure Worksheet_GetFieldOwnId()
    var
        Pg: TestPage "LT GFI Worksheet";
    begin
        Initialize();
        Pg.OpenEdit();
        Note('own', ReadWork(Pg, 2044414625));
        Pg.Close();
        Expect('own', 'found Alpha');
    end;

    // The Name control of the ListPlus 97686.
    [Test]
    procedure ListPlus_GetFieldOwnId()
    var
        Pg: TestPage "LT GFI ListPlus";
    begin
        Initialize();
        Pg.OpenEdit();
        Note('own', ReadPlus(Pg, 1633031500));
        Pg.Close();
        Expect('own', 'found Alpha');
    end;

    // The Name control of the CardPart 97687 the Card 97688 shows, through the TestPart.
    [Test]
    procedure CardPartOfHost_GetFieldOwnId()
    var
        Pg: TestPage "LT GFI Part Host";
    begin
        Initialize();
        Pg.OpenEdit();
        Note('own', ReadHost(Pg, 1480767443));
        Pg.Close();
        Expect('own', 'found Alpha');
    end;
}
