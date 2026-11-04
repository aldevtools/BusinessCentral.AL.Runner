//! Issue 6926: Next() from the last row of a part whose control says `Editable = false`. The repro of the issue: a Card host
//! with a non-editable, DelayedInsert part linked to it, walked with First and Next.

use al_testkit::text;

const ISSUE_SRC: &str = r#"
table 50100 "Repro Inbound"
{
    fields { field(1; "Entry no."; Code[10]) { } }
    keys { key(PK; "Entry no.") { Clustered = true; } }
}

table 50101 "Repro Trp"
{
    fields
    {
        field(1; "Transport no."; Code[10]) { }
        field(2; "Entry no. inbound"; Code[10]) { }
    }
    keys { key(PK; "Transport no.") { Clustered = true; } }
}

page 50100 "Repro Trp Sub"
{
    PageType = ListPart;
    SourceTable = "Repro Trp";
    ApplicationArea = All;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Transport no."; Rec."Transport no.") { ApplicationArea = All; }
            }
        }
    }
}

page 50101 "Repro Inbound Card"
{
    PageType = Card;
    SourceTable = "Repro Inbound";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field("Entry no."; Rec."Entry no.") { ApplicationArea = All; }
            part(subform; "Repro Trp Sub")
            {
                ApplicationArea = All;
                Editable = false;
                SubPageLink = "Entry no. inbound" = field("Entry no.");
            }
        }
    }
}

codeunit 50100 "Repro Next"
{
    procedure Walk(): Text
    var
        Inbound: Record "Repro Inbound";
        Trp: Record "Repro Trp";
        Card: TestPage "Repro Inbound Card";
        T: Text;
        Rows: Integer;
    begin
        Inbound."Entry no." := 'I1';
        Inbound.Insert();
        Trp."Transport no." := 'T1';
        Trp."Entry no. inbound" := 'I1';
        Trp.Insert();
        Trp."Transport no." := 'T2';
        Trp.Insert();

        Card.OpenEdit();
        Card.GoToRecord(Inbound);
        if Card.subform.First() then
            repeat
                Rows += 1;
                T += Card.subform."Transport no.".Value() + ';';
            until not Card.subform.Next() or (Rows > 4);
        exit(T);
    end;
}
"#;

/// The repro of issue 6926.
#[test]
fn next_walks_the_rows_of_a_part_whose_control_is_not_editable() {
    assert_eq!(text(al_testkit::run_in(ISSUE_SRC, "Repro Next", "Walk")), "T1;T2;");
}

/// The objects of corpus pages/parts/R6926PartNextReadOnly.al (tables 95167-95168, pages 95169-95172) and a codeunit that runs
/// the probe's steps on each part.
const PROBE_SRC: &str = r#"
table 95167 "LT R6926 Head"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[10]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 95168 "LT R6926 Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Code[10]) { DataClassification = SystemMetadata; }
        field(2; "Head No."; Code[10]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }
}

page 95169 "LT R6926 Lines"
{
    PageType = ListPart;
    SourceTable = "LT R6926 Line";
    ApplicationArea = All;
    UsageCategory = None;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }
}

page 95170 "LT R6926 Lines RO"
{
    PageType = ListPart;
    SourceTable = "LT R6926 Line";
    ApplicationArea = All;
    UsageCategory = None;
    DelayedInsert = true;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }
}

page 95171 "LT R6926 Lines NoIns"
{
    PageType = ListPart;
    SourceTable = "LT R6926 Line";
    ApplicationArea = All;
    UsageCategory = None;
    DelayedInsert = true;
    InsertAllowed = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }
}

page 95172 "LT R6926 Card"
{
    PageType = Card;
    SourceTable = "LT R6926 Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(CtlRO; "LT R6926 Lines")
            {
                ApplicationArea = All;
                Editable = false;
                SubPageLink = "Head No." = field("No.");
            }
            part(PageRO; "LT R6926 Lines RO")
            {
                ApplicationArea = All;
                SubPageLink = "Head No." = field("No.");
            }
            part(NoIns; "LT R6926 Lines NoIns")
            {
                ApplicationArea = All;
                SubPageLink = "Head No." = field("No.");
            }
            part(Plain; "LT R6926 Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Head No." = field("No.");
            }
        }
    }
}

