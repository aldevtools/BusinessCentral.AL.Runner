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

// G4-01: after a Page-typed ModalPageHandler closed a Page variable, SetSelectionFilter hands the key filter on the row the
// handler gave SetRecord (also after LookupCancel; Sel's filters, group 2 too, are replaced), `Id: 0` with count 0 without
// SetRecord; a TestPage handler that moved with GoToKey(3) gives `Id: 3`. OpenNew on a list that is not editable opens on its first
// row without OnNewRecord; on an editable list it runs OnNewRecord and the selection is `Id: 0` (Y41SelectionAfterClose.al:198
// _HandlerSetRecord_LookupOK, :221 _FilterGroup2View, :251 _NoSetRecord, :272 _Cancel, :295 OpenNew_List_SourceTableView, :309
// _NoView, :323 _TestPageHandler_GoToKeyOk).
const SEL: &str = r#"
table 69430 "SL Row" { fields { field(1; Id; Integer) { } field(2; Grp; Code[10]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69431 "SL Log" { SingleInstance = true; var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ';'; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end; }
page 69432 "SL List" { PageType = List; SourceTable = "SL Row";
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } } } }
    procedure GetSelected(var Sel: Record "SL Row") begin CurrPage.SetSelectionFilter(Sel); end;
    procedure CurrentId(): Integer begin exit(Rec.Id); end; }
page 69433 "SL New List" { PageType = List; SourceTable = "SL Row"; SourceTableView = where(Grp = const('A')); Editable = false;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } } } }
    var L: Codeunit "SL Log";
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('new'); end;
    trigger OnAfterGetCurrRecord() var S: Record "SL Row"; begin CurrPage.SetSelectionFilter(S);
        L.Add('agcr id=' + Format(Rec.Id) + ' filters=' + S.GetFilters() + ' count=' + Format(S.Count())); end; }
page 69434 "SL New List NoView" { PageType = List; SourceTable = "SL Row";
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } } } }
    var L: Codeunit "SL Log";
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('new'); end;
    trigger OnAfterGetCurrRecord() var S: Record "SL Row"; begin CurrPage.SetSelectionFilter(S);
        L.Add('agcr id=' + Format(Rec.Id) + ' filters=' + S.GetFilters() + ' count=' + Format(S.Count())); end; }
codeunit 69435 T
{
    Subtype = Test;
    var L: Codeunit "SL Log"; RowId: Integer;
    local procedure Seed() var R: Record "SL Row"; i: Integer; begin R.DeleteAll();
        for i := 1 to 5 do begin R.Id := i; R.Grp := 'A'; if i > 3 then R.Grp := 'B'; R.Insert(); end; L.Take(); end;
    local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1, want %2', Got, Want); end;
    local procedure Describe(var S: Record "SL Row"): Text var T: Text; begin
        T := Format(S.Count()) + '|' + S.GetFilters() + '|' + Format(S.MarkedOnly()) + '|';
        if S.FindSet() then repeat T += Format(S.Id) + ','; until S.Next() = 0; exit(T); end;
    [ModalPageHandler] procedure SetRecordOk(var P: Page "SL List"; var Response: Action) var R: Record "SL Row"; begin
        Check(Format(P.CurrentId()), '0'); R.Get(RowId); P.SetRecord(R); Response := Action::LookupOK; end;
    [ModalPageHandler] procedure SetRecordCancel(var P: Page "SL List"; var Response: Action) var R: Record "SL Row"; begin
        R.Get(RowId); P.SetRecord(R); Response := Action::LookupCancel; end;
    [ModalPageHandler] procedure PlainOk(var P: Page "SL List"; var Response: Action) begin Response := Action::LookupOK; end;
    [ModalPageHandler] procedure GoToOk(var P: TestPage "SL List") begin P.GoToKey(3); P.OK().Invoke(); end;
    [Test] [HandlerFunctions('SetRecordOk')] procedure SetRecordLookupOK() var S: Record "SL Row"; P: Page "SL List"; begin
        Seed(); RowId := 3; P.LookupMode(true); Check(Format(P.RunModal()), 'LookupOK'); Check(Format(P.CurrentId()), '3');
        P.GetSelected(S); Check(Describe(S), '1|Id: 3|No|3,'); end;
    [Test] [HandlerFunctions('SetRecordOk')] procedure FilterGroup2View() var S: Record "SL Row"; P: Page "SL List"; begin
        Seed(); RowId := 2; S.FilterGroup(2); S.SetRange(Grp, 'A'); S.FilterGroup(0); P.SetTableView(S); P.LookupMode(true);
        Check(Format(P.RunModal()), 'LookupOK'); Check(Format(P.CurrentId()), '2');
        P.GetSelected(S); Check(Describe(S), '1|Id: 2|No|2,'); S.FilterGroup(2); Check(S.GetFilters(), ''); end;
    [Test] [HandlerFunctions('PlainOk')] procedure NoSetRecord() var S: Record "SL Row"; P: Page "SL List"; begin
        Seed(); P.LookupMode(true); Check(Format(P.RunModal()), 'LookupOK'); Check(Format(P.CurrentId()), '0');
        P.GetSelected(S); Check(Describe(S), '0|Id: 0|No|'); end;
    [Test] [HandlerFunctions('SetRecordCancel')] procedure SetRecordCancelled() var S: Record "SL Row"; P: Page "SL List"; begin
        Seed(); RowId := 3; P.LookupMode(true); Check(Format(P.RunModal()), 'LookupCancel'); Check(Format(P.CurrentId()), '3');
        P.GetSelected(S); Check(Describe(S), '1|Id: 3|No|3,'); end;
    [Test] [HandlerFunctions('GoToOk')] procedure TestPageGoTo() var S: Record "SL Row"; P: Page "SL List"; begin
        Seed(); P.LookupMode(true); Check(Format(P.RunModal()), 'LookupOK'); Check(Format(P.CurrentId()), '3');
        P.GetSelected(S); Check(Describe(S), '1|Id: 3|No|3,'); end;
    [Test] procedure OpenNewNotEditable() var TP: TestPage "SL New List"; begin
        Seed(); TP.OpenNew(); Check(L.Take(), 'agcr id=1 filters=Id: 1 count=1'); Check(TP.IdCtl.Value(), '1'); TP.Close(); end;
    [Test] procedure OpenNewEditable() var TP: TestPage "SL New List NoView"; begin
        Seed(); TP.OpenNew(); Check(L.Take(), 'new;agcr id=0 filters=Id: 0 count=0'); Check(TP.IdCtl.Value(), '0'); TP.Close(); end;
}
"#;

