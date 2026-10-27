//! A field's OnDrillDown sets marks and MarkedOnly on Rec and calls `CurrPage.Update(false)` on a Worksheet with
//! OnAfterGetRecord and OnAfterGetCurrRecord that has a part, in the content area or in the FactBoxes area: the page then lists
//! only the marked row, as the same page without a part does (al-runner issue 5886, the reporter's run on real BC
//! 29.0.54011.55407, 6/6 tests; corpus writer SF's pins in pages/lt-sc-outside-pages/ScDrillDownMarkedOnly.al SF_WithPart_*,
//! SF_WithFactBox_*). The fixture is the reporter's repro; each procedure raises an error when the page shows the wrong rows.

const SRC: &str = r##"table 50100 "Repro Operation"
{
    fields
    {
        field(1; "Operation No."; Code[10]) { }
        field(2; "Order No."; Code[20]) { }
    }
    keys
    {
        key(PK; "Operation No.") { }
    }
}

table 50101 "Repro Component"
{
    fields
    {
        field(1; "Order No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
    }
    keys
    {
        key(PK; "Order No.", "Line No.") { }
    }
}

page 50102 "Repro Components Part"
{
    PageType = ListPart;
    SourceTable = "Repro Component";
    ApplicationArea = All;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }
}

page 50103 "Repro Ops With Part"
{
    PageType = Worksheet;
    SourceTable = "Repro Operation";
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;
    DeleteAllowed = false;

    layout
    {
        area(Content)
        {
            group(Tiles)
            {
                field(MarkedCount; MarkedCount)
                {
                    ApplicationArea = All;
                    Caption = 'Operation 10';
                    Editable = false;

                    trigger OnDrillDown()
                    begin
                        Rec.ClearMarks();
                        if Rec.Get('10') then
                            Rec.Mark(true);
                        Rec.MarkedOnly(true);
                        CurrPage.Update(false);
                    end;
                }
            }
            repeater(Lines)
            {
                field("Operation No."; Rec."Operation No.") { ApplicationArea = All; }
            }
            part(Components; "Repro Components Part")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("Order No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
    end;

    trigger OnAfterGetCurrRecord()
    begin
    end;

    var
        MarkedCount: Integer;
}

page 50104 "Repro Ops With FactBox"
{
    PageType = Worksheet;
    SourceTable = "Repro Operation";
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;
    DeleteAllowed = false;

    layout
    {
        area(Content)
        {
            group(Tiles)
            {
                field(MarkedCount; MarkedCount)
                {
                    ApplicationArea = All;
                    Caption = 'Operation 10';
                    Editable = false;

                    trigger OnDrillDown()
                    begin
                        Rec.ClearMarks();
                        if Rec.Get('10') then
                            Rec.Mark(true);
                        Rec.MarkedOnly(true);
                        CurrPage.Update(false);
                    end;
                }
            }
            repeater(Lines)
            {
                field("Operation No."; Rec."Operation No.") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Components; "Repro Components Part")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("Order No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
    end;

    trigger OnAfterGetCurrRecord()
    begin
    end;

    var
        MarkedCount: Integer;
}

page 50105 "Repro Ops Without Part"
{
    PageType = Worksheet;
    SourceTable = "Repro Operation";
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;
    DeleteAllowed = false;

    layout
    {
        area(Content)
        {
            group(Tiles)
            {
                field(MarkedCount; MarkedCount)
                {
                    ApplicationArea = All;
                    Caption = 'Operation 10';
                    Editable = false;

                    trigger OnDrillDown()
                    begin
                        Rec.ClearMarks();
                        if Rec.Get('10') then
                            Rec.Mark(true);
                        Rec.MarkedOnly(true);
                        CurrPage.Update(false);
                    end;
                }
            }
            repeater(Lines)
            {
                field("Operation No."; Rec."Operation No.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
    end;

    trigger OnAfterGetCurrRecord()
    begin
    end;

    var
        MarkedCount: Integer;
}

codeunit 50110 T
{

    procedure WithPartGoToRecordMissesTheUnmarkedRow()
    var
        Operation20: Record "Repro Operation";
        Operations: TestPage "Repro Ops With Part";
    begin
        Setup();
        Operation20.Get('20');
        Operations.OpenEdit();
        Operations.MarkedCount.Drilldown();
        if Operations.GoToRecord(Operation20) then
            Error('Operation 20 is listed after the drilldown marked only operation 10.');
        Operations.Close();
    end;

    procedure WithPartListsOnlyTheMarkedRow()
    var
        Operations: TestPage "Repro Ops With Part";
        Listed: Text;
    begin
        Setup();
        Operations.OpenEdit();
        Operations.MarkedCount.Drilldown();
        if Operations.First() then
            repeat
                Listed += ' ' + Operations."Operation No.".Value();
            until not Operations.Next();
        if Listed <> ' 10' then
            Error('Listed after the drilldown:%1', Listed);
        Operations.Close();
    end;

    procedure WithFactBoxGoToRecordMissesTheUnmarkedRow()
    var
        Operation20: Record "Repro Operation";
        Operations: TestPage "Repro Ops With FactBox";
    begin
        Setup();
        Operation20.Get('20');
        Operations.OpenEdit();
        Operations.MarkedCount.Drilldown();
        if Operations.GoToRecord(Operation20) then
            Error('Operation 20 is listed after the drilldown marked only operation 10.');
        Operations.Close();
    end;

    procedure WithFactBoxListsOnlyTheMarkedRow()
    var
        Operations: TestPage "Repro Ops With FactBox";
        Listed: Text;
    begin
        Setup();
        Operations.OpenEdit();
        Operations.MarkedCount.Drilldown();
        if Operations.First() then
            repeat
                Listed += ' ' + Operations."Operation No.".Value();
            until not Operations.Next();
        if Listed <> ' 10' then
            Error('Listed after the drilldown:%1', Listed);
        Operations.Close();
    end;

    procedure WithoutPartGoToRecordMissesTheUnmarkedRow()
    var
        Operation20: Record "Repro Operation";
        Operations: TestPage "Repro Ops Without Part";
    begin
        Setup();
        Operation20.Get('20');
        Operations.OpenEdit();
        Operations.MarkedCount.Drilldown();
        if Operations.GoToRecord(Operation20) then
            Error('Operation 20 is listed after the drilldown marked only operation 10.');
        Operations.Close();
    end;

    procedure WithoutPartListsOnlyTheMarkedRow()
    var
        Operations: TestPage "Repro Ops Without Part";
        Listed: Text;
    begin
        Setup();
        Operations.OpenEdit();
        Operations.MarkedCount.Drilldown();
        if Operations.First() then
            repeat
                Listed += ' ' + Operations."Operation No.".Value();
            until not Operations.Next();
        if Listed <> ' 10' then
            Error('Listed after the drilldown:%1', Listed);
        Operations.Close();
    end;

    local procedure Setup()
    var
        Operation: Record "Repro Operation";
        Component: Record "Repro Component";
    begin
        Operation.DeleteAll();
        Component.DeleteAll();
        Operation."Operation No." := '10';
        Operation."Order No." := 'A';
        Operation.Insert();
        Operation."Operation No." := '20';
        Operation."Order No." := 'A';
        Operation.Insert();
        Component."Order No." := 'A';
        Component."Line No." := 10000;
        Component.Insert();
    end;
}
"##;

#[test]
fn a_page_with_a_part_lists_only_the_marked_row_after_the_drilldown() {
    let mut b = al_testkit::Built::from_source(SRC);
    for p in [
        "WithPartGoToRecordMissesTheUnmarkedRow",
        "WithPartListsOnlyTheMarkedRow",
        "WithFactBoxGoToRecordMissesTheUnmarkedRow",
        "WithFactBoxListsOnlyTheMarkedRow",
        "WithoutPartGoToRecordMissesTheUnmarkedRow",
        "WithoutPartListsOnlyTheMarkedRow",
    ] {
        let r = b.run_in("T", p);
        assert!(r.is_ok(), "{p}: {r:?}");
    }
}
