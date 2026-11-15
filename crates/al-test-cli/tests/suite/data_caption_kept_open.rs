//! The two Worksheet opens whose DataCaptionExpression run at the open main skips under the assumption
//! `data-caption-two-microsoft-opens-skipped` (`data_caption_kept_open.rs` in al-vm): a temporary Worksheet without inserts whose
//! expression is a page procedure that returns a StrSubstNo of a label and a Text field of Rec when that field is not empty, opened
//! by a Page variable's `Run()` into a Trap; and one whose expression returns a Text variable of a codeunit variable of the page,
//! opened by a Page variable's `SetTableView` and `RunModal()` for a ModalPageHandler. Every neighbor keeps main's result: a
//! refusal at the open (`data_caption_open_neighbor`), or main's path for the read Y50ListCaptionExpr.al pins.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::isolation::TestIsolation;

/// Each test of codeunit `T` in `src`: its outcome (debug form) and the assumptions it ran through.
fn run(src: &str) -> Vec<(String, String, Vec<Assumption>)> {
    let l = compile(src);
    let b = blocked(&l);
    let engine = al_vm::Engine::new(l.program.clone(), b);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("codeunit T");
    let mut hits = Vec::new();
    let o = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    o.into_iter()
        .map(|(name, outcome)| {
            let h = hits
                .iter()
                .filter(|(n, _): &&(String, Vec<Assumption>)| *n == name)
                .flat_map(|(_, h)| h.iter().copied())
                .collect();
            (name, format!("{outcome:?}"), h)
        })
        .collect()
}

fn get<'a>(o: &'a [(String, String, Vec<Assumption>)], name: &str) -> (&'a str, &'a [Assumption]) {
    o.iter()
        .find(|(n, ..)| n == name)
        .map(|(_, o, h)| (o.as_str(), h.as_slice()))
        .unwrap_or_else(|| panic!("no test {name}"))
}

