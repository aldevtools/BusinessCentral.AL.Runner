table 50110 "NB SPNB Line"
{
    fields
    {
        field(1; "Header No."; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; "Arrival No."; Code[10]) { }
        field(4; Note; Text[30]) { }
        field(5; Qty; Integer) { }
    }
    keys { key(PK; "Header No.", "Line No.") { Clustered = true; } }
}

page 50110 "NB SPNB Part"
{
    PageType = ListPart;
    SourceTable = "NB SPNB Line";
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

page 50111 "NB SPNB List"
{
    PageType = List;
    SourceTable = "NB SPNB Line";
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

page 50112 "NB SPNB Card"
{
    PageType = Card;
    SourceTable = "Repro Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(CodeSpaced; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival No." = filter(<> '');
            }
            part(CodeTight; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival No." = filter(<>'');
            }
            part(CodeAlone; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Arrival No." = filter(<> '');
            }
            part(TextSpaced; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), Note = filter(<> '');
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(RunCode)
            {
                ApplicationArea = All;
                RunObject = page "NB SPNB List";
                RunPageLink = "Arrival No." = filter(<> '');
            }
            action(RunText)
            {
                ApplicationArea = All;
                RunObject = page "NB SPNB List";
                RunPageLink = Note = filter(<> '');
            }
        }
    }
}

page 50113 "NB SPNB Int Card"
{
    PageType = Card;
    SourceTable = "Repro Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(IntPart; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), Qty = filter(<> '');
            }
        }
    }
}

page 50114 "NB SPNB Wide Card"
{
    PageType = Card;
    SourceTable = "Repro Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(WidePart; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival No." = filter(<>  '');
            }
        }
    }
}

page 50115 "NB SPNB Quoted Card"
{
    PageType = Card;
    SourceTable = "Repro Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(QuotedPart; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival No." = filter('<>''''');
            }
        }
    }
}

page 50116 "NB SPNB Union Card"
{
    PageType = Card;
    SourceTable = "Repro Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(UnionPart; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival No." = filter(<> '' | 'A1');
            }
        }
    }
}

page 50117 "NB SPNB Const Card"
{
    PageType = Card;
    SourceTable = "Repro Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(ConstPart; "NB SPNB Part")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival No." = const(<> '');
            }
        }
    }
}

page 50118 "NB SPNB IntRun Card"
{
    PageType = Card;
    SourceTable = "Repro Header";
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
            action(RunInt)
            {
                ApplicationArea = All;
                RunObject = page "NB SPNB List";
                RunPageLink = Qty = filter(<> '');
            }
        }
    }
}
