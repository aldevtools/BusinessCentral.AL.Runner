//! FilterGroup(-1) on a page's Rec (gap G5-21, pages/lt-y50-or-group-page-rec/Y50OrGroupPageRec.al): OnOpenPage's OR filters
//! are dropped after it, marks and MarkedOnly stay (the page shows the marked rows); a handed record loses them before
//! OnOpenPage and the list opens on its row as a list opens on a row OnOpenPage left Rec on.
//! AL source -> compile -> run as a test codeunit.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62560 "OG Entry" { fields { field(1; Id; Integer) { } field(2; Approver; Code[20]) { } field(3; Sender; Code[20]) { } }
    keys { key(PK; Id) { Clustered = true; } } }
codeunit 62561 "OG Log" { SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin if Entries <> '' then Entries += ';'; Entries += E; end;
    procedure Take() R: Text begin R := Entries; Entries := ''; end;
    procedure Describe(var R: Record "OG Entry"): Text
    var G: Integer; T: Text;
    begin
        G := R.FilterGroup(); R.FilterGroup(-1); T := R.GetFilters(); R.FilterGroup(G);
        exit('gm1=' + T + ' n=' + Format(R.Count()));
    end;
    procedure Seed()
    var E: Record "OG Entry";
    begin
        E.DeleteAll(); Add2(1, 'U1', 'X'); Add2(2, 'X', 'U1'); Add2(3, 'X', 'X'); Add2(4, 'U1', 'U1'); Add2(5, 'Y', 'Y'); Take();
    end;
    local procedure Add2(Id: Integer; A: Code[20]; S: Code[20]) var E: Record "OG Entry"; begin E.Id := Id; E.Approver := A; E.Sender := S; E.Insert(); end; }
page 62562 "OG Marks" { PageType = List; SourceTable = "OG Entry";
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
    actions { area(Processing) { action(Show) { trigger OnAction() begin Log.Add('act ' + Log.Describe(Rec)); end; } } }
    trigger OnOpenPage()
    begin
        Rec.FilterGroup(-1); Rec.SetRange(Approver, 'U1'); Rec.SetRange(Sender, 'U1');
        if Rec.FindSet() then repeat Rec.Mark(true); until Rec.Next() = 0;
        Rec.MarkedOnly(true); Rec.FilterGroup(0);
    end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Format(Rec.Id)); end;
    var Log: Codeunit "OG Log"; }
page 62563 "OG Plain" { PageType = List; SourceTable = "OG Entry";
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
    trigger OnOpenPage() begin Log.Add('open ' + Log.Describe(Rec)); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Format(Rec.Id)); end;
    var Log: Codeunit "OG Log"; }
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62570 T {{ Subtype = Test; {tests} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert!(!o.is_empty());
    o.into_iter().map(|(n, r)| (n.to_string(), r)).collect()
}

fn passes(tests: &str) {
    let bad: Vec<_> = outcomes(tests)
        .into_iter()
        .filter(|(_, o)| *o != Outcome::Pass)
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn or_filters_left_by_onopenpage_are_dropped_and_the_marks_stay() {
    passes(
        r#"var Log: Codeunit "OG Log";
        [Test] procedure Walk() var P: TestPage "OG Marks"; T: Text; begin
            Log.Seed(); P.OpenView();
            T := P.Id.Value + ':' + Log.Take();
            if T <> '4:R:4;R:2;R:1;R:4;R:4;C:4' then Error('open %1', T);
            P.Show.Invoke(); T := Log.Take();
            if not T.StartsWith('R:4;act gm1=Marked: Yes n=3') then Error('act %1', T);
            T := ''; if P.First() then repeat T += P.Id.Value; until not P.Next();
            if T <> '124' then Error('walk %1', T); end;
        [Test] procedure Handed() var P: TestPage "OG Plain"; E: Record "OG Entry"; T: Text; begin
            Log.Seed();
            E.FilterGroup(-1); E.SetRange(Approver, 'U1'); E.SetRange(Sender, 'U1');
            if E.FindSet() then repeat E.Mark(true); until E.Next() = 0;
            E.FilterGroup(0); E.MarkedOnly(true);
            P.Trap(); Page.Run(Page::"OG Plain", E);
            T := P.Id.Value + ':' + Log.Take();
            if T <> '4:open gm1=Marked: Yes n=3;R:4;R:2;R:1;R:4;R:4;C:4' then Error('open %1', T); end;"#,
    );
}
