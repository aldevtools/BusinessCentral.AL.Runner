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

// Neighbors of the measured open (review of al-runner PR 8009). The probe hands a record with one SetRange on a Code field, no
// field argument, ascending on the primary key (DataCaptionExprWorksheet.al:349, :357-359); its OnInit and OnOpenPage write no
// filter, row or page variable (:369 `OnOpenPage(item=)`), and set CurrPage.Editable at most once, in OnOpenPage (:162-167).
const NEIGHBORS: &str = r#"
table 69960 "DX Row" { fields { field(1; "Item No."; Code[20]) { } field(2; "Vendor No."; Code[20]) { } } keys { key(PK; "Item No.", "Vendor No.") { } } }
codeunit 69961 "DX Log"
{
    SingleInstance = true;
    var Lines: Text; Mode: Text;
    procedure Wipe() begin Lines := ''; Mode := ''; end;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ' | '; Lines += Line; end;
    procedure Joined(): Text begin exit(Lines); end;
    procedure SetMode(M: Text) begin Mode := M; end;
    procedure GetMode(): Text begin exit(Mode); end;
}
page 69962 "DX Ws"
{
    PageType = Worksheet; SourceTable = "DX Row"; Caption = 'DX WS'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { group(F) { field(VendNoFilterCtl; VendNoFilter) { } field(ItemNoFilterCtl; ItemNoFilter) { } } repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Log: Codeunit "DX Log"; VendNoFilter: Text[250]; ItemNoFilter: Text[250];
    local procedure GetCaption(): Text[250]
    begin
        if Rec.GetFilters <> '' then begin VendNoFilter := Rec.GetFilter("Vendor No."); ItemNoFilter := Rec.GetFilter("Item No."); end;
        Log.Add('DC');
        exit('Cap ' + ItemNoFilter + '/' + VendNoFilter);
    end;
    trigger OnInit()
    begin
        if Log.GetMode() = 'initoff' then CurrPage.Editable := false;
    end;
    trigger OnOpenPage()
    var Row: Record "DX Row";
    begin
        case Log.GetMode() of
            'true': CurrPage.Editable := true;
            'false': CurrPage.Editable := false;
            'offon': begin CurrPage.Editable := false; CurrPage.Editable := true; end;
            'setrange': Rec.SetRange("Item No.", 'I1');
            'insert': begin Row."Item No." := 'I9'; Row."Vendor No." := 'VX'; Row.Insert(); end;
            'swap': begin Rec.SetRange("Item No."); Rec.SetRange("Vendor No.", 'V1'); end;
            'fg': Rec.FilterGroup(2);
            'delete': Row.DeleteAll();
            'global': ItemNoFilter := 'X';
        end;
    end;
}
codeunit 69963 "DX Run"
{
    var Log: Codeunit "DX Log";
    local procedure Seed(Rows: Integer; Mode: Text)
    var Row: Record "DX Row";
    begin
        Row.DeleteAll(); Log.Wipe(); Log.SetMode(Mode);
        if Rows = 1 then begin Row."Item No." := 'I1'; Row."Vendor No." := 'V1'; Row.Insert(); end;
    end;
    procedure Plain(Rows: Integer; Mode: Text; Item: Code[20]): Text
    var Row: Record "DX Row";
    begin
        Seed(Rows, Mode); Row.SetRange("Item No.", Item); Page.Run(Page::"DX Ws", Row);
        exit(Log.Joined());
    end;
    procedure FieldArg(): Text
    var Row: Record "DX Row";
    begin
        Seed(0, ''); Row.SetRange("Item No.", 'I9'); Page.Run(Page::"DX Ws", Row, Row."Vendor No.");
        exit(Log.Joined());
    end;
    procedure RangeFilter(): Text
    var Row: Record "DX Row";
    begin
        Seed(0, ''); Row.SetFilter("Item No.", 'I5..I9'); Page.Run(Page::"DX Ws", Row);
        exit(Log.Joined());
    end;
    procedure Descending(): Text
    var Row: Record "DX Row";
    begin
        Seed(0, ''); Row.SetRange("Item No.", 'I9'); Row.Ascending(false); Page.Run(Page::"DX Ws", Row);
        exit(Log.Joined());
    end;
}
page 69967 "DX Ws Field"
{
    PageType = Worksheet; SourceTable = "DX Row"; Caption = 'DX WSF'; DataCaptionExpression = Rec."Item No."; UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
}
codeunit 69966 T
{
    Subtype = Test;
    var R: Codeunit "DX Run"; Log: Codeunit "DX Log";
    local procedure Expect(Got: Text; Want: Text) begin if Got <> Want then Error('%1', Got); end;
    // Measured cells: :586 one row, :572 empty view, :478 empty view and CurrPage.Editable := false.
    [Test] [HandlerFunctions('H')] procedure D00_OneRow() begin Expect(R.Plain(1, '', 'I1'), 'DC | H cap=DX WS - Cap I1/ item= vend='); end;
    [Test] [HandlerFunctions('H')] procedure D01_Empty() begin Expect(R.Plain(0, 'true', 'I9'), 'DC | DC | H cap=DX WS - Cap I9/ item=I9 vend='); end;
    [Test] [HandlerFunctions('H')] procedure D02_EmptyEditableOff() begin Expect(R.Plain(0, 'false', 'I9'), 'H cap=DX WS item= vend='); end;
    // Main's results: OnOpenPage sets a filter, inserts a row, swaps the filter, fills a page variable.
    [Test] [HandlerFunctions('H')] procedure D11_OpenSetRange() begin Expect(R.Plain(1, 'setrange', 'I9'), 'DC | H cap=DX WS - Cap I1/ item=I1 vend='); end;
    [Test] [HandlerFunctions('H')] procedure D12_OpenInsert() begin Expect(R.Plain(0, 'insert', 'I9'), 'DC | H cap=DX WS - Cap I9/ item=I9 vend='); end;
    [Test] [HandlerFunctions('H')] procedure D15_OpenSwap() begin Expect(R.Plain(1, 'swap', 'I9'), 'DC | H cap=DX WS - Cap /V1 item= vend=V1'); end;
    [Test] [HandlerFunctions('H')] procedure D22_OpenGlobal() begin Expect(R.Plain(1, 'global', 'I1'), 'DC | H cap=DX WS - Cap I1/ item=I1 vend='); end;
    // Main's refusal: a field argument, a range, descending order, Editable false then true, FilterGroup(2) left active, Editable
    // := false in OnInit, the rows deleted.
    [Test] [HandlerFunctions('H')] procedure D04_FieldArg() begin R.FieldArg(); end;
    [Test] [HandlerFunctions('H')] procedure D05_RangeFilter() begin R.RangeFilter(); end;
    [Test] [HandlerFunctions('H')] procedure D06_Descending() begin R.Descending(); end;
    [Test] [HandlerFunctions('H')] procedure D16_OpenOffOn() begin R.Plain(0, 'offon', 'I9'); end;
    [Test] [HandlerFunctions('H')] procedure D17_OpenFilterGroup() begin R.Plain(0, 'fg', 'I9'); end;
    [Test] [HandlerFunctions('H')] procedure D18_InitOff() begin R.Plain(0, 'initoff', 'I9'); end;
    [Test] [HandlerFunctions('H')] procedure D21_OpenDelete() begin R.Plain(1, 'delete', 'I1'); end;
    // A DataCaptionExpression that is a field of Rec (measured: a procedure of the page, :111).
    [Test] [HandlerFunctions('HF')] procedure D13_FieldExpr()
    var Row: Record "DX Row";
    begin Row.DeleteAll(); Row.SetRange("Item No.", 'I9'); Page.Run(Page::"DX Ws Field", Row); end;
    [PageHandler] procedure HF(var P: TestPage "DX Ws Field") begin Log.Add('H cap=' + P.Caption()); end;
    [PageHandler] procedure H(var P: TestPage "DX Ws")
    begin Log.Add('H cap=' + P.Caption() + ' item=' + P.ItemNoFilterCtl.Value + ' vend=' + P.VendNoFilterCtl.Value); end;
}
"#;

#[test]
fn neighbors_of_the_measured_open_keep_main_outcome() {
    let outcomes = outcomes_in(NEIGHBORS, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    for t in [
        "D00_OneRow",
        "D01_Empty",
        "D02_EmptyEditableOff",
        "D11_OpenSetRange",
        "D12_OpenInsert",
        "D15_OpenSwap",
        "D22_OpenGlobal",
    ] {
        assert_eq!(get(t), "Some(Pass)", "{t}");
    }
    for t in [
        "D04_FieldArg",
        "D05_RangeFilter",
        "D06_Descending",
        "D16_OpenOffOn",
        "D17_OpenFilterGroup",
        "D18_InitOff",
        "D21_OpenDelete",
        "D13_FieldExpr",
    ] {
        let o = get(t);
        assert!(o.contains("NotRun") && o.contains("on a new row"), "{t}: {o}");
    }
}
