//! A List over a temporary source whose OnOpenPage filled it and changed its key and direction (gap G5-19,
//! pages/lt-y50-temp-list-key-desc/Y50TempListKeyDesc.al), and a temporary tree Worksheet whose OnOpenPage changed its key
//! (gap G5-17, pages/lt-y50-tree-expand-leaf/Y50TreeExpandLeaf.al): the page opens on Rec's row.
//! AL source -> compile -> run as a test codeunit.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62540 "TLK Mail" { fields { field(1; Id; Integer) { } field(2; Day; Integer) { } field(3; Indent; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } key(ByDay; Day) { } } }
codeunit 62541 "TLK Log" { SingleInstance = true;
    var Entries: Text; Mode: Text;
    procedure Add(E: Text) begin if Entries <> '' then Entries += ';'; Entries += E; end;
    procedure Take() R: Text begin R := Entries; Entries := ''; end;
    procedure SetMode(M: Text) begin Mode := M; Entries := ''; end;
    procedure Fill(var M: Record "TLK Mail")
    begin
        if Mode in ['ALL', 'SOME'] then begin AddRow(M, 1, 4); AddRow(M, 3, 1); end;
        if Mode = 'ALL' then begin AddRow(M, 2, 2); AddRow(M, 4, 3); end;
        if Mode <> 'ALL' then begin M.Id := 2; M.Day := 2; end;
    end;
    local procedure AddRow(var M: Record "TLK Mail"; Id: Integer; Day: Integer) begin M.Id := Id; M.Day := Day; M.Insert(); end; }
page 62542 "TLK List" { PageType = List; SourceTable = "TLK Mail"; SourceTableTemporary = true; InsertAllowed = false;
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
    trigger OnOpenPage() begin Log.Fill(Rec); Rec.SetCurrentKey(Day); Rec.Ascending(false); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Format(Rec.Id)); end;
    var Log: Codeunit "TLK Log"; }
page 62543 "TLK Tree" { PageType = Worksheet; SourceTable = "TLK Mail"; SourceTableTemporary = true; InsertAllowed = false;
    layout { area(Content) { repeater(R) { ShowAsTree = true; IndentationColumn = Rec.Indent; field(Id; Rec.Id) { } } } }
    trigger OnOpenPage() begin Rec.Id := 1; Rec.Day := 1; Rec.Insert(); Rec.Id := 2; Rec.Day := 2; Rec.Insert(); Rec.SetCurrentKey(Day); end; }
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62550 T {{ Subtype = Test; {tests} }}");
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
fn a_temporary_list_in_a_changed_key_opens_on_recs_row() {
    passes(
        r#"var Log: Codeunit "TLK Log";
        local procedure Walk(var L: TestPage "TLK List") T: Text begin if L.First() then repeat T += L.Id.Value; until not L.Next(); end;
        [Test] procedure LastInserted() var L: TestPage "TLK List"; T: Text; begin
            Log.SetMode('ALL'); L.Trap(); Page.Run(Page::"TLK List");
            T := L.Id.Value + ':' + Log.Take();
            if T <> '4:R:4;R:4;R:2;R:3;R:4;R:1;R:4;R:4;R:4;C:4' then Error('open %1', T);
            T := Walk(L); if T <> '1423' then Error('walk %1', T); end;
        [Test] procedure BufferNotInserted() var L: TestPage "TLK List"; T: Text; begin
            Log.SetMode('SOME'); L.Trap(); Page.Run(Page::"TLK List");
            T := L.Id.Value + ':' + Log.Take();
            if T <> '3:R:3;R:1;R:3;R:3;R:3;C:3' then Error('open %1', T); end;
        [Test] procedure EmptyView() var L: TestPage "TLK List"; M: Record "TLK Mail"; T: Text; begin
            Log.SetMode('NONE'); L.Trap(); Page.Run(Page::"TLK List");
            T := L.Id.Value + ':' + Log.Take();
            if T <> ':C:0' then Error('open %1', T);
            M.Id := 2; if L.GoToRecord(M) then Error('gotorecord'); if L.First() then Error('first');
            L.Close(); Log.SetMode('NONE'); L.OpenView(); T := L.Id.Value + ':' + Log.Take();
            if T <> ':' then Error('openview %1', T); end;"#,
    );
}

#[test]
fn a_temporary_tree_worksheet_in_a_changed_key_opens_on_recs_row() {
    passes(
        r#"[Test] [HandlerFunctions('H')] procedure Tree() var P: Page "TLK Tree"; begin P.RunModal(); end;
        [ModalPageHandler] procedure H(var P: TestPage "TLK Tree") begin if P.Id.Value <> '2' then Error('open %1', P.Id.Value); end;"#,
    );
}
