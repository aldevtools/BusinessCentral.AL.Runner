//! A Worksheet with its own DataCaptionExpression outside the measured cells of DataCaptionExprWorksheet.al
//! (`data_caption_open.rs`): BC runs the expression at the open (:412, :439, :572, :586), main at a caption read. An expression
//! that can neither raise an error nor leave anything behind (`Engine::caption_open_risk`) keeps main's open and refuses only at
//! the first test read of the caption, the one thing the run at the open can change (the edge gate of 2026-11-14: Microsoft's
//! Navigate, opened through a page variable and read through its page-variable controls, went from pass to not run). Any other
//! expression refuses at the open. The kept open is not measured: it runs through the assumption
//! `data-caption-pure-expression-open-skipped`, so its passes are flagged.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::isolation::TestIsolation;

/// Each test of codeunit `T` in `src`: its outcome (debug form) and whether it ran through the kept-open assumption. `block`
/// names routines (object, routine) to mark as blocked by lowering before the run.
fn run(src: &str, block: &[(&str, &str)]) -> Vec<(String, String, bool)> {
    let l = compile(src);
    let mut b = blocked(&l);
    for (object, routine) in block {
        let oi = l
            .program
            .objects
            .iter()
            .position(|o| o.name == *object)
            .expect("object");
        let ri = l.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name.eq_ignore_ascii_case(routine))
            .expect("routine");
        b[oi][ri] = Some("blocked by the test".into());
    }
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
            let flagged = hits.iter().any(|(n, h): &(String, Vec<Assumption>)| {
                *n == name && h.contains(&Assumption::DataCaptionPureExpressionOpenSkipped)
            });
            (name, format!("{outcome:?}"), flagged)
        })
        .collect()
}

fn get<'a>(o: &'a [(String, String, bool)], name: &str) -> (&'a str, bool) {
    o.iter()
        .find(|(n, ..)| n == name)
        .map(|(_, o, f)| (o.as_str(), *f))
        .unwrap_or_else(|| panic!("no test {name}"))
}

