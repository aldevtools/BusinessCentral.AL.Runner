//! Expand(true) on a row without children under a parent, on a tree with OnAfterGetRecord and OnAfterGetCurrRecord: nothing
//! happens, no trigger runs, the cursor stays and the rows shown stay (al-runner issue 6947; corpus
//! pages/tree-expand-leaf/TreeExpandIndentedLeaf.al, measured on BC 27.5, 28.5 and 29.0).

const SRC: &str = r#"
codeunit 69970 "TL Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin if Entries <> '' then Entries += ';'; Entries += T; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
table 69971 "TL Row" { fields { field(1; "Line No."; Integer) { } field(2; Indent; Integer) { } } keys { key(PK; "Line No.") { } } }
page 69972 "TL Tree"
{
    PageType = Worksheet; SourceTable = "TL Row"; SourceTableTemporary = true; InsertAllowed = false; DeleteAllowed = false;
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { IndentationColumn = Rec.Indent; ShowAsTree = true; TreeInitialState = CollapseAll;
        field(LineNo; Rec."Line No.") { Editable = false; } } } }
    trigger OnOpenPage()
    begin
        Add(10, 0); Add(20, 1); Add(30, 2); Add(40, 1); Add(50, 0); Add(60, 1); Add(70, 0);
        if Rec.FindFirst() then;
    end;
    local procedure Add(L: Integer; I: Integer) begin Rec.Init(); Rec."Line No." := L; Rec.Indent := I; Rec.Insert(); end;
    trigger OnAfterGetRecord() begin Log.Add('R' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C' + Format(Rec."Line No.")); end;
    var Log: Codeunit "TL Log";
}
codeunit 69973 C
{
    local procedure Step(var P: TestPage "TL Tree"): Text
    var Log: Codeunit "TL Log";
    begin
        P.Expand(true);
        exit(Format(P.IsExpanded()) + '/' + P.LineNo.Value() + ':' + Log.Take());
    end;
    local procedure Walk(var P: TestPage "TL Tree") T: Text
    var Log: Codeunit "TL Log";
    begin
        if P.First() then repeat T += P.LineNo.Value() + ','; until not P.Next();
        T += ':' + Log.Take();
    end;
    procedure LeafLevel2(): Text
    var P: TestPage "TL Tree"; Log: Codeunit "TL Log"; T: Text;
    begin
        P.OpenEdit();
        P.Expand(true);
        P.Next();
        P.Expand(true);
        P.Next();
        Log.Take();
        T := Step(P);
        T += '|' + Format(P.Next()) + '/' + P.LineNo.Value() + ':' + Log.Take();
        exit(T + '|' + Walk(P));
    end;
    procedure ExpandEvery(): Text
    var P: TestPage "TL Tree"; T: Text;
    begin
        P.OpenEdit();
        if P.First() then repeat T += P.LineNo.Value() + ','; P.Expand(true); until not P.Next();
        exit(T);
    end;
}
"#;

// TreeExpandIndentedLeaf.al Collapsed_ExpandTrue_LeafLevel2 `expand 30` `Yes/No/30:`, `next 3` `Yes/40`, walk
// `10,20,30,40,50,70,` (each row read R;R;C).
#[test]
fn expand_of_a_leaf_at_level_2_changes_nothing() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "LeafLevel2")),
        "No/30:|Yes/40:R40;R40;C40|10,20,30,40,50,70,:R10;R10;C10;R20;R20;C20;R30;R30;C30;R40;R40;C40;R50;R50;C50;R70;R70;C70"
    );
}

// Collapsed_WalkExpandingEveryRow `walked` `10,20,30,40,50,60,70,`: the reporter's walk.
#[test]
fn the_walk_that_expands_every_row_reaches_every_row() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ExpandEvery")),
        "10,20,30,40,50,60,70,"
    );
}