codeunit 50990 "R6926 Test"
{
    local procedure Seed()
    var
        Head: Record "LT R6926 Head";
        Line: Record "LT R6926 Line";
    begin
        Line.DeleteAll();
        Head.DeleteAll();
        Head."No." := 'H1';
        Head.Insert();
        Head."No." := 'H2';
        Head.Insert();
        Line."Line No." := 'L1'; Line."Head No." := 'H1'; Line.Insert();
        Line."Line No." := 'L2'; Line."Head No." := 'H1'; Line.Insert();
        Line."Line No." := 'L9'; Line."Head No." := 'H2'; Line.Insert();
    end;

    local procedure Open(var Card: TestPage "LT R6926 Card")
    var
        Head: Record "LT R6926 Head";
    begin
        Head.Get('H1');
        Card.OpenEdit();
        Card.GoToRecord(Head);
    end;

    local procedure Stored(): Text
    var
        Line: Record "LT R6926 Line";
    begin
        exit(Format(Line.Count()));
    end;

    procedure WalkCtlRO(): Text
    var
        Card: TestPage "LT R6926 Card";
        T: Text;
        i: Integer;
        Going: Boolean;
    begin
        Seed();
        Open(Card);
        T := 'first=' + Format(Card.CtlRO.First()) + '[' + Card.CtlRO."Line No.".Value() + ']';
        Going := true;
        while Going and (i < 4) do begin
            i += 1;
            Going := Card.CtlRO.Next();
            if Going then
                T += ' Yes[' + Card.CtlRO."Line No.".Value() + ']'
            else
                T += ' No';
        end;
        Card.Close();
        exit(T + ' stored=' + Stored());
    end;

    procedure MovesCtlRO(): Text
    var
        Card: TestPage "LT R6926 Card";
        Line: Record "LT R6926 Line";
        T: Text;
    begin
        Seed();
        Open(Card);
        T := 'last=' + Format(Card.CtlRO.Last()) + '[' + Card.CtlRO."Line No.".Value() + ']';
        T += ' next=' + Format(Card.CtlRO.Next()) + '[' + Card.CtlRO."Line No.".Value() + ']';
        T += ' previous=' + Format(Card.CtlRO.Previous()) + '[' + Card.CtlRO."Line No.".Value() + ']';
        Card.Close();
        Open(Card);
        Line.Get('L2');
        T += ' goto=' + Format(Card.CtlRO.GoToRecord(Line)) + '[' + Card.CtlRO."Line No.".Value() + ']';
        T += ' next=' + Format(Card.CtlRO.Next()) + '[' + Card.CtlRO."Line No.".Value() + ']';
        Line.Get('L9');
        T += ' other=' + Format(Card.CtlRO.GoToRecord(Line)) + '[' + Card.CtlRO."Line No.".Value() + ']';
        Card.Close();
        exit(T + ' stored=' + Stored());
    end;

    procedure WalkPageRO(): Text
    var
        Card: TestPage "LT R6926 Card";
        T: Text;
        i: Integer;
        Going: Boolean;
    begin
        Seed();
        Open(Card);
        T := 'first=' + Format(Card.PageRO.First()) + '[' + Card.PageRO."Line No.".Value() + ']';
        Going := true;
        while Going and (i < 4) do begin
            i += 1;
            Going := Card.PageRO.Next();
            if Going then
                T += ' Yes[' + Card.PageRO."Line No.".Value() + ']'
            else
                T += ' No';
        end;
        Card.Close();
        exit(T + ' stored=' + Stored());
    end;

    procedure MovesPageRO(): Text
    var
        Card: TestPage "LT R6926 Card";
        Line: Record "LT R6926 Line";
        T: Text;
    begin
        Seed();
        Open(Card);
        T := 'last=' + Format(Card.PageRO.Last()) + '[' + Card.PageRO."Line No.".Value() + ']';
        T += ' next=' + Format(Card.PageRO.Next()) + '[' + Card.PageRO."Line No.".Value() + ']';
        T += ' previous=' + Format(Card.PageRO.Previous()) + '[' + Card.PageRO."Line No.".Value() + ']';
        Card.Close();
        Open(Card);
        Line.Get('L2');
        T += ' goto=' + Format(Card.PageRO.GoToRecord(Line)) + '[' + Card.PageRO."Line No.".Value() + ']';
        T += ' next=' + Format(Card.PageRO.Next()) + '[' + Card.PageRO."Line No.".Value() + ']';
        Line.Get('L9');
        T += ' other=' + Format(Card.PageRO.GoToRecord(Line)) + '[' + Card.PageRO."Line No.".Value() + ']';
        Card.Close();
        exit(T + ' stored=' + Stored());
    end;

    procedure WalkNoIns(): Text
    var
        Card: TestPage "LT R6926 Card";
        T: Text;
        i: Integer;
        Going: Boolean;
    begin
        Seed();
        Open(Card);
        T := 'first=' + Format(Card.NoIns.First()) + '[' + Card.NoIns."Line No.".Value() + ']';
        Going := true;
        while Going and (i < 4) do begin
            i += 1;
            Going := Card.NoIns.Next();
            if Going then
                T += ' Yes[' + Card.NoIns."Line No.".Value() + ']'
            else
                T += ' No';
        end;
        Card.Close();
        exit(T + ' stored=' + Stored());
    end;

    procedure MovesNoIns(): Text
    var
        Card: TestPage "LT R6926 Card";
        Line: Record "LT R6926 Line";
        T: Text;
    begin
        Seed();
        Open(Card);
        T := 'last=' + Format(Card.NoIns.Last()) + '[' + Card.NoIns."Line No.".Value() + ']';
        T += ' next=' + Format(Card.NoIns.Next()) + '[' + Card.NoIns."Line No.".Value() + ']';
        T += ' previous=' + Format(Card.NoIns.Previous()) + '[' + Card.NoIns."Line No.".Value() + ']';
        Card.Close();
        Open(Card);
        Line.Get('L2');
        T += ' goto=' + Format(Card.NoIns.GoToRecord(Line)) + '[' + Card.NoIns."Line No.".Value() + ']';
        T += ' next=' + Format(Card.NoIns.Next()) + '[' + Card.NoIns."Line No.".Value() + ']';
        Line.Get('L9');
        T += ' other=' + Format(Card.NoIns.GoToRecord(Line)) + '[' + Card.NoIns."Line No.".Value() + ']';
        Card.Close();
        exit(T + ' stored=' + Stored());
    end;

    procedure WalkPlain(): Text
    var
        Card: TestPage "LT R6926 Card";
        T: Text;
        i: Integer;
        Going: Boolean;
    begin
        Seed();
        Open(Card);
        T := 'first=' + Format(Card.Plain.First()) + '[' + Card.Plain."Line No.".Value() + ']';
        Going := true;
        while Going and (i < 4) do begin
            i += 1;
            Going := Card.Plain.Next();
            if Going then
                T += ' Yes[' + Card.Plain."Line No.".Value() + ']'
            else
                T += ' No';
        end;
        Card.Close();
        exit(T + ' stored=' + Stored());
    end;

    procedure MovesPlain(): Text
    var
        Card: TestPage "LT R6926 Card";
        Line: Record "LT R6926 Line";
        T: Text;
    begin
        Seed();
        Open(Card);
        T := 'last=' + Format(Card.Plain.Last()) + '[' + Card.Plain."Line No.".Value() + ']';
        T += ' next=' + Format(Card.Plain.Next()) + '[' + Card.Plain."Line No.".Value() + ']';
        T += ' previous=' + Format(Card.Plain.Previous()) + '[' + Card.Plain."Line No.".Value() + ']';
        Card.Close();
        Open(Card);
        Line.Get('L2');
        T += ' goto=' + Format(Card.Plain.GoToRecord(Line)) + '[' + Card.Plain."Line No.".Value() + ']';
        T += ' next=' + Format(Card.Plain.Next()) + '[' + Card.Plain."Line No.".Value() + ']';
        Line.Get('L9');
        T += ' other=' + Format(Card.Plain.GoToRecord(Line)) + '[' + Card.Plain."Line No.".Value() + ']';
        Card.Close();
        exit(T + ' stored=' + Stored());
    end;
}
"#;

