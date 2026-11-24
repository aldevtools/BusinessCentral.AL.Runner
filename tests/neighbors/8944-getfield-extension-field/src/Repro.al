table 50115 "Repro ExtField Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
        field(3; Note; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50116 "Repro ExtField Card"
{
    PageType = Card;
    SourceTable = "Repro ExtField Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }
}

pageextension 50117 "Repro ExtField Card Ext" extends "Repro ExtField Card"
{
    layout
    {
        addlast(General)
        {
            field(ExtNote; Rec.Note) { ApplicationArea = All; }
        }
    }
}
