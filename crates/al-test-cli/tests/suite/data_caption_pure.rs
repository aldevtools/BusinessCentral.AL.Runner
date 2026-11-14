//! A Worksheet with its own DataCaptionExpression outside the measured cells of DataCaptionExprWorksheet.al
//! (`data_caption_open.rs`): BC runs the expression at the open (:412, :439, :572, :586), main at a caption read. An expression
//! that can neither raise an error nor leave anything behind (`Engine::caption_open_risk`) keeps main's open and refuses only at
//! the first test read of the caption, the one thing the run at the open can change (the edge gate of 2026-11-14: Microsoft's
//! Navigate, opened through a page variable and read through its page-variable controls, went from pass to not run). Any other
//! expression refuses at the open.

use al_testkit::outcomes_in;

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
    let outcomes = outcomes_in(SRC, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    for t in [
        "N01_Var_Trap_Controls",
        "N03_Var_Handler_Controls",
        "N04_Static_Handler_Close",
        "N05_IfGet_Trap_Close",
    ] {
        assert_eq!(get(t), "Some(Pass)", "{t}");
    }
    let caption = get("N02_Var_Trap_Caption");
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
        let o = get(t);
        assert!(
            o.contains("NotRun") && o.contains("can raise an error or leave something behind") && o.contains(why),
            "{t}: {o}"
        );
    }
}
