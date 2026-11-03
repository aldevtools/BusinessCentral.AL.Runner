//! ADR-062 stage 3: Execute denials inside table triggers, page triggers, actions and event subscribers. Values: corpus
//! `tests/al-language/permissions/lt-y84-perm-shapes/Y84PermExecute.al` (Y84_Exec_*, BC 29.0 Linux CI): a table that runs a trigger is
//! checked for Execute before the table data permission; code a trigger runs is checked; the code an event subscriber calls is not;
//! a codeunit call from OnOpenPage under a lowered session ends the open with FormAbort:Permission; an Execute denial inside
//! an action's OnAction comes out of Invoke with the code TestWrapped:Permission.

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
codeunit 51540 "XT Target" { trigger OnRun() begin end; procedure Ping(): Text begin exit('ping'); end; }
codeunit 51541 "XT Target2" { trigger OnRun() begin end; procedure Ping(): Text begin exit('ping'); end; }
codeunit 51542 "XT Target3" { trigger OnRun() begin end; procedure Ping(): Text begin exit('ping'); end; }
codeunit 51543 "XT Target4" { trigger OnRun() begin end; procedure Ping(): Text begin exit('ping'); end; }

table 51540 "XT Trig"
{
    fields { field(1; Id; Integer) { } field(2; Mode; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
    trigger OnInsert()
    var Target: Codeunit "XT Target";
    begin
        if Mode = 1 then Codeunit.Run(Codeunit::"XT Target") else if Target.Ping() = '' then;
    end;
}
table 51541 "XT Sub" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }

page 51540 "XT Open"
{
    PageType = Card; SourceTable = "XT Sub"; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { ApplicationArea = All; } } }
    trigger OnOpenPage()
    var Target: Codeunit "XT Target3";
    begin
        if Target.Ping() = '' then;
    end;
}
page 51541 "XT Action"
{
    PageType = Card; SourceTable = "XT Sub"; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { ApplicationArea = All; } } }
    actions { area(Processing) { action(RunIt) { ApplicationArea = All; trigger OnAction() begin Codeunit.Run(Codeunit::"XT Target4"); end; } } }
}
codeunit 51544 "XT Subscriber"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Table, Database::"XT Sub", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertSub(var Rec: Record "XT Sub"; RunTrigger: Boolean)
    var Target: Codeunit "XT Target2";
    begin
        if Target.Ping() = '' then;
    end;
}

permissionset 51540 "XT TD" { Assignable = true; Permissions = tabledata "XT Trig" = RIMD, tabledata "XT Sub" = RIMD; }
permissionset 51541 "XT Table X" { Assignable = true; Permissions = tabledata "XT Trig" = RIMD, table "XT Trig" = X; }
permissionset 51542 "XT Table And Target" { Assignable = true; Permissions = tabledata "XT Trig" = RIMD, table "XT Trig" = X, codeunit "XT Target" = X; }
permissionset 51543 "XT Page" { Assignable = true; Permissions = tabledata "XT Sub" = RIMD, page "XT Open" = X; }
permissionset 51544 "XT Page And Target" { Assignable = true; Permissions = tabledata "XT Sub" = RIMD, page "XT Open" = X, codeunit "XT Target3" = X; }
permissionset 51545 "XT Action Page" { Assignable = true; Permissions = tabledata "XT Sub" = RIMD, page "XT Action" = X; }

