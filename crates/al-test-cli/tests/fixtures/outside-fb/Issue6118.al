// The reporter's repro of al-runner issue 6118 (green on real BC), app and test files in one source.
table 50100 "Repro Planner Setting 50100"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Resource No."; Code[20]) { }
    }
    keys
    {
        key(PK; "Primary Key") { }
    }
}

table 50101 "Repro Resource 50101"
{
    LookupPageId = "Repro Resource List 50102";

    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { }
    }
}

page 50102 "Repro Resource List 50102"
{
    PageType = List;
    SourceTable = "Repro Resource 50101";
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

page 50103 "Repro Resource Part 50103"
{
    PageType = ListPart;
    SourceTable = "Repro Resource 50101";
    ApplicationArea = All;
    Editable = false;

    layout
    {
        area(Content)
        {
            field(Scope; Scope)
            {
                ApplicationArea = All;
                Caption = 'Scope';
            }
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
        }
    }

    procedure ShowScope(NewScope: Code[20])
    begin
        Scope := NewScope;
        CurrPage.Update(false);
    end;

    var
        Scope: Code[20];
}

page 50104 "Repro Schedule 50104"
{
    PageType = List;
    SourceTable = "Repro Resource 50101";
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
                        Resource: Record "Repro Resource 50101";
                    begin
                        if Page.RunModal(Page::"Repro Resource List 50102", Resource) <> Action::LookupOK then
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
            }
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
            part(Resources; "Repro Resource Part 50103")
            {
                ApplicationArea = All;
            }
        }
    }

    trigger OnOpenPage()
    begin
        if not Setting.Get() then
            Setting.Insert();
    end;

    var
        Setting: Record "Repro Planner Setting 50100";
}

page 50105 "Repro Part Upd Schedule 50105"
{
    PageType = List;
    SourceTable = "Repro Resource 50101";
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
                        Resource: Record "Repro Resource 50101";
                    begin
                        if Page.RunModal(Page::"Repro Resource List 50102", Resource) <> Action::LookupOK then
                            exit(false);
                        Text := Resource."No.";
                        exit(true);
                    end;

                    trigger OnValidate()
                    begin
                        Setting.Modify();
                        CurrPage.Resources.Page.ShowScope(Setting."Resource No.");
                        CurrPage.Update(false);
                    end;
                }
            }
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
            }
            part(Resources; "Repro Resource Part 50103")
            {
                ApplicationArea = All;
            }
        }
    }

    trigger OnOpenPage()
    begin
        if not Setting.Get() then
            Setting.Insert();
    end;

    var
        Setting: Record "Repro Planner Setting 50100";
}

codeunit 50110 "Repro Lookup Part Update Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupShowsThePickedResourceWhenOnValidateUpdatesThePart()
    var
        Schedule: TestPage "Repro Part Upd Schedule 50105";
    begin
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        if Schedule.ResourceNo.Value() <> 'R2' then
            Error('Resource No. shows [%1] after the lookup.', Schedule.ResourceNo.Value());
        Schedule.Close();
    end;

    [Test]
    [HandlerFunctions('PickSecondResource')]
    procedure LookupShowsThePickedResourceWhenThePartIsNotUpdated()
    var
        Schedule: TestPage "Repro Schedule 50104";
    begin
        Setup();
        Schedule.OpenEdit();
        Schedule.ResourceNo.Lookup();
        if Schedule.ResourceNo.Value() <> 'R2' then
            Error('Resource No. shows [%1] after the lookup.', Schedule.ResourceNo.Value());
        Schedule.Close();
    end;

    [ModalPageHandler]
    procedure PickSecondResource(var ResourceList: TestPage "Repro Resource List 50102")
    begin
        ResourceList.GoToKey('R2');
        ResourceList.OK().Invoke();
    end;

    local procedure Setup()
    var
        Setting: Record "Repro Planner Setting 50100";
        Resource: Record "Repro Resource 50101";
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

