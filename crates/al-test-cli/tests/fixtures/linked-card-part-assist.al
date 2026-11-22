table 50900 "Linked Part Host Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Detail Code"; Code[20]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50901 "Linked Part Detail Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; "Before Assist"; Code[20]) { }
        field(3; Marker; Integer) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

page 50902 "Linked Part Detail"
{
    PageType = CardPart;
    SourceTable = "Linked Part Detail Row";
    ApplicationArea = All;
    InsertAllowed = false;
    DeleteAllowed = false;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Code; Rec.Code)
                {
                    Editable = false;
                    trigger OnAssistEdit()
                    var
                        Detail: Record "Linked Part Detail Row";
                        FilterGroup: Integer;
                        DetailCode: Text;
                    begin
                        FilterGroup := Rec.FilterGroup();
                        Rec.FilterGroup(4);
                        DetailCode := Rec.GetFilter(Code);
                        Rec.FilterGroup(FilterGroup);

                        Detail.Code := CopyStr(DetailCode, 1, MaxStrLen(Detail.Code));
                        Detail."Before Assist" := Rec.Code;
                        Detail.Marker := 1;
                        Detail.Insert(true);
                        Rec := Detail;
                    end;
                }
                field(Marker; Rec.Marker)
                {
                    trigger OnAssistEdit()
                    begin
                        Rec.Marker += 1;
                        Rec.Modify();
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }
}

page 50903 "Linked Part Host"
{
    PageType = Document;
    SourceTable = "Linked Part Host Row";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Id; Rec.Id) { }
                field("Detail Code"; Rec."Detail Code") { }
            }
            part(Detail; "Linked Part Detail")
            {
                SubPageLink = Code = field("Detail Code");
                UpdatePropagation = Both;
            }
        }
    }
}

codeunit 50904 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure AssistEditUsesTheLinkedPrimaryKey()
    var
        Detail: Record "Linked Part Detail Row";
        Host: TestPage "Linked Part Host";
    begin
        OpenHost(Host, 1, 'DETAIL-A');
        Host.Detail.Code.AssistEdit();
        Detail.Get('DETAIL-A');
        Detail.TestField("Before Assist", 'DETAIL-A');
        Detail.TestField(Marker, 1);
        Host.Close();
    end;

    [Test]
    procedure UpdateAfterAssistEditShowsInsertedDetail()
    var
        Detail: Record "Linked Part Detail Row";
        Host: TestPage "Linked Part Host";
    begin
        OpenHost(Host, 2, 'DETAIL-B');
        Host.Detail.Code.AssistEdit();
        Host.Detail.Marker.AssistEdit();
        Host.Detail.Marker.AssertEquals(2);
        Detail.Get('DETAIL-B');
        Detail.TestField(Marker, 2);
        Host.Close();
    end;

    local procedure OpenHost(var Host: TestPage "Linked Part Host"; Id: Integer; DetailCode: Code[20])
    var
        Header: Record "Linked Part Host Row";
    begin
        Header.Id := Id;
        Header.Insert();
        Host.OpenEdit();
        Host.GoToRecord(Header);
        Host."Detail Code".SetValue(DetailCode);
    end;
}