#[test]
fn selection_filter_of_a_page_closed_by_a_typed_handler_and_of_open_new() {
    passes(SEL);
}

// G4-04: Rec in the OnOpenPage of a page an action opens through RunPageLink, for a ListPlus, Document and Worksheet target, an
// Option, Date and filter('P1') link, a FlowFilter link (no field filled, CalcFields uses it) and a temporary List that inserts in
// OnOpenPage (Y41RunPageLinkOnOpen.al:440 RPL_ListPlus_FieldLinks, :458 _List_FlowFilterLink, :476 _TempList_InsertInOpen, :499
// _Card_OptionLink, :517 _Card_DateLink, :535 _Card_FilterEntry, :571 _Document_FieldLink, :589 _Worksheet_FieldLink).
fn rpl_target(id: u32, name: &str, page_type: &str, tag: &str, extra: &str) -> String {
    format!(
        r#"page {id} "{name}" {{ PageType = {page_type}; SourceTable = "RL Target"; {extra}
    layout {{ area(Content) {{ repeater(Rows) {{ field(CodeCtl; Rec."Code") {{ }} field(NumberCtl; Rec.Number) {{ }} }} }} }}
    var L: Codeunit "RL Log";
    trigger OnOpenPage() begin L.Add('{tag} open ' + Rec.Describe() + ' read=' + Rec."Code" + '/' + Format(Rec.Number)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('{tag} agcr ' + Rec.Describe()); end; }}
"#
    )
}

