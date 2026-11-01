//! Gap cycle 4, batch 4a: the page shapes corpus writer Y41 pinned (pages/lt-y41-page-probes, corpus PR 964).
//! AL source -> compile -> run as a test codeunit; the values are the ones real BC produced there.

use al_test_cli::report::Outcome;

fn passes(src: &str) {
    let o = al_testkit::outcomes_in(src, "T");
    assert!(!o.is_empty());
    let bad: Vec<_> = o.into_iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:?}");
}

// G4-02: `Page.RunModal(Id, Rec)` called as a statement leaves Rec as it was, whatever the page read or saved; the expression form
// takes the stored row (Y41RunModalStatementRec.al:200 RMS_Card_StaleRec_Ok, :222 _ExpressionForm, :286 RMS_Card_PageEdits_Ok,
// :300 RMS_List_Next_Ok, :333 RMS_List_Next_Ok_ExpressionForm).
const RMS: &str = r#"
table 69400 "RS Row" { fields { field(1; "Code"; Code[20]) { } field(2; Name; Text[50]) { } field(3; Counter; Integer) { } }
    keys { key(PK; "Code") { Clustered = true; } } }
page 69401 "RS Card" { PageType = Card; SourceTable = "RS Row"; InsertAllowed = false;
    layout { area(Content) { group(G) { field(CodeCtl; Rec."Code") { } field(NameCtl; Rec.Name) { } } } } }
page 69402 "RS List" { PageType = List; SourceTable = "RS Row";
    layout { area(Content) { repeater(R) { field(CodeCtl; Rec."Code") { } field(NameCtl; Rec.Name) { } } } } }
codeunit 69403 T
{
    Subtype = Test;
    var G: Record "RS Row";
    local procedure Seed() var R: Record "RS Row"; begin R.DeleteAll();
        R."Code" := 'A'; R.Name := 'alpha'; R.Counter := 1; R.Insert(); R."Code" := 'B'; R.Name := 'bravo'; R.Insert(); end;
    local procedure Stale() var O: Record "RS Row"; begin O.Get('A'); O.Name := 'db'; O.Counter := 5; O.Modify(); end;
    local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1, want %2', Got, Want); end;
    [ModalPageHandler] procedure OkHandler(var P: TestPage "RS Card") begin P.OK().Invoke(); end;
    [ModalPageHandler] procedure SetNameOkHandler(var P: TestPage "RS Card") begin P.NameCtl.SetValue('page'); P.OK().Invoke(); end;
    [ModalPageHandler] procedure NextOkHandler(var P: TestPage "RS List") begin P.Next(); P.OK().Invoke(); end;
    [Test] [HandlerFunctions('OkHandler')] procedure StaleStatement() begin
        Seed(); G.Get('A'); Stale(); G.SetRecFilter(); Page.RunModal(Page::"RS Card", G);
        Check(G.Name + '|' + Format(G.Counter) + '|' + G.GetFilters(), 'alpha|1|Code: A'); end;
    [Test] [HandlerFunctions('OkHandler')] procedure StaleExpression() var A: Action; begin
        Seed(); G.Get('A'); Stale(); G.SetRecFilter(); A := Page.RunModal(Page::"RS Card", G);
        Check(Format(A) + '|' + G.Name + '|' + Format(G.Counter), 'LookupOK|db|5'); end;
    [Test] [HandlerFunctions('SetNameOkHandler')] procedure PageEditsStatement() var D: Record "RS Row"; begin
        Seed(); G.Get('A'); G.SetRecFilter(); Page.RunModal(Page::"RS Card", G); D.Get('A');
        Check(G.Name + '|' + D.Name, 'alpha|page'); end;
    [Test] [HandlerFunctions('NextOkHandler')] procedure ListNextStatement() var R: Record "RS Row"; begin
        Seed(); R.Get('A'); Page.RunModal(Page::"RS List", R); Check(R."Code" + '|' + R.GetFilters(), 'A|'); end;
    [Test] [HandlerFunctions('NextOkHandler')] procedure ListNextExpression() var R: Record "RS Row"; A: Action; begin
        Seed(); R.Get('A'); A := Page.RunModal(Page::"RS List", R); Check(Format(A) + '|' + R."Code", 'LookupOK|B'); end;
}
"#;