fn probe(routine: &str) -> String {
    text(al_testkit::run_in(PROBE_SRC, "R6926 Test", routine))
}

/// R6926PartNextReadOnly.al R6926_{CtlRO,PageRO,NoIns}_Walk, _LastNextPrevious, _GoToRecord: a part that is not editable (its
/// control or its page) or allows no insert has no new-row line; Next from its last row answers No and the part stays there.
#[test]
fn a_part_that_is_not_editable_or_allows_no_insert_stops_at_its_last_row() {
    for p in ["CtlRO", "PageRO", "NoIns"] {
        assert_eq!(probe(&format!("Walk{p}")), "first=Yes[L1] Yes[L2] No stored=3", "{p}");
        assert_eq!(
            probe(&format!("Moves{p}")),
            "last=Yes[L2] next=No[L2] previous=Yes[L1] goto=Yes[L2] next=No[L2] other=No[L2] stored=3",
            "{p}"
        );
    }
}

/// R6926PartNextReadOnly.al R6926_Plain_*: an editable part walks on from its last row onto the blank new-row line.
#[test]
fn an_editable_part_walks_onto_its_new_row_line() {
    assert_eq!(probe("WalkPlain"), "first=Yes[L1] Yes[L2] Yes[] No stored=3");
    assert_eq!(
        probe("MovesPlain"),
        "last=Yes[L2] next=Yes[] previous=Yes[L2] goto=Yes[L2] next=Yes[] other=No[] stored=3"
    );
}
