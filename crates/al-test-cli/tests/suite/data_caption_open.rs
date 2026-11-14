//! The DataCaptionExpression of a Worksheet or a Card with DelayedInsert that AL code runs with `Page.Run(Id, Rec)` and a
//! PageHandler answers (`data_caption_open.rs`, corpus PR 1392 pages/properties/DataCaptionExprWorksheet.al). These tests hold a
//! measured cell, the shapes next to the cells to main's outcome, and the refusals of the new path.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50300 "DO Row" { fields { field(1; "Item No."; Code[20]) { } field(2; "Vendor No."; Code[20]) { } } keys { key(PK; "Item No.", "Vendor No.") { } } }
codeunit 50301 "DO Log"
{
    SingleInstance = true;
    var Lines: Text; Mode: Text;
    procedure Wipe() begin Lines := ''; Mode := ''; end;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ' | '; Lines += Line; end;
    procedure Joined(): Text begin exit(Lines); end;
    procedure SetMode(M: Text) begin Mode := M; end;
    procedure GetMode(): Text begin exit(Mode); end;
}
page 50302 "DO Ws"
{
    PageType = Worksheet; SourceTable = "DO Row"; Caption = 'DO WS'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { group(F) { field(ItemNoFilterCtl; ItemNoFilter) { } } repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Log: Codeunit "DO Log"; ItemNoFilter: Text[250];
    local procedure GetCaption(): Text[250]
    begin
        if Rec.GetFilters <> '' then ItemNoFilter := Rec.GetFilter("Item No.");
        Log.Add('DC');
        if Log.GetMode() = 'empty' then exit('');
        exit('Cap ' + ItemNoFilter);
    end;
}
page 50303 "DO Ws Agr"
{
    PageType = Worksheet; SourceTable = "DO Row"; Caption = 'DO WS'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { group(F) { field(ItemNoFilterCtl; ItemNoFilter) { } } repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Log: Codeunit "DO Log"; ItemNoFilter: Text[250];
    local procedure GetCaption(): Text[250]
    begin
        if Rec.GetFilters <> '' then ItemNoFilter := Rec.GetFilter("Item No.");
        Log.Add('DC');
        exit('Cap ' + ItemNoFilter);
    end;
    trigger OnAfterGetRecord() begin end;
}
page 50304 "DO Card"
{
    PageType = Card; SourceTable = "DO Row"; Caption = 'DO Card'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { group(F) { field(ItemNoFilterCtl; ItemNoFilter) { } } group(G) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Log: Codeunit "DO Log"; ItemNoFilter: Text[250];
    local procedure GetCaption(): Text[250]
    begin
        if Rec.GetFilters <> '' then ItemNoFilter := Rec.GetFilter("Item No.");
        Log.Add('DC');
        exit('Cap ' + ItemNoFilter);
    end;
}
codeunit 50305 T
{
    Subtype = Test;
    var Log: Codeunit "DO Log";
    local procedure Seed(Rows: Integer)
    var Row: Record "DO Row"; i: Integer;
    begin
        Row.DeleteAll(); Log.Wipe();
        for i := 1 to Rows do begin Row."Item No." := 'I1'; Row."Vendor No." := 'V' + Format(i); Row.Insert(); end;
    end;
    local procedure Check(Expected: Text)
    begin
        if Log.Joined() <> Expected then Error('%1', Log.Joined());
    end;
    // The measured cell (DataCaptionExprWorksheet.al:586): one row, the expression once before the handler, the control empty.
    [Test] [HandlerFunctions('WsHandler')]
    procedure Ws_OneRow()
    var Row: Record "DO Row";
    begin
        Seed(1); Row.SetRange("Item No.", 'I1'); Page.Run(Page::"DO Ws", Row);
        Check('DC | H cap=DO WS - Cap I1 item=');
    end;
    // Two rows: no cell; main evaluates the expression at the caption read.
    [Test] [HandlerFunctions('WsHandler')]
    procedure Ws_TwoRows()
    var Row: Record "DO Row";
    begin
        Seed(2); Row.SetRange("Item No.", 'I1'); Page.Run(Page::"DO Ws", Row);
        Check('DC | H cap=DO WS - Cap I1 item=I1');
    end;
    // RunModal on one row: not measured; main's outcome.
    [Test] [HandlerFunctions('WsModalHandler')]
    procedure Ws_RunModal_OneRow()
    var Row: Record "DO Row";
    begin
        Seed(1); Row.SetRange("Item No.", 'I1'); Page.RunModal(Page::"DO Ws", Row);
        Check('DC | H cap=DO WS - Cap I1 item=I1');
    end;
    // A Worksheet with OnAfterGetRecord: not measured; main's outcome.
    [Test] [HandlerFunctions('WsAgrHandler')]
    procedure WsAgr_OneRow()
    var Row: Record "DO Row";
    begin
        Seed(1); Row.SetRange("Item No.", 'I1'); Page.Run(Page::"DO Ws Agr", Row);
        Check('DC | H cap=DO WS - Cap I1 item=I1');
    end;
    // A Card without DelayedInsert: not measured; main runs the expression twice (DataCaptionExprOrder.al's Card).
    [Test] [HandlerFunctions('CardHandler')]
    procedure Card_NoDelayedInsert_OneRow()
    var Row: Record "DO Row";
    begin
        Seed(1); Row.SetRange("Item No.", 'I1'); Page.Run(Page::"DO Card", Row);
        Check('DC | DC | H cap=DO Card - Cap I1 item=');
    end;
    // After the measured open only the caption, control reads and the close are let through.
    [Test] [HandlerFunctions('WsNextHandler')]
    procedure Ws_OneRow_NextRefused()
    var Row: Record "DO Row";
    begin
        Seed(1); Row.SetRange("Item No.", 'I1'); Page.Run(Page::"DO Ws", Row);
    end;
    // An empty value of the expression is not measured.
    [Test] [HandlerFunctions('WsHandler')]
    procedure Ws_OneRow_EmptyValueRefused()
    var Row: Record "DO Row";
    begin
        Seed(1); Log.SetMode('empty'); Row.SetRange("Item No.", 'I1'); Page.Run(Page::"DO Ws", Row);
    end;
    [PageHandler] procedure WsHandler(var P: TestPage "DO Ws")
    begin Log.Add('H cap=' + P.Caption() + ' item=' + P.ItemNoFilterCtl.Value); end;
    [ModalPageHandler] procedure WsModalHandler(var P: TestPage "DO Ws")
    begin Log.Add('H cap=' + P.Caption() + ' item=' + P.ItemNoFilterCtl.Value); end;
    [PageHandler] procedure WsAgrHandler(var P: TestPage "DO Ws Agr")
    begin Log.Add('H cap=' + P.Caption() + ' item=' + P.ItemNoFilterCtl.Value); end;
    [PageHandler] procedure CardHandler(var P: TestPage "DO Card")
    begin Log.Add('H cap=' + P.Caption() + ' item=' + P.ItemNoFilterCtl.Value); end;
    [PageHandler] procedure WsNextHandler(var P: TestPage "DO Ws")
    begin P.Next(); end;
}
"#;

#[test]
fn the_data_caption_expression_at_a_handled_page_run() {
    let outcomes = outcomes_in(SRC, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    assert_eq!(get("Ws_OneRow"), "Some(Pass)");
    for t in [
        "Ws_TwoRows",
        "Ws_RunModal_OneRow",
        "WsAgr_OneRow",
        "Card_NoDelayedInsert_OneRow",
    ] {
        assert_eq!(get(t), "Some(Pass)", "{t}");
    }
    let next = get("Ws_OneRow_NextRefused");
    assert!(next.contains("NotRun") && next.contains("ran at the open"), "{next}");
    let empty = get("Ws_OneRow_EmptyValueRefused");
    assert!(
        empty.contains("NotRun") && empty.contains("empty value or trailing blanks"),
        "{empty}"
    );
}