#[test]
fn run_modal_as_a_statement_leaves_rec_as_it_was() {
    passes(RMS);
}

// G4-03: GoToKey / GoToRecord on a list with OnFindRecord, OnNextRecord, OnAfterGetRecord and OnAfterGetCurrRecord, 30 rows: the
// client walks from the first row one move at a time (`R:x;F:==Yes;R:x;C:x`); on the last row of the first window it first reads
// the rest of the view (`R:R020;N:1=1;R:R021;...;N:1=0`). A miss walks every row and goes back to the first row
// (Y41GoToKeyFindListCurr.al:157 _PastWindow_GoToRecord, :199 _LastRow, :213 _Missing, :227 _FromOtherRow, :253 _ThenNext).
fn gk_step(i: u32) -> String {
    format!("R:R{i:03};F:==Yes;R:R{i:03};C:R{i:03}")
}

fn gk_walk(from: u32, to: u32) -> String {
    let mut parts = Vec::new();
    for i in from..=to {
        if i == 20 {
            parts.push(
                (20..=30)
                    .map(|j| format!("R:R{j:03}"))
                    .collect::<Vec<_>>()
                    .join(";N:1=1;")
                    + ";N:1=0",
            );
        }
        parts.push(gk_step(i));
    }
    parts.join(";")
}

