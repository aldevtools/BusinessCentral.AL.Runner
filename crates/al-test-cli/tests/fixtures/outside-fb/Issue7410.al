// Issue 7410: the reporter's generalized pattern. A field CaptionClass that an OnResolveCaptionClass subscriber resolves, and
// a test that reads the caption of its TestPage control twice after OpenEdit. Measured on BC 29 by the reporter: both reads
// give 'Length'.
codeunit 42 "Caption Class"
{
    [IntegrationEvent(false, false)]
    procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
    end;
}

table 50100 "My Table"
{
    DataClassification = CustomerContent;
    fields
    {
        field(1; "No."; Code[20]) { }
        field(50100; "My Dimension 1"; Decimal) { CaptionClass = 'MYAPP99,2,1'; }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

page 50100 "My Card"
{
    PageType = Card;
    SourceTable = "My Table";
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { }
                field("My Dimension 1"; Rec."My Dimension 1") { }
            }
        }
    }
}

codeunit 50101 "My Caption Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", OnResolveCaptionClass, '', true, true)]
    local procedure ResolveMyArea(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        if CaptionArea <> 'MYAPP99' then exit;
        Caption := 'Length'; Resolved := true;
    end;
}

codeunit 50102 "My Caption Tests"
{
    Subtype = Test;

    [Test]
    procedure SecondCaptionReadOfResolvedField()
    var
        Assert: Codeunit "Library Assert";
        MyCard: TestPage "My Card";
    begin
        MyCard.OpenEdit();
        Assert.AreEqual('Length', MyCard."My Dimension 1".Caption(), '');
        Assert.AreEqual('Length', MyCard."My Dimension 1".Caption(), '');
    end;
}
