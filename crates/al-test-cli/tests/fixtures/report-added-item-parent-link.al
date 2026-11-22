table 50900 "Probe Dataset Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; ParentId; Integer) { }
        field(3; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

report 50901 "Probe Dataset Link"
{
    ProcessingOnly = false;
    dataset
    {
        dataitem(Header; "Probe Dataset Row")
        {
            DataItemTableView = where(Id = const(1));
            dataitem(Line; "Probe Dataset Row")
            {
                DataItemTableView = where(Id = const(3));
                column(BaseLineKey; Id) { }
            }
        }
    }
}

reportextension 50902 "Probe Dataset Link Extension" extends "Probe Dataset Link"
{
    dataset
    {
        addbefore(Line)
        {
            dataitem(AddedHeader; "Probe Dataset Row")
            {
                DataItemLink = ParentId = field(Id);
                DataItemLinkReference = Header;
                DataItemTableView = where(Id = filter(2|5));
                column(AddedHeaderKey; Id) { }
                dataitem(AddedLine; "Probe Dataset Row")
                {
                    DataItemLink = ParentId = field(Id);
                    DataItemLinkReference = AddedHeader;
                    DataItemTableView = where(Id = filter(4|6));
                    column(AddedLineKey; Id) { }
                }
            }
        }
    }
}

codeunit 50903 T
{
    Subtype = Test;

    [Test]
    procedure AddedItemsLinkToTheirNamedImmediateParents()
    var
        Buffer: Record "Probe Dataset Row" temporary;
        Output: OutStream;
        Input: InStream;
        Line: Text;
        Xml: Text;
    begin
        Seed(1, 0);
        Seed(2, 1);
        Seed(3, 1);
        Seed(4, 2);
        Seed(5, 9);
        Seed(6, 9);
        Buffer.Data.CreateOutStream(Output);
        Report.SaveAs(Report::"Probe Dataset Link", '', ReportFormat::Xml, Output);
        Buffer.Data.CreateInStream(Input, TextEncoding::UTF8);
        while not Input.EOS() do begin
            Input.ReadText(Line);
            Xml += Line;
        end;

        if not Xml.Contains('<Column name="AddedHeaderKey">2</Column>') then
            Error('The matching added child is missing.');
        if not Xml.Contains('<Column name="AddedLineKey">4</Column>') then
            Error('The matching added grandchild is missing.');
        if Xml.Contains('<Column name="AddedHeaderKey">5</Column>') or
           Xml.Contains('<Column name="AddedLineKey">6</Column>') then
            Error('A row outside the parent link was included.');
    end;

    local procedure Seed(Id: Integer; ParentId: Integer)
    var
        Row: Record "Probe Dataset Row";
    begin
        Row.Id := Id;
        Row.ParentId := ParentId;
        Row.Insert();
    end;
}
