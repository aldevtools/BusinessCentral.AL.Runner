//! The built-in View and Edit of a list without a card, and a reopen of the same TestPage variable after them (al-runner issue
//! 8501; corpus PR 1467, open: testpage/lifecycle/ReopenAfterListModeSwitch.al, measured on a local BC of each supported major).
//! The pinned corpus does not hold the probe yet, so these tests keep its measured paths and the refusals around them.

use al_builtins::native::Signal;
use al_testkit::{Built, run_in, text};

const SRC: &str = r#"
table 69890 "LMR Line"
{
    DataClassification = SystemMetadata;
    fields { field(1; "Line No."; Integer) { } field(2; Description; Text[30]) { } }
    keys { key(PK; "Line No.") { Clustered = true; } }
}
codeunit 69891 "LMR Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Line: Text) begin if Entries <> '' then Entries += ' '; Entries += Line; end;
    procedure Take(): Text var R: Text; begin R := '[' + Entries + ']'; Entries := ''; exit(R); end;
}
// The list of ReopenAfterListModeSwitch.al:39-88 (corpus PR 1467), line for line.
page 69892 "LMR Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
                field(Opens; OpenCount) { ApplicationArea = All; Caption = 'Opens'; Editable = false; }
            }
        }
    }
    var Log: Codeunit "LMR Log"; OpenCount: Integer;
    trigger OnOpenPage() begin OpenCount += 1; Log.Add('open'); end;
    trigger OnAfterGetRecord() begin Log.Add('agr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('new'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
// The probe list with one more statement in one trigger (review of PR 8540, round 2: B1, B2, B8).
page 69906 "LMR Editable Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
                field(Opens; OpenCount) { ApplicationArea = All; Caption = 'Opens'; Editable = false; }
            }
        }
    }
    var Log: Codeunit "LMR Log"; OpenCount: Integer;
    trigger OnOpenPage() begin OpenCount += 1; Log.Add('open'); CurrPage.Editable(false); end;
    trigger OnAfterGetRecord() begin Log.Add('agr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('new'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 69907 "LMR Assign Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
                field(Opens; OpenCount) { ApplicationArea = All; Caption = 'Opens'; Editable = false; }
            }
        }
    }
    var Log: Codeunit "LMR Log"; OpenCount: Integer;
    trigger OnOpenPage() begin OpenCount += 1; Log.Add('open'); end;
    trigger OnAfterGetRecord() begin Log.Add('agr(' + Format(Rec."Line No.") + ')'); Rec.Description := 'x' + Format(Rec."Line No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('new'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 69908 "LMR Count Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
                field(Opens; OpenCount) { ApplicationArea = All; Caption = 'Opens'; Editable = false; }
            }
        }
    }
    var Log: Codeunit "LMR Log"; OpenCount: Integer;
    trigger OnOpenPage() begin OpenCount += 1; Log.Add('open'); end;
    trigger OnAfterGetRecord() begin Log.Add('agr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Format(Rec."Line No.") + ')'); OpenCount += 100; end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('new'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 69893 "LMR Plain Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Description; Rec.Description) { } } } }
}
page 69894 "LMR Agr Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Description; Rec.Description) { } } } }
    var Log: Codeunit "LMR Log";
    trigger OnAfterGetRecord() begin Log.Add('agr(' + Format(Rec."Line No.") + ')'); end;
}
page 69895 "LMR Card"
{
    PageType = Card; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("Line No."; Rec."Line No.") { } field(Description; Rec.Description) { } } }
}
table 69897 "LMR FLine"
{
    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Description; Text[30]) { }
        field(3; Lines; Integer) { FieldClass = FlowField; CalcFormula = count("LMR Line" where("Line No." = field("Line No."))); }
    }
    keys { key(PK; "Line No.") { Clustered = true; } }
}
page 69898 "LMR FLines"
{
    PageType = List; SourceTable = "LMR FLine"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Lines; Rec.Lines) { } } } }
    var Log: Codeunit "LMR Log";
    trigger OnOpenPage() begin Log.Add('open'); end;
    trigger OnAfterGetRecord() begin Log.Add('agr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('new'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
page 69899 "LMR Plain FLines"
{
    PageType = List; SourceTable = "LMR FLine"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Lines; Rec.Lines) { } } } }
}
page 69900 "LMR Ext Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Description; Rec.Description) { } } } }
}
pageextension 69901 "LMR Ext Lines Ext" extends "LMR Ext Lines"
{
    var Log: Codeunit "LMR Log";
    trigger OnAfterGetRecord() begin Log.Add('ext agr'); end;
}
page 69902 "LMR Sub Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Description; Rec.Description) { } } } }
    trigger OnOpenPage() begin end;
}
codeunit 69903 "LMR Sub"
{
    [EventSubscriber(ObjectType::Page, Page::"LMR Sub Lines", 'OnOpenPageEvent', '', false, false)]
    local procedure OnOpenPage(var Rec: Record "LMR Line")
    var Log: Codeunit "LMR Log";
    begin
        Log.Add('sub open');
    end;
}
page 69904 "LMR Expr Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Shown; Format(Rec.Description)) { } } } }
}
codeunit 69905 "LMR Holder"
{
    SingleInstance = true;
    var G: TestPage "LMR Lines";

    procedure ViewEditClose()
    begin
        G.OpenEdit();
        G.First();
        G.View().Invoke();
        G.Edit().Invoke();
        G.Close();
    end;

    procedure OpenEdit(): Text
    begin
        G.OpenEdit();
        G.First();
        exit(Format(G.Edit().Enabled()) + '|' + Format(G.View().Enabled()));
    end;

    procedure TakeViewClose()
    begin
        G.OpenEdit();
        G.View().Enabled();
        G.Close();
    end;

    procedure BoundViewEnabled(): Text
    begin
        G.OpenEdit();
        exit(Format(G.View().Enabled()));
    end;
}
codeunit 69896 "LMR Test"
{
    var Log: Codeunit "LMR Log";

    local procedure Init()
    var Line: Record "LMR Line";
    begin
        Line.DeleteAll();
        Line."Line No." := 10000; Line.Description := 'a'; Line.Insert();
        Line."Line No." := 20000; Line.Description := 'b'; Line.Insert();
        Log.Take();
    end;

    [TryFunction]
    local procedure TryView(var P: TestPage "LMR Lines") begin P.View().Invoke(); end;

    [TryFunction]
    local procedure TryEdit(var P: TestPage "LMR Lines") begin P.Edit().Invoke(); end;

    local procedure Modes(var P: TestPage "LMR Lines"): Text
    begin
        exit(Format(P.Editable()) + '|' + Format(P.Edit().Enabled()) + '|' + Format(P.View().Enabled()));
    end;

    procedure ViewEditCloseOpenEdit(): Text
    var P: TestPage "LMR Lines"; R: Text;
    begin
        Init();
        P.OpenEdit();
        P.First();
        Log.Take();
        P.View().Invoke();
        R := 'view=' + Log.Take();
        P.Edit().Invoke();
        R += ' edit=' + Log.Take();
        P.Close();
        P.OpenEdit();
        R += ' open=' + Log.Take() + ' modes=' + Modes(P);
        P.Close();
        exit(R);
    end;

    procedure BoundActions(): Text
    var P: TestPage "LMR Lines"; R: Text;
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
        P.Edit().Invoke();
        P.Close();
        P.OpenEdit();
        P.First();
        Log.Take();
        if not TryView(P) then R := 'view=' + GetLastErrorText() + Log.Take();
        R += ' modes=' + Modes(P);
        P.Close();
        Clear(P);
        P.OpenEdit();
        P.First();
        Log.Take();
        if TryView(P) then R += ' cleared view=' + Log.Take() + ' modes=' + Modes(P);
        P.Close();
        exit(R);
    end;

    procedure EditOnListOpenedForView(): Text
    var P: TestPage "LMR Lines"; R: Text;
    begin
        Init();
        P.OpenView();
        P.First();
        Log.Take();
        P.Edit().Invoke();
        R := 'edit=' + Log.Take() + ' modes=' + Modes(P);
        P.Close();
        P.OpenEdit();
        R += ' open=' + Log.Take() + ' modes=' + Modes(P);
        P.Close();
        exit(R);
    end;

    procedure OpenEditWhileOpen(): Text
    var P: TestPage "LMR Lines"; R: Text;
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
        P.Edit().Invoke();
        Log.Take();
        asserterror P.OpenEdit();
        R := GetLastErrorText() + Log.Take() + ' modes=' + Modes(P);
        P.Close();
        exit(R);
    end;

    procedure BoundVisible()
    var P: TestPage "LMR Lines";
    begin
        Init();
        P.OpenEdit();
        P.View().Enabled();
        P.Close();
        P.OpenEdit();
        P.View().Visible();
    end;

    procedure BoundOnCard()
    var P: TestPage "LMR Card";
    begin
        Init();
        P.OpenEdit();
        P.Edit().Enabled();
        P.Close();
        P.OpenEdit();
        P.Edit().Enabled();
    end;

    procedure BoundToViewModeInvoked()
    var P: TestPage "LMR Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
        P.Close();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
    end;

    procedure ViewOnSecondRow()
    var P: TestPage "LMR Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.Next();
        P.View().Invoke();
    end;

    procedure ValueAfterEditFromView()
    var P: TestPage "LMR Lines";
    begin
        Init();
        P.OpenView();
        P.First();
        P.Edit().Invoke();
        P.Description.Value();
    end;

    procedure NextAfterEditFromView()
    var P: TestPage "LMR Lines";
    begin
        Init();
        P.OpenView();
        P.First();
        P.Edit().Invoke();
        P.Next();
    end;

    procedure OpenViewAfterEditFromView()
    var P: TestPage "LMR Lines";
    begin
        Init();
        P.OpenView();
        P.First();
        P.Edit().Invoke();
        P.Close();
        P.OpenView();
    end;

    procedure EditOnPlainListOpenedForView()
    var P: TestPage "LMR Plain Lines";
    begin
        Init();
        P.OpenView();
        P.First();
        P.Edit().Invoke();
    end;

    procedure PlainListReopenAfterViewEdit()
    var P: TestPage "LMR Plain Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
        P.Edit().Invoke();
        P.Close();
        P.OpenEdit();
    end;

    procedure ViewOnAgrList(): Text
    var P: TestPage "LMR Agr Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        Log.Take();
        P.View().Invoke();
        exit(Log.Take() + Format(P.Editable()));
    end;

    procedure ViewOnExtList()
    var P: TestPage "LMR Ext Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
    end;

    procedure ViewOnSubscribedList()
    var P: TestPage "LMR Sub Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
    end;

    procedure ViewOnExprList()
    var P: TestPage "LMR Expr Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
    end;

    local procedure InitF()
    var Line: Record "LMR FLine";
    begin
        Init();
        Line.DeleteAll();
        Line."Line No." := 10000; Line.Description := 'a'; Line.Insert();
        Line."Line No." := 20000; Line.Description := 'b'; Line.Insert();
    end;

    procedure ViewOnPlainFlowFieldList()
    var P: TestPage "LMR Plain FLines";
    begin
        InitF();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
    end;

    procedure ViewOnProbeFlowFieldList()
    var P: TestPage "LMR FLines";
    begin
        InitF();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
    end;

    procedure ViewOnFilteredProbeList()
    var P: TestPage "LMR Lines";
    begin
        Init();
        P.OpenEdit();
        P.Filter.SetFilter("Line No.", '20000..');
        P.First();
        P.View().Invoke();
    end;

    local procedure ChangeStored()
    var Line: Record "LMR Line";
    begin
        Line.Get(10000);
        Line.Description := 'z';
        Line.Modify();
    end;

    procedure ViewOnPlainListAfterStoreChange()
    var P: TestPage "LMR Plain Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        ChangeStored();
        P.View().Invoke();
    end;

    procedure EditOnPlainListAfterStoreChange()
    var P: TestPage "LMR Plain Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
        ChangeStored();
        P.Edit().Invoke();
    end;

    procedure ViewEditOnPlainList(): Text
    var P: TestPage "LMR Plain Lines"; R: Text;
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.Next();
        P.View().Invoke();
        R := Format(P.Editable()) + '|' + P.Description.Value();
        P.Edit().Invoke();
        exit(R + '|' + Format(P.Editable()) + '|' + P.Description.Value());
    end;


    procedure ViewEditOnEditableList()
    var P: TestPage "LMR Editable Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
        P.Edit().Invoke();
        P.Close();
        P.OpenEdit();
    end;

    procedure ViewEditOnAssignList()
    var P: TestPage "LMR Assign Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
        P.Edit().Invoke();
        P.Close();
        P.OpenEdit();
    end;

    procedure ViewEditOnCountList()
    var P: TestPage "LMR Count Lines";
    begin
        Init();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
        P.Edit().Invoke();
        P.Close();
        P.OpenEdit();
    end;

    procedure GlobalViewEditClose()
    var H: Codeunit "LMR Holder";
    begin
        Init();
        H.ViewEditClose();
    end;

    procedure GlobalOpenEdit(): Text
    var H: Codeunit "LMR Holder";
    begin
        exit(H.OpenEdit());
    end;

    procedure GlobalTakeViewClose()
    var H: Codeunit "LMR Holder";
    begin
        Init();
        H.TakeViewClose();
    end;

    procedure GlobalBoundViewEnabled(): Text
    var H: Codeunit "LMR Holder";
    begin
        exit(H.BoundViewEnabled());
    end;
}
"#;

