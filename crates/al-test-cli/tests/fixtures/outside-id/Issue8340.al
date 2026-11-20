// The reporter's repro of al-runner issue 8340, green on real BC 29.1 (as posted on the issue).
table 50201 "ALR Probe Parent"
{
    DataClassification = CustomerContent;
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Description; Text[50]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

page 50201 "ALR Probe Parent Card"
{
    PageType = Card;
    SourceTable = "ALR Probe Parent";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field("Code"; Rec."Code") { }
            field(Description; Rec.Description) { }
        }
    }
    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Error('OnNewRecord ran');
    end;
}

codeunit 50214 "ALR P OpenEditOnEmptyCard"
{
    Subtype = Test;

    [Test]
    procedure OpenEditOnEmptyCard()
    var
        Card: TestPage "ALR Probe Parent Card";
    begin
        Card.OpenEdit();
        Card.Close();
    end;
}
