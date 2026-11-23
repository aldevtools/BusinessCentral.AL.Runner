table 50142 "Repro Cue Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50143 "Repro Cue Card"
{
    PageType = Card;
    SourceTable = "Repro Cue Row";
    ApplicationArea = All;
    UsageCategory = Documents;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
            cuegroup(Cues)
            {
                field(CueName; Rec.Name) { }
            }
        }
    }
}