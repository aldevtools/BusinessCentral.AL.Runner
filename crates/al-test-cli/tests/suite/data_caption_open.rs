//! The DataCaptionExpression of a Worksheet or a Card with DelayedInsert that AL code runs with `Page.Run(Id, Rec)` and a
//! PageHandler answers (`data_caption_open.rs`, corpus PR 1392 pages/properties/DataCaptionExprWorksheet.al). These tests hold a
//! measured cell, the shapes next to the cells to main's outcome or, for a Worksheet, to a refusal, and the
//! refusals of the new path.

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
    assert_eq!(get("Card_NoDelayedInsert_OneRow"), "Some(Pass)");
    // A Worksheet with rows in no measured cell refuses at the open, naming the fact it misses (issue 6924 review).
    for (t, miss) in [
        ("Ws_TwoRows", "no measured cell combines"),
        ("Ws_RunModal_OneRow", "other than Page.Run with a PageHandler"),
        ("WsAgr_OneRow", "a trigger other than OnInit"),
    ] {
        let o = get(t);
        assert!(
            o.contains("NotRun") && o.contains("outside the measured cells") && o.contains(miss),
            "{t}: {o}"
        );
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
    // The handed record holds field values (review of al-runner PR 8009, round 2: N20, D01, D04): the probe's record is a fresh
    // variable (DataCaptionExprWorksheet.al:341-359).
    procedure Handed(Rows: Integer; How: Text; Item: Code[20]): Text
    var Row: Record "DX Row";
    begin
        Seed(Rows, '');
        case How of
            'get': Row.Get('I1', 'V1');
            'vendor': Row."Vendor No." := 'V7';
            'item': Row."Item No." := 'I5';
        end;
        Row.SetRange("Item No.", Item); Page.Run(Page::"DX Ws", Row);
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
    // The handed record after a Get on the one-row view: main's outcome (the expression runs at the caption read).
    [Test] [HandlerFunctions('H')] procedure D26_HandedAfterGetOneRow() begin Expect(R.Handed(1, 'get', 'I1'), 'DC | H cap=DX WS - Cap I1/ item=I1 vend='); end;
    // Main's refusal: a field argument, a range, descending order, Editable false then true, FilterGroup(2) left active, Editable
    // := false in OnInit, the rows deleted.
    [Test] [HandlerFunctions('H')] procedure D04_FieldArg() begin R.FieldArg(); end;
    [Test] [HandlerFunctions('H')] procedure D05_RangeFilter() begin R.RangeFilter(); end;
    [Test] [HandlerFunctions('H')] procedure D06_Descending() begin R.Descending(); end;
    [Test] [HandlerFunctions('H')] procedure D16_OpenOffOn() begin R.Plain(0, 'offon', 'I9'); end;
    [Test] [HandlerFunctions('H')] procedure D17_OpenFilterGroup() begin R.Plain(0, 'fg', 'I9'); end;
    [Test] [HandlerFunctions('H')] procedure D18_InitOff() begin R.Plain(0, 'initoff', 'I9'); end;
    [Test] [HandlerFunctions('H')] procedure D21_OpenDelete() begin R.Plain(1, 'delete', 'I1'); end;
    // The handed record after a Get, or with a field assigned, on the empty view: main's refusal.
    [Test] [HandlerFunctions('H')] procedure D23_HandedAfterGet() begin R.Handed(1, 'get', 'I9'); end;
    [Test] [HandlerFunctions('H')] procedure D24_HandedVendorSet() begin R.Handed(0, 'vendor', 'I9'); end;
    [Test] [HandlerFunctions('H')] procedure D25_HandedItemSet() begin R.Handed(0, 'item', 'I9'); end;
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
fn neighbors_of_the_measured_open_keep_main_outcome_or_refuse() {
    let outcomes = outcomes_in(NEIGHBORS, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    for t in ["D00_OneRow", "D01_Empty", "D02_EmptyEditableOff"] {
        assert_eq!(get(t), "Some(Pass)", "{t}");
    }
    // Worksheets AL code ran, in no measured cell, whose expression writes page variables and the log: they refuse at the open
    // and name the fact they miss (a field argument too, review of al-runner PR 8023). D13's expression, a field of Rec, leaves
    // nothing behind: it refuses at the handler's caption read.
    for (t, miss) in [
        ("D04_FieldArg", "not one AL code ran with Page.Run(Id, Rec)"),
        ("D11_OpenSetRange", "changed Rec's filters"),
        ("D12_OpenInsert", "wrote to the database"),
        ("D15_OpenSwap", "changed Rec's filters"),
        ("D22_OpenGlobal", "not an empty Text or Code"),
        ("D26_HandedAfterGetOneRow", "fields hold values"),
        ("D05_RangeFilter", "not one SetRange to one Code value"),
        ("D06_Descending", "on another key or order"),
        ("D16_OpenOffOn", "more than once"),
        ("D17_OpenFilterGroup", "changed Rec's filters, key, order, filter group"),
        ("D18_InitOff", "assigned in OnInit"),
        ("D21_OpenDelete", "wrote to the database"),
        ("D23_HandedAfterGet", "fields hold values"),
        ("D24_HandedVendorSet", "fields hold values"),
        ("D25_HandedItemSet", "fields hold values"),
        ("D13_FieldExpr", "not a call of a procedure of the page"),
    ] {
        let o = get(t);
        assert!(
            o.contains("NotRun") && o.contains("outside the measured cells") && o.contains(miss),
            "{t}: {o}"
        );
    }
    assert!(get("D13_FieldExpr").contains("the test read the caption"));
}

// The page of al-runner issue 6924 (APPD), as the reporter described it: filter controls with OnValidate and OnLookup, a third
// control on a Date variable, and an OnOpenPage that asks a codeunit and assigns its answer to CurrPage.Editable. The probe's
// controls have no trigger and show Text variables (DataCaptionExprWorksheet.al:120-130).
const REPORTED: &str = r#"
table 69950 "RP Row" { fields { field(1; "Item No."; Code[20]) { } field(2; "Vendor No."; Code[20]) { } } keys { key(PK; "Item No.", "Vendor No.") { } } }
codeunit 69951 "RP License" { procedure IsInLicense(): Boolean begin exit(true); end; }
page 69952 "RP Ws Trig"
{
    PageType = Worksheet; SourceTable = "RP Row"; Caption = 'RP WS'; DataCaptionExpression = GetCaption(); DelayedInsert = true; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(F)
            {
                field(VendNoFilterCtl; VendNoFilter) { trigger OnValidate() begin SetRecFilters(); end; trigger OnLookup(var Text: Text): Boolean begin exit(false); end; }
                field(ItemNoFilterCtl; ItemNoFilter) { trigger OnValidate() begin SetRecFilters(); end; trigger OnLookup(var Text: Text): Boolean begin exit(false); end; }
            }
            repeater(L) { field(ItemNoCtl; Rec."Item No.") { } }
        }
    }
    var VendNoFilter: Text[250]; ItemNoFilter: Text[250]; IsInLicense: Boolean;
    local procedure GetCaption(): Text[250]
    begin
        if Rec.GetFilters <> '' then begin VendNoFilter := Rec.GetFilter("Vendor No."); ItemNoFilter := Rec.GetFilter("Item No."); end;
        exit('Cap ' + ItemNoFilter + '/' + VendNoFilter);
    end;
    local procedure SetRecFilters() begin Rec.SetFilter("Item No.", ItemNoFilter); CurrPage.Update(false); end;
    trigger OnOpenPage() var License: Codeunit "RP License"; begin IsInLicense := License.IsInLicense(); CurrPage.Editable := IsInLicense; end;
}
page 69953 "RP Ws Date"
{
    PageType = Worksheet; SourceTable = "RP Row"; Caption = 'RP WS'; DataCaptionExpression = GetCaption(); DelayedInsert = true; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(F) { field(VendNoFilterCtl; VendNoFilter) { } field(ItemNoFilterCtl; ItemNoFilter) { } field(StartingDateFilterCtl; StartingDateFilter) { } }
            repeater(L) { field(ItemNoCtl; Rec."Item No.") { } }
        }
    }
    var VendNoFilter: Text[250]; ItemNoFilter: Text[250]; StartingDateFilter: Date; IsInLicense: Boolean;
    local procedure GetCaption(): Text[250]
    begin
        if Rec.GetFilters <> '' then begin VendNoFilter := Rec.GetFilter("Vendor No."); ItemNoFilter := Rec.GetFilter("Item No."); end;
        exit('Cap ' + ItemNoFilter + '/' + VendNoFilter);
    end;
    trigger OnOpenPage() var License: Codeunit "RP License"; begin IsInLicense := License.IsInLicense(); CurrPage.Editable := IsInLicense; end;
}
page 69954 "RP List Trig"
{
    PageType = List; SourceTable = "RP Row"; Caption = 'RP List'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(F) { field(ItemNoFilterCtl; ItemNoFilter) { trigger OnValidate() begin Rec.SetFilter("Item No.", ItemNoFilter); end; } }
            repeater(L) { field(ItemNoCtl; Rec."Item No.") { } }
        }
    }
    var ItemNoFilter: Text[250];
    local procedure GetCaption(): Text[250]
    begin
        if Rec.GetFilters <> '' then ItemNoFilter := Rec.GetFilter("Item No.");
        exit('Cap ' + ItemNoFilter);
    end;
}
codeunit 69955 T
{
    Subtype = Test;
    var Seen: Text;
    local procedure Seed(Rows: Integer)
    var Row: Record "RP Row";
    begin
        Row.DeleteAll(); Seen := '';
        if Rows = 1 then begin Row."Item No." := 'I1'; Row."Vendor No." := 'V1'; Row.Insert(); end;
    end;
    [Test] [HandlerFunctions('HTrig')] procedure R1_ControlTriggers_Empty()
    var Row: Record "RP Row";
    begin Seed(0); Row.SetRange("Item No.", 'I9'); Page.Run(Page::"RP Ws Trig", Row); if Seen <> 'item=I9 vend=' then Error('Wrong Filter on Page'); end;
    [Test] [HandlerFunctions('HDate')] procedure R2_DateVariable_Empty()
    var Row: Record "RP Row";
    begin Seed(0); Row.SetRange("Item No.", 'I9'); Page.Run(Page::"RP Ws Date", Row); end;
    // Outside the neighborhood: a view with a row, and a List page. Main's path.
    [Test] [HandlerFunctions('HTrig')] procedure R3_ControlTriggers_OneRow()
    var Row: Record "RP Row";
    begin Seed(1); Row.SetRange("Item No.", 'I1'); Page.Run(Page::"RP Ws Trig", Row); if Seen <> 'item= vend=' then Error('%1', Seen); end;
    [Test] [HandlerFunctions('HList')] procedure R4_List_Empty()
    var Row: Record "RP Row";
    begin Seed(0); Row.SetRange("Item No.", 'I9'); Page.Run(Page::"RP List Trig", Row); if Seen <> 'item=' then Error('%1', Seen); end;
    [PageHandler] procedure HTrig(var P: TestPage "RP Ws Trig")
    begin Seen := 'item=' + P.ItemNoFilterCtl.Value + ' vend=' + P.VendNoFilterCtl.Value; end;
    [PageHandler] procedure HDate(var P: TestPage "RP Ws Date")
    begin if P.ItemNoFilterCtl.Value <> 'I9' then Error('Wrong Filter on Page'); end;
    [PageHandler] procedure HList(var P: TestPage "RP List Trig")
    begin Seen := 'item=' + P.ItemNoFilterCtl.Value; end;
}
"#;

