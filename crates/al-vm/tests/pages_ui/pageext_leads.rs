//! Page extension forms from al-runner issues 6149, 6150, 6158 and 6159: extension triggers that answer a Boolean
//! (OnInsertRecord, OnModifyRecord), OnAfterAction in `modify(action)`, actionrefs added to promoted groups and to
//! area(Promoted) a page does not declare, and a protected variable one extension declares and another one uses.
//! Each test names the corpus test it mirrors.

use al_testkit::{Built, text};

const LOG: &str = r#"
table 52500 "LD Row"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 52500 "LD Log"
{
    SingleInstance = true;
    var Entries: Text; InsertAnswer: Boolean; ModifyAnswer: Boolean;
    procedure Reset() begin Entries := ''; InsertAnswer := true; ModifyAnswer := true; end;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
    procedure SetInsertAnswer(V: Boolean) begin InsertAnswer := V; end;
    procedure GetInsertAnswer(): Boolean begin exit(InsertAnswer); end;
    procedure SetModifyAnswer(V: Boolean) begin ModifyAnswer := V; end;
    procedure GetModifyAnswer(): Boolean begin exit(ModifyAnswer); end;
}
"#;

// ---- OnAfterAction in modify(action) (pages/lt-pageext-action-after/PageExtensionActionOnAfterAction.al) ----

const AFTER_ACTION: &str = r#"
page 52501 "LD Host"
{
    PageType = Card;
    layout { area(Content) { field(Dummy; Dummy) { } } }
    actions { area(Processing) {
        action(Alpha) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('base Alpha'); end; }
        action(Beta) { }
        action(Gamma) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('base Gamma'); Error('gamma failure'); end; }
    } }
    var Dummy: Integer;
}
pageextension 52502 "LD Host One" extends "LD Host"
{
    actions {
        modify(Alpha) { trigger OnAfterAction() var L: Codeunit "LD Log"; begin L.Add('one Alpha'); end; }
        modify(Beta) { trigger OnAfterAction() var L: Codeunit "LD Log"; begin L.Add('one Beta'); end; }
        modify(Gamma) { trigger OnAfterAction() var L: Codeunit "LD Log"; begin L.Add('one Gamma'); end; }
    }
}
pageextension 52503 "LD Host Two" extends "LD Host"
{
    actions { modify(Alpha) { trigger OnAfterAction() var L: Codeunit "LD Log"; begin L.Add('two Alpha'); end; } }
}
codeunit 52504 C
{
    procedure F(): Text
    var H: TestPage "LD Host"; L: Codeunit "LD Log"; R: Text;
    begin
        L.Reset();
        H.OpenView();
        H.Alpha.Invoke();
        R := L.Take() + '|';
        H.Beta.Invoke();
        R += L.Take() + '|';
        asserterror H.Gamma.Invoke();
        R += GetLastErrorText() + '/' + GetLastErrorCode() + '/' + L.Take();
        H.Close();
        exit(R);
    end;
}
"#;

// P195_ActionWithOnAction_RunsBaseThenExtensions, P195_ActionWithoutBaseOnAction_RunsOnAfterAction,
// P195_BaseOnActionRaises_OnAfterActionSkipped.
#[test]
fn on_after_action_runs_after_the_action_then_each_extension_and_not_after_an_error() {
    let r = Built::from_source(&format!("{LOG}{AFTER_ACTION}")).run("F");
    assert_eq!(
        text(r),
        "base Alpha;one Alpha;two Alpha;|one Beta;|gamma failure/TestWrapped:Dialog/base Gamma;"
    );
}

// ---- OnNewRecord, OnInsertRecord, OnModifyRecord, OnAfterGetRecord of a page and two extensions (al-runner issue 6149; corpus
// pageext/lt-ue-pageext-leads/UeTriggerTests.al, corpus branch al-runtime-rs-staging-ue at 001970d3) ----

