//! The xRec OnInsertRecord of an AutoSplitKey ListPart sees when SetValue on a second control inserts a new line (al-vm
//! `pages/draft.rs` `xrec_of_split_part_line_at_second_control`, assumption `part-split-key-second-control-xrec`, open corpus
//! PR 1399): the measured shape gets the row the line was started below, a neighbor shape keeps main's outcome.

use al_testrun::Outcome;

const SRC: &str = r#"
table 69980 "XS Head" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 69981 "XS Line"
{
    fields { field(1; "Head Code"; Code[10]) { } field(2; "Line No."; Integer) { } field(3; "Seq No."; Integer) { } field(4; Descr; Text[30]) { } }
    keys { key(PK; "Head Code", "Line No.") { } key(Seq; "Head Code", "Seq No.") { } }
}
codeunit 69982 "XS Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Line: Text) begin Entries += Line + ';'; end;
    procedure Take() Result: Text begin Result := Entries; Entries := ''; end;
}
page 69983 "XS Lines"
{
    PageType = ListPart; SourceTable = "XS Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var Log: Codeunit "XS Log";
    begin
        Log.Add('INS ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No."));
        exit(true);
    end;
}
page 69984 "XS Card Seq"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines") { SubPageLink = "Head Code" = field(Code); SubPageView = sorting("Head Code", "Seq No."); } } }
}
page 69985 "XS Card SeqDesc"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines") { SubPageLink = "Head Code" = field(Code); SubPageView = sorting("Head Code", "Seq No.") order(descending); } } }
}
page 69987 "XS Card PK"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines") { SubPageLink = "Head Code" = field(Code); } } }
}
page 69988 "XS Doc PK"
{
    PageType = Document; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines") { SubPageLink = "Head Code" = field(Code); } } }
}
codeunit 69986 C
{
    Subtype = Test;

    var Log: Codeunit "XS Log";

    local procedure Seed()
    var Head: Record "XS Head"; Line: Record "XS Line";
    begin
        Line.DeleteAll(); Head.DeleteAll();
        Head.Code := 'H1'; Head.Insert();
        AddLine(10000, 20); AddLine(20000, 40); AddLine(30000, 10); AddLine(40000, 30);
        Log.Take();
    end;

    local procedure AddLine(LineNo: Integer; SeqNo: Integer)
    var Line: Record "XS Line";
    begin
        Line."Head Code" := 'H1'; Line."Line No." := LineNo; Line."Seq No." := SeqNo; Line.Insert();
    end;

    local procedure Check(Expected: Text)
    var Actual: Text;
    begin
        Actual := Log.Take();
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    // The measured shape (PartSplitKeyInsertTiming.al:552): after Last in Seq order the line is started below 20000.
    [Test]
    procedure T_SeqAfterLast()
    var P: TestPage "XS Card Seq";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines."Seq No.".SetValue(25);
        Check('INS 35000 x=20000;');
        P.Close();
    end;

    // Not measured: a SubPageView in descending order. It keeps main's outcome, the blank xRec.
    [Test]
    procedure T_SeqDescAfterLast()
    var P: TestPage "XS Card SeqDesc";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines."Seq No.".SetValue(5);
        Check('INS 50000 x=0;');
        P.Close();
    end;
}
"#;

fn outcome(name: &str) -> Outcome {
    al_testkit::outcomes_in(SRC, "C")
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
}

#[test]
fn a_line_inserted_at_the_second_control_after_the_last_row_sees_that_row_as_xrec() {
    assert_eq!(outcome("T_SeqAfterLast"), Outcome::Pass);
}

#[test]
fn a_part_read_in_descending_order_keeps_the_blank_xrec() {
    assert_eq!(outcome("T_SeqDescAfterLast"), Outcome::Pass);
}