fn refused(routine: &str, part: &str) {
    match run_in(SRC, "LMR Test", routine) {
        Err(Signal::Unsupported(m)) => assert!(m.contains(part), "{routine}: {m}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn view_and_edit_read_the_row_and_the_page_opens_fresh_after_close() {
    // ViewEdit_Close_OpenEdit_Reads: view-1, edit-1 `[agr(10000) agcr(10000)]`, open-2 as a first open, Edit No, View Yes.
    assert_eq!(
        text(run_in(SRC, "LMR Test", "ViewEditCloseOpenEdit")),
        "view=[agr(10000) agcr(10000)] edit=[agr(10000) agcr(10000)] open=[close open agr(10000) agr(20000) agr(10000) agr(10000) \
         agcr(10000)] modes=Yes|No|Yes"
    );
}

#[test]
fn an_action_taken_on_an_earlier_open_raises_server_page_not_open_until_clear() {
    // ViewEdit_Close_OpenEdit_SwitchAgain `view-2 = Server page is not open. []`, `after view-2 = Yes|Yes|No|Yes`;
    // ViewEdit_Close_Clear_OpenEdit_SwitchAgain: after Clear View works and reads the row.
    assert_eq!(
        text(run_in(SRC, "LMR Test", "BoundActions")),
        "view=Server page is not open.[] modes=Yes|No|Yes cleared view=[agr(10000) agcr(10000)] modes=No|Yes|No"
    );
}

#[test]
fn edit_switches_the_list_opened_for_view() {
    // OpenView_Edit_Close_OpenEdit_Reads: `edit-1 = [agr(10000) agcr(10000)]`, `edit1|page = Yes`, Edit No, View Yes.
    assert_eq!(
        text(run_in(SRC, "LMR Test", "EditOnListOpenedForView")),
        "edit=[agr(10000) agcr(10000)] modes=Yes|No|Yes open=[close open agr(10000) agr(20000) agr(10000) agr(10000) \
         agcr(10000)] modes=Yes|No|Yes"
    );
}

#[test]
fn open_edit_on_the_open_page_after_view_and_edit_raises_already_open() {
    // ViewEdit_OpenEditWithoutClose: `The TestPage is already open.`, nothing logged, the page as it was.
    assert_eq!(
        text(run_in(SRC, "LMR Test", "OpenEditWhileOpen")),
        "The TestPage is already open.[] modes=Yes|No|Yes"
    );
}

#[test]
fn an_unmeasured_use_of_an_action_taken_on_an_earlier_open_is_refused() {
    refused("BoundVisible", "taken on an earlier open");
    refused("BoundOnCard", "taken on an earlier open");
    refused("BoundToViewModeInvoked", "taken on an earlier open");
}

#[test]
fn a_switch_of_the_probe_list_off_its_measured_row_is_refused() {
    refused("ViewOnSecondRow", "reads its row again");
}

#[test]
fn after_edit_on_the_list_opened_for_view_only_the_measured_calls_run() {
    refused("ValueAfterEditFromView", "opened for view to edit mode");
    refused(
        "NextAfterEditFromView",
        "TestPage.Next after the built-in Edit switched",
    );
    refused(
        "OpenViewAfterEditFromView",
        "TestPage.OpenView after the built-in Edit switched",
    );
}

#[test]
fn edit_on_another_list_opened_for_view_is_refused() {
    // Main switched nothing there; the probe shows BC switches the mode and reads the row, measured on its list only.
    refused(
        "EditOnPlainListOpenedForView",
        "Edit of a list without a card opened for view",
    );
}

#[test]
fn other_lists_keep_main_s_result() {
    // The reopen after View and Edit stays refused on a list the probe did not measure. View and Edit on the plain list of the
    // pins (no trigger, Normal fields, the row as stored) switch without a read, on any row, as on main
    // (testpage/R6977ListAfterView.al:125, :187; TestPageModeSwitchAction_Tests.al:199-200 after Next).
    refused(
        "PlainListReopenAfterViewEdit",
        "TestPage.OpenEdit after the built-in View and then Edit",
    );
    assert_eq!(text(run_in(SRC, "LMR Test", "ViewEditOnPlainList")), "No|b|Yes|b");
}

#[test]
fn a_switch_on_a_list_that_can_see_the_read_is_refused() {
    // Corpus PR 1467: BC reads the row again on View and on Edit. Main's no-read switch stays only where nothing sees it.
    for routine in [
        "ViewOnAgrList",
        "ViewOnExtList",
        "ViewOnSubscribedList",
        "ViewOnExprList",
        "ViewOnPlainFlowFieldList",
        "ViewOnPlainListAfterStoreChange",
        "EditOnPlainListAfterStoreChange",
    ] {
        refused(routine, "could see the row read again");
    }
}

/// Routine `routine` of "LMR Test" compiled for BC `bc`, whose platform fields every table gets.
fn run_on(bc: &str, routine: &str) -> Result<al_builtins::value::Value, Signal> {
    run_src_on(SRC, bc, "LMR Test", routine)
}

/// Routine `routine` of codeunit `object` in `src`, compiled for BC `bc`.
fn run_src_on(src: &str, bc: &str, object: &str, routine: &str) -> Result<al_builtins::value::Value, Signal> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let apps = vec![al_sema::workspace::AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == object).unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let mut s = al_vm::Session::new(al_vm::Engine::with_target(l.program, blocked, Some(target)));
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn on_bc_29_the_platform_s_user_name_flowfields_keep_the_measured_list() {
    // The probe ran on BC 27, 28 and 29 with the same result; on BC 29 every table has the platform's user-name
    // FlowFields, so they are part of the measured table.
    for bc in ["27.5", "29.0"] {
        assert_eq!(
            text(run_on(bc, "ViewEditCloseOpenEdit")),
            "view=[agr(10000) agcr(10000)] edit=[agr(10000) agcr(10000)] open=[close open agr(10000) agr(20000) agr(10000) \
             agr(10000) agcr(10000)] modes=Yes|No|Yes",
            "BC {bc}"
        );
    }
}

/// The probe list that also shows the platform's user-name FlowField of BC 29 (review of PR 8540, round 2: F1).
const SRC_29: &str = r#"
page 69909 "LMR User Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
                field(Opens; OpenCount) { ApplicationArea = All; Caption = 'Opens'; Editable = false; }
                field(CreatedByName; Rec.SystemCreatedByUserName) { ApplicationArea = All; }
            }
        }
    }
    var Log: Codeunit "LMR Log"; OpenCount: Integer;
    trigger OnOpenPage() begin OpenCount += 1; Log.Add('open'); end;
    trigger OnAfterGetRecord() begin Log.Add('agr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('new'); end;
    trigger OnClosePage() begin Log.Add('close'); end;
}
codeunit 69910 "LMR Test 29"
{
    procedure ViewOnUserList()
    var Line: Record "LMR Line"; P: TestPage "LMR User Lines";
    begin
        Line."Line No." := 10000; Line.Insert();
        P.OpenEdit();
        P.First();
        P.View().Invoke();
    end;
}
"#;

