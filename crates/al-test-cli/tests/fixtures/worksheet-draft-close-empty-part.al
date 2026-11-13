table 50970 "Draft Probe Header"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Batch; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; Item; Code[20]) { }
        field(4; Quantity; Integer) { }
    }
    keys { key(PK; Batch, "Line No.") { Clustered = true; } }
}

table 50971 "Draft Probe Detail"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Header Code"; Code[20]) { }
        field(2; "Line No."; Integer) { }
    }
    keys { key(PK; "Header Code", "Line No.") { Clustered = true; } }
}

codeunit 50972 "Draft Probe Trace"
{
    SingleInstance = true;
    var
        Calls: Text;

    procedure Add(Entry: Text)
    begin
        Calls += Entry + '|';
    end;

    procedure Reset()
    begin
        Calls := '';
    end;

    procedure Read(): Text
    begin
        exit(Calls);
    end;
}

page 50973 "Draft Probe Detail Part"
{
    PageType = ListPart;
    SourceTable = "Draft Probe Detail";
    ApplicationArea = All;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        Trace: Codeunit "Draft Probe Trace";
    begin
        Trace.Add('part.new');
    end;

    trigger OnAfterGetCurrRecord()
    var
        Trace: Codeunit "Draft Probe Trace";
    begin
        Trace.Add('part.current');
    end;
}

page 50974 "Draft Probe Worksheet"
{
    PageType = Worksheet;
    SourceTable = "Draft Probe Header";
    DelayedInsert = true;
    AutoSplitKey = true;
    MultipleNewLines = true;
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Item; Rec.Item) { }
                field(Quantity; Rec.Quantity) { }
            }
        }
        area(FactBoxes)
        {
            part(Details; "Draft Probe Detail Part")
            {
                SubPageLink = "Header Code" = field(Item);
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.SetRange(Batch, 'PLAN');
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Rec.Batch := 'PLAN';
    end;
}

codeunit 50975 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CloseStoresWorksheetRowWithoutReadingEmptyPart()
    var
        Header: Record "Draft Probe Header";
        Trace: Codeunit "Draft Probe Trace";
        Worksheet: TestPage "Draft Probe Worksheet";
    begin
        Header.DeleteAll();
        Worksheet.OpenEdit();
        Worksheet.Item.SetValue('ITEM');
        Worksheet.Quantity.SetValue(1);
        Trace.Reset();
        Worksheet.Close();
        if Trace.Read() <> '' then
            Error('Closing read the empty part: %1', Trace.Read());
        Header.Get('PLAN', 10000);
        Header.TestField(Item, 'ITEM');
        Header.TestField(Quantity, 1);
    end;
}