const TRIGGERS: &str = r#"
page 52510 "LD List"
{
    PageType = List;
    SourceTable = "LD Row";
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } } }
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "LD Log"; begin L.Add('base.New'); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean var L: Codeunit "LD Log"; begin L.Add('base.Ins ' + Format(Rec.Id)); exit(true); end;
    trigger OnModifyRecord(): Boolean var L: Codeunit "LD Log"; begin L.Add('base.Mod ' + Format(Rec.Id) + '=' + Format(Rec.Amount)); exit(true); end;
    trigger OnAfterGetRecord() var L: Codeunit "LD Log"; begin L.Add('base.AGR ' + Format(Rec.Id)); end;
}
pageextension 52511 "LD List Ext1" extends "LD List"
{
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "LD Log"; begin L.Add('ext1.New'); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean var L: Codeunit "LD Log"; begin L.Add('ext1.Ins ' + Format(Rec.Id)); exit(L.GetInsertAnswer()); end;
    trigger OnModifyRecord(): Boolean var L: Codeunit "LD Log"; begin L.Add('ext1.Mod ' + Format(Rec.Id) + '=' + Format(Rec.Amount)); exit(true); end;
    trigger OnAfterGetRecord() var L: Codeunit "LD Log"; begin L.Add('ext1.AGR ' + Format(Rec.Id)); end;
}
pageextension 52512 "LD List Ext2" extends "LD List"
{
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "LD Log"; begin L.Add('ext2.New'); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean var L: Codeunit "LD Log"; begin L.Add('ext2.Ins ' + Format(Rec.Id)); exit(true); end;
    trigger OnModifyRecord(): Boolean var L: Codeunit "LD Log"; begin L.Add('ext2.Mod ' + Format(Rec.Id) + '=' + Format(Rec.Amount)); exit(L.GetModifyAnswer()); end;
    trigger OnAfterGetRecord() var L: Codeunit "LD Log"; begin L.Add('ext2.AGR ' + Format(Rec.Id)); end;
}
codeunit 52513 C
{
    local procedure Seed()
    var R: Record "LD Row";
    begin
        R.DeleteAll(); R.Id := 3; R.Amount := 10; R.Insert();
    end;

    procedure Modify(Answer: Boolean): Text
    var P: TestPage "LD List"; L: Codeunit "LD Log"; R: Record "LD Row"; T: Text;
    begin
        Seed(); L.Reset(); L.SetModifyAnswer(Answer);
        P.OpenEdit();
        T := L.Take() + '|';
        P.GoToKey(3);
        P.Amount.SetValue(11);
        T += L.Take() + '|';
        P.Close();
        T += L.Take() + '|';
        R.Get(3);
        exit(T + Format(R.Amount));
    end;
    procedure ModifyTrue(): Text begin exit(Modify(true)); end;
    procedure ModifyFalse(): Text begin exit(Modify(false)); end;

    procedure NewRow(): Text
    var P: TestPage "LD List"; L: Codeunit "LD Log"; R: Record "LD Row"; T: Text;
    begin
        R.DeleteAll(); L.Reset();
        P.OpenEdit();
        T := L.Take() + '|';
        P.New();
        T += L.Take() + '|';
        P.Id.SetValue(5);
        T += L.Take() + '|';
        P.Amount.SetValue(7);
        T += L.Take() + '|';
        P.Close();
        T += L.Take() + '|';
        R.Get(5);
        exit(T + Format(R.Amount));
    end;

    procedure NewRowVetoed(): Text
    var L: Codeunit "LD Log"; R: Record "LD Row"; T: Text;
    begin
        R.DeleteAll(); L.Reset(); L.SetInsertAnswer(false);
        T := Format(TryNewRow()) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + L.Take() + '|';
        exit(T + Format(R.Get(5)));
    end;

    [TryFunction]
    local procedure TryNewRow()
    var P: TestPage "LD List";
    begin
        P.OpenEdit(); P.New(); P.Id.SetValue(5); P.Amount.SetValue(7); P.Close();
    end;
}
"#;

fn triggers(routine: &str) -> String {
    text(Built::from_source(&format!("{LOG}{TRIGGERS}")).run(routine))
}

// ModifyRow_BaseThenExtensionsInOrder: each trigger runs on the page first, then on the extensions in the order they apply.
#[test]
fn on_modify_record_runs_on_the_page_then_on_each_extension() {
    assert_eq!(
        triggers("ModifyTrue"),
        "base.AGR 3;ext1.AGR 3;ext2.AGR 3;||base.Mod 3=11;ext1.Mod 3=11;ext2.Mod 3=11;|11"
    );
}

// ModifyRow_SecondExtensionAnswersFalse: every trigger still runs; the false answer keeps the old value.
#[test]
fn a_false_on_modify_record_of_an_extension_keeps_the_old_value() {
    assert_eq!(
        triggers("ModifyFalse"),
        "base.AGR 3;ext1.AGR 3;ext2.AGR 3;||base.Mod 3=11;ext1.Mod 3=11;ext2.Mod 3=11;|10"
    );
}

// NewRow_BaseThenExtensionsInOrder.
#[test]
fn on_new_and_on_insert_record_run_on_the_page_then_on_each_extension() {
    assert_eq!(
        triggers("NewRow"),
        "base.New;ext1.New;ext2.New;base.New;ext1.New;ext2.New;|base.New;ext1.New;ext2.New;||\
         base.Ins 5;ext1.Ins 5;ext2.Ins 5;|base.Mod 5=7;ext1.Mod 5=7;ext2.Mod 5=7;|7"
    );
}

