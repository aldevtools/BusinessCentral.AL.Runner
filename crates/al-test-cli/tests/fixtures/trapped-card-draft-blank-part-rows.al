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

table 50976 "Draft Probe Card Header"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Category; Code[20]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnInsert()
    begin
        if Code = '' then
            Code := 'NEW';
    end;
}

page 50977 "Draft Probe Card"
{
    PageType = Card;
    SourceTable = "Draft Probe Card Header";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Code; Rec.Code) { }
                field(Category; Rec.Category) { }
            }
            part(Details; "Draft Probe Detail Part")
            {
                SubPageLink = "Header Code" = field(Code);
            }
        }
    }
}

codeunit 50978 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SetValueOnNewCardWithBlankLinkedRows()
    var
        Header: Record "Draft Probe Card Header";
        Detail: Record "Draft Probe Detail";
        Card: TestPage "Draft Probe Card";
    begin
        Header.DeleteAll();
        Detail.DeleteAll();
        Header.Code := 'EXISTING';
        Header.Insert();
        Detail."Header Code" := '';
        Detail."Line No." := 10000;
        Detail.Insert();
        Card.Trap();
        Page.Run(Page::"Draft Probe Card", Header);
        Card.New();
        Card.Category.SetValue('CATEGORY');
        if Card.Category.Value() <> 'CATEGORY' then
            Error('Expected the new category to be shown.');
        Card.Close();
        Header.Get('NEW');
        Header.TestField(Category, 'CATEGORY');
        Detail.Get('', 10000);
    end;
}