const SRC: &str = r#"
table 69980 "DN Row" { fields { field(1; "Item No."; Code[20]) { } field(2; "Table Name"; Text[50]) { } } keys { key(PK; "Item No.") { } } }
page 69981 "DN Nav"
{
    PageType = Worksheet; SourceTable = "DN Row"; SourceTableTemporary = true; Caption = 'DN NAV'; DataCaptionExpression = GetCaptionText(); UsageCategory = None;
    layout { area(Content) { group(F) { field(DocNoCtl; DocNo) { } } repeater(L) { field(TableNameCtl; Rec."Table Name") { } } } }
    var DocNo: Code[20]; CapTxt: Label 'Selected - %1';
    procedure SetDoc(No: Code[20]) begin DocNo := No; end;
    trigger OnOpenPage() begin Rec.Init(); Rec."Item No." := 'A'; Rec."Table Name" := 'T ' + DocNo; Rec.Insert(); end;
    local procedure GetCaptionText(): Text
    begin
        if Rec."Table Name" <> '' then
            exit(StrSubstNo(CapTxt, Rec."Table Name"));
        exit('');
    end;
}
page 69982 "DN IfGet"
{
    PageType = Worksheet; SourceTable = "DN Row"; Caption = 'DN IFGET'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    local procedure GetCaption(): Text
    var R: Record "DN Row";
    begin
        if R.FindFirst() then
            exit(R."Table Name");
        exit('');
    end;
}
page 69983 "DN Get"
{
    PageType = Worksheet; SourceTable = "DN Row"; Caption = 'DN GET'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    local procedure GetCaption(): Text
    var R: Record "DN Row";
    begin
        R.Get('X');
        exit(R."Table Name");
    end;
}
page 69984 "DN Div"
{
    PageType = Worksheet; SourceTable = "DN Row"; Caption = 'DN DIV'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    local procedure GetCaption(): Text begin exit(Format(100 div StrLen(Rec."Table Name"))); end;
}
page 69985 "DN Err"
{
    PageType = Worksheet; SourceTable = "DN Row"; Caption = 'DN ERR'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    local procedure GetCaption(): Text begin if Rec."Table Name" = '' then Error('no name'); exit(Rec."Table Name"); end;
}
page 69986 "DN Evt"
{
    PageType = Worksheet; SourceTable = "DN Row"; Caption = 'DN EVT'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    local procedure GetCaption(): Text var T: Text; begin OnGetCaption(T); exit(T); end;
    [IntegrationEvent(false, false)] local procedure OnGetCaption(var T: Text) begin end;
}
page 69987 "DN Len"
{
    PageType = Worksheet; SourceTable = "DN Row"; Caption = 'DN LEN'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    local procedure GetCaption(): Text[10] begin exit('Cap ' + Rec."Table Name"); end;
}
codeunit 69988 T
{
    Subtype = Test;
    var Seen: Text;
    // Microsoft's Navigate shape: a page variable, a procedure of the page sets a page variable, then Run.
    [Test] procedure N01_Var_Trap_Controls()
    var Pg: Page "DN Nav"; P: TestPage "DN Nav";
    begin
        P.Trap(); Pg.SetDoc('D1'); Pg.Run();
        if P.DocNoCtl.Value <> 'D1' then Error('doc %1', P.DocNoCtl.Value);
        if P.TableNameCtl.Value <> 'T D1' then Error('table %1', P.TableNameCtl.Value);
        P.Close();
    end;
    [Test] procedure N02_Var_Trap_Caption()
    var Pg: Page "DN Nav"; P: TestPage "DN Nav";
    begin P.Trap(); Pg.SetDoc('D1'); Pg.Run(); Seen := P.Caption(); end;
    [Test] [HandlerFunctions('NavDoc')] procedure N03_Var_Handler_Controls()
    var Pg: Page "DN Nav";
    begin Pg.SetDoc('D2'); Pg.Run(); if Seen <> 'D2' then Error('%1', Seen); end;
    [Test] [HandlerFunctions('NavClose')] procedure N04_Static_Handler_Close()
    begin Page.Run(Page::"DN Nav"); if Seen <> 'closed' then Error('%1', Seen); end;
    // A FindFirst whose result is used cannot raise: main's open.
    [Test] procedure N05_IfGet_Trap_Close()
    var P: TestPage "DN IfGet";
    begin P.Trap(); Page.Run(Page::"DN IfGet"); P.Close(); end;
    // Expressions that can raise an error at the open: refused there.
    [Test] procedure N06_Get_Trap()
    var P: TestPage "DN Get";
    begin P.Trap(); Page.Run(Page::"DN Get"); P.Close(); end;
    [Test] procedure N07_Div_Trap()
    var P: TestPage "DN Div";
    begin P.Trap(); Page.Run(Page::"DN Div"); P.Close(); end;
    [Test] procedure N08_Err_Trap()
    var P: TestPage "DN Err";
    begin P.Trap(); Page.Run(Page::"DN Err"); P.Close(); end;
    [Test] procedure N09_Evt_Trap()
    var P: TestPage "DN Evt";
    begin P.Trap(); Page.Run(Page::"DN Evt"); P.Close(); end;
    [Test] procedure N10_BoundedReturn_Trap()
    var P: TestPage "DN Len";
    begin P.Trap(); Page.Run(Page::"DN Len"); P.Close(); end;
    [PageHandler] procedure NavDoc(var P: TestPage "DN Nav") begin Seen := P.DocNoCtl.Value; end;
    [PageHandler] procedure NavClose(var P: TestPage "DN Nav") begin Seen := 'closed'; P.Close(); end;
}
"#;

#[test]
fn an_expression_that_cannot_raise_keeps_main_open_until_a_caption_read() {
    let outcomes = run(SRC, &[]);
    let get = |name: &str| get(&outcomes, name);
    for t in [
        "N01_Var_Trap_Controls",
        "N03_Var_Handler_Controls",
        "N04_Static_Handler_Close",
        "N05_IfGet_Trap_Close",
    ] {
        assert_eq!(get(t), ("Pass", true), "{t}");
    }
    let (caption, _) = get("N02_Var_Trap_Caption");
    assert!(
        caption.contains("NotRun") && caption.contains("outside the measured cells") && caption.contains("the caption"),
        "{caption}"
    );
    for (t, why) in [
        ("N06_Get_Trap", "Record.Get"),
        ("N07_Div_Trap", "divides"),
        ("N08_Err_Trap", "Dialog.Error"),
        ("N09_Evt_Trap", "raises the event"),
        ("N10_BoundedReturn_Trap", "Text or Code of a fixed length"),
    ] {
        let (o, flagged) = get(t);
        assert!(!flagged, "{t}");
        assert!(
            o.contains("NotRun") && o.contains("can raise an error or leave something behind") && o.contains(why),
            "{t}: {o}"
        );
    }
}