// al-runner issue 6924 (APPD): main's path ran the expression only at a caption read, so the handler read empty controls where BC
// ran it twice before the handler (DataCaptionExprWorksheet.al:412). Control triggers and a Date variable are not measured: on the
// empty view the page refuses before the handler and names the fact it misses.
#[test]
fn the_reported_worksheet_on_an_empty_view_refuses() {
    let outcomes = outcomes_in(REPORTED, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    let trig = get("R1_ControlTriggers_Empty");
    assert!(
        trig.contains("NotRun") && trig.contains("a control has a trigger"),
        "{trig}"
    );
    let date = get("R2_DateVariable_Empty");
    assert!(
        date.contains("NotRun") && date.contains("not an empty Text or Code"),
        "{date}"
    );
}

// A List page keeps main's path; a Worksheet with a row in no cell refuses too.
#[test]
fn the_reported_list_keeps_main_outcome_and_a_row_refuses() {
    let outcomes = outcomes_in(REPORTED, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    assert_eq!(get("R4_List_Empty"), "Some(Pass)");
    let one_row = get("R3_ControlTriggers_OneRow");
    assert!(
        one_row.contains("NotRun")
            && one_row.contains("outside the measured cells")
            && one_row.contains("a control has a trigger"),
        "{one_row}"
    );
}

// A Worksheet with its own DataCaptionExpression outside the measured cells (review of al-runner PR 8023: every run kind, and a
// handler that reads none of what the open changes). BC ran the expression at the open in the measured cells
// (DataCaptionExprWorksheet.al:412, :439, :572, :586); main runs it at a caption read. An expression that leaves nothing behind
// changes only what the caption and the controls on page variables or expressions show: main's open is kept and the first such read
// refuses. An expression that can leave something behind refuses at the open, for every run kind.
const LAZY: &str = r#"
table 69970 "DL Row" { fields { field(1; "Item No."; Code[20]) { } field(2; "Vendor No."; Code[20]) { } } keys { key(PK; "Item No.", "Vendor No.") { } } }
page 69971 "DL Ws Field"
{
    PageType = Worksheet; SourceTable = "DL Row"; Caption = 'DL WS'; DataCaptionExpression = Rec."Item No."; UsageCategory = None;
    layout { area(Content) { group(F) { field(NoteCtl; Note) { } } repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Note: Text[30];
}
page 69972 "DL Ws Pure"
{
    PageType = Worksheet; SourceTable = "DL Row"; Caption = 'DL WSP'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { group(F) { field(NoteCtl; Note) { } } repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Note: Text[30];
    local procedure GetCaption(): Text[250] begin exit('Cap ' + Rec."Item No."); end;
}
page 69973 "DL Ws Lit"
{
    PageType = Worksheet; SourceTable = "DL Row"; Caption = 'DL WSL'; DataCaptionExpression = 'Lit'; UsageCategory = None;
    layout { area(Content) { repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
}
page 69974 "DL Ws Fx"
{
    PageType = Worksheet; SourceTable = "DL Row"; Caption = 'DL WSX'; DataCaptionExpression = GetCaption(); UsageCategory = None;
    layout { area(Content) { group(F) { field(NoteCtl; Note) { } } repeater(L) { field(ItemNoCtl; Rec."Item No.") { } } } }
    var Note: Text[30];
    local procedure GetCaption(): Text[250] begin Note := 'x'; exit('Cap'); end;
}
codeunit 69975 T
{
    Subtype = Test;
    var Seen: Text;
    local procedure Seed(Rows: Integer; var Row: Record "DL Row")
    var i: Integer;
    begin
        Row.DeleteAll(); Seen := '';
        for i := 1 to Rows do begin Row.Init(); Row."Item No." := 'I1'; Row."Vendor No." := 'V' + Format(i); Row.Insert(); end;
        Row.Reset(); Row.SetRange("Item No.", 'I1');
    end;
    // Static Page.Run(Id, Rec): the handler reads nothing the open changes, or a control of Rec.
    [Test] [HandlerFunctions('FieldClose')] procedure L01_Static_Close()
    var Row: Record "DL Row";
    begin Seed(0, Row); Page.Run(Page::"DL Ws Field", Row); if Seen <> 'closed' then Error('%1', Seen); end;
    [Test] [HandlerFunctions('FieldCaption')] procedure L02_Static_Caption()
    var Row: Record "DL Row";
    begin Seed(2, Row); Page.Run(Page::"DL Ws Field", Row); end;
    [Test] [HandlerFunctions('PureItem')] procedure L03_Static_RecControl()
    var Row: Record "DL Row";
    begin Seed(2, Row); Page.Run(Page::"DL Ws Pure", Row); if Seen <> 'item=I1' then Error('%1', Seen); end;
    [Test] [HandlerFunctions('PureNote')] procedure L04_Static_VarControl()
    var Row: Record "DL Row";
    begin Seed(2, Row); Page.Run(Page::"DL Ws Pure", Row); end;
    [Test] [HandlerFunctions('LitClose')] procedure L05_Static_Literal_Close()
    var Row: Record "DL Row";
    begin Seed(1, Row); Page.Run(Page::"DL Ws Lit", Row); if Seen <> 'closed' then Error('%1', Seen); end;
    // A page variable set up with SetTableView or SetRecord, then Run or RunModal.
    [Test] [HandlerFunctions('FieldCaption')] procedure L06_Var_Caption()
    var Row: Record "DL Row"; Pg: Page "DL Ws Field";
    begin Seed(2, Row); Pg.SetTableView(Row); Pg.Run(); end;
    [Test] [HandlerFunctions('PureItemModal')] procedure L07_VarModal_RecControl()
    var Row: Record "DL Row"; Pg: Page "DL Ws Pure";
    begin Seed(2, Row); Row.FindFirst(); Pg.SetRecord(Row); Pg.RunModal(); if Seen <> 'item=I1' then Error('%1', Seen); end;
    [Test] [HandlerFunctions('FieldClose')] procedure L08_Var_Close()
    var Row: Record "DL Row"; Pg: Page "DL Ws Field";
    begin Seed(2, Row); Pg.SetTableView(Row); Pg.Run(); if Seen <> 'closed' then Error('%1', Seen); end;
    // A field number argument.
    [Test] [HandlerFunctions('FieldCaption')] procedure L09_FieldNo_Caption()
    var Row: Record "DL Row";
    begin Seed(2, Row); Page.Run(Page::"DL Ws Field", Row, Row."Vendor No."); end;
    [Test] [HandlerFunctions('FieldClose')] procedure L10_FieldNo_Close()
    var Row: Record "DL Row";
    begin Seed(2, Row); Page.Run(Page::"DL Ws Field", Row, Row."Vendor No."); if Seen <> 'closed' then Error('%1', Seen); end;
    // A Trap instead of a handler.
    [Test] procedure L11_Trap_Caption()
    var Row: Record "DL Row"; P: TestPage "DL Ws Field";
    begin Seed(2, Row); P.Trap(); Page.Run(Page::"DL Ws Field", Row); Seen := P.Caption(); end;
    [Test] procedure L12_Trap_RecControl()
    var Row: Record "DL Row"; P: TestPage "DL Ws Field";
    begin Seed(2, Row); P.Trap(); Page.Run(Page::"DL Ws Field", Row); if P.ItemNoCtl.Value <> 'I1' then Error('%1', P.ItemNoCtl.Value); P.Close(); end;
    // An expression that writes a variable of the page: refused at the open, for every run kind.
    [Test] [HandlerFunctions('FxClose')] procedure L13_Fx_Var_Close()
    var Row: Record "DL Row"; Pg: Page "DL Ws Fx";
    begin Seed(2, Row); Pg.SetTableView(Row); Pg.Run(); end;
    [Test] procedure L14_Fx_Trap()
    var Row: Record "DL Row"; P: TestPage "DL Ws Fx";
    begin Seed(2, Row); P.Trap(); Page.Run(Page::"DL Ws Fx", Row); P.Close(); end;
    [Test] [HandlerFunctions('FxClose')] procedure L15_Fx_Static_Close()
    var Row: Record "DL Row";
    begin Seed(2, Row); Page.Run(Page::"DL Ws Fx", Row); end;
    [PageHandler] procedure FieldClose(var P: TestPage "DL Ws Field") begin Seen := 'closed'; P.Close(); end;
    [PageHandler] procedure FieldCaption(var P: TestPage "DL Ws Field") begin Seen := P.Caption(); end;
    [PageHandler] procedure PureItem(var P: TestPage "DL Ws Pure") begin P.First(); Seen := 'item=' + P.ItemNoCtl.Value; end;
    [ModalPageHandler] procedure PureItemModal(var P: TestPage "DL Ws Pure") begin Seen := 'item=' + P.ItemNoCtl.Value; end;
    [PageHandler] procedure PureNote(var P: TestPage "DL Ws Pure") begin Seen := P.NoteCtl.Value; end;
    [PageHandler] procedure LitClose(var P: TestPage "DL Ws Lit") begin Seen := 'closed'; P.Close(); end;
    [PageHandler] procedure FxClose(var P: TestPage "DL Ws Fx") begin P.Close(); end;
}
"#;

#[test]
fn an_unmeasured_worksheet_open_refuses_at_the_read_it_changes() {
    let outcomes = outcomes_in(LAZY, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    // Main's outcome where the test reads nothing the open changes.
    for t in [
        "L01_Static_Close",
        "L03_Static_RecControl",
        "L05_Static_Literal_Close",
        "L07_VarModal_RecControl",
        "L08_Var_Close",
        "L10_FieldNo_Close",
        "L12_Trap_RecControl",
    ] {
        assert_eq!(get(t), "Some(Pass)", "{t}");
    }
    for (t, read) in [
        ("L02_Static_Caption", "the caption"),
        ("L04_Static_VarControl", "a control on a page variable or an expression"),
        ("L06_Var_Caption", "the caption"),
        ("L09_FieldNo_Caption", "the caption"),
        ("L11_Trap_Caption", "the caption"),
    ] {
        let o = get(t);
        assert!(
            o.contains("NotRun") && o.contains("outside the measured cells") && o.contains(read),
            "{t}: {o}"
        );
    }
    for t in ["L13_Fx_Var_Close", "L14_Fx_Trap", "L15_Fx_Static_Close"] {
        let o = get(t);
        assert!(
            o.contains("NotRun") && o.contains("outside the measured cells") && o.contains("writes the variable Note"),
            "{t}: {o}"
        );
    }
}
