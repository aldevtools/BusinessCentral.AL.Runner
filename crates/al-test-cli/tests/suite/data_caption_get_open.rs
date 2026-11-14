//! The DataCaptionExpression of a Worksheet that AL code opens with a handler or after a Trap, in the cells of corpus PR 1401
//! pages/properties/DataCaptionGetOpen.al (`data_caption_get_open.rs`), and the TestPage filter '=0' / '=1' on a temporary
//! Worksheet the test opened with OpenEdit (testpage/filters/WorksheetOpenEditTemp.al, `filter_eq_text.rs`). These tests hold a
//! measured cell, the refusals of the new paths, and neighbors that keep the refusal.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50320 "GO Row" { fields { field(1; "Code"; Code[20]) { } field(2; Name; Text[50]) { } field(3; IsHist; Boolean) { } } keys { key(PK; "Code") { } } }
table 50321 "GO Other" { fields { field(1; "Code"; Code[20]) { } field(2; Title; Text[50]) { } } keys { key(PK; "Code") { } } }
codeunit 50322 "GO Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Wipe() begin Lines := ''; end;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ' | '; Lines += Line; end;
    procedure Joined(): Text begin exit(Lines); end;
}
page 50323 "GO Get"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Get'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { group(H) { field(GCtl; G) { } } repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log"; G: Text[30];
    local procedure Cap(): Text
    var Other: Record "GO Other"; V: Text;
    begin
        if Other.Get('K1') then V := Other.Title else V := 'none';
        G := 'ran'; Log.Add('DC=' + V); exit(V);
    end;
    trigger OnOpenPage() begin end;
}
page 50324 "GO Miss"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Miss'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    local procedure Cap(): Text
    var Other: Record "GO Other";
    begin
        Other.Get('NOPE'); exit(Other.Title);
    end;
    trigger OnOpenPage() begin end;
}
page 50325 "GO Write"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Write'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    local procedure Cap(): Text
    var Other: Record "GO Other";
    begin
        Other."Code" := CopyStr(Rec."Code" + 'W', 1, 20); if Other.Insert() then; exit('w');
    end;
    trigger OnOpenPage() begin end;
}
page 50326 "GO Temp"
{
    PageType = Worksheet; SourceTable = "GO Row"; SourceTableTemporary = true; Caption = 'GO Temp'; UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    trigger OnOpenPage()
    begin
        Rec."Code" := 'A'; Rec.IsHist := false; Rec.Insert();
        Rec."Code" := 'B'; Rec.IsHist := true; Rec.Insert();
        if Rec.FindFirst() then;
    end;
}
page 50327 "GO Db"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Db'; UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
}
page 50340 "GO SetRange"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO SetRange'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    begin
        Log.Add('DC'); Rec.SetRange("Code", 'R1'); exit('V' + Rec."Code");
    end;
    trigger OnOpenPage() begin end;
}
page 50341 "GO Assign"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Assign'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    begin
        Log.Add('DC'); Rec.Name := 'Zed'; exit('V' + Rec."Code");
    end;
    trigger OnOpenPage() begin end;
}
page 50342 "GO Next"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Next'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    begin
        Log.Add('DC'); if Rec.Next() = 0 then; exit('V' + Rec."Code");
    end;
    trigger OnOpenPage() begin end;
}
page 50343 "GO FindLast"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO FindLast'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    begin
        Log.Add('DC'); if Rec.FindLast() then; exit('V' + Rec."Code");
    end;
    trigger OnOpenPage() begin end;
}
page 50344 "GO Commit"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Commit'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    begin
        Log.Add('DC'); Commit(); exit('V' + Rec."Code");
    end;
    trigger OnOpenPage() begin end;
}
page 50345 "GO Update"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Update'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    begin
        Log.Add('DC'); CurrPage.Update(false); exit('V' + Rec."Code");
    end;
    trigger OnOpenPage() begin end;
}
page 50346 "GO Msg"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Msg'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    begin
        Log.Add('DC'); Message('m'); exit('V' + Rec."Code");
    end;
    trigger OnOpenPage() begin end;
}
page 50347 "GO Close"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Close'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    begin
        Log.Add('DC'); CurrPage.Close(); exit('V' + Rec."Code");
    end;
    trigger OnOpenPage() begin end;
}
page 50348 "GO Pure"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Pure'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } field(NameCtl; Rec.Name) { } } } }
    local procedure Cap(): Text begin exit('Cap ' + Rec.Name); end;
    trigger OnOpenPage() begin end;
}
page 50349 "GO Own"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO Own'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { group(H) { field(GCtl; G) { } } repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log"; G: Text[30];
    local procedure Cap(): Text
    var Twin: Record "GO Row"; V: Text;
    begin
        if Twin.Get(Rec."Code") then V := Twin.Name else V := 'none';
        G := 'ran'; Log.Add('DC=' + V); exit(V);
    end;
    trigger OnOpenPage() begin end;
}
page 50350 "GO NoOpen"
{
    PageType = Worksheet; SourceTable = "GO Row"; Caption = 'GO NoOpen'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "GO Log";
    local procedure Cap(): Text
    var Other: Record "GO Other"; V: Text;
    begin
        if Other.Get('K1') then V := Other.Title else V := 'none';
        Log.Add('DC=' + V); exit(V);
    end;
}
codeunit 50328 T
{
    Subtype = Test;
    var Log: Codeunit "GO Log";
    local procedure Seed(Rows: Integer)
    var Row: Record "GO Row"; Other: Record "GO Other"; i: Integer;
    begin
        Row.DeleteAll(); Other.DeleteAll(); Log.Wipe();
        for i := 1 to Rows do begin Row."Code" := 'R' + Format(i); Row.Name := 'N' + Format(i); Row.Insert(); end;
        Other."Code" := 'K1'; Other.Title := 'T1'; Other.Insert();
    end;
    local procedure Check(Expected: Text)
    begin
        if Log.Joined() <> Expected then Error('%1', Log.Joined());
    end;
    // DataCaptionGetOpen.al:365 Get_None_Trap: three rows, three runs before the Trap's reads, the global hidden.
    [Test]
    procedure Get_Three_Trap()
    var Row: Record "GO Row"; TP: TestPage "GO Get";
    begin
        Seed(3); TP.Trap(); Page.Run(Page::"GO Get", Row);
        Log.Add('T cap=' + TP.Caption() + ' g=' + TP.GCtl.Value + ' code=' + TP.CodeCtl.Value); TP.Close();
        Check('DC=T1 | DC=T1 | DC=T1 | T cap=GO Get - T1 g= code=R1');
    end;
    // :517 Get_EmptyCode_Run: an empty view, two runs, the global shown.
    [Test] [HandlerFunctions('GetHandler')]
    procedure Get_Empty_Run()
    var Row: Record "GO Row";
    begin
        Seed(3); Row.SetRange("Code", ''); Page.Run(Page::"GO Get", Row);
        Check('DC=T1 | DC=T1 | H cap=GO Get - T1 g=ran code=');
    end;
    // A Text filter on the handed record: no measured form; the refusal stays.
    [Test] [HandlerFunctions('GetHandler')]
    procedure Get_TextFilter_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Row.SetFilter(Name, 'N*'); Page.Run(Page::"GO Get", Row);
    end;
    // The expression raises (:643 Miss_None_Run raised a wrapped error): refused.
    [Test] [HandlerFunctions('MissHandler')]
    procedure Miss_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Miss", Row);
    end;
    // The expression writes to the database: no cell writes; refused.
    [Test] [HandlerFunctions('WriteHandler')]
    procedure Write_Refused()
    var Row: Record "GO Row";
    begin
        Seed(2); Page.Run(Page::"GO Write", Row);
    end;
    // WorksheetOpenEditTemp.al: '=1' on a temporary Worksheet opened with OpenEdit sets no filter.
    [Test]
    procedure Temp_OpenEdit_Eq1()
    var TP: TestPage "GO Temp";
    begin
        TP.OpenEdit(); TP.Filter.SetFilter(IsHist, '=1');
        if TP.Filter.GetFilter(IsHist) <> '' then Error('filter %1', TP.Filter.GetFilter(IsHist));
        TP.First(); if TP.CodeCtl.Value <> 'A' then Error('first %1', TP.CodeCtl.Value);
        TP.Close();
    end;
    // Another Boolean word under OpenEdit: not measured; refused.
    [Test]
    procedure Temp_OpenEdit_EqYes_Refused()
    var TP: TestPage "GO Temp";
    begin
        TP.OpenEdit(); TP.Filter.SetFilter(IsHist, '=Yes');
    end;
    // '=1' after the test's own filter on the field, under OpenEdit: not measured; refused.
    [Test]
    procedure Temp_OpenEdit_AfterFilter_Refused()
    var TP: TestPage "GO Temp";
    begin
        TP.OpenEdit(); TP.Filter.SetFilter(IsHist, '1'); TP.Filter.SetFilter(IsHist, '=1');
    end;
    // A database Worksheet under OpenEdit: not measured; refused.
    [Test]
    procedure Db_OpenEdit_Eq1_Refused()
    var TP: TestPage "GO Db";
    begin
        Seed(2); TP.OpenEdit(); TP.Filter.SetFilter(IsHist, '=1');
    end;
    // :977 Own_None_Run: an expression that reads Rec under Run, one run per row.
    [Test] [HandlerFunctions('OwnHandler')]
    procedure Own_Three_Run()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Own", Row);
        Check('DC=N1 | DC=N2 | DC=N3 | H cap=GO Own - N1 g=');
    end;
    // After the measured open only the caption, control reads and Close follow (:1192-1193): a Next refuses.
    [Test] [HandlerFunctions('GetNextHandler')]
    procedure Get_Three_HandlerNext_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Get", Row);
    end;
    // No cell reads Rec under RunModal (the Own cells are Run and Trap): refused.
    [Test] [HandlerFunctions('OwnModalHandler')]
    procedure Own_Three_RunModal_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.RunModal(Page::"GO Own", Row);
    end;
    // No filter on an empty table: the None cell had three rows; refused.
    [Test] [HandlerFunctions('GetHandler')]
    procedure Get_NoFilter_Empty_Refused()
    var Row: Record "GO Row";
    begin
        Seed(0); Page.Run(Page::"GO Get", Row);
    end;
    // A page without OnOpenPage: every measured page has it; refused.
    [Test] [HandlerFunctions('NoOpenHandler')]
    procedure NoOpen_Three_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO NoOpen", Row);
    end;
    [Test] [HandlerFunctions('SetRangeHandler')]
    procedure Body_SetRange_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO SetRange", Row);
    end;
    [PageHandler] procedure SetRangeHandler(var P: TestPage "GO SetRange")
    begin end;
    [Test] [HandlerFunctions('AssignHandler')]
    procedure Body_Assign_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Assign", Row);
    end;
    [PageHandler] procedure AssignHandler(var P: TestPage "GO Assign")
    begin end;
    [Test] [HandlerFunctions('NextHandler')]
    procedure Body_Next_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Next", Row);
    end;
    [PageHandler] procedure NextHandler(var P: TestPage "GO Next")
    begin end;
    [Test] [HandlerFunctions('FindLastHandler')]
    procedure Body_FindLast_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO FindLast", Row);
    end;
    [PageHandler] procedure FindLastHandler(var P: TestPage "GO FindLast")
    begin end;
    [Test] [HandlerFunctions('CommitHandler')]
    procedure Body_Commit_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Commit", Row);
    end;
    [PageHandler] procedure CommitHandler(var P: TestPage "GO Commit")
    begin end;
    [Test] [HandlerFunctions('UpdateHandler')]
    procedure Body_Update_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Update", Row);
    end;
    [PageHandler] procedure UpdateHandler(var P: TestPage "GO Update")
    begin end;
    [Test] [HandlerFunctions('MsgHandler')]
    procedure Body_Msg_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Msg", Row);
    end;
    [PageHandler] procedure MsgHandler(var P: TestPage "GO Msg")
    begin end;
    [Test] [HandlerFunctions('CloseHandler')]
    procedure Body_Close_Refused()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Close", Row);
    end;
    [PageHandler] procedure CloseHandler(var P: TestPage "GO Close")
    begin end;
    [PageHandler] procedure GetNextHandler(var P: TestPage "GO Get")
    begin P.Next(); end;
    [PageHandler] procedure OwnHandler(var P: TestPage "GO Own")
    begin Log.Add('H cap=' + P.Caption() + ' g=' + P.GCtl.Value); end;
    [ModalPageHandler] procedure OwnModalHandler(var P: TestPage "GO Own")
    begin end;
    [PageHandler] procedure NoOpenHandler(var P: TestPage "GO NoOpen")
    begin end;
    [PageHandler] procedure GetHandler(var P: TestPage "GO Get")
    begin Log.Add('H cap=' + P.Caption() + ' g=' + P.GCtl.Value + ' code=' + P.CodeCtl.Value); end;
    [PageHandler] procedure MissHandler(var P: TestPage "GO Miss")
    begin end;
    [PageHandler] procedure WriteHandler(var P: TestPage "GO Write")
    begin end;
}
codeunit 50351 TP
{
    Subtype = Test;
    var Got: Text;
    local procedure Seed(Rows: Integer)
    var Row: Record "GO Row"; i: Integer;
    begin
        Row.DeleteAll(); Got := '';
        for i := 1 to Rows do begin Row."Code" := 'R' + Format(i); Row.Name := 'N' + Format(i); Row.Insert(); end;
    end;
    // A Worksheet whose expression can neither raise nor leave anything behind keeps main's open (review of al-runner PR 8108):
    // the handler moves, writes and invokes as on main.
    [Test] [HandlerFunctions('PureNextHandler')]
    procedure Pure_Three_HandlerNext()
    var Row: Record "GO Row";
    begin
        Seed(3); Page.Run(Page::"GO Pure", Row); if Got <> 'first=R1 next=R2' then Error('%1', Got);
    end;
    [Test] [HandlerFunctions('PureLastHandler')]
    procedure Pure_Two_HandlerLast()
    var Row: Record "GO Row";
    begin
        Seed(2); Page.Run(Page::"GO Pure", Row); Row.FindFirst(); if Got <> 'last=R2' then Error('%1', Got); if Row.Name <> 'N1' then Error('name %1', Row.Name);
    end;
    [Test] [HandlerFunctions('PureSetHandler')]
    procedure Pure_Empty_HandlerSetValue()
    var Row: Record "GO Row";
    begin
        Seed(0); Page.Run(Page::"GO Pure", Row); if Got <> 'set' then Error('%1', Got); if Row.Count() <> 1 then Error('count %1', Row.Count());
    end;
    [Test]
    procedure Pure_Three_TrapNext()
    var Row: Record "GO Row"; TP: TestPage "GO Pure";
    begin
        Seed(3); TP.Trap(); Page.Run(Page::"GO Pure", Row); TP.Next(); Got := 'next=' + TP.CodeCtl.Value; TP.Close();
        if Got <> 'next=R2' then Error('%1', Got);
    end;
    [PageHandler] procedure PureNextHandler(var P: TestPage "GO Pure")
    begin Got := 'first=' + P.CodeCtl.Value; P.Next(); Got += ' next=' + P.CodeCtl.Value; end;
    [PageHandler] procedure PureLastHandler(var P: TestPage "GO Pure")
    begin P.Last(); Got := 'last=' + P.CodeCtl.Value; end;
    [PageHandler] procedure PureSetHandler(var P: TestPage "GO Pure")
    begin P.CodeCtl.SetValue('NEW'); P.NameCtl.SetValue('x'); Got := 'set'; end;
}
"#;

