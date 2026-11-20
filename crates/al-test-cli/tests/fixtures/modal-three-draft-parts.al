table 50351 "Three Part Probe Row"
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
}

page 50352 "Three Part Probe Document"
{
    PageType = Document;
    SourceTable = "Three Part Probe Row";
    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
            }
            part(Lines; "Three Part Probe Part")
            {
                ApplicationArea = All;
                SubPageLink = Id = field(Id);
            }
            part(OtherLines; "Three Part Probe Part")
            {
                ApplicationArea = All;
                SubPageLink = Id = field(Id);
            }
            part(ThirdLines; "Three Part Probe Part")
            {
                ApplicationArea = All;
                SubPageLink = Id = field(Id);
            }
        }
    }
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    var
        Trace: Codeunit "Three Part Probe Trace";
    begin
        Trace.Add('Document', CloseAction);
        exit(true);
    end;
}

page 50353 "Three Part Probe Part"
{
    PageType = ListPart;
    SourceTable = "Three Part Probe Row";
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
                field(Payload; Rec.Payload)
                {
                    ApplicationArea = All;
                    trigger OnValidate()
                    begin
                        Error('Repro part validation failed.');
                    end;
                }
            }
        }
    }
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    var
        Trace: Codeunit "Three Part Probe Trace";
    begin
        Trace.Add('Part', CloseAction);
        exit(true);
    end;
}

codeunit 50354 "Three Part Probe Trace"
{
    SingleInstance = true;
    var
        Value: Text;

    procedure Reset()
    begin
        Clear(Value);
    end;

    procedure Add(Origin: Text; CloseAction: Action)
    begin
        Value += StrSubstNo('%1:%2|', Origin, Format(CloseAction));
    end;

    procedure Read(): Text
    begin
        exit(Value);
    end;
}

codeunit 50355 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('InvokeFailure')]
    procedure ThreeDraftListPartsCloseAndKeepValidationError()
    var
        Row: Record "Three Part Probe Row";
        Document: Page "Three Part Probe Document";
        Trace: Codeunit "Three Part Probe Trace";
    begin
        Row.Init();
        Row.Id := 1;
        Row.Insert();
        Trace.Reset();
        Document.SetRecord(Row);

        asserterror Document.RunModal();

        if StrPos(GetLastErrorText(), 'Repro part validation failed.') = 0 then
            Error('Unexpected error: %1', GetLastErrorText());
        if Trace.Read() <> 'Part:None|Part:None|Part:None|Document:OK|' then
            Error('Unexpected close trace: %1', Trace.Read());
    end;

    [ModalPageHandler]
    procedure InvokeFailure(var Document: TestPage "Three Part Probe Document")
    begin
        Document.Lines.New();
        Document.OtherLines.New();
        Document.ThirdLines.New();
        Document.Lines.Payload.SetValue('INPUT');
    end;
}