fn rpl_source() -> String {
    let mut src = String::from(
        r#"
table 69440 "RL Host" { fields { field(1; "Code"; Code[20]) { } field(2; Number; Integer) { } field(3; Kind; Option) { OptionMembers = A,B; }
    field(4; Day; Date) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 69441 "RL Target" { fields { field(1; "Code"; Code[20]) { } field(2; Number; Integer) { } field(3; Kind; Option) { OptionMembers = A,B; }
    field(4; Day; Date) { } field(5; "Code Filter"; Code[20]) { FieldClass = FlowFilter; }
    field(6; "Host Count"; Integer) { FieldClass = FlowField; CalcFormula = count("RL Host" where("Code" = field("Code Filter"))); } }
    keys { key(PK; "Code", Number) { Clustered = true; } }
    procedure Describe(): Text begin
        exit('code=' + Rec."Code" + ' number=' + Format(Rec.Number) + ' kind=' + Format(Rec.Kind) + ' day=' + Format(Rec.Day, 0, 9) +
            ' filters=' + Rec.GetFilters() + ' gf0=' + Rec.GetFilter("Code") + ' count=' + Format(Rec.Count())); end; }
codeunit 69442 "RL Log" { SingleInstance = true; var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ';'; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end; }
page 69443 "RL FF List" { PageType = List; SourceTable = "RL Target";
    layout { area(Content) { repeater(Rows) { field(CodeCtl; Rec."Code") { } field(NumberCtl; Rec.Number) { } } } }
    var L: Codeunit "RL Log";
    trigger OnOpenPage() begin Rec.CalcFields("Host Count");
        L.Add('list open ' + Rec.Describe() + ' cf=' + Rec.GetFilter("Code Filter") + ' hostcount=' + Format(Rec."Host Count")); end; }
page 69444 "RL Temp List" { PageType = List; SourceTable = "RL Target"; SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) { field(CodeCtl; Rec."Code") { } field(NumberCtl; Rec.Number) { } } } }
    var L: Codeunit "RL Log";
    trigger OnOpenPage() begin L.Add('temp open ' + Rec.Describe()); Rec."Code" := '';
        L.Add('temp insert blank=' + Format(Rec.Insert()) + ' ' + Rec.Describe());
        Rec.Init(); Rec."Code" := 'T1'; Rec.Number := 1; Rec.Insert(); Rec.Init(); Rec."Code" := 'P1'; Rec.Number := 2; Rec.Insert();
        L.Add('temp after inserts ' + Rec.Describe()); end; }
page 69445 "RL Host Card" { PageType = Card; SourceTable = "RL Host";
    layout { area(Content) { group(G) { field(CodeCtl; Rec."Code") { } } } }
    actions { area(Processing) {
        action(ToListPlus) { RunObject = page "RL ListPlus"; RunPageLink = "Code" = field("Code"), Number = field(Number); }
        action(ToDocument) { RunObject = page "RL Document"; RunPageLink = "Code" = field("Code"), Number = field(Number); }
        action(ToWorksheet) { RunObject = page "RL Worksheet"; RunPageLink = "Code" = field("Code"), Number = field(Number); }
        action(ToFlowFilterList) { RunObject = page "RL FF List"; RunPageLink = "Code Filter" = field("Code"); }
        action(ToTempList) { RunObject = page "RL Temp List"; RunPageLink = "Code" = field("Code"); }
        action(CardOption) { RunObject = page "RL Card"; RunPageLink = Kind = field(Kind); }
        action(CardDate) { RunObject = page "RL Card"; RunPageLink = Day = field(Day); }
        action(CardFilter) { RunObject = page "RL Card"; RunPageLink = "Code" = filter('P1'); } } } }
codeunit 69446 T
{
    Subtype = Test;
    var L: Codeunit "RL Log";
    local procedure Seed(var Host: TestPage "RL Host Card") var H: Record "RL Host"; T: Record "RL Target"; begin
        H.DeleteAll(); T.DeleteAll();
        H."Code" := 'P1'; H.Number := 7; H.Kind := 1; H.Day := 20260115D; H.Insert();
        H."Code" := 'P2'; H.Number := 9; H.Kind := 0; H.Day := 20260116D; H.Insert();
        T."Code" := 'P1'; T.Number := 7; T.Kind := 0; T.Day := 20260115D; T.Insert();
        T."Code" := 'P1'; T.Number := 8; T.Kind := 1; T.Day := 20260116D; T.Insert();
        T."Code" := 'P2'; T.Number := 7; T.Kind := 0; T.Day := 20260115D; T.Insert();
        Host.OpenView(); Host.GoToKey('P1'); L.Take(); end;
    local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1, want %2', Got, Want); end;
    [Test] procedure ListPlus() var H: TestPage "RL Host Card"; P: TestPage "RL ListPlus"; begin
        Seed(H); P.Trap(); H.ToListPlus.Invoke();
        Check(L.Take(), 'listplus open code=P1 number=7 kind=A day= filters=Code: P1, Number: 7 gf0=P1 count=1 read=P1/7;listplus agcr code=P1 number=7 kind=A day=2026-01-15 filters=Code: P1, Number: 7 gf0=P1 count=1'); end;
    [Test] procedure Document() var H: TestPage "RL Host Card"; P: TestPage "RL Document"; begin
        Seed(H); P.Trap(); H.ToDocument.Invoke();
        Check(L.Take(), 'document open code=P1 number=7 kind=A day= filters=Code: P1, Number: 7 gf0=P1 count=1 read=P1/7;document agcr code=P1 number=7 kind=A day=2026-01-15 filters=Code: P1, Number: 7 gf0=P1 count=1'); end;
    [Test] procedure Worksheet() var H: TestPage "RL Host Card"; P: TestPage "RL Worksheet"; begin
        Seed(H); P.Trap(); H.ToWorksheet.Invoke();
        Check(L.Take(), 'worksheet open code=P1 number=7 kind=A day= filters=Code: P1, Number: 7 gf0=P1 count=1 read=P1/7;worksheet agcr code=P1 number=7 kind=A day=2026-01-15 filters=Code: P1, Number: 7 gf0=P1 count=1'); end;
    [Test] procedure FlowFilterLink() var H: TestPage "RL Host Card"; P: TestPage "RL FF List"; begin
        Seed(H); P.Trap(); H.ToFlowFilterList.Invoke();
        Check(L.Take(), 'list open code= number=0 kind=A day= filters=Code Filter: P1 gf0= count=3 cf=P1 hostcount=1');
        Check(P.CodeCtl.Value + '/' + P.NumberCtl.Value, 'P1/7'); end;
    [Test] procedure TempInsert() var H: TestPage "RL Host Card"; P: TestPage "RL Temp List"; S: Text; begin
        Seed(H); P.Trap(); H.ToTempList.Invoke();
        Check(L.Take(), 'temp open code=P1 number=0 kind=A day= filters=Code: P1 gf0=P1 count=0;temp insert blank=Yes code= number=0 kind=A day= filters=Code: P1 gf0=P1 count=0;temp after inserts code=P1 number=2 kind=A day= filters=Code: P1 gf0=P1 count=1');
        if P.First() then repeat S += P.CodeCtl.Value + '/' + P.NumberCtl.Value + ','; until not P.Next(); Check(S, 'P1/2,P1/0,'); end;
    [Test] procedure OptionLink() var H: TestPage "RL Host Card"; P: TestPage "RL Card"; begin
        Seed(H); P.Trap(); H.CardOption.Invoke();
        Check(L.Take(), 'card open code= number=0 kind=B day= filters=Kind: B gf0= count=1 read=/0;card agcr code=P1 number=8 kind=B day=2026-01-16 filters=Kind: B gf0= count=1'); end;
    [Test] procedure DateLink() var H: TestPage "RL Host Card"; P: TestPage "RL Card"; begin
        Seed(H); P.Trap(); H.CardDate.Invoke();
        Check(L.Take(), 'card open code= number=0 kind=A day=2026-01-15 filters=Day: 01/15/26 gf0= count=2 read=/0;card agcr code=P1 number=7 kind=A day=2026-01-15 filters=Day: 01/15/26 gf0= count=2'); end;
    [Test] procedure FilterEntry() var H: TestPage "RL Host Card"; P: TestPage "RL Card"; begin
        Seed(H); P.Trap(); H.CardFilter.Invoke();
        Check(L.Take(), 'card open code=P1 number=0 kind=A day= filters=Code: P1 gf0=P1 count=2 read=P1/0;card agcr code=P1 number=7 kind=A day=2026-01-15 filters=Code: P1 gf0=P1 count=2'); end;
}
"#,
    );
    src += &rpl_target(69447, "RL ListPlus", "ListPlus", "listplus", "");
    src += &rpl_target(69448, "RL Document", "Document", "document", "");
    src += &rpl_target(69449, "RL Worksheet", "Worksheet", "worksheet", "");
    src += &rpl_target(69450, "RL Card", "Card", "card", "");
    src
}