fn gk_source() -> String {
    let past = gk_walk(2, 27);
    let missing = format!("{};{}", gk_walk(2, 30), gk_step(1));
    let second = format!("{};{}", gk_step(1), gk_walk(2, 27));
    format!(
        r#"
table 69410 "GK Row" {{ fields {{ field(1; "No."; Code[20]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
codeunit 69411 "GK Log" {{ SingleInstance = true; var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ';'; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end; }}
page 69412 "GK List" {{ PageType = List; SourceTable = "GK Row"; Editable = false;
    layout {{ area(Content) {{ repeater(Rows) {{ field(NoCtl; Rec."No.") {{ }} }} }} }}
    actions {{ area(Processing) {{ action(ShownAction) {{ Visible = ShowAct; trigger OnAction() begin end; }} }} }}
    var ShowAct: Boolean; L: Codeunit "GK Log";
    trigger OnFindRecord(Which: Text): Boolean var F: Boolean; begin F := Rec.Find(Which); L.Add('F:' + Which + '=' + Format(F)); exit(F); end;
    trigger OnNextRecord(Steps: Integer): Integer var M: Integer; begin M := Rec.Next(Steps); L.Add('N:' + Format(Steps) + '=' + Format(M)); exit(M); end;
    trigger OnAfterGetRecord() begin L.Add('R:' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin ShowAct := Rec."No." <> ''; L.Add('C:' + Rec."No."); end; }}
codeunit 69413 T
{{
    Subtype = Test;
    var L: Codeunit "GK Log";
    local procedure Open(var P: TestPage "GK List") var R: Record "GK Row"; i: Integer; begin
        R.DeleteAll(); for i := 1 to 30 do begin R."No." := 'R' + Format(i, 0, '<Integer,3><Filler Character,0>'); R.Insert(); end;
        P.OpenView(); L.Take(); end;
    local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1, want %2', Got, Want); end;
    [Test] procedure PastWindowGoToRecord() var P: TestPage "GK List"; R: Record "GK Row"; begin
        Open(P); R.Get('R027'); Check(Format(P.GoToRecord(R)), 'Yes'); Check(L.Take(), '{past}');
        Check(P.NoCtl.Value + Format(P.ShownAction.Visible()), 'R027Yes');
        Check(Format(P.Next()) + P.NoCtl.Value, 'YesR028'); Check(L.Take(), '{next}');
        Check(Format(P.Previous()) + P.NoCtl.Value, 'YesR027'); Check(L.Take(), '{prev}'); end;
    [Test] procedure LastRow() var P: TestPage "GK List"; begin
        Open(P); Check(Format(P.GoToKey('R030')), 'Yes'); Check(L.Take(), '{last}'); Check(P.NoCtl.Value, 'R030'); end;
    [Test] procedure Missing() var P: TestPage "GK List"; begin
        Open(P); Check(Format(P.GoToKey('R999')), 'No'); Check(L.Take(), '{missing}'); Check(P.NoCtl.Value, 'R001'); end;
    [Test] procedure FromOtherRow() var P: TestPage "GK List"; begin
        Open(P); Check(Format(P.GoToKey('R005')), 'Yes'); Check(L.Take(), '{first}');
        Check(Format(P.GoToKey('R027')), 'Yes'); Check(L.Take(), '{second}'); Check(P.NoCtl.Value, 'R027'); end;
}}
"#,
        next = gk_step(28),
        prev = gk_step(27),
        last = gk_walk(2, 30),
        first = gk_walk(2, 5),
    )
}

#[test]
fn go_to_key_on_a_find_list_with_current_record_trigger_walks_past_the_window() {
    passes(&gk_source());
}

// G4-05: SetTableView hands a Page variable the marks and MarkedOnly of the record (the page shows only the marked rows, also for a
// temporary record and with no mark at all), and SetSelectionFilter after LookupOK hands back the marked set; SetRecord hands no
// marks over (Y41PageVarMarkedOnly.al:140 _Shown, :158 _SelectionAfter, :186 _FilterGroups, :222 _NoMarks, :240
// PVM_SetRecord_MarkedOnly, :258 _TempRec).
const PVM: &str = r#"
table 69420 "PV Row" { fields { field(1; "No."; Code[20]) { } field(2; Group; Integer) { } field(3; Qty; Decimal) { } }
    keys { key(PK; "No.") { Clustered = true; } } }
codeunit 69421 "PV Log" { SingleInstance = true; var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ';'; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end; }
page 69422 "PV List" { PageType = List; SourceTable = "PV Row"; Editable = false;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnOpenPage() var L: Codeunit "PV Log"; begin
        L.Add('open filters=' + Rec.GetFilters() + ' mo=' + Format(Rec.MarkedOnly()) + ' count=' + Format(Rec.Count()) + ' at=' + Rec."No."); end; }
codeunit 69423 T
{
    Subtype = Test;
    var L: Codeunit "PV Log";
    local procedure Seed() var R: Record "PV Row"; i: Integer; begin R.DeleteAll();
        for i := 1 to 6 do begin R."No." := 'R' + Format(i); R.Group := 1; if i > 4 then R.Group := 2; R.Qty := 10 * i; R.Insert(); end; end;
    local procedure Mark(var R: Record "PV Row"; Keys: Text) var P: Text; begin foreach P in Keys.Split(',') do begin R.Get(P); R.Mark(true); end; end;
    local procedure Walk(var P: TestPage "PV List") var S: Text; begin
        S := 'at=' + P.NoCtl.Value() + ' shown='; if P.First() then repeat S += P.NoCtl.Value() + ','; until not P.Next(); L.Add(S); end;
    local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1, want %2', Got, Want); end;
    local procedure Describe(var R: Record "PV Row"): Text var S: Text; begin
        S := R.GetFilters() + '|' + Format(R.MarkedOnly()) + '|' + Format(R.Count()) + '|';
        if R.FindSet() then repeat S += R."No." + ','; until R.Next() = 0; exit(S); end;
    [ModalPageHandler] procedure WalkCancel(var P: TestPage "PV List") begin Walk(P); P.Cancel().Invoke(); end;
    [ModalPageHandler] procedure WalkR4Ok(var P: TestPage "PV List") begin Walk(P); L.Add('goto=' + Format(P.GoToKey('R4'))); P.OK().Invoke(); end;
    [Test] [HandlerFunctions('WalkCancel')] procedure Shown() var R: Record "PV Row"; P: Page "PV List"; begin
        Seed(); R.SetRange(Group, 1); Mark(R, 'R2,R4'); R.MarkedOnly(true); P.SetTableView(R); P.LookupMode(true);
        Check(Format(P.RunModal()), 'LookupCancel');
        Check(L.Take(), 'open filters=Marked: Yes, Group: 1 mo=Yes count=2 at=;at=R2 shown=R2,R4,'); end;
    [Test] [HandlerFunctions('WalkR4Ok')] procedure SelectionAfter() var R: Record "PV Row"; R2: Record "PV Row"; P: Page "PV List"; begin
        Seed(); R.SetRange(Group, 1); Mark(R, 'R2,R4'); R.MarkedOnly(true); P.SetTableView(R); P.LookupMode(true);
        Check(Format(P.RunModal()), 'LookupOK'); L.Take();
        P.SetSelectionFilter(R2); Check(Describe(R2), 'Marked: Yes, Group: 1|Yes|2|R2,R4,'); end;
    [Test] [HandlerFunctions('WalkR4Ok')] procedure FilterGroups() var R: Record "PV Row"; R2: Record "PV Row"; P: Page "PV List"; begin
        Seed(); R.FilterGroup(-1); R.SetFilter(Qty, '>=%1', 30); R.SetFilter(Group, '%1', 2); R.FilterGroup(2); R.SetRange(Group, 1);
        if R.FindSet() then repeat R.Mark(true); until R.Next() = 0;
        R.MarkedOnly(true); R.FilterGroup(0); P.SetTableView(R); P.LookupMode(true);
        Check(Format(P.RunModal()), 'LookupOK');
        Check(L.Take(), 'open filters=Marked: Yes mo=Yes count=2 at=;at=R3 shown=R3,R4,;goto=Yes');
        P.SetSelectionFilter(R2); Check(Describe(R2), 'Marked: Yes|Yes|2|R3,R4,'); end;
    [Test] [HandlerFunctions('WalkCancel')] procedure NoMarks() var R: Record "PV Row"; P: Page "PV List"; begin
        Seed(); R.SetRange(Group, 1); R.MarkedOnly(true); P.SetTableView(R); P.LookupMode(true); P.RunModal();
        Check(L.Take(), 'open filters=Marked: Yes, Group: 1 mo=Yes count=0 at=;at= shown='); end;
    [Test] [HandlerFunctions('WalkCancel')] procedure SetRecordMarked() var R: Record "PV Row"; P: Page "PV List"; begin
        Seed(); Mark(R, 'R2,R4'); R.MarkedOnly(true); R.Get('R4'); P.SetRecord(R); P.LookupMode(true); P.RunModal();
        Check(L.Take(), 'open filters= mo=No count=6 at=R4;at=R4 shown=R1,R2,R3,R4,R5,R6,'); end;
    [Test] [HandlerFunctions('WalkCancel')] procedure TempRec() var S: Record "PV Row"; R: Record "PV Row" temporary; P: Page "PV List"; begin
        Seed(); if S.FindSet() then repeat R := S; R.Insert(); until S.Next() = 0;
        R.SetRange(Group, 1); Mark(R, 'R2,R4'); R.MarkedOnly(true); P.SetTableView(R); P.LookupMode(true); P.RunModal();
        Check(L.Take(), 'open filters=Marked: Yes, Group: 1 mo=Yes count=2 at=;at=R2 shown=R2,R4,'); end;
}
"#;

#[test]
fn a_page_variable_takes_the_marks_through_set_table_view_and_not_through_set_record() {
    passes(PVM);
}
