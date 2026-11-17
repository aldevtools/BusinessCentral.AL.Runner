table 50920 "Action Header Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Description; Text[50]) { }
        field(3; "Linked Code"; Code[20])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("Action Link Row".Code where(Id = field(Id)));
        }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50921 "Action Link Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Code; Code[20]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50922 "Action Header Document"
{
    PageType = Document;
    SourceTable = "Action Header Row";
    ApplicationArea = All;
    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Id; Rec.Id) { }
                field(Description; Rec.Description) { }
            }
        }
    }
}

pageextension 50923 "Action Header Ext" extends "Action Header Document"
{
    actions
    {
        addlast(Processing)
        {
            action(CalculateAndWrite)
            {
                trigger OnAction()
                begin
                    Rec.CalcFields("Linked Code");
                    Rec.TestField("Linked Code", 'RELATED');
                    WriteCommittedDescription(Rec.Id);
                end;
            }
        }
    }

    local procedure WriteCommittedDescription(Id: Integer)
    var
        Header: Record "Action Header Row";
    begin
        Header.Get(Id);
        Header.Description := 'Written by action';
        Header.Modify(true);
        Commit();
    end;
}

codeunit 50924 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CalculatedFieldDoesNotSaveStaleDocument()
    var
        Header: Record "Action Header Row";
        Document: TestPage "Action Header Document";
    begin
        Seed(Header, 1);
        Document.OpenView();
        Document.GoToRecord(Header);
        Document.CalculateAndWrite.Invoke();
        Header.Get(1);
        Header.TestField(Description, 'Written by action');
        Document.Description.AssertEquals('Written by action');
        Document.Close();
    end;

    local procedure Seed(var Header: Record "Action Header Row"; Id: Integer)
    var
        Link: Record "Action Link Row";
    begin
        Header.Id := Id;
        Header.Description := 'Original';
        Header.Insert();
        Link.Id := Id;
        Link.Code := 'RELATED';
        Link.Insert();
        Commit();
    end;
}