#[test]
fn rec_in_the_on_open_page_of_a_run_page_link_target_in_the_y41_shapes() {
    passes(&rpl_source());
}

// G4-04 T7: a RunPageView beside the link fills its const field too; GetFilters shows only the link (Y41RunPageLinkOnOpen.al:553
// RPL_Card_LinkPlusView).
#[test]
fn a_run_page_view_beside_the_link_fills_its_const_field() {
    let src = rpl_source()
        .replace(
            r#"action(CardOption) {"#,
            r#"action(CardLinkView) { RunObject = page "RL Card"; RunPageLink = "Code" = field("Code"); RunPageView = sorting(Number) where(Number = const(7)); }
        action(CardOption) {"#,
        )
        .replace(
            "[Test] procedure OptionLink()",
            r#"[Test] procedure LinkPlusView() var H: TestPage "RL Host Card"; P: TestPage "RL Card"; begin
        Seed(H); P.Trap(); H.CardLinkView.Invoke();
        Check(L.Take(), 'card open code=P1 number=7 kind=A day= filters=Code: P1 gf0=P1 count=1 read=P1/7;card agcr code=P1 number=7 kind=A day=2026-01-15 filters=Code: P1 gf0=P1 count=1'); end;
    [Test] procedure OptionLink()"#,
        );
    passes(&src);
}
