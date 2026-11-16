table 50920 "Probe Host"
{
    DataClassification = SystemMetadata;
    fields { field(1; Id; Code[20]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50921 "Probe Line"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; ParentId; Code[20]) { }
        field(2; LineNo; Integer) { }
        field(3; ItemNo; Code[20]) { }
    }
    keys { key(PK; ParentId, LineNo) { Clustered = true; } }
}

table 50922 "Probe Shadow"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; LineNo; Integer) { }
        field(2; Value; Code[20]) { }
    }
    keys { key(PK; LineNo) { Clustered = true; } }
}

codeunit 50923 "Probe Shadow Subscriber"
{
    EventSubscriberInstance = Manual;

    procedure Initialize(var Shadow: Record "Probe Shadow" temporary)
    begin
        Rows.Copy(Shadow, true);
    end;

    [EventSubscriber(ObjectType::Table, Database::"Probe Line", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnInsert(var Rec: Record "Probe Line"; RunTrigger: Boolean)
    begin
        Rows.Init();
        Rows.LineNo := Rec.LineNo;
        Rows.Insert();
    end;

    var
        Rows: Record "Probe Shadow" temporary;
}

page 50924 "Probe Auto Split Lines"
{
    PageType = ListPart;
    SourceTable = "Probe Line";
    ApplicationArea = All;
    AutoSplitKey = true;
    DelayedInsert = true;
    PopulateAllFields = true;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(ItemNo; Rec.ItemNo)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        CurrPage.SaveRecord();
                        if NamedProcedureCalled then
                            Error('CurrPage.SaveRecord invoked the page procedure.');
                        if not Shadow.Get(Rec.LineNo) then
                            Error('Shadow line %1 was absent after SaveRecord.', Rec.LineNo);
                        Shadow.Value := Rec.ItemNo;
                        Shadow.Modify();
                    end;
                }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Clear(Shadow);
    end;

    procedure Bind(var Shared: Record "Probe Shadow" temporary)
    begin
        Shadow.Copy(Shared, true);
        Subscriber.Initialize(Shadow);
        BindSubscription(Subscriber);
    end;

    internal procedure SaveRecord()
    begin
        NamedProcedureCalled := true;
    end;

    procedure Unbind()
    begin
        UnbindSubscription(Subscriber);
    end;

    var
        Subscriber: Codeunit "Probe Shadow Subscriber";
        NamedProcedureCalled: Boolean;
        Shadow: Record "Probe Shadow" temporary;
}

page 50925 "Probe Auto Split Document"
{
    PageType = Document;
    SourceTable = "Probe Host";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            group(Header)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
            }
            part(Lines; "Probe Auto Split Lines")
            {
                ApplicationArea = All;
                SubPageLink = ParentId = field(Id);
            }
        }
    }

    trigger OnOpenPage()
    begin
        CurrPage.Lines.Page.Bind(HostShadow);
    end;

    trigger OnClosePage()
    begin
        CurrPage.Lines.Page.SaveRecord();
        CurrPage.Lines.Page.Unbind();
    end;

    procedure Initialize(var Shared: Record "Probe Shadow" temporary)
    begin
        HostShadow.Copy(Shared, true);
    end;

    var
        HostShadow: Record "Probe Shadow" temporary;
}

codeunit 50926 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('HandleDocument')]
    procedure SaveRecordPublishesAutoSplitKeyToBoundSubscriber()
    var
        Host: Record "Probe Host";
        Line: Record "Probe Line";
        Shadow: Record "Probe Shadow" temporary;
        Document: Page "Probe Auto Split Document";
    begin
        Host.DeleteAll();
        Line.DeleteAll();
        Host.Id := 'P';
        Host.Insert();
        Host.FilterGroup(7);
        Host.SetRecFilter();
        Host.FilterGroup(0);
        Document.Initialize(Shadow);
        Commit();
        Document.SetTableView(Host);
        Document.RunModal();
        if not Shadow.Get(10000) then
            Error('The shared shadow row was not stored.');
        if not Line.Get('P', 10000) then
            Error('The first line was not stored with key 10000.');
        if Line.ItemNo <> 'ITEM' then
            Error('The stored item number was %1.', Line.ItemNo);
    end;

    [ModalPageHandler]
    procedure HandleDocument(var Document: TestPage "Probe Auto Split Document")
    begin
        Document.Lines.ItemNo.SetValue('ITEM');
        Document.OK().Invoke();
    end;
}
