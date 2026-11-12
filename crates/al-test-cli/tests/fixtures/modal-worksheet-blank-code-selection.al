// BC Documentation: https://learn.microsoft.com/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-update-method
// Scope: Cloud-compatible standalone repro
// BC versions: 27.5, 28.5, 29.0
// local object: The Worksheet source view, action, and filter group define the measured shape.

table 50820 "Refresh Probe Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Code[10]) { }
        field(2; Batch; Code[10]) { }
        field(3; VisibleKind; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50821 "Refresh Probe Worksheet"
{
    PageType = Worksheet;
    SourceTable = "Refresh Probe Row";
    SourceTableView = where(VisibleKind = const(true));
    DelayedInsert = true;
    SaveValues = true;
    InsertAllowed = false;
    ApplicationArea = All;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(ImportRow)
            {
                ApplicationArea = All;
                trigger OnAction()
                var
                    Row: Record "Refresh Probe Row";
                begin
                    Row.Id := 'SEVEN';
                    Row.Batch := 'SELECTED';
                    Row.VisibleKind := true;
                    Row.Insert();
                    CurrPage.Update(false);
                end;
            }
            action(CheckSelection)
            {
                ApplicationArea = All;
                trigger OnAction()
                var
                    Selected: Record "Refresh Probe Row";
                begin
                    CurrPage.SetSelectionFilter(Selected);
                    if (Selected.Count() <> 0) or (Selected.GetRangeMin(Id) <> '') then
                        Error('Unexpected selection: %1|%2', Selected.Count(), Selected.GetView(false));
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.FilterGroup(2);
        Rec.SetRange(Batch, 'SELECTED');
        Rec.FilterGroup(0);
        if Rec.FindFirst() then;
    end;
}

codeunit 50822 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('EmptyWorksheetHandler')]
    procedure ModalWorksheetWithNoMatchingRows()
    begin
        Initialize();
        Page.RunModal(Page::"Refresh Probe Worksheet");
    end;

    [ModalPageHandler]
    procedure EmptyWorksheetHandler(var Worksheet: TestPage "Refresh Probe Worksheet")
    begin
        Worksheet.CheckSelection.Invoke();
    end;

    local procedure Initialize()
    var
        Row: Record "Refresh Probe Row";
    begin
        Row.DeleteAll();
        Row.Id := 'ONE';
        Row.Batch := 'HIDDEN';
        Row.VisibleKind := true;
        Row.Insert();
    end;
}
