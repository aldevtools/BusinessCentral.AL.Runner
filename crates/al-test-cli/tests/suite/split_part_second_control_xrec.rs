//! The xRec OnInsertRecord of an AutoSplitKey ListPart sees when SetValue on a second control inserts a new line (al-vm
//! `pages/split_part_second_control_xrec.rs`, pages/parts/PartSplitKeyInsertTiming.al): the
//! measured shapes get the row the line was started below; every neighbor shape keeps main's outcome, the blank xRec.

use al_testrun::Outcome;

const SRC: &str = r#"
table 69980 "XS Head" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 69981 "XS Line"
{
    fields { field(1; "Head Code"; Code[10]) { } field(2; "Line No."; Integer) { } field(3; "Seq No."; Integer) { } field(4; Descr; Text[30]) { } field(5; Qty; Decimal) { } }
    keys { key(PK; "Head Code", "Line No.") { } key(Seq; "Head Code", "Seq No.") { } }
}
// The code of table 62722 of the probe (PartSplitKeyInsertTiming.al): field OnValidate and OnInsert that log and set a field.
table 69988 "XS Line W"
{
    fields
    {
        field(1; "Head Code"; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; "Seq No."; Integer) { trigger OnValidate() var Log: Codeunit "XS Log"; begin Log.Add('T-V Seq ' + Format("Line No.")); Qty := Qty + 1; end; }
        field(4; Descr; Text[30]) { trigger OnValidate() var Log: Codeunit "XS Log"; begin Log.Add('T-V Descr ' + Format("Line No.")); Qty := 7; end; }
        field(5; Qty; Decimal) { }
    }
    keys { key(PK; "Head Code", "Line No.") { } key(Seq; "Head Code", "Seq No.") { } }
    trigger OnInsert() var Log: Codeunit "XS Log"; begin Log.Add('T-I ' + Format("Line No.")); Descr := Descr + '+ins'; end;
}
// Not measured: a field OnValidate that deletes the row the line was started below.
table 69989 "XS Line D"
{
    fields
    {
        field(1; "Head Code"; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; "Seq No."; Integer) { }
        field(4; Descr; Text[30]) { trigger OnValidate() var L: Record "XS Line D"; Log: Codeunit "XS Log"; begin if L.Get("Head Code", 40000) then L.Delete(); Log.Add('T-V del'); end; }
        field(5; Qty; Decimal) { }
    }
    keys { key(PK; "Head Code", "Line No.") { } key(Seq; "Head Code", "Seq No.") { } }
}
// Not measured: a table OnInsert that reads xRec.
table 69990 "XS Line X"
{
    fields { field(1; "Head Code"; Code[10]) { } field(2; "Line No."; Integer) { } field(3; "Seq No."; Integer) { } field(4; Descr; Text[30]) { } field(5; Qty; Decimal) { } }
    keys { key(PK; "Head Code", "Line No.") { } key(Seq; "Head Code", "Seq No.") { } }
    trigger OnInsert() var Log: Codeunit "XS Log"; begin Log.Add('T-I ' + Format("Line No.") + ' tx=' + Format(xRec."Line No.")); end;
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
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } field(Qty; Rec.Qty) { } } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var Log: Codeunit "XS Log";
    begin
        Log.Add('INS ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No."));
        exit(true);
    end;
}
page 69991 "XS Lines W"
{
    PageType = ListPart; SourceTable = "XS Line W"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } field(Qty; Rec.Qty) { } } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var Log: Codeunit "XS Log";
    begin
        Log.Add('INS ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No."));
        exit(true);
    end;
}
page 69992 "XS Lines D"
{
    PageType = ListPart; SourceTable = "XS Line D"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } field(Qty; Rec.Qty) { } } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var Log: Codeunit "XS Log";
    begin
        Log.Add('INS ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No."));
        exit(true);
    end;
}
page 69993 "XS Lines X"
{
    PageType = ListPart; SourceTable = "XS Line X"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } field(Qty; Rec.Qty) { } } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var Log: Codeunit "XS Log";
    begin
        Log.Add('INS ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No."));
        exit(true);
    end;
}
page 69994 "XS Lines False"
{
    PageType = ListPart; SourceTable = "XS Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } field(Qty; Rec.Qty) { } } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var Log: Codeunit "XS Log";
    begin
        Log.Add('INS ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No."));
        exit(false);
    end;
}
page 69995 "XS Lines StvPK"
{
    PageType = ListPart; SourceTable = "XS Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None; SourceTableView = sorting("Head Code", "Line No.");
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } field(Qty; Rec.Qty) { } } } }
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
page 69996 "XS Card W"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines W") { SubPageLink = "Head Code" = field(Code); SubPageView = sorting("Head Code", "Seq No."); } } }
}
page 69997 "XS Card D"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines D") { SubPageLink = "Head Code" = field(Code); } } }
}
page 69998 "XS Card X"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines X") { SubPageLink = "Head Code" = field(Code); SubPageView = sorting("Head Code", "Seq No."); } } }
}
page 69999 "XS Card False"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines False") { SubPageLink = "Head Code" = field(Code); } } }
}
page 69979 "XS Card StvPK"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines StvPK") { SubPageLink = "Head Code" = field(Code); } } }
}
codeunit 69986 C
{
    Subtype = Test;

    var Log: Codeunit "XS Log";

    // Lines of H1 (Line No./Seq No.): 10000/20, 20000/40, 30000/10, 40000/30; in Seq No. order the last is 20000. H2 has
    // the one line 10000. H3 has 25 lines, 10000 to 250000.
    local procedure Seed()
    var Head: Record "XS Head"; Line: Record "XS Line"; W: Record "XS Line W"; D: Record "XS Line D"; X: Record "XS Line X"; i: Integer;
    begin
        Line.DeleteAll(); W.DeleteAll(); D.DeleteAll(); X.DeleteAll(); Head.DeleteAll();
        Head.Code := 'H1'; Head.Insert();
        Head.Code := 'H2'; Head.Insert();
        Head.Code := 'H3'; Head.Insert();
        AddLine(10000, 20); AddLine(20000, 40); AddLine(30000, 10); AddLine(40000, 30);
        Line."Head Code" := 'H2'; Line."Line No." := 10000; Line.Insert();
        for i := 1 to 25 do begin
            Line."Head Code" := 'H3'; Line."Line No." := i * 10000; Line."Seq No." := i; Line.Insert();
        end;
        Log.Take();
    end;

    local procedure AddLine(LineNo: Integer; SeqNo: Integer)
    var Line: Record "XS Line"; W: Record "XS Line W"; D: Record "XS Line D"; X: Record "XS Line X";
    begin
        Line."Head Code" := 'H1'; Line."Line No." := LineNo; Line."Seq No." := SeqNo; Line.Insert();
        W."Head Code" := 'H1'; W."Line No." := LineNo; W."Seq No." := SeqNo; W.Insert();
        D."Head Code" := 'H1'; D."Line No." := LineNo; D."Seq No." := SeqNo; D.Insert();
        X."Head Code" := 'H1'; X."Line No." := LineNo; X."Seq No." := SeqNo; X.Insert();
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

    // The measured table code (PartSplitKeyInsertTiming.al:1197 TableTrig_Card_DescrSeq, table 62722).
    [Test]
    procedure T_TableCodeW()
    var P: TestPage "XS Card W";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines."Seq No.".SetValue(25);
        Check('T-V Descr 0;INS 35000 x=20000;T-I 35000;T-V Seq 35000;');
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

    // Not measured: a part of more rows than the client holds (main's outcome).
    [Test]
    procedure T_BigPartAfterLast()
    var P: TestPage "XS Card PK";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H3'); P.Lines.Last(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines.Qty.SetValue(3);
        Check('INS 260000 x=0;');
        P.Close();
    end;

    // Not measured: no move before New() (main's outcome).
    [Test]
    procedure T_OneRowNoMove()
    var P: TestPage "XS Card PK";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H2'); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines.Qty.SetValue(3);
        Check('INS 20000 x=0;');
        P.Close();
    end;

    // Not measured: First before New() (main's outcome).
    [Test]
    procedure T_OneRowFirst()
    var P: TestPage "XS Card Seq";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H2'); P.Lines.First(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines.Qty.SetValue(3);
        Check('INS 20000 x=0;');
        P.Close();
    end;

    // Not measured: the part's own SourceTableView, in primary key order (main's outcome).
    [Test]
    procedure T_OwnViewPK()
    var P: TestPage "XS Card StvPK";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines.Qty.SetValue(3);
        Check('INS 50000 x=0;');
        P.Close();
    end;

    // Not measured: the first value typed into the key control (main's outcome).
    [Test]
    procedure T_KeyFirst()
    var P: TestPage "XS Card PK";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New();
        P.Lines."Line No.".SetValue(45000); P.Lines.Descr.SetValue('a');
        Check('INS 45000 x=0;');
        P.Close();
    end;

    // Not measured: a field OnValidate that deletes the row the line was started below (main's outcome).
    [Test]
    procedure T_ValidateDeletesAnchor()
    var P: TestPage "XS Card D";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines.Qty.SetValue(3);
        Check('T-V del;INS 50000 x=0;');
        P.Close();
    end;

    // Not measured: a table OnInsert that reads xRec (main's outcome).
    [Test]
    procedure T_TableReadsXRec()
    var P: TestPage "XS Card X";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines."Seq No.".SetValue(25);
        Check('INS 35000 x=0;T-I 35000 tx=35000;');
        P.Close();
    end;

    // Not measured: OnInsertRecord answers false; the third control inserts again (main's outcome both times).
    [Test]
    procedure T_InsertFalseThenThird()
    var P: TestPage "XS Card False";
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New();
        P.Lines.Descr.SetValue('a'); P.Lines.Qty.SetValue(3); P.Lines."Seq No.".SetValue(25);
        Check('INS 50000 x=0;INS 50000 x=0;');
    end;
}
"#;

fn outcomes() -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(SRC, "C")
}

fn outcome_of(all: &[(String, Outcome)], name: &str) -> Outcome {
    all.iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
        .clone()
}

#[test]
fn a_line_inserted_at_the_second_control_after_the_last_row_sees_that_row_as_xrec() {
    let all = outcomes();
    for name in ["T_SeqAfterLast", "T_TableCodeW"] {
        assert_eq!(outcome_of(&all, name), Outcome::Pass, "{name}");
    }
}

#[test]
fn every_neighbor_shape_keeps_the_blank_xrec_of_main() {
    let all = outcomes();
    let mut wrong = Vec::new();
    for name in [
        "T_SeqDescAfterLast",
        "T_BigPartAfterLast",
        "T_OneRowNoMove",
        "T_OneRowFirst",
        "T_OwnViewPK",
        "T_KeyFirst",
        "T_ValidateDeletesAnchor",
        "T_TableReadsXRec",
        "T_InsertFalseThenThird",
    ] {
        let got = outcome_of(&all, name);
        if got != Outcome::Pass {
            wrong.push(format!("{name}: {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
