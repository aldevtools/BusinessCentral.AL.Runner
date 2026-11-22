//! The two Worksheet opens whose DataCaptionExpression run at the open main skips (`data_caption_kept_open.rs` in al-vm,
//! pinned by al-language/pages/properties/DataCaptionKeptOpen.al): a temporary Worksheet without inserts whose
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
    begin R.SetRange("Entry No.", 1); Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;

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
const NEIGHBOR: &str =
    "a Worksheet with DataCaptionExpression that AL code opened with a handler or a Trap, outside the measured cells";

#[test]
fn the_two_pinned_opens_keep_mains_open_unflagged() {
    let out = run(SRC);
    // DataCaptionKeptOpen.al:436-485 and :584-606; K17 is Y50ListCaptionExpr.al's read on main's path.
    for t in ["K01_Nav_Trap_VarRun", "K11_Lines_Modal_View", "K17_Lines_Modal_Only"] {
        let (o, h) = get(&out, t);
        assert_eq!(o, "Pass", "{t}");
        assert!(h.is_empty(), "{t}: {h:?}");
    }
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

/// A copy of page "KO Nav" (the trapped shape's page) as page `id` `name`, with `props` added to its properties, `label` as the
/// text of its caption label and `extra` added to its body.
fn nav_page(id: u32, name: &str, props: &str, label: &str, extra: &str) -> String {
    format!(
        r#"page {id} "{name}"
{{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; InsertAllowed = false; Caption = 'KO';
    DataCaptionExpression = GetCaptionText(); UsageCategory = None; {props}
    layout {{ area(Content) {{ group(F) {{ field(DocCtl; DocNo) {{ }} }} repeater(L) {{ field(NameCtl; Rec."Doc Name") {{ }} }} }} }}
    var DocNo: Code[20]; SelectedLbl: Label '{label}';
    procedure SetDoc(No: Code[20]) begin DocNo := No; end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec."Doc Name" := 'N ' + DocNo; Rec.Insert(); end;
    local procedure GetCaptionText(): Text
    begin
        if Rec."Doc Name" <> '' then
            exit(StrSubstNo(SelectedLbl, Rec."Doc Name"));
        exit('');
    end;
    {extra}
}}
"#
    )
}

/// The trapped shape on pages that differ from "KO Nav" in one fact each: test `X<id>` opens page `id`.
fn neighbor_src() -> String {
    let mut src = String::from(
        r#"table 69900 "KO Entry" { fields { field(1; "Entry No."; Integer) { } field(2; "Doc Name"; Text[50]) { } } keys { key(PK; "Entry No.") { } } }
"#,
    );
    let lbl = "Picked - %1";
    src += &nav_page(69910, "KO Base", "", lbl, "");
    // The two shapes of the review of the pull request, as in its probe apps p3.al X02 and p4.al X09.
    src += &nav_page(69911, "KO ExtExpr", "", lbl, "");
    src += "pageextension 69931 \"KO ExtExpr X\" extends \"KO ExtExpr\" { DataCaptionExpression = Boom(); procedure Boom(): Text begin Error('boom'); end; }\n";
    src += &nav_page(69912, "KO ExtInsert", "", lbl, "");
    src += "pageextension 69932 \"KO ExtInsert X\" extends \"KO ExtInsert\" { InsertAllowed = true; }\n";
    src += &nav_page(69913, "KO ExtTemp", "", lbl, "");
    src += "pageextension 69933 \"KO ExtTemp X\" extends \"KO ExtTemp\" { SourceTableTemporary = false; }\n";
    src += &nav_page(69914, "KO ExtTrigger", "", lbl, "");
    src += "pageextension 69934 \"KO ExtTrigger X\" extends \"KO ExtTrigger\" { trigger OnOpenPage() begin end; }\n";
    src += &nav_page(69915, "KO Sub", "", lbl, "");
    src += "codeunit 69935 \"KO Sub Mgt\" { [EventSubscriber(ObjectType::Page, Page::\"KO Sub\", 'OnOpenPageEvent', '', false, false)] local procedure OnOpen(var Rec: Record \"KO Entry\") begin end; }\n";
    src += &nav_page(
        69916,
        "KO GetCurr",
        "",
        lbl,
        "trigger OnAfterGetCurrRecord() begin end;",
    );
    src += &nav_page(69917, "KO NotEditable", "Editable = false;", lbl, "");
    src += &nav_page(69918, "KO Delayed", "DelayedInsert = true;", lbl, "");
    src += &nav_page(69919, "KO TenHole", "", "Picked %10", "");
    src += &nav_page(
        69920,
        "KO Closing",
        "",
        lbl,
        "trigger OnInit() begin end; trigger OnClosePage() begin end;",
    );
    src += &nav_page(69921, "KO ExtEditable", "", lbl, "");
    src += "pageextension 69936 \"KO ExtEditable X\" extends \"KO ExtEditable\" { Editable = false; }\n";
    src += "codeunit 69909 T\n{\n    Subtype = Test;\n";
    for (id, name) in [
        (69910, "KO Base"),
        (69911, "KO ExtExpr"),
        (69912, "KO ExtInsert"),
        (69913, "KO ExtTemp"),
        (69914, "KO ExtTrigger"),
        (69915, "KO Sub"),
        (69916, "KO GetCurr"),
        (69917, "KO NotEditable"),
        (69918, "KO Delayed"),
        (69919, "KO TenHole"),
        (69920, "KO Closing"),
        (69921, "KO ExtEditable"),
    ] {
        src += &format!(
            "    [Test] procedure X{id}() var Pg: Page \"{name}\"; P: TestPage \"{name}\"; begin P.Trap(); Pg.SetDoc('D1'); Pg.Run(); end;\n"
        );
    }
    src += "}\n";
    src
}

#[test]
fn the_page_facts_of_the_pinned_pages_keep_mains_open() {
    let out = run(&neighbor_src());
    // The pinned page facts (DataCaptionKeptOpen.al:97-149): OnOpenPage is its only trigger.
    let (o, h) = get(&out, "X69910");
    assert_eq!(o, "Pass");
    assert!(h.is_empty(), "{h:?}");
}

#[test]
fn a_page_outside_the_evidence_page_facts_keeps_the_refusal_at_the_open() {
    let out = run(&neighbor_src());
    let wrong = [
        "X69911", // an extension's DataCaptionExpression that raises (review probe p3.al X02)
        "X69912", // an extension's InsertAllowed = true (review probe p4.al X09)
        "X69913", // an extension's SourceTableTemporary = false
        "X69914", // an extension's OnOpenPage trigger
        "X69915", // an OnOpenPageEvent subscriber
        "X69916", // a page trigger other than OnOpenPage
        "X69920", // OnInit and OnClosePage next to OnOpenPage (only Microsoft's tests had them; the pin has OnOpenPage alone)
        "X69917", // Editable = false
        "X69918", // DelayedInsert = true
        "X69919", // a label 'Picked %10': placeholder ten, no %1
        "X69921", // an extension's Editable = false
    ]
    .into_iter()
    .filter(|t| {
        let (o, h) = get(&out, t);
        !(h.is_empty() && o.contains("NotRun") && o.contains(NEIGHBOR))
    })
    .collect::<Vec<_>>();
    assert!(wrong.is_empty(), "not refused at the open: {wrong:?}");
}

/// The modal shape (DataCaptionKeptOpen.al:659-666: a temporary record filtered to `"Entry No." = 1` handed by SetTableView,
/// `RunModal()` as a statement, the caption from a codeunit without properties) with one fact changed each, the four shapes the
/// deleted assumption data-caption-two-microsoft-opens-skipped still wanted measured (review probe codeunit 50125 D1-D4).
const MODAL_SRC: &str = r#"
table 69900 "KO Entry" { fields { field(1; "Entry No."; Integer) { } field(2; "Doc Name"; Text[50]) { } field(3; Qty; Decimal) { } } keys { key(PK; "Entry No.") { } } }
codeunit 69901 "KO Mgt" { var CaptionText: Text; procedure SetCaption(T: Text) begin CaptionText := T; end; procedure GetCaption(): Text begin exit(CaptionText); end; }
codeunit 69911 "KO Single" { SingleInstance = true; var CaptionText: Text; procedure SetCaption(T: Text) begin CaptionText := T; end; procedure GetCaption(): Text begin exit(CaptionText); end; }
page 69906 "KO Lines"
{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; InsertAllowed = false; Caption = 'KO LINES';
    DataCaptionExpression = Mgt.GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(QtyCtl; Rec.Qty) { } } } }
    var Mgt: Codeunit "KO Mgt";
    procedure SetSource(T: Text) begin Mgt.SetCaption(T); end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec.Qty := 3; Rec.Insert(); end;
}
page 69912 "KO Lines Single"
{
    PageType = Worksheet; SourceTable = "KO Entry"; SourceTableTemporary = true; InsertAllowed = false; Caption = 'KO LINES S';
    DataCaptionExpression = Mgt.GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(QtyCtl; Rec.Qty) { } } } }
    var Mgt: Codeunit "KO Single";
    procedure SetSource(T: Text) begin Mgt.SetCaption(T); end;
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec.Qty := 3; Rec.Insert(); end;
}
codeunit 69909 T
{
    Subtype = Test;
    [Test] [HandlerFunctions('LinesHandler')] procedure D0_Pinned()
    var Pg: Page "KO Lines"; R: Record "KO Entry" temporary;
    begin R.SetRange("Entry No.", 1); Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;
    [Test] [HandlerFunctions('LinesHandler')] procedure D1_ResultUsed()
    var Pg: Page "KO Lines"; R: Record "KO Entry" temporary; A: Action;
    begin R.SetRange("Entry No.", 1); Pg.SetSource('Src'); Pg.SetTableView(R); A := Pg.RunModal(); end;
    [Test] [HandlerFunctions('LinesHandler')] procedure D2_StoredRecord()
    var Pg: Page "KO Lines"; R: Record "KO Entry";
    begin R.SetRange("Entry No.", 1); Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;
    [Test] [HandlerFunctions('LinesSingleHandler')] procedure D3_SingleInstanceSource()
    var Pg: Page "KO Lines Single"; R: Record "KO Entry" temporary;
    begin R.SetRange("Entry No.", 1); Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;
    [Test] [HandlerFunctions('LinesHandler')] procedure D4_NonKeyFilter()
    var Pg: Page "KO Lines"; R: Record "KO Entry" temporary;
    begin R.SetRange(Qty, 3); Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;
    [Test] [HandlerFunctions('LinesHandler')] procedure D5_NoFilter()
    var Pg: Page "KO Lines"; R: Record "KO Entry" temporary;
    begin Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;
    [Test] [HandlerFunctions('LinesHandler')] procedure D6_OtherKeyValue()
    var Pg: Page "KO Lines"; R: Record "KO Entry" temporary;
    begin R.SetRange("Entry No.", 2); Pg.SetSource('Src'); Pg.SetTableView(R); Pg.RunModal(); end;
    [ModalPageHandler] procedure LinesHandler(var P: TestPage "KO Lines") begin P.First(); end;
    [ModalPageHandler] procedure LinesSingleHandler(var P: TestPage "KO Lines Single") begin P.First(); end;
}
"#;

#[test]
fn the_modal_shape_with_one_fact_changed_keeps_the_refusal_at_the_open() {
    let out = run(MODAL_SRC);
    let (o, h) = get(&out, "D0_Pinned");
    assert_eq!(o, "Pass");
    assert!(h.is_empty(), "{h:?}");
    let wrong = [
        "D1_ResultUsed",           // RunModal's result is used
        "D2_StoredRecord",         // SetTableView of a stored record
        "D3_SingleInstanceSource", // a SingleInstance codeunit is the caption source
        "D4_NonKeyFilter",         // the filter is on a field outside the key
        "D5_NoFilter",             // no filter at all
        "D6_OtherKeyValue",        // the key filter holds another value than the pin's 1
    ]
    .into_iter()
    .filter(|t| {
        let (o, h) = get(&out, t);
        !(h.is_empty() && o.contains("NotRun") && o.contains(NEIGHBOR))
    })
    .collect::<Vec<_>>();
    assert!(wrong.is_empty(), "not refused at the open: {wrong:?}");
}
