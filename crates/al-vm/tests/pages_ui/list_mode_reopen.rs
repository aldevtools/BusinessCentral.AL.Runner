//! The built-in View and Edit of a list without a card, and a reopen of the same TestPage variable after them (al-runner issue
//! 8501; corpus PR 1467, open: testpage/lifecycle/ReopenAfterListModeSwitch.al, measured on a local BC of each supported major).
//! The pinned corpus does not hold the probe yet, so these tests keep its measured paths and the refusals around them.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69890 "LMR Line"
{
    fields { field(1; "Line No."; Integer) { } field(2; Description; Text[30]) { } }
    keys { key(PK; "Line No.") { Clustered = true; } }
}
codeunit 69891 "LMR Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin if Entries <> '' then Entries += ' '; Entries += Entry; end;
    procedure Take(): Text var R: Text; begin R := '[' + Entries + ']'; Entries := ''; exit(R); end;
}
page 69892 "LMR Lines"
{
    PageType = List; SourceTable = "LMR Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Description; Rec.Description) { } } } }
    var Log: Codeunit "LMR Log";
    trigger OnOpenPage() begin Log.Add('open'); end;
    trigger OnAfterGetRecord() begin Log.Add('agr(' + Format(Rec."Line No.") + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('agcr(' + Format(Rec."Line No.") + ')'); end;
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
    // The reopen after View and Edit stays refused on a list the probe did not measure, and View on a list with other code
    // reads no row, as on main.
    refused(
        "PlainListReopenAfterViewEdit",
        "TestPage.OpenEdit after the built-in View and then Edit",
    );
    assert_eq!(text(run_in(SRC, "LMR Test", "ViewOnAgrList")), "[]No");
}
