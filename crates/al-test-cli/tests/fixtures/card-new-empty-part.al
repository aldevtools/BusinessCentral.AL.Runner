// BC Documentation: https://learn.microsoft.com/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpage-opennew-method
// Scope: Cloud-compatible standalone repro
// BC versions: 27.5, 28.5, 29.0
// local object: The Document control, linked part, and header insert trigger define the measured shape.

// local object: the table OnInsert assigns the key used by the variable control trigger.
table 50810 "Document Repro Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Payload; Text[20]) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    trigger OnInsert()
    begin
        if Id = 0 then
            Id := 10;
    end;
}

// local object: the Document page opens in New mode and validates an unbound control.
page 50811 "Document Repro Page"
{
    PageType = Card;
    SourceTable = "Document Repro Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
                field(Scan; InputText)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Payload := InputText;
                        Rec.Modify();
                    end;
                }
            }
        }
    }

    var
        InputText: Text[20];
}

codeunit 50812 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure HostNewWithEmptyPartTrace()
    var
        Document: TestPage "Document Repro With Part";
        Row: Record "Document Repro Row";
        Trace: Codeunit "Probe Part Trace";
        Calls: Integer;
    begin
        Initialize();
        Row.Id := 1;
        Row.Insert();
        Document.OpenEdit();
        Trace.Reset();
        Document.New();
        Calls := Trace.Count();
        Document.Close();
        if Calls <> 0 then
            Error('Host New current-record observation: %1', Calls);
    end;

    local procedure Initialize()
    var
        Row: Record "Document Repro Row";
    begin
        Row.DeleteAll();
    end;
}

page 50813 "Document Repro With Part"
{
    PageType = Card;
    SourceTable = "Document Repro Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
                field(Scan; InputText)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Payload := InputText;
                        Rec.Modify();
                    end;
                }
            }
            part(Lines; "Document Repro Lines")
            {
                ApplicationArea = All;
                SubPageLink = HeaderId = field(Id);
            }
        }
    }

    var
        InputText: Text[20];
}


table 50814 "Document Repro Line"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; HeaderId; Integer) { }
        field(2; Id; Integer) { }
    }
    keys { key(PK; HeaderId, Id) { Clustered = true; } }
}

page 50815 "Document Repro Lines"
{
    PageType = ListPart;
    SourceTable = "Document Repro Line";
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
    trigger OnAfterGetCurrRecord()
    var Trace: Codeunit "Probe Part Trace";
    begin
        Trace.Hit();
    end;

}

codeunit 50816 "Probe Part Trace"
{
    SingleInstance = true;
    var Calls: Integer;

    procedure Reset()
    begin
        Clear(Calls);
    end;

    procedure Hit()
    begin
        Calls += 1;
    end;

    procedure Count(): Integer
    begin
        exit(Calls);
    end;
}