/// The reviewer's neighbor shapes of round 1 (al-runner PR 8035): each one raises or leaves state when a test runs it
/// directly, so the open refuses and names it. The K pages keep main's open, flagged.
const NEIGHBORS: &str = r#"
codeunit 56001 "ZR Log" { SingleInstance = true; var N: Integer; procedure Hit() begin N += 1; end; }
table 56000 "ZR Row"
{
    fields { field(1; "Item No."; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; "Item No.") { } }
    procedure ReadName(): Text begin exit(Name); end;
}
table 56003 "ZR Cap"
{
    fields
    {
        field(1; Id; Integer) { }
        field(4; "Cc Err"; Integer) { Caption = 'Cc Err'; CaptionClass = CcErr(); }
        field(5; "Cc Log"; Integer) { Caption = 'Cc Log'; CaptionClass = CcLog(); }
    }
    keys { key(PK; Id) { } }
    procedure CcErr(): Text[80] begin Error('boom from CaptionClass'); end;
    procedure CcLog(): Text[80] var Log: Codeunit "ZR Log"; begin Log.Hit(); exit('3,Logged'); end;
}
page 56010 "ZR KTextAdd"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR KTextAdd'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text begin exit('Cap ' + Rec.Name); end;
    
}
page 56011 "ZR KTableProc"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR KTableProc'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text begin exit(Rec.ReadName()); end;
    
}
page 56012 "ZR KFieldCaption"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR KFieldCaption'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text begin exit(Rec.FieldCaption(Name)); end;
    
}
page 56013 "ZR KDelChr"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR KDelChr'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text begin exit(DelChr(Rec.Name, '<>', ' ')); end;
    
}
page 56014 "ZR KStrSubstNo"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR KStrSubstNo'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Big: Integer;
    trigger OnOpenPage() begin Big := 5; end;
    local procedure GetCaption(): Text begin exit(StrSubstNo('%1 %2', Rec.Name, Big)); end;
    
}
page 56015 "ZR KFindNext"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR KFindNext'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text var R: Record "ZR Row"; begin if R.FindFirst() then if R.Next() <> 0 then exit(R.Name); exit(''); end;
    
}
page 56016 "ZR RIntAdd"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RIntAdd'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Big: Integer;
    trigger OnOpenPage() begin Big := 2147483647; end;
    local procedure GetCaption(): Text begin exit(Format(Big + Big)); end;
    
}
page 56017 "ZR RIntAddLocal"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RIntAddLocal'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Big: Integer;
    trigger OnOpenPage() begin Big := 2147483647; end;
    local procedure GetCaption(): Text var I: Integer; begin I := Big + 1; exit(Format(I)); end;
    
}
page 56018 "ZR RDateAdd"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RDateAdd'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var D: Date;
    trigger OnOpenPage() begin D := 99991231D; end;
    local procedure GetCaption(): Text begin exit(Format(D + 1)); end;
    
}
page 56019 "ZR RFcErr"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RFcErr'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var G: Record "ZR Cap";
    
    local procedure GetCaption(): Text begin exit(G.FieldCaption("Cc Err")); end;
    
}
page 56020 "ZR RFcLocal"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RFcLocal'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text var R: Record "ZR Cap"; begin exit(R.FieldCaption("Cc Log")); end;
    
}
page 56021 "ZR RFcPlainOnCcTable"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RFcPlainOnCcTable'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text var R: Record "ZR Cap"; begin exit(R.GetFilters()); end;
    
}
page 56022 "ZR RFmtStr"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RFmtStr'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Big: Integer;
    trigger OnOpenPage() begin Big := 5; end;
    local procedure GetCaption(): Text begin exit(Format(Big, 0, '<Bogus>')); end;
    
}
page 56023 "ZR RFmtNum"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RFmtNum'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Big: Integer;
    trigger OnOpenPage() begin Big := 5; end;
    local procedure GetCaption(): Text begin exit(Format(Big, 0, 99)); end;
    
}
page 56024 "ZR RDelChrWhere"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RDelChrWhere'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text begin exit(DelChr(Rec.Name, 'X', 'a')); end;
    
}
page 56025 "ZR RRecursion"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RRecursion'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text begin exit(Deep(1)); end;
    local procedure Deep(N: Integer): Text begin exit(Deep(N)); end;
}
page 56026 "ZR RWhile"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RWhile'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text var B: Boolean; begin B := true; while B do B := true; exit(''); end;
    
}
page 56027 "ZR RFor"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RFor'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Big: Integer;
    trigger OnOpenPage() begin Big := 2147483647; end;
    local procedure GetCaption(): Text var I: Integer; T: Text; begin for I := Big to Big do T := T + 'x'; exit(T); end;
    
}
page 56028 "ZR RFindSetLock"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RFindSetLock'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text var R: Record "ZR Row"; begin if R.FindSet(true) then exit(R.Name); exit(''); end;
    
}
page 56029 "ZR RBlocked"
{
    PageType = Worksheet; SourceTable = "ZR Row"; Caption = 'ZR RBlocked'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    
    
    local procedure GetCaption(): Text begin exit(Helper()); end;
    local procedure Helper(): Text begin exit('h'); end;
}
codeunit 56002 T
{
    Subtype = Test;
    [Test] procedure KTextAdd()
    var P: TestPage "ZR KTextAdd"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR KTextAdd"); P.Close();
    end;
    [Test] procedure KTableProc()
    var P: TestPage "ZR KTableProc"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR KTableProc"); P.Close();
    end;
    [Test] procedure KFieldCaption()
    var P: TestPage "ZR KFieldCaption"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR KFieldCaption"); P.Close();
    end;
    [Test] procedure KDelChr()
    var P: TestPage "ZR KDelChr"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR KDelChr"); P.Close();
    end;
    [Test] procedure KStrSubstNo()
    var P: TestPage "ZR KStrSubstNo"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR KStrSubstNo"); P.Close();
    end;
    [Test] procedure KFindNext()
    var P: TestPage "ZR KFindNext"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR KFindNext"); P.Close();
    end;
    [Test] procedure RIntAdd()
    var P: TestPage "ZR RIntAdd"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RIntAdd"); P.Close();
    end;
    [Test] procedure RIntAddLocal()
    var P: TestPage "ZR RIntAddLocal"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RIntAddLocal"); P.Close();
    end;
    [Test] procedure RDateAdd()
    var P: TestPage "ZR RDateAdd"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RDateAdd"); P.Close();
    end;
    [Test] procedure RFcErr()
    var P: TestPage "ZR RFcErr"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RFcErr"); P.Close();
    end;
    [Test] procedure RFcLocal()
    var P: TestPage "ZR RFcLocal"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RFcLocal"); P.Close();
    end;
    [Test] procedure RFcPlainOnCcTable()
    var P: TestPage "ZR RFcPlainOnCcTable"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RFcPlainOnCcTable"); P.Close();
    end;
    [Test] procedure RFmtStr()
    var P: TestPage "ZR RFmtStr"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RFmtStr"); P.Close();
    end;
    [Test] procedure RFmtNum()
    var P: TestPage "ZR RFmtNum"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RFmtNum"); P.Close();
    end;
    [Test] procedure RDelChrWhere()
    var P: TestPage "ZR RDelChrWhere"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RDelChrWhere"); P.Close();
    end;
    [Test] procedure RRecursion()
    var P: TestPage "ZR RRecursion"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RRecursion"); P.Close();
    end;
    [Test] procedure RWhile()
    var P: TestPage "ZR RWhile"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RWhile"); P.Close();
    end;
    [Test] procedure RFor()
    var P: TestPage "ZR RFor"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RFor"); P.Close();
    end;
    [Test] procedure RFindSetLock()
    var P: TestPage "ZR RFindSetLock"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RFindSetLock"); P.Close();
    end;
    [Test] procedure RBlocked()
    var P: TestPage "ZR RBlocked"; R: Record "ZR Row";
    begin
        R.Init(); R."Item No." := 'A'; R.Name := 'First'; if R.Insert() then;
        P.Trap(); Page.Run(Page::"ZR RBlocked"); P.Close();
    end;
}
"#;

