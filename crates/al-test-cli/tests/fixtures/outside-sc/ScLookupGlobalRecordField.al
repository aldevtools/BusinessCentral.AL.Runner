// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/testfield/testfield-lookup-method
// Scope: in-scope
// Fixtures used: own tables 89004 and 89005, pages 89006 and 89007, test codeunit 89008
// Probe: from al-runner issue 5843 (reporter's real-BC run on 29.0.54011.55407); every literal asserted below is what
//   BC produced. Other BC versions are checked by the corpus CI matrix.
// Facts: TestField.Lookup() on a header control bound to a field of a page global record (Setting."Resource No."),
//   which OnOpenPage reads with Get or inserts. The control's OnLookup runs a list page modally and answers true with
//   the picked "No." in Text; its OnValidate does Setting.Modify() and CurrPage.Update(false). Read after the Lookup:
//   the control's Value() and, after Close, the saved setting row. Also: the same OnLookup cancelled (answers false
//   without touching Text), an OnValidate without CurrPage.Update, and an OnLookup that sets Text and answers false.
// BC versions: all (CI)
// Neighbours: pages/run-modal/TestFieldLookupTriggers.al pins Lookup on controls bound to Rec fields.

table 89004 "LT SC Planner Setting"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Resource No."; Code[20]) { }
        field(3; "Resource No. 2"; Code[20]) { }
        field(4; "Resource No. 3"; Code[20]) { }
    }
    keys
    {
        key(PK; "Primary Key") { }
    }
}

table 89005 "LT SC Resource"
{
    DataClassification = SystemMetadata;
    LookupPageId = "LT SC Resource List";

    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { }
    }
}

page 89006 "LT SC Resource List"
{
    PageType = List;
    SourceTable = "LT SC Resource";
    ApplicationArea = All;
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }
}

page 89007 "LT SC Schedule"
{
    PageType = List;
    SourceTable = "LT SC Resource";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(Filters)
            {
                field(ResourceNo; Setting."Resource No.")
                {
                    ApplicationArea = All;
                    Caption = 'Resource No.';

                    trigger OnLookup(var Text: Text): Boolean
                    var
                        Resource: Record "LT SC Resource";
                    begin
                        if Page.RunModal(Page::"LT SC Resource List", Resource) <> Action::LookupOK then
                            exit(false);
                        Text := Resource."No.";
                        exit(true);
                    end;

                    trigger OnValidate()
                    begin
                        Setting.Modify();
                        CurrPage.Update(false);
                    end;
                }
                field(ResourceNoNoUpdate; Setting."Resource No. 2")
                {
                    ApplicationArea = All;
                    Caption = 'Resource No. 2';

                    trigger OnLookup(var Text: Text): Boolean
                    var
                        Resource: Record "LT SC Resource";
                    begin
                        if Page.RunModal(Page::"LT SC Resource List", Resource) <> Action::LookupOK then
                            exit(false);
                        Text := Resource."No.";
                        exit(true);
                    end;

                    trigger OnValidate()
                    begin
                        Setting.Modify();
                    end;
                }
                field(ResourceNoFalse; Setting."Resource No. 3")
                {
                    ApplicationArea = All;
                    Caption = 'Resource No. 3';

                    trigger OnLookup(var Text: Text): Boolean
                    begin
                        Text := 'R3';
                        exit(false);
                    end;

                    trigger OnValidate()
                    begin
                        Setting.Modify();
                        CurrPage.Update(false);
                    end;
                }
            }
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    begin
        if not Setting.Get() then
            Setting.Insert();
    end;

    var
        Setting: Record "LT SC Planner Setting";
}

codeunit 89008 "LT SC Lookup Global Rec Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupShowsThePickedResource()
    var
        Schedule: TestPage "LT SC Schedule";
    begin
        // [GIVEN] OnLookup answers true with R2 in Text; OnValidate modifies and calls CurrPage.Update(false)
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        // [THEN] the control shows the picked value
        Assert.AreEqual('R2', Schedule.ResourceNo.Value(), 'ResourceNo after the lookup');
        Schedule.Close();
    end;

    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupSavesThePickedResource()
    var
        Setting: Record "LT SC Planner Setting";
        Schedule: TestPage "LT SC Schedule";
    begin
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        Schedule.Close();
        // [THEN] OnValidate ran with the picked value
        Setting.Get();
        Assert.AreEqual('R2', Setting."Resource No.", 'Setting after the lookup');
    end;

    [Test]
    [HandlerFunctions('CancelResource')]
    procedure LookupCancelledKeepsTheValue()
    var
        Setting: Record "LT SC Planner Setting";
        Schedule: TestPage "LT SC Schedule";
        Shown: Text;
    begin
        // [GIVEN] the setting holds R1; the modal page is cancelled, so OnLookup answers false
        Setup();
        Setting."Resource No." := 'R1';
        Setting.Insert();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        Shown := Schedule.ResourceNo.Value();
        Schedule.Close();
        Setting.Get();
        // [THEN] the control and the row keep R1
        Assert.AreEqual('shows=[R1] saved=[R1]', 'shows=[' + Shown + '] saved=[' + Setting."Resource No." + ']', 'cancelled lookup');
    end;

    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupWithoutUpdateShowsThePickedResource()
    var
        Setting: Record "LT SC Planner Setting";
        Schedule: TestPage "LT SC Schedule";
        Shown: Text;
    begin
        // [GIVEN] OnLookup answers true with R2; OnValidate modifies only (no CurrPage.Update)
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNoNoUpdate.Lookup();
        Shown := Schedule.ResourceNoNoUpdate.Value();
        Schedule.Close();
        Setting.Get();
        Assert.AreEqual('shows=[R2] saved=[R2]', 'shows=[' + Shown + '] saved=[' + Setting."Resource No. 2" + ']', 'lookup without Update');
    end;

    [Test]
    procedure LookupAnsweringFalseWithTextSetIgnoresTheText()
    var
        Setting: Record "LT SC Planner Setting";
        Schedule: TestPage "LT SC Schedule";
        Shown: Text;
    begin
        // [GIVEN] OnLookup sets Text to R3 and answers false
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNoFalse.Lookup();
        Shown := Schedule.ResourceNoFalse.Value();
        Schedule.Close();
        Setting.Get();
        Assert.AreEqual('shows=[] saved=[]', 'shows=[' + Shown + '] saved=[' + Setting."Resource No. 3" + ']', 'lookup answering false');
    end;

    [ModalPageHandler]
    procedure PickSecondResource(var ResourceList: TestPage "LT SC Resource List")
    begin
        ResourceList.GoToKey('R2');
        ResourceList.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure CancelResource(var ResourceList: TestPage "LT SC Resource List")
    begin
        ResourceList.GoToKey('R2');
        ResourceList.Cancel().Invoke();
    end;

    local procedure Setup()
    var
        Setting: Record "LT SC Planner Setting";
        Resource: Record "LT SC Resource";
    begin
        Setting.DeleteAll();
        Resource.DeleteAll();
        Resource."No." := 'R1';
        Resource.Insert();
        Resource."No." := 'R2';
        Resource.Insert();
        Resource."No." := 'R3';
        Resource.Insert();
    end;
}
