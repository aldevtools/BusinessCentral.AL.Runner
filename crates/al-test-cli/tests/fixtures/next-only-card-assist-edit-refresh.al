table 50960 "Next Refresh Header"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Reference; Text[40]) { }
        field(3; Result; Text[40]) { }
        field(4; Active; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50961 "Next Refresh Line"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Header Id"; Integer) { }
        field(2; Text; Text[40]) { }
    }
    keys { key(PK; "Header Id") { Clustered = true; } }
}

codeunit 50962 "Next Refresh Sync"
{
    procedure RunSync(var Header: Record "Next Refresh Header")
    var
        Stored: Record "Next Refresh Header";
        Line: Record "Next Refresh Line";
    begin
        Stored.Get(Header.Id);
        Stored.Reference := Header.Reference;
        Stored.Result := 'Mapped';
        Stored.Modify();
        Header.Get(Stored.Id);
        Line."Header Id" := Header.Id;
        Line.Text := 'New detail';
        Line.Insert();
    end;
}

page 50963 "Next Refresh Lines"
{
    PageType = ListPart;
    SourceTable = "Next Refresh Line";
    ApplicationArea = All;
    InsertAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Detail; Rec.Text) { }
            }
        }
    }
}

codeunit 50966 "Next Refresh Trace"
{
    SingleInstance = true;

    var
        Reads: Integer;

    procedure RecordRead()
    begin
        Reads += 1;
    end;

    procedure Reset()
    begin
        Reads := 0;
    end;

    procedure CheckNoRead()
    begin
        if Reads <> 0 then
            Error('Expected no current-record trigger after AssistEdit, got %1', Reads);
    end;
}

page 50964 "Next Refresh Card"
{
    PageType = Card;
    SourceTable = "Next Refresh Header";
    SourceTableView = where(Active = const(true));
    ApplicationArea = All;
    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Id; Rec.Id) { }
                field(Reference; Rec.Reference)
                {
                    trigger OnAssistEdit()
                    var
                        Sync: Codeunit "Next Refresh Sync";
                    begin
                        Sync.RunSync(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Refresh; Rec.Result)
                {
                    trigger OnAssistEdit()
                    begin
                        CurrPage.Update(false);
                    end;
                }
                field(ObservedResult; ObservedResult) { }
            }
            part(Lines; "Next Refresh Lines")
            {
                SubPageLink = "Header Id" = field(Id);
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Trace: Codeunit "Next Refresh Trace";
    begin
        Trace.RecordRead();
        ObservedResult := Rec.Result;
    end;

    trigger OnNextRecord(Steps: Integer): Integer
    begin
        Error('Refreshing the current row must not navigate with OnNextRecord');
    end;

    var
        ObservedResult: Text[40];
}

codeunit 50965 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure AssistEditRefreshesAfterSyncStoredTheEditedRow()
    var
        Trace: Codeunit "Next Refresh Trace";
        Header: Record "Next Refresh Header";
        Card: TestPage "Next Refresh Card";
    begin
        Seed(1);
        Card.OpenEdit();
        Card.GoToKey(1);
        Card.Reference.SetValue('New reference');
        Trace.Reset();
        Card.Reference.AssistEdit();
        Trace.CheckNoRead();
        Card.Id.AssertEquals(1);
        Card.Reference.AssertEquals('New reference');
        Card.Refresh.AssertEquals('Mapped');
        Card.ObservedResult.AssertEquals('');
        Card.Lines.Detail.AssertEquals('New detail');
        Header.Get(1);
        Header.TestField(Reference, 'New reference');
        Header.TestField(Result, 'Mapped');
        Card.Close();
    end;

    [Test]
    procedure AssistEditUpdateFalseKeepsTheUnsavedEdit()
    var
        Header: Record "Next Refresh Header";
        Card: TestPage "Next Refresh Card";
    begin
        Header.DeleteAll();
        Seed(2);
        Card.OpenEdit();
        Card.GoToKey(2);
        Card.Reference.SetValue('Unsaved edit');
        Card.Refresh.AssistEdit();
        Card.Id.AssertEquals(2);
        Card.Reference.AssertEquals('Unsaved edit');
        Header.Get(2);
        Header.TestField(Reference, 'Original');
        Card.Close();
    end;

    local procedure Seed(Id: Integer)
    var
        Header: Record "Next Refresh Header";
    begin
        Header.Id := Id;
        Header.Reference := 'Original';
        Header.Active := true;
        Header.Insert();
    end;
}
