//! AutoSplitKey on a ListPart read in another order than the primary key (crates/al-vm/src/pages/split_key_view.rs): the shapes
//! pages/parts/PartSplitKeyOtherSortKey.al does not pin stay refused.

use al_testrun::Outcome;

const SRC: &str = r#"
table 69990 "SV Head" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 69991 "SV Line"
{
    fields { field(1; "Head Code"; Code[10]) { } field(2; "Line No."; Integer) { } field(3; "Seq No."; Integer) { } field(4; Descr; Text[30]) { } }
    keys { key(PK; "Head Code", "Line No.") { } key(Seq; "Head Code", "Seq No.") { } }
}
page 69992 "SV Lines"
{
    PageType = ListPart; SourceTable = "SV Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } } } }
}
page 69993 "SV Card"
{
    PageType = Card; SourceTable = "SV Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Code; Rec.Code) { } part(Lines; "SV Lines") { SubPageLink = "Head Code" = field(Code); SubPageView = sorting("Head Code", "Seq No."); } } }
}
page 69994 "SV List"
{
    PageType = List; SourceTable = "SV Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } part(Lines; "SV Lines") { SubPageLink = "Head Code" = field(Code); SubPageView = sorting("Head Code", "Seq No."); } } }
}
codeunit 69995 C
{
    Subtype = Test;

    local procedure Seed(A: Integer; B: Integer; C: Integer)
    var Head: Record "SV Head"; Line: Record "SV Line";
    begin
        Line.DeleteAll(); Head.DeleteAll();
        Head.Code := 'H1'; Head.Insert();
        AddLine(A, 1); AddLine(B, 2); AddLine(C, 3);
    end;

    local procedure AddLine(LineNo: Integer; SeqNo: Integer)
    var Line: Record "SV Line";
    begin
        Line."Head Code" := 'H1'; Line."Line No." := LineNo; Line."Seq No." := SeqNo; Line.Insert();
    end;

    // Not pinned: a host that is not a Card.
    [Test]
    procedure T_ListHost()
    var P: TestPage "SV List";
    begin
        Seed(10000, 20000, 30000);
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Close();
    end;

    // Not pinned: a negative number next to the line.
    [Test]
    procedure T_Negative()
    var P: TestPage "SV Card";
    begin
        Seed(-30000, -20000, -10000);
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.Last(); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Close();
    end;

    // Not pinned: the number is taken and no whole number is free between it and the next stored row (Seq order 10000, 40000,
    // 20000: below 10000 the number 20000 is taken, 20001 follows).
    [Test]
    procedure T_TakenNoRoom()
    var P: TestPage "SV Card";
    begin
        Seed(10000, 40000, 20000);
        AddLine(20001, 4);
        P.OpenEdit(); P.GoToKey('H1'); P.Lines.First(); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Close();
    end;
}
"#;

fn not_run(name: &str) -> String {
    let outcomes = al_testkit::outcomes_in(SRC, "C");
    match outcomes
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
    {
        Outcome::NotRun(why) => why,
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn a_part_read_in_another_order_on_a_list_host_is_refused() {
    let why = not_run("T_ListHost");
    assert!(why.contains("not read in primary key order"), "{why}");
}

#[test]
fn a_negative_number_next_to_the_line_is_refused() {
    let why = not_run("T_Negative");
    assert!(why.contains("next to a negative number"), "{why}");
}

#[test]
fn a_taken_number_without_room_after_it_is_refused() {
    let why = not_run("T_TakenNoRoom");
    assert!(why.contains("no number is free after it"), "{why}");
}
