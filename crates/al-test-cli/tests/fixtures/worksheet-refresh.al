// BC Documentation: https://learn.microsoft.com/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-update-method
// Scope: Cloud-compatible standalone repro
// BC versions: 27.5, 28.5, 29.0
// local object: The Worksheet source view, action, and filter group define the measured shape.

table 50120 "Refresh Probe Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Batch; Code[10]) { }
        field(3; VisibleKind; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50121 "Refresh Probe Worksheet"
{
    PageType = Worksheet;
    SourceTable = "Refresh Probe Row";
    SourceTableView = where(VisibleKind = const(true));
    DelayedInsert = true;
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
                    Row.Id := 7;
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
                    if (Selected.Count() <> 1) or not Selected.FindFirst() then
                        Error('Expected one selected row. Got %1.', Selected.Count());
                    Selected.TestField(Id, 7);
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

codeunit 50122 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TestPage_Update_NewWorksheetRow_IsSelectable()
    var
        Worksheet: TestPage "Refresh Probe Worksheet";
    begin
        Initialize();
        Worksheet.OpenEdit();
        Worksheet.ImportRow.Invoke();
        Worksheet.CheckSelection.Invoke();
        Worksheet.Close();
    end;


    [Test]
    procedure TestPage_First_AfterWorksheetUpdate_FindsImportedRow()
    var
        Worksheet: TestPage "Refresh Probe Worksheet";
    begin
        Initialize();
        Worksheet.OpenEdit();
        Worksheet.ImportRow.Invoke();
        if not Worksheet.First() then
            Error('Expected to find the imported row after Update.');
        Worksheet.CheckSelection.Invoke();
        Worksheet.Close();
    end;

    local procedure Initialize()
    var
        Row: Record "Refresh Probe Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Batch := 'HIDDEN';
        Row.VisibleKind := true;
        Row.Insert();
    end;
}
