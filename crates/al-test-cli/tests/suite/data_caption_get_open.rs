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
        Seed(2); Page.Run(Page::"GO Miss", Row);
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
    [PageHandler] procedure GetHandler(var P: TestPage "GO Get")
    begin Log.Add('H cap=' + P.Caption() + ' g=' + P.GCtl.Value + ' code=' + P.CodeCtl.Value); end;
    [PageHandler] procedure MissHandler(var P: TestPage "GO Miss")
    begin end;
    [PageHandler] procedure WriteHandler(var P: TestPage "GO Write")
    begin end;
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
    for (t, why) in [
        ("Get_TextFilter_Refused", "outside the measured cells"),
        ("Miss_Refused", "raised an error at the open"),
        ("Write_Refused", "wrote to the database at the open"),
    ] {
        let o = get(t);
        assert!(o.contains("NotRun") && o.contains(why), "{t}: {o}");
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