#[test]
fn an_expression_that_can_raise_or_leave_state_refuses_at_the_open_and_names_why() {
    let outcomes = run(NEIGHBORS, &[("ZR RBlocked", "Helper")]);
    for t in [
        "KTextAdd",
        "KTableProc",
        "KFieldCaption",
        "KDelChr",
        "KStrSubstNo",
        "KFindNext",
    ] {
        assert_eq!(get(&outcomes, t), ("Pass", true), "{t}");
    }
    let mut wrong = Vec::new();
    for (t, why) in [
        ("RIntAdd", "adds values that are not both Text or Code"),
        ("RIntAddLocal", "adds values that are not both Text or Code"),
        ("RDateAdd", "adds values that are not both Text or Code"),
        ("RFcErr", "a table whose fields can have a CaptionClass"),
        ("RFcLocal", "a table whose fields can have a CaptionClass"),
        ("RFcPlainOnCcTable", "a table whose fields can have a CaptionClass"),
        ("RFmtStr", "calls System.Format with more than one argument"),
        ("RFmtNum", "calls System.Format with more than one argument"),
        ("RDelChrWhere", "calls Text.DelChr with a Where argument other than"),
        ("RRecursion", "calls itself"),
        ("RWhile", "runs a loop"),
        ("RFor", "runs a loop"),
        ("RFindSetLock", "calls Record.FindSet with an argument"),
        ("RBlocked", "the runtime cannot run"),
    ] {
        let (o, flagged) = get(&outcomes, t);
        if flagged
            || !(o.contains("NotRun") && o.contains("can raise an error or leave something behind") && o.contains(why))
        {
            wrong.push(format!("{t} (flagged {flagged}): {o}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
