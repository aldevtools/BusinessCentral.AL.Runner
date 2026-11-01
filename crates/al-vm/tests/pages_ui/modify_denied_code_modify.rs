//! A row that page code changed and stored itself (Rec.Modify) on a page whose ModifyAllowed is false (pages/draft.rs). Each test
//! names the corpus test of pages/lt-y61-page-probes/Y61ModifyDeniedSave.al that pins it.

const SRC: &str = r#"
table 69940 "MD Row"
{
    fields { field(1; Code; Code[10]) { } field(2; Name; Text[30]) { } field(3; Status; Integer) { } }
    keys { key(PK; Code) { } }
    procedure Fix() begin Status := 9; Modify(); end;
    procedure FixNoModify() begin Status := 9; end;
}
page 69941 "MD Temp List"
{
    PageType = List; SourceTable = "MD Row"; SourceTableTemporary = true; Editable = false; InsertAllowed = false;
    ModifyAllowed = false; DeleteAllowed = false; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(CodeCtl; Rec.Code) { }
                field(StatusCtl; Rec.Status) { }
                field(LinkCtl; LinkTxt) { trigger OnDrillDown() begin Rec.Fix(); end; }
                field(Link2Ctl; LinkTxt) { trigger OnDrillDown() begin Rec.FixNoModify(); end; }
            }
        }
    }
    var LinkTxt: Text;
    trigger OnOpenPage()
    begin
        Add('A'); Add('B'); Add('C');
        Rec.FindFirst();
    end;
    local procedure Add(C: Code[10]) begin Rec.Init(); Rec.Code := C; Rec.Name := 'n' + C; Rec.Status := 1; Rec.Insert(); end;
    trigger OnAfterGetRecord() begin LinkTxt := 'fix'; end;
}
page 69942 "MD List"
{
    PageType = List; SourceTable = "MD Row"; ModifyAllowed = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(CodeCtl; Rec.Code) { } field(StatusCtl; Rec.Status) { } } } }
    actions { area(Processing) { action(FixAct) { trigger OnAction() begin Rec.Fix(); end; } } }
}
codeunit 69943 C
{
    local procedure Seed()
    var R: Record "MD Row";
    begin
        R.Code := 'A'; R.Name := 'nA'; R.Status := 1; R.Insert();
        R.Code := 'B'; R.Name := 'nB'; R.Status := 1; R.Insert();
    end;

    local procedure Stored(C: Code[10]): Text
    var R: Record "MD Row";
    begin
        R.Get(C);
        exit(R.Name + '/' + Format(R.Status));
    end;

    procedure TempDrillDownModify(): Text
    var P: TestPage "MD Temp List"; R: Text;
    begin
        Seed();
        P.OpenView();
        P.First();
        P.LinkCtl.Drilldown();
        R := P.CodeCtl.Value() + '/' + P.StatusCtl.Value();
        P.First();
        R += '|' + P.CodeCtl.Value() + '/' + P.StatusCtl.Value();
        P.Next();
        R += '|' + P.CodeCtl.Value() + '/' + P.StatusCtl.Value();
        P.Close();
        exit(R + '|' + Stored('A'));
    end;

    procedure TempDrillDownAssign(): Text
    var P: TestPage "MD Temp List";
    begin
        Seed();
        P.OpenView();
        P.First();
        P.Link2Ctl.Drilldown();
        P.Next();
        P.First();
        exit(P.CodeCtl.Value() + '/' + P.StatusCtl.Value());
    end;

    procedure DbActionModify(): Text
    var P: TestPage "MD List"; R: Text;
    begin
        Seed();
        P.OpenEdit();
        P.First();
        P.FixAct.Invoke();
        R := P.CodeCtl.Value() + '/' + P.StatusCtl.Value() + '|' + Stored('A');
        P.Next();
        R += '|' + P.CodeCtl.Value() + '/' + P.StatusCtl.Value();
        P.Close();
        exit(R + '|' + Stored('A'));
    end;
}
"#;

/// Y61ModifyDeniedSave.al:164 G617a_Temp_NotEditable_DrillDownModify_ThenFirst: the drill-down's Rec.Modify stores the row in the
/// page's temporary table; First shows the stored value, Next moves on, Close raises nothing, the database row is untouched.
#[test]
fn a_drill_down_that_modifies_rec_on_a_temporary_read_only_list() {
    let mut built = al_testkit::Built::from_source(SRC);
    let out = al_testkit::text(built.run_in("C", "TempDrillDownModify"));
    assert_eq!(out, "A/9|A/9|B/1|nA/1");
}

/// Y61ModifyDeniedSave.al:203 G617a_Temp_NotEditable_DrillDownAssign_ThenFirst: a drill-down that only assigns leaves a changed row
/// whose save is not built here (the error at Close is recorded as the last error, the step does not raise): refused.
#[test]
fn a_drill_down_that_only_assigns_on_a_temporary_read_only_list_is_refused() {
    let mut built = al_testkit::Built::from_source(SRC);
    let r = built.run_in("C", "TempDrillDownAssign");
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

/// Y61ModifyDeniedSave.al:281 G617a_Db_Editable_ActionModify: an action whose code stores the row saves without an error.
#[test]
fn an_action_that_modifies_rec_on_a_list_that_denies_modify() {
    let mut built = al_testkit::Built::from_source(SRC);
    let out = al_testkit::text(built.run_in("C", "DbActionModify"));
    assert_eq!(out, "A/9|nA/9|B/1|nA/9");
}