#[test]
fn the_probe_list_that_shows_a_platform_flowfield_is_not_the_measured_list() {
    // The probe's list shows two fields of Rec and a page variable; a shown FlowField can change on the read.
    match run_src_on(&format!("{SRC}{SRC_29}"), "29.0", "LMR Test 29", "ViewOnUserList") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("could see the row read again"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_probe_list_with_one_more_statement_in_a_trigger_is_not_the_measured_list() {
    // The gate compares the probe's page, table and log procedure line for line: one more statement in OnOpenPage,
    // OnAfterGetRecord or OnAfterGetCurrRecord is other code, and its triggers see the read.
    refused("ViewEditOnEditableList", "could see the row read again");
    refused("ViewEditOnAssignList", "could see the row read again");
    refused("ViewEditOnCountList", "could see the row read again");
}

#[test]
fn the_probe_list_with_a_filter_or_a_flowfield_is_not_the_measured_list() {
    // The probe's list has no filter and a table of Normal fields: a filtered view or a FlowField takes the other path,
    // which refuses because the page's triggers see the read.
    refused("ViewOnFilteredProbeList", "could see the row read again");
    refused("ViewOnProbeFlowFieldList", "could see the row read again");
}

#[test]
fn a_global_testpage_does_not_carry_the_switch_or_its_actions_into_the_next_test_method() {
    // The probe measures the earlier open and the reopen inside one test method.
    let refused_in = |b: &mut Built, routine: &str, part: &str| match b.run_in("LMR Test", routine) {
        Err(Signal::Unsupported(m)) => assert!(m.contains(part), "{routine}: {m}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    };
    let mut b = Built::from_source(SRC);
    b.session.start_test_method();
    b.run_in("LMR Test", "GlobalViewEditClose").unwrap();
    b.session.start_test_method();
    refused_in(
        &mut b,
        "GlobalOpenEdit",
        "TestPage.OpenEdit after the built-in View and then Edit",
    );

    let mut b = Built::from_source(SRC);
    b.session.start_test_method();
    b.run_in("LMR Test", "GlobalTakeViewClose").unwrap();
    b.session.start_test_method();
    refused_in(&mut b, "GlobalBoundViewEnabled", "taken on an earlier open");

    // Inside one test method the same calls run as measured.
    let mut b = Built::from_source(SRC);
    b.session.start_test_method();
    b.run_in("LMR Test", "GlobalViewEditClose").unwrap();
    assert_eq!(text(b.run_in("LMR Test", "GlobalOpenEdit")), "No|Yes");
    let mut b = Built::from_source(SRC);
    b.session.start_test_method();
    b.run_in("LMR Test", "GlobalTakeViewClose").unwrap();
    assert_eq!(text(b.run_in("LMR Test", "GlobalBoundViewEnabled")), "Yes");
}
