table 50140 "NB SPNBR Header"
{
    fields
    {
        field(1; "No."; Code[10]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50141 "NB SPNBR Line"
{
    fields
    {
        field(1; "Header No."; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; "Arrival No."; Code[10]) { }
        field(4; "Arrival Filter"; Code[10]) { FieldClass = FlowFilter; }
        field(5; "Header Lookup"; Code[10])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("NB SPNBR Header"."No." where("No." = field("Arrival No.")));
        }
    }
    keys { key(PK; "Header No.", "Line No.") { Clustered = true; } }
}

// One fact flipped against corpus PR 1548 each: the part page is a CardPart.
page 50140 "NB SPNBR CardPart"
{
    PageType = CardPart;
    SourceTable = "NB SPNBR Line";
    layout
    {
        area(Content)
        {
            field(LineNo; Rec."Line No.") { ApplicationArea = All; }
        }
    }
}

page 50141 "NB SPNBR ListPart"
{
    PageType = ListPart;
    SourceTable = "NB SPNBR Line";
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(LineNo; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }
}

// The RunObject target is a Card.
page 50142 "NB SPNBR Target Card"
{
    PageType = Card;
    SourceTable = "NB SPNBR Line";
    layout
    {
        area(Content)
        {
            field(LineNo; Rec."Line No.") { ApplicationArea = All; }
        }
    }
}

page 50143 "NB SPNBR CardPart Host"
{
    PageType = Card;
    SourceTable = "NB SPNBR Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(CardPartLink; "NB SPNBR CardPart")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival No." = filter(<> '');
            }
        }
    }
}

page 50144 "NB SPNBR FlowFilter Host"
{
    PageType = Card;
    SourceTable = "NB SPNBR Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(FlowFilterLink; "NB SPNBR ListPart")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival Filter" = filter(<> '');
            }
        }
    }
}

page 50145 "NB SPNBR FlowField Host"
{
    PageType = Card;
    SourceTable = "NB SPNBR Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(FlowFieldLink; "NB SPNBR ListPart")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Header Lookup" = filter(<> '');
            }
        }
    }
}

page 50146 "NB SPNBR Run Host"
{
    PageType = Card;
    SourceTable = "NB SPNBR Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
        }
    }
    actions
    {
        area(Processing)
        {
            action(RunCard)
            {
                ApplicationArea = All;
                RunObject = page "NB SPNBR Target Card";
                RunPageLink = "Arrival No." = filter(<> '');
            }
        }
    }
}
