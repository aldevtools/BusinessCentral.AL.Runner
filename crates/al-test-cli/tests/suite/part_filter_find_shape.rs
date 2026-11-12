//! A ListPart that serves its rows from OnFindRecord and OnNextRecord after a TestPart filter (crates/al-vm/src/pages/
//! part_filter_find.rs): only the pinned host, a ListPlus opened for view, reads `find(-)` without the step back
//! (testpage/parts/PartFilterExpressionControl.al:356). Every other host, open mode and call keeps the reads of the list before.

use al_testrun::Outcome;

const SRC: &str = r#"
table 69940 "RW Item" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 69941 "RW Entry"
{
    fields { field(1; "Item No."; Code[10]) { } field(2; "Entry No."; Integer) { } field(3; Day; Date) { } }
    keys { key(PK; "Item No.", "Entry No.") { } }
}
codeunit 69942 "RW Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Line: Text) begin Entries += Line + ' '; end;
    procedure Take() Result: Text begin Result := '[' + Entries + ']'; Entries := ''; end;
}
page 69943 "RW Find Entries"
{
    PageType = ListPart; SourceTable = "RW Entry"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } field(Day; Rec.Day) { } field(V; V) { } } } }
    var V: Integer;
    trigger OnFindRecord(Which: Text): Boolean var L: Codeunit "RW Log"; begin L.Add('find(' + Which + ')'); exit(Rec.Find(Which)); end;
    trigger OnNextRecord(Steps: Integer): Integer var L: Codeunit "RW Log"; begin L.Add('next(' + Format(Steps) + ')'); exit(Rec.Next(Steps)); end;
    trigger OnAfterGetRecord() var L: Codeunit "RW Log"; begin L.Add('agr' + Format(Rec."Entry No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "RW Log"; begin L.Add('agcr' + Format(Rec."Entry No.")); end;
    procedure SetV(N: Integer) begin V := N; end;
}
page 69944 "RW ListPlus Linked"
{
    PageType = ListPlus; SourceTable = "RW Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(E; "RW Find Entries") { SubPageLink = "Item No." = field("No."); } } }
}
page 69945 "RW Card Linked"
{
    PageType = Card; SourceTable = "RW Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(E; "RW Find Entries") { SubPageLink = "Item No." = field("No."); } } }
}
page 69946 "RW Doc Unlinked"
{
    PageType = Document; SourceTable = "RW Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(E; "RW Find Entries") { } } }
    trigger OnAfterGetRecord() begin CurrPage.E.Page.SetV(StrLen(Rec."No.") + 40); end;
}
codeunit 69947 "RW Tests"
{
    Subtype = Test;
    var Log: Codeunit "RW Log";
    local procedure Seed()
    var Item: Record "RW Item"; E: Record "RW Entry"; i: Integer;
    begin
        E.DeleteAll(); Item.DeleteAll();
        Item."No." := 'I1'; Item.Insert(); Item."No." := 'I22'; Item.Insert();
        for i := 1 to 4 do begin E.Init(); E."Item No." := 'I1'; E."Entry No." := i; E.Day := DMY2Date(i, 1, 2025); E.Insert(); end;
        for i := 5 to 6 do begin E.Init(); E."Item No." := 'I22'; E."Entry No." := i; E.Day := DMY2Date(i, 1, 2025); E.Insert(); end;
        Log.Take();
    end;
    [Test] procedure A_ListPlusView_Filter() var P: TestPage "RW ListPlus Linked";
    begin Seed(); P.OpenView(); P.GoToKey('I1'); Log.Take(); P.E.Filter.SetFilter("Entry No.", '2..3'); Error('pinned-like: %1 row=%2', Log.Take(), P.E."Entry No.".Value); end;
    [Test] procedure N1_CardView_Filter() var P: TestPage "RW Card Linked";
    begin Seed(); P.OpenView(); P.GoToKey('I1'); Log.Take(); P.E.Filter.SetFilter("Entry No.", '2..3'); Error('%1 row=%2', Log.Take(), P.E."Entry No.".Value); end;
    [Test] procedure N2_ListPlusEdit_Filter() var P: TestPage "RW ListPlus Linked";
    begin Seed(); P.OpenEdit(); P.GoToKey('I1'); Log.Take(); P.E.Filter.SetFilter("Entry No.", '2..3'); Error('%1 row=%2', Log.Take(), P.E."Entry No.".Value); end;
    [Test] procedure N3_ListPlusView_OpenThenFirst() var P: TestPage "RW ListPlus Linked"; B: Boolean;
    begin Seed(); P.OpenView(); P.GoToKey('I1'); Log.Take(); B := P.E.First(); Error('first=%1 %2', B, Log.Take()); end;
    [Test] procedure N3b_ListPlusEdit_OpenThenFirst() var P: TestPage "RW ListPlus Linked"; B: Boolean;
    begin Seed(); P.OpenEdit(); P.GoToKey('I1'); Log.Take(); B := P.E.First(); Error('first=%1 %2', B, Log.Take()); end;
    [Test] procedure N5_DocEdit_Unlinked_GoToKeyMoves() var P: TestPage "RW Doc Unlinked"; A: Text;
    begin Seed(); P.OpenEdit(); P.GoToKey('I1'); A := P.E.V.Value; Log.Take(); P.GoToKey('I22'); Error('before=%1 after=%2 %3', A, P.E.V.Value, Log.Take()); end;
    [Test] procedure N6_ListPlusEdit_FilterNoRow_First() var P: TestPage "RW ListPlus Linked"; T: Text; B: Boolean;
    begin Seed(); P.OpenEdit(); P.GoToKey('I1'); Log.Take(); P.E.Filter.SetFilter("Entry No.", '99'); T := Log.Take(); B := P.E.First(); Error('%1 first=%2 %3', T, B, Log.Take()); end;
}
"#;

fn failure(outcomes: &[(String, Outcome)], name: &str) -> String {
    match &outcomes
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
    {
        Outcome::Fail(why) => why.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn only_a_list_plus_host_opened_for_view_lands_from_the_view_start() {
    let o = al_testkit::outcomes_in(SRC, "RW Tests");
    let mut wrong = Vec::new();
    let mut check = |name: &str, want: &str| {
        let f = failure(&o, name);
        if !f.starts_with(want) {
            wrong.push(format!("{name}: {f}"));
        }
    };
    // The pinned host (PartFilterExpressionControl.al:356 `P.find(-)=1 ... P.next=1 P.find(=)=1`).
    check(
        "A_ListPlusView_Filter",
        "pinned-like: [find(-) agr2 next(1) agr3 next(1) agr2 find(=) agr2 agcr2 ] row=2",
    );
    // Not pinned: the reads of the list before.
    let before = "[find(=><) agr2 next(1) agr3 next(1) agr2 next(-1) find(=) agr2 agcr2 ] row=2";
    check("N1_CardView_Filter", before);
    check("N2_ListPlusEdit_Filter", before);
    check("N3_ListPlusView_OpenThenFirst", "first=Yes [agr1 find(=) agr1 agcr1 ]");
    check("N3b_ListPlusEdit_OpenThenFirst", "first=Yes [agr1 find(=) agr1 agcr1 ]");
    check(
        "N5_DocEdit_Unlinked_GoToKeyMoves",
        "before=42 after=43 [find(=><) agr1 next(1) agr2 next(1) agr3 next(1) agr4 next(1) agr5 next(1) agr6 next(1) agr1 next(-1) find(=) agr1 agcr1 ]",
    );
    check(
        "N6_ListPlusEdit_FilterNoRow_First",
        "[find(=><) agcr99 agcr99 agcr99 ] first=No [find(-) ]",
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