// NewRow_FirstExtensionAnswersFalse: the row is not stored, and the save of the later edit fails with DB:RecordNotFound.
#[test]
fn a_false_on_insert_record_of_an_extension_leaves_the_row_unstored() {
    assert_eq!(
        triggers("NewRowVetoed"),
        "No|The LD Row does not exist. Identification fields and values: Id='5'|DB:RecordNotFound|\
         base.New;ext1.New;ext2.New;base.New;ext1.New;ext2.New;base.New;ext1.New;ext2.New;\
         base.Ins 5;ext1.Ins 5;ext2.Ins 5;base.Mod 5=7;ext1.Mod 5=7;ext2.Mod 5=7;|No"
    );
}

// ---- actionrefs added to promoted groups and to area(Promoted) a page does not declare (al-runner issues 6150, 6158; corpus
// pageext/lt-ue-pageext-leads/UePromotedTests.al, corpus branch al-runtime-rs-staging-ue at 001970d3) ----

const PROMOTED: &str = r#"
page 52520 "LD Process"
{
    PageType = Card;
    SourceTable = "LD Row";
    layout { area(Content) { field(Id; Rec.Id) { } } }
    actions {
        area(Processing) { action(BaseAct) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('BaseAct'); end; } }
        area(Promoted) { group(Category_Process) { actionref(BaseAct_Promoted; BaseAct) { } } }
    }
}
pageextension 52521 "LD Process Ext" extends "LD Process"
{
    actions {
        addlast(Processing) {
            action(ExtNew) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('ExtNew'); end; }
            action(ExtReport) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('ExtReport'); end; }
            action(ExtProcess) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('ExtProcess'); end; }
        }
        addlast(Category_New) { actionref(ExtNew_Promoted; ExtNew) { } }
        addlast(Category_Report) { actionref(ExtReport_Promoted; ExtReport) { } }
        addlast(Category_Process) { actionref(ExtProcess_Promoted; ExtProcess) { } }
    }
}
page 52522 "LD No Area"
{
    PageType = Card;
    SourceTable = "LD Row";
    layout { area(Content) { field(Id; Rec.Id) { } } }
    actions { area(Processing) { action(BaseAct) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('BaseAct'); end; } } }
}
pageextension 52523 "LD No Area Ext" extends "LD No Area"
{
    actions {
        addlast(Processing) {
            action(ExtLast) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('ExtLast'); end; }
            action(ExtFirst) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('ExtFirst'); end; }
        }
        addlast(Promoted) { actionref(ExtLast_Promoted; ExtLast) { } }
        addfirst(Promoted) { actionref(ExtFirst_Promoted; ExtFirst) { } }
    }
}
page 52524 "LD No Actions"
{
    PageType = Card;
    SourceTable = "LD Row";
    layout { area(Content) { field(Id; Rec.Id) { } } }
}
pageextension 52525 "LD No Actions Ext" extends "LD No Actions"
{
    actions {
        addlast(Navigation) {
            action(NavAct) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('NavAct'); end; }
            action(NavAct2) { trigger OnAction() var L: Codeunit "LD Log"; begin L.Add('NavAct2'); end; }
        }
        addfirst(Promoted) { actionref(NavAct_Promoted; NavAct) { } }
        addlast(Promoted) { actionref(NavAct2_Promoted; NavAct2) { } }
        addlast(Category_New) { actionref(NavAct_New; NavAct) { } }
    }
}
codeunit 52526 C
{
    local procedure Seed() var R: Record "LD Row"; L: Codeunit "LD Log"; begin R.DeleteAll(); R.Id := 1; R.Insert(); L.Reset(); end;

    procedure Process(): Text
    var P: TestPage "LD Process"; L: Codeunit "LD Log"; T: Text;
    begin
        Seed();
        P.OpenEdit();
        T := Format(P.ExtNew_Promoted.Visible()) + Format(P.ExtReport_Promoted.Visible()) + Format(P.ExtProcess_Promoted.Visible())
            + Format(P.BaseAct_Promoted.Visible()) + '|';
        P.ExtNew_Promoted.Invoke(); P.ExtReport_Promoted.Invoke(); P.ExtProcess_Promoted.Invoke(); P.BaseAct_Promoted.Invoke();
        P.Close();
        exit(T + L.Take());
    end;

    procedure NoArea(): Text
    var P: TestPage "LD No Area"; L: Codeunit "LD Log"; T: Text;
    begin
        Seed();
        P.OpenEdit();
        T := Format(P.ExtLast_Promoted.Visible()) + Format(P.ExtFirst_Promoted.Visible()) + '|';
        P.ExtLast_Promoted.Invoke(); P.ExtFirst_Promoted.Invoke(); P.BaseAct.Invoke();
        P.Close();
        exit(T + L.Take());
    end;

    procedure NoActions(): Text
    var P: TestPage "LD No Actions"; L: Codeunit "LD Log"; T: Text;
    begin
        Seed();
        P.OpenView();
        T := Format(P.NavAct.Visible()) + Format(P.NavAct_Promoted.Visible()) + Format(P.NavAct2_Promoted.Visible())
            + Format(P.NavAct_New.Visible()) + '|';
        P.NavAct.Invoke(); P.NavAct_Promoted.Invoke(); P.NavAct2_Promoted.Invoke(); P.NavAct_New.Invoke();
        P.Close();
        exit(T + L.Take());
    end;
}
"#;