#[test]
fn data_caption_get_cells_run_and_their_neighbors_refuse() {
    let outcomes = outcomes_in(SRC, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    assert_eq!(get("Get_Three_Trap"), "Some(Pass)");
    assert_eq!(get("Get_Empty_Run"), "Some(Pass)");
    assert_eq!(get("Own_Three_Run"), "Some(Pass)");
    let main_refusal = "can raise an error or leave something behind at the open";
    for (t, why) in [
        ("Get_TextFilter_Refused", "outside the measured cells"),
        ("Miss_Refused", "raised an error at the open"),
        ("Write_Refused", main_refusal),
        ("Own_Three_RunModal_Refused", main_refusal),
        (
            "Get_Three_HandlerNext_Refused",
            "ran at the open as DataCaptionGetOpen.al",
        ),
        ("Get_NoFilter_Empty_Refused", main_refusal),
        ("NoOpen_Three_Refused", main_refusal),
        ("Body_SetRange_Refused", main_refusal),
        ("Body_Assign_Refused", main_refusal),
        ("Body_Next_Refused", main_refusal),
        ("Body_FindLast_Refused", main_refusal),
        ("Body_Commit_Refused", main_refusal),
        ("Body_Update_Refused", main_refusal),
        ("Body_Msg_Refused", main_refusal),
        ("Body_Close_Refused", main_refusal),
    ] {
        let o = get(t);
        assert!(o.contains("NotRun") && o.contains(why), "{t}: {o}");
    }
}

// A Worksheet whose expression can neither raise nor leave anything behind keeps main's open and main's answers to the handler's
// moves and writes (al-runner PR 8108 review, round 1: main answered these, the measured open had refused them).
#[test]
fn a_pure_expression_keeps_mains_open() {
    let outcomes = outcomes_in(SRC, "TP");
    for t in [
        "Pure_Three_HandlerNext",
        "Pure_Two_HandlerLast",
        "Pure_Empty_HandlerSetValue",
        "Pure_Three_TrapNext",
    ] {
        let o = outcomes.iter().find(|(n, _)| n == t).map(|(_, o)| o);
        assert_eq!(format!("{o:?}"), "Some(Pass)", "{t}");
    }
}

#[test]
fn eq_text_on_a_temporary_worksheet_opened_with_open_edit() {
    let outcomes = outcomes_in(SRC, "T");
    let get = |name: &str| {
        let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
        format!("{o:?}")
    };
    assert_eq!(get("Temp_OpenEdit_Eq1"), "Some(Pass)");
    for (t, why) in [
        ("Temp_OpenEdit_EqYes_Refused", "opened with OpenEdit"),
        ("Temp_OpenEdit_AfterFilter_Refused", "opened with OpenEdit"),
        ("Db_OpenEdit_Eq1_Refused", "opened with OpenEdit"),
    ] {
        let o = get(t);
        assert!(o.contains("NotRun") && o.contains(why), "{t}: {o}");
    }
}
