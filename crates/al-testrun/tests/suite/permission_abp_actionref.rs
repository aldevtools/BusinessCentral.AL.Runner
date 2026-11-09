//! ADR-062 probe P21: a page that lists AccessByPermission-guarded actions again with actionref (area(Promoted), a group, a page
//! extension). Values: corpus `permissions/lt-y94-page-open-abp/Y94AbpActionRef.al` and `permission-sets/Y90PermActionRef.al`
//! (BC 27, 28, 29): the page opens under a restricted session, the guarded action is off when the guard is not held (TestActionNotFound
//! on Invoke) and runs when it is held; the actionref changes nothing. A TestPage call on the actionref of a guarded action that is off
//! is not pinned and refuses.

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
table 51570 "AR Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 51571 "AR Guard" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }

page 51570 "AR Card"
{
    PageType = Card; SourceTable = "AR Row"; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { ApplicationArea = All; } } }
    actions
    {
        area(Processing)
        {
            action(PlainAct) { ApplicationArea = All; trigger OnAction() begin end; }
            action(GuardAct) { ApplicationArea = All; AccessByPermission = tabledata "AR Guard" = R; trigger OnAction() begin end; }
        }
        area(Promoted)
        {
            group(Category_Process)
            {
                actionref(PlainRef; PlainAct) { }
                actionref(GuardRef; GuardAct) { }
            }
        }
    }
}
pageextension 51570 "AR Ext" extends "AR Card"
{
    actions
    {
        addlast(Processing) { action(ExtAct) { ApplicationArea = All; AccessByPermission = tabledata "AR Guard" = R; trigger OnAction() begin end; } }
        addlast(Category_Process) { actionref(ExtRef; ExtAct) { } }
    }
}

permissionset 51570 "AR none" { Assignable = true; Permissions = tabledata "AR Row" = RIMD, page "AR Card" = X; }
permissionset 51572 "AR noread" { Assignable = true; Permissions = page "AR Card" = X; }
permissionset 51571 "AR ar" { Assignable = true; Permissions = tabledata "AR Row" = RIMD, page "AR Card" = X, tabledata "AR Guard" = R; }

codeunit 51572 "AR Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('want %1, got %2', Want, Got);
    end;

    [TryFunction]
    local procedure TryInvoke(var P: TestPage "AR Card"; What: Text)
    begin
        case What of
            'plain': P.PlainAct.Invoke();
            'guard': P.GuardAct.Invoke();
            'ext': P.ExtAct.Invoke();
        end;
    end;

    local procedure Inv(var P: TestPage "AR Card"; What: Text): Text
    begin
        if TryInvoke(P, What) then
            exit('ok');
        exit(GetLastErrorCode());
    end;

    local procedure Facts(SetName: Code[30]): Text
    var
        P: TestPage "AR Card";
        Res: Text;
    begin
        Mock.Start();
        Mock.ClearAssignments();
        Mock.SetExactPermissionSet(SetName);
        P.OpenView();
        Res := Inv(P, 'plain') + ' ' + Inv(P, 'guard') + ' ' + Inv(P, 'ext');
        P.Close();
        Mock.ClearAssignments();
        Mock.Stop();
        exit(Res);
    end;

    [Test]
    procedure WithoutTheGuardTheActionIsOff()
    begin
        Expect(Facts('AR none'), 'ok TestActionNotFound TestActionNotFound');
    end;

    [Test]
    procedure WithTheGuardTheActionRuns()
    begin
        Expect(Facts('AR ar'), 'ok ok ok');
    end;

    // Y94AbpActionRef.al Y94_Ref_GrantsB 'exact d365 basic': the sets do not grant Read on the empty source table, the card opens
    // and its controls answer. An action's Invoke reads the table again after the open: not pinned, refused.
    [Test]
    procedure AnEmptyUnreadableSourceTableOpens()
    var
        P: TestPage "AR Card";
        Row: Record "AR Row";
    begin
        Row.DeleteAll();
        Mock.Start();
        Mock.ClearAssignments();
        Mock.SetExactPermissionSet('AR noread');
        P.OpenView();
        Expect(Format(P.PlainAct.Visible()) + ' ' + Format(P.IdCtl.Visible()), 'Yes Yes');
        P.Close();
        Mock.ClearAssignments();
        Mock.Stop();
    end;

    // With rows the open is ADR-062 probe P18 (FormAbort:Permission on open, not pinned in this shape): refused.
    [Test]
    procedure AnUnreadableSourceTableWithRowsRefuses()
    var
        P: TestPage "AR Card";
        Row: Record "AR Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Mock.Start();
        Mock.ClearAssignments();
        Mock.SetExactPermissionSet('AR noread');
        P.OpenView();
    end;

    [Test]
    procedure TheRefOfAnOffActionRefuses()
    var
        P: TestPage "AR Card";
    begin
        Mock.Start();
        Mock.ClearAssignments();
        Mock.SetExactPermissionSet('AR none');
        P.OpenView();
        if P.GuardRef.Visible() then;
    end;
}
"#;

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_main(MAIN, &RunOptions::default()))
}

fn outcome(test: &str) -> Outcome {
    results()
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

#[test]
fn without_the_guard_the_promoted_action_is_off() {
    assert_eq!(outcome("WithoutTheGuardTheActionIsOff"), Outcome::Pass);
}
#[test]
fn with_the_guard_the_promoted_action_runs() {
    assert_eq!(outcome("WithTheGuardTheActionRuns"), Outcome::Pass);
}
#[test]
fn an_empty_unreadable_source_table_opens() {
    assert_eq!(outcome("AnEmptyUnreadableSourceTableOpens"), Outcome::Pass);
}
#[test]
fn an_unreadable_source_table_with_rows_refuses() {
    match outcome("AnUnreadableSourceTableWithRowsRefuses") {
        Outcome::NotRun(why) => assert!(why.contains("P16-P18"), "{why}"),
        o => panic!("{o:?}"),
    }
}
#[test]
fn a_testpage_call_on_the_actionref_of_an_off_action_refuses() {
    match outcome("TheRefOfAnOffActionRefuses") {
        Outcome::NotRun(why) => assert!(why.contains("actionref"), "{why}"),
        o => panic!("{o:?}"),
    }
}
