//! `New()` on a ListPart of a host the test opened with OpenView (al-vm `pages/part_view_host_new.rs`,
//! pages/parts/PartSplitKeyInsertTiming.al:1353, :1383): the measured shape raises BC's error, another part refuses.

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
    trigger OnNewRecord(BelowxRec: Boolean)
    begin
    end;

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
page 69988 "XS Mid"
{
    PageType = CardPart; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "XS Lines") { SubPageLink = "Head Code" = field(Code); } } }
}
page 69989 "XS Card Nested"
{
    PageType = Card; SourceTable = "XS Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Mid; "XS Mid") { SubPageLink = Code = field(Code); } } }
}
codeunit 69986 C
{
    Subtype = Test;

    local procedure Seed()
    var Head: Record "XS Head"; Line: Record "XS Line";
    begin
        Line.DeleteAll(); Head.DeleteAll();
        Head.Code := 'H1'; Head.Insert();
        Line."Head Code" := 'H1'; Line."Line No." := 10000; Line."Seq No." := 2; Line.Insert();
        Line."Line No." := 20000; Line."Seq No." := 1; Line.Insert();
    end;

    // The measured shape (PartSplitKeyInsertTiming.al:1383 Card_OpenView_New).
    [Test]
    procedure T_ViewSeq()
    var P: TestPage "XS Card Seq"; Line: Record "XS Line";
    begin
        Seed();
        P.OpenView(); P.GoToKey('H1'); P.Lines.Last();
        if P.Lines.Descr.Editable() then
            Error('editable');
        asserterror P.Lines.New();
        if GetLastErrorText() <> 'New method failed because Insert is not allowed. Page = XS Lines, Id = 69983.' then
            Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'DB:ClientInsertDenied' then
            Error('code %1', GetLastErrorCode());
        P.Close();
        Line.SetRange("Head Code", 'H1');
        if Line.Count() <> 0 then
            Error('lines %1', Line.Count());
    end;

    // Not measured: a part read in primary key order.
    [Test]
    procedure T_ViewPK()
    var P: TestPage "XS Card PK";
    begin
        Seed();
        P.OpenView(); P.GoToKey('H1'); P.Lines.Last();
        if P.Lines.Descr.Editable() then
            Error('editable');
        asserterror P.Lines.New();
    end;

    // Not measured: New() without the part's Last before it.
    [Test]
    procedure T_ViewSeqNoMove()
    var P: TestPage "XS Card Seq";
    begin
        Seed();
        P.OpenView(); P.GoToKey('H1');
        asserterror P.Lines.New();
    end;
}
codeunit 69990 C2
{
    Subtype = Test;

    // Not measured: a part of a part of the host the test opened with OpenView (main ran the New() with no error).
    [Test]
    procedure T_ViewNestedPart()
    var P: TestPage "XS Card Nested"; Head: Record "XS Head"; Line: Record "XS Line";
    begin
        Line.DeleteAll(); Head.DeleteAll();
        Head.Code := 'H1'; Head.Insert();
        Line."Head Code" := 'H1'; Line."Line No." := 10000; Line.Insert();
        P.OpenView(); P.GoToKey('H1');
        asserterror P.Mid.Lines.New();
    end;
}
"#;

fn outcome(name: &str) -> Outcome {
    outcome_in("C", name)
}

fn outcome_in(codeunit: &str, name: &str) -> Outcome {
    al_testkit::outcomes_in(SRC, codeunit)
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
}

#[test]
fn new_on_the_measured_part_of_a_view_host_raises_the_insert_denied_error() {
    assert_eq!(outcome("T_ViewSeq"), Outcome::Pass);
}

#[test]
fn new_on_another_part_of_a_view_host_is_refused() {
    for name in ["T_ViewPK", "T_ViewSeqNoMove"] {
        match outcome(name) {
            Outcome::NotRun(why) => assert!(why.contains("the test opened with OpenView"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn new_on_a_part_of_a_part_of_a_view_host_is_refused() {
    match outcome_in("C2", "T_ViewNestedPart") {
        Outcome::NotRun(why) => assert!(why.contains("the test opened with OpenView"), "{why}"),
        other => panic!("{other:?}"),
    }
}