fn promoted(routine: &str) -> String {
    text(Built::from_source(&format!("{LOG}{PROMOTED}")).run(routine))
}

// CategoryNew_BaseDeclaresOnlyCategoryProcess.
#[test]
fn actionrefs_added_into_category_groups_the_page_does_not_declare_are_there_and_run() {
    assert_eq!(promoted("Process"), "YesYesYesYes|ExtNew;ExtReport;ExtProcess;BaseAct;");
}

// Promoted_BaseDeclaresNoPromotedArea.
#[test]
fn actionrefs_added_into_a_promoted_area_the_page_does_not_declare_are_there_and_run() {
    assert_eq!(promoted("NoArea"), "YesYes|ExtLast;ExtFirst;BaseAct;");
}

// Promoted_BaseDeclaresNoActions, CategoryNew_BaseDeclaresNoActions.
#[test]
fn actionrefs_added_to_a_page_without_actions_are_there_and_run() {
    assert_eq!(promoted("NoActions"), "YesYesYesYes|NavAct;NavAct;NavAct2;NavAct;");
}

// ---- a protected variable one page extension declares, used by another extension of the same page (al-runner issue 6159;
// corpus pageext/lt-ue-pageext-leads/UeProtectedVarTests.al, corpus branch al-runtime-rs-staging-ue at 001970d3) ----

const PV_BASE: &str = r#"
table 52530 "LD PV Row" { fields { field(1; Id; Integer) { } field(2; Amount; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 52530 "LD PV Card"
{
    PageType = Card;
    SourceTable = "LD PV Row";
    layout { area(Content) { field(Id; Rec.Id) { } } }
}
"#;

const PV_MID: &str = r#"
pageextension 52531 "LD PV Mid" extends "LD PV Card"
{
    layout {
        addlast(Content) { field(MidAmount; Rec.Amount) { Visible = ShowAmount; } }
        addlast(Content) { field(TraceField; Trace) { } }
    }
    trigger OnOpenPage()
    begin
        Trace += 'mid(' + Format(ShowAmount) + ')';
        ShowAmount := true;
    end;
    protected var
        ShowAmount: Boolean;
        Trace: Text;
}
"#;

const PV_TOP: &str = r#"
pageextension 52532 "LD PV Top" extends "LD PV Card"
{
    layout { addlast(Content) { field(TopAmount; Rec.Amount) { Visible = ShowAmount; } } }
    trigger OnOpenPage()
    begin
        Trace += 'top(' + Format(ShowAmount) + ')';
        ShowAmount := not ShowAmount;
    end;
}
codeunit 52533 C
{
    procedure F(): Text
    var R: Record "LD PV Row"; P: TestPage "LD PV Card"; T: Text;
    begin
        R.DeleteAll(); R.Id := 1; R.Amount := 5; R.Insert();
        P.OpenView();
        T := P.TraceField.Value() + '|' + Format(P.MidAmount.Visible()) + '|' + Format(P.TopAmount.Visible());
        P.Close();
        exit(T);
    end;
}
"#;

// ProtectedVar_OfAnotherExtension_IsSharedAndAssignable: the variable is shared, and the declaring extension's OnOpenPage runs first.
#[test]
fn a_protected_variable_of_another_extension_of_the_page_is_shared() {
    let r = Built::from_source(&format!("{PV_BASE}{PV_MID}{PV_TOP}")).run("F");
    assert_eq!(text(r), "mid(No)top(Yes)|No|No");
}

// The same in three apps (base page, the declaring extension, the using one that depends on both), as al-runner issue 6159
// reports it; observed on real BC by corpus writer UE (not in the corpus: assumption pageext-sibling-protected-var-cross-app).
#[test]
fn a_protected_variable_of_an_extension_in_a_dependency_app_is_shared() {
    use al_sema::workspace::AppSource;
    let app = |i: usize, deps: Vec<String>, src: &str| AppSource {
        id: format!("app{i}"),
        name: format!("app{i}"),
        dependencies: deps,
        files: vec![(format!("f{i}.al"), src.to_string())],
    };
    let apps = vec![
        app(0, vec![], PV_BASE),
        app(1, vec!["app0".into()], PV_MID),
        app(2, vec!["app0".into(), "app1".into()], PV_TOP),
    ];
    let (ws, l) = al_sema::lower::compile(apps, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let r = Built::new(l).run("F");
    assert_eq!(text(r), "mid(No)top(Yes)|No|No");
}