const SRC: &str = r#"
table 69900 "KO Entry" { fields { field(1; "Entry No."; Integer) { } field(2; "Doc Name"; Text[50]) { } field(3; Qty; Decimal) { } } keys { key(PK; "Entry No.") { } } }
codeunit 69901 "KO Mgt"
{
    var CaptionText: Text; Bounded: Text[30];
    procedure SetCaption(T: Text) begin CaptionText := T; Bounded := CopyStr(T, 1, 30); end;
    procedure GetCaption(): Text begin exit(CaptionText); end;
    procedure GetCaptionPlus(): Text begin exit(CaptionText + '!'); end;
    procedure GetBounded(): Text begin exit(Bounded); end;
}
page 69902 "KO Nav"
{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; InsertAllowed = false; Caption = 'KO NAV';
    DataCaptionExpression = GetCaptionText(); UsageCategory = None;
    layout { area(Content) { group(F) { field(DocCtl; DocNo) { } } repeater(L) { field(NameCtl; Rec."Doc Name") { } } } }
    var DocNo: Code[20]; SelectedLbl: Label 'Picked - %1';
    procedure SetDoc(No: Code[20]) begin DocNo := No; end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec."Doc Name" := 'N ' + DocNo; Rec.Insert(); end;
    local procedure GetCaptionText(): Text
    begin
        if Rec."Doc Name" <> '' then
            exit(StrSubstNo(SelectedLbl, Rec."Doc Name"));
        exit('');
    end;
}
page 69903 "KO Nav Inserts"
{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; Caption = 'KO NAV I';
    DataCaptionExpression = GetCaptionText(); UsageCategory = None;
    layout { area(Content) { group(F) { field(DocCtl; DocNo) { } } repeater(L) { field(NameCtl; Rec."Doc Name") { } } } }
    var DocNo: Code[20]; SelectedLbl: Label 'Picked - %1';
    procedure SetDoc(No: Code[20]) begin DocNo := No; end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec."Doc Name" := 'N ' + DocNo; Rec.Insert(); end;
    local procedure GetCaptionText(): Text
    begin
        if Rec."Doc Name" <> '' then
            exit(StrSubstNo(SelectedLbl, Rec."Doc Name"));
        exit('');
    end;
}
page 69904 "KO Nav Plus"
{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; InsertAllowed = false; Caption = 'KO NAV P';
    DataCaptionExpression = GetCaptionText(); UsageCategory = None;
    layout { area(Content) { group(F) { field(DocCtl; DocNo) { } } repeater(L) { field(NameCtl; Rec."Doc Name") { } } } }
    var DocNo: Code[20];
    procedure SetDoc(No: Code[20]) begin DocNo := No; end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec."Doc Name" := 'N ' + DocNo; Rec.Insert(); end;
    local procedure GetCaptionText(): Text
    begin
        if Rec."Doc Name" <> '' then
            exit('Picked - ' + Rec."Doc Name");
        exit('');
    end;
}
page 69905 "KO Nav Stored"
{
    PageType = Worksheet; SourceTable = "KO Entry"; InsertAllowed = false; Caption = 'KO NAV S';
    DataCaptionExpression = GetCaptionText(); UsageCategory = None;
    layout { area(Content) { group(F) { field(DocCtl; DocNo) { } } repeater(L) { field(NameCtl; Rec."Doc Name") { } } } }
    var DocNo: Code[20]; SelectedLbl: Label 'Picked - %1';
    procedure SetDoc(No: Code[20]) begin DocNo := No; end;
    local procedure GetCaptionText(): Text
    begin
        if Rec."Doc Name" <> '' then
            exit(StrSubstNo(SelectedLbl, Rec."Doc Name"));
        exit('');
    end;
}
page 69906 "KO Lines"
{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; InsertAllowed = false; Caption = 'KO LINES';
    DataCaptionExpression = Mgt.GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(QtyCtl; Rec.Qty) { } } } }
    actions { area(Processing) { action(Calc) { trigger OnAction() begin Rec.ModifyAll(Qty, 7); end; } } }
    var Mgt: Codeunit "KO Mgt";
    procedure SetSource(T: Text) begin Mgt.SetCaption(T); end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec.Qty := 3; Rec.Insert(); end;
}
page 69907 "KO Lines Plus"
{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; InsertAllowed = false; Caption = 'KO LINES P';
    DataCaptionExpression = Mgt.GetCaptionPlus(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(QtyCtl; Rec.Qty) { } } } }
    var Mgt: Codeunit "KO Mgt";
    procedure SetSource(T: Text) begin Mgt.SetCaption(T); end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec.Qty := 3; Rec.Insert(); end;
}
page 69908 "KO Lines Bounded"
{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; InsertAllowed = false; Caption = 'KO LINES B';
    DataCaptionExpression = Mgt.GetBounded(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(QtyCtl; Rec.Qty) { } } } }
    var Mgt: Codeunit "KO Mgt";
    procedure SetSource(T: Text) begin Mgt.SetCaption(T); end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec.Qty := 3; Rec.Insert(); end;
}
codeunit 69909 T
{
    Subtype = Test;
    var Seen: Text;

    // The tests that pass run first: a refused test before them would make the runner distrust what they read.
    // The trapped Page variable Run of the evidence: main's open, flagged.
    [Test] procedure K01_Nav_Trap_VarRun()
    var Pg: Page "KO Nav"; P: TestPage "KO Nav"; F: Text;
    begin
        P.Trap(); Pg.SetDoc('D1'); Pg.Run();
        if P.DocCtl.Value <> 'D1' then Error('doc %1', P.DocCtl.Value);
        F := P.Filter.GetFilter("Entry No.");
        P.First();
        if P.NameCtl.Value <> 'N D1' then Error('name %1', P.NameCtl.Value);
        if P.Next() then Error('second row');
    end;
    // The modal Page variable run after SetTableView of the evidence: main's open, flagged.
    [Test] [HandlerFunctions('LinesHandler')] procedure K11_Lines_Modal_View()
    var Pg: Page "KO Lines"; R: Record "KO Entry" temporary;
    begin
        R.SetRange("Entry No.", 1);
        Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal();
        if Seen <> 'qty 7' then Error('seen %1', Seen);
    end;
    // A page variable's RunModal() alone is Y50ListCaptionExpr.al's read: main's path, not flagged.
    [Test] [HandlerFunctions('LinesHandler')] procedure K17_Lines_Modal_Only()
    var Pg: Page "KO Lines";
    begin Pg.RunModal(); if Seen <> 'qty 7' then Error('seen %1', Seen); end;

    // The caption is what the skipped run would show: refused.
    [Test] procedure K02_Nav_Trap_VarRun_Caption()
    var Pg: Page "KO Nav"; P: TestPage "KO Nav";
    begin P.Trap(); Pg.SetDoc('D1'); Pg.Run(); Seen := P.Caption(); end;
    [Test] [HandlerFunctions('LinesCaptionHandler')] procedure K12_Lines_Modal_View_Caption()
    var Pg: Page "KO Lines"; R: Record "KO Entry" temporary;
    begin Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;

    // Neighbors: each differs in one fact and keeps main's refusal at the open.
    [Test] procedure K03_Nav_Trap_StaticRun()
    var P: TestPage "KO Nav";
    begin P.Trap(); Page.Run(Page::"KO Nav"); end;
    [Test] [HandlerFunctions('NavHandler')] procedure K04_Nav_Handler_VarRun()
    var Pg: Page "KO Nav";
    begin Pg.SetDoc('D2'); Pg.Run(); end;
    [Test] procedure K05_Nav_Inserts_Trap_VarRun()
    var Pg: Page "KO Nav Inserts"; P: TestPage "KO Nav Inserts";
    begin P.Trap(); Pg.SetDoc('D1'); Pg.Run(); end;
    [Test] procedure K06_Nav_PlusBody_Trap_VarRun()
    var Pg: Page "KO Nav Plus"; P: TestPage "KO Nav Plus";
    begin P.Trap(); Pg.SetDoc('D1'); Pg.Run(); end;
    [Test] procedure K07_Nav_Stored_Trap_VarRun()
    var Pg: Page "KO Nav Stored"; P: TestPage "KO Nav Stored";
    begin P.Trap(); Pg.SetDoc('D1'); Pg.Run(); end;
    [Test] procedure K08_Nav_Trap_VarSetRecordRun()
    var Pg: Page "KO Nav"; P: TestPage "KO Nav"; R: Record "KO Entry" temporary;
    begin P.Trap(); Pg.SetRecord(R); Pg.Run(); end;
    [Test] [HandlerFunctions('LinesHandler')] procedure K13_Lines_Modal_Static()
    var R: Record "KO Entry" temporary;
    begin Page.RunModal(Page::"KO Lines", R); end;
    [Test] [HandlerFunctions('LinesPlusHandler')] procedure K14_Lines_PlusBody_Modal_View()
    var Pg: Page "KO Lines Plus"; R: Record "KO Entry" temporary;
    begin Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;
    [Test] [HandlerFunctions('LinesBoundedHandler')] procedure K15_Lines_BoundedBody_Modal_View()
    var Pg: Page "KO Lines Bounded"; R: Record "KO Entry" temporary;
    begin Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;
    [Test] [HandlerFunctions('LinesPageHandler')] procedure K16_Lines_NonModal_View()
    var Pg: Page "KO Lines"; R: Record "KO Entry" temporary;
    begin Pg.SetSource('Src'); Pg.SetTableView(R); Pg.Run(); end;
    // The two shapes swapped: the Navigate-like page run modally, the Order Promising-like page trapped.
    [Test] [HandlerFunctions('NavModalHandler')] procedure K18_Nav_Body_Modal_View()
    var Pg: Page "KO Nav"; R: Record "KO Entry" temporary;
    begin Pg.SetDoc('D3'); Pg.SetTableView(R); Pg.RunModal(); end;
    [Test] procedure K19_Lines_Body_Trap_VarRun()
    var Pg: Page "KO Lines"; P: TestPage "KO Lines";
    begin P.Trap(); Pg.SetSource('Src'); Pg.Run(); end;

    [PageHandler] procedure NavHandler(var P: TestPage "KO Nav") begin Seen := P.DocCtl.Value; end;
    [ModalPageHandler] procedure NavModalHandler(var P: TestPage "KO Nav") begin Seen := P.DocCtl.Value; end;
    [ModalPageHandler] procedure LinesHandler(var P: TestPage "KO Lines")
    begin P.Calc.Invoke(); P.First(); Seen := 'qty ' + Format(P.QtyCtl.AsDecimal()); end;
    [ModalPageHandler] procedure LinesCaptionHandler(var P: TestPage "KO Lines") begin Seen := P.Caption(); end;
    [ModalPageHandler] procedure LinesPlusHandler(var P: TestPage "KO Lines Plus") begin P.First(); end;
    [ModalPageHandler] procedure LinesBoundedHandler(var P: TestPage "KO Lines Bounded") begin P.First(); end;
    [PageHandler] procedure LinesPageHandler(var P: TestPage "KO Lines") begin P.First(); end;
}
"#;