codeunit 51545 "XT Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";
        Binder: Codeunit "XT Subscriber";

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('want %1, got %2', Want, Got);
    end;

    local procedure InsertTrig(Mode: Integer)
    var Trig: Record "XT Trig";
    begin
        Trig.Id := 1;
        Trig.Mode := Mode;
        Trig.Insert(true);
    end;

    [Test]
    procedure TableWithTriggerNeedsExecuteBeforeData()
    var
        T: Text;
        C: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('XT TD');
        asserterror InsertTrig(1);
        T := GetLastErrorText();
        C := GetLastErrorCode();
        Mock.ClearAssignments();
        ClearLastError();
        Expect(T, 'Sorry, the current permissions prevented the action. (Table 51540 XT Trig Execute: main)');
        Expect(C, 'DB:ExecuteObjDenied');
        Mock.Stop();
    end;

    [Test]
    procedure TriggerCodeIsCheckedForExecute()
    var
        T: Text;
        C: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('XT Table X');
        asserterror InsertTrig(2);
        T := GetLastErrorText();
        C := GetLastErrorCode();
        Mock.ClearAssignments();
        ClearLastError();
        Expect(T, 'Sorry, the current permissions prevented the action. (CodeUnit 51540 XT Target Execute: main)');
        Expect(C, 'DB:ExecuteObjDenied');
        Mock.Stop();
    end;

    [Test]
    procedure TriggerCodeWithGrantsRuns()
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('XT Table And Target');
        InsertTrig(1);
        Mock.ClearAssignments();
        Mock.Stop();
    end;

    [Test]
    procedure SubscriberCallsAreNotChecked()
    var
        Sub: Record "XT Sub";
    begin
        Mock.Start();
        BindSubscription(Binder);
        Mock.SetExactPermissionSet('XT TD');
        Sub.Id := 2;
        Sub.Insert(true);
        Mock.ClearAssignments();
        UnbindSubscription(Binder);
        Mock.Stop();
    end;

    [Test]
    procedure OpenPageCodeunitCallEndsTheOpen()
    var
        P: TestPage "XT Open";
        T: Text;
        C: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('XT Page');
        asserterror P.OpenView();
        T := GetLastErrorText();
        C := GetLastErrorCode();
        Mock.ClearAssignments();
        ClearLastError();
        Expect(T, 'Page XT Open has to close.');
        Expect(C, 'FormAbort:Permission');
        Mock.Stop();
    end;

    [Test]
    procedure OpenPageCodeunitCallEndsTheOpenAlsoWithTheGrant()
    var
        P: TestPage "XT Open";
        T: Text;
        C: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('XT Page And Target');
        asserterror P.OpenView();
        T := GetLastErrorText();
        C := GetLastErrorCode();
        Mock.ClearAssignments();
        ClearLastError();
        Expect(T, 'Page XT Open has to close.');
        Expect(C, 'FormAbort:Permission');
        Mock.Stop();
    end;

    [Test]
    procedure ActionExecuteDenialIsWrapped()
    var
        P: TestPage "XT Action";
        T: Text;
        C: Text;
    begin
        Mock.Start();
        Mock.SetExactPermissionSet('XT Action Page');
        P.OpenView();
        asserterror P.RunIt.Invoke();
        T := GetLastErrorText();
        C := GetLastErrorCode();
        P.Close();
        Mock.ClearAssignments();
        ClearLastError();
        Expect(T, 'Sorry, the current permissions prevented the action. (CodeUnit 51543 XT Target4 Execute: main)');
        Expect(C, 'TestWrapped:Permission');
        Mock.Stop();
    end;
}
"#;

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_main(MAIN, &RunOptions::default()))
}

fn passes(test: &str) {
    let o = results()
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone();
    assert_eq!(o, Outcome::Pass, "{test}");
}

#[test]
fn a_table_with_a_trigger_needs_execute_before_the_data_permission() {
    passes("TableWithTriggerNeedsExecuteBeforeData");
}
#[test]
fn the_code_a_trigger_runs_is_checked_for_execute() {
    passes("TriggerCodeIsCheckedForExecute");
}
#[test]
fn trigger_code_runs_with_the_grants() {
    passes("TriggerCodeWithGrantsRuns");
}
#[test]
fn the_code_a_subscriber_calls_is_not_checked() {
    passes("SubscriberCallsAreNotChecked");
}
#[test]
fn a_codeunit_call_from_on_open_page_ends_the_open() {
    passes("OpenPageCodeunitCallEndsTheOpen");
}
#[test]
fn a_codeunit_call_from_on_open_page_ends_the_open_also_with_the_grant() {
    passes("OpenPageCodeunitCallEndsTheOpenAlsoWithTheGrant");
}
#[test]
fn an_execute_denial_in_an_action_is_wrapped() {
    passes("ActionExecuteDenialIsWrapped");
}
