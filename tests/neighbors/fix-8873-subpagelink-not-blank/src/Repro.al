table 50100 "Repro Header"
{
    fields { field(1; "No."; Code[10]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50101 "Repro Line"
{
    fields
    {
        field(1; "Header No."; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; "Arrival No."; Code[10]) { }
    }
    keys { key(PK; "Header No.", "Line No.") { Clustered = true; } }
}

page 50100 "Repro Lines"
{
    PageType = ListPart;
    SourceTable = "Repro Line";
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field("Arrival No."; Rec."Arrival No.") { ApplicationArea = All; }
            }
        }
    }
}

page 50101 "Repro Card"
{
    PageType = Card;
    SourceTable = "Repro Header";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "Repro Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Header No." = field("No."), "Arrival No." = filter(<> '');
            }
        }
    }
}

codeunit 50100 "Repro Tests"
{
    Subtype = Test;

    [Test]
    procedure CardWithNonBlankLink_Opens()
    var
        Header: Record "Repro Header";
        ReproCard: TestPage "Repro Card";
    begin
        Header."No." := 'H1';
        Header.Insert();
        ReproCard.OpenView();
        ReproCard.GoToRecord(Header);
        ReproCard.Close();
    end;
}
