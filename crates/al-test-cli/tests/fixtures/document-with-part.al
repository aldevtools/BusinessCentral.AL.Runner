// BC Documentation: https://learn.microsoft.com/dynamics365/business-central/dev-itpro/developer/methods-auto/testpage/testpage-opennew-method
// Scope: Cloud-compatible standalone repro
// BC versions: 27.5, 28.5, 29.0
// local object: The Document control, linked part, and header insert trigger define the measured shape.

// local object: the table OnInsert assigns the key used by the variable control trigger.
table 50110 "Document Repro Row"
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
page 50111 "Document Repro Page"
{
    PageType = Document;
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

codeunit 50112 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TestPage_OpenNew_VariableControlCanModifyDocument()
    var
        Row: Record "Document Repro Row";
        Document: TestPage "Document Repro Page";
    begin
        Initialize();

        Document.OpenNew();
        Document.Scan.Value('SCANNED');
        Row.Get(10);
        if Row.Payload <> 'SCANNED' then
            Error('Expected stored payload SCANNED. Got %1.', Row.Payload);
        Document.Close();
    end;

    [Test]
    procedure TestPage_OpenNew_VariableControlCanModifyDocumentWithPart()
    var
        Row: Record "Document Repro Row";
        Document: TestPage "Document Repro With Part";
    begin
        Initialize();

        Document.OpenNew();
        Document.Scan.Value('SCANNED');
        Row.Get(10);
        if Row.Payload <> 'SCANNED' then
            Error('Expected stored payload SCANNED. Got %1.', Row.Payload);
        Document.Close();
    end;

    local procedure Initialize()
    var
        Row: Record "Document Repro Row";
    begin
        Row.DeleteAll();
    end;
}

page 50113 "Document Repro With Part"
{
    PageType = Document;
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


table 50114 "Document Repro Line"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; HeaderId; Integer) { }
        field(2; Id; Integer) { }
    }
    keys { key(PK; HeaderId, Id) { Clustered = true; } }
}

page 50115 "Document Repro Lines"
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
}
