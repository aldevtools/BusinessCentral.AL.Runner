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