/// The start of main's refusal at the open (`data_caption_open_neighbor`).
const NEIGHBOR: &str = "a Worksheet with DataCaptionExpression that AL code opened with a handler or a Trap, outside the measured cells";

#[test]
fn the_two_evidence_opens_keep_mains_open_flagged() {
    let out = run(SRC);
    let flagged = [Assumption::DataCaptionTwoMicrosoftOpensSkipped];
    for t in ["K01_Nav_Trap_VarRun", "K11_Lines_Modal_View"] {
        let (o, h) = get(&out, t);
        assert_eq!(o, "Pass", "{t}");
        assert!(h.contains(&flagged[0]), "{t}: {h:?}");
    }
    // Y50ListCaptionExpr.al's read: main's path, not flagged.
    let (o, h) = get(&out, "K17_Lines_Modal_Only");
    assert_eq!(o, "Pass");
    assert!(!h.contains(&flagged[0]), "{h:?}");
}

#[test]
fn a_caption_read_after_a_kept_open_refuses() {
    let out = run(SRC);
    for t in ["K02_Nav_Trap_VarRun_Caption", "K12_Lines_Modal_View_Caption"] {
        let (o, _) = get(&out, t);
        assert!(
            o.contains("NotRun") && o.contains("did not run at the open, where BC runs it"),
            "{t}: {o}"
        );
    }
}

#[test]
fn every_neighbor_keeps_the_refusal_at_the_open() {
    let out = run(SRC);
    for t in [
        "K03_Nav_Trap_StaticRun",
        "K04_Nav_Handler_VarRun",
        "K05_Nav_Inserts_Trap_VarRun",
        "K06_Nav_PlusBody_Trap_VarRun",
        "K07_Nav_Stored_Trap_VarRun",
        "K08_Nav_Trap_VarSetRecordRun",
        "K13_Lines_Modal_Static",
        "K14_Lines_PlusBody_Modal_View",
        "K15_Lines_BoundedBody_Modal_View",
        "K16_Lines_NonModal_View",
        "K18_Nav_Body_Modal_View",
        "K19_Lines_Body_Trap_VarRun",
    ] {
        let (o, h) = get(&out, t);
        assert!(h.is_empty(), "{t}: {h:?}");
        assert!(o.contains("NotRun") && o.contains(NEIGHBOR), "{t}: {o}");
    }
}
